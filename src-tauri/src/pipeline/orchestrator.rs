//! PTT lifecycle: record → STT → optional polish → inject → events.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::time::{sleep, Duration};

use crate::audio::AudioRecorder;
use crate::config::{load_config, IdleBehavior};
use crate::error::{OtoError, OtoResult};
use crate::injection::{inject_text, InjectResult};
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::providers::{client_from_config, PolishContext, SpeechToText, TextPolisher};

struct Inner {
    recorder: Option<AudioRecorder>,
    listening: bool,
    /// Bumped on new sessions / cancel so delayed error timeouts don't clobber later work.
    epoch: u64,
    /// Last captured WAV bytes (for STT / test_transcription).
    last_wav: Option<Vec<u8>>,
}

pub struct Pipeline {
    app: AppHandle,
    inner: Mutex<Inner>,
}

impl Pipeline {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Mutex::new(Inner {
                recorder: None,
                listening: false,
                epoch: 0,
                last_wav: None,
            }),
        }
    }

    fn emit(&self, event: PipelineEvent) {
        let _ = self.app.emit("pipeline://event", event);
    }

    fn emit_state(&self, state: PipelineState) {
        self.emit(PipelineEvent::state(state, None));
    }

    /// Position overlay from config or bottom-center of the current monitor, then show.
    fn show_overlay(&self) {
        if let Some(w) = self.app.get_webview_window("overlay") {
            position_overlay(&w);
            let _ = w.show();
        }
    }

    /// Hide overlay unless appearance is set to minimal dormant pill.
    fn hide_overlay(&self) {
        let keep = load_config()
            .map(|c| c.idle_behavior == IdleBehavior::Minimal)
            .unwrap_or(false);
        if keep {
            return;
        }
        if let Some(w) = self.app.get_webview_window("overlay") {
            let _ = w.hide();
        }
    }

    fn bump_epoch(&self) -> OtoResult<u64> {
        let mut inner = self.lock_inner()?;
        inner.epoch = inner.epoch.wrapping_add(1);
        Ok(inner.epoch)
    }

    /// Error state stays ~4s (or until cancel/dismiss), then idle.
    async fn finish_error(&self, message: String) {
        let epoch = self.bump_epoch().unwrap_or(0);
        self.emit(PipelineEvent::Error {
            message: message.clone(),
        });
        // Ensure overlay is visible for the error flash.
        self.show_overlay();
        sleep(Duration::from_secs(4)).await;
        // Skip if user dismissed or a new session started.
        let still = self
            .lock_inner()
            .map(|g| g.epoch == epoch && !g.listening)
            .unwrap_or(false);
        if still {
            self.emit_state(PipelineState::Idle);
            self.hide_overlay();
        }
    }

    fn lock_inner(&self) -> OtoResult<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| OtoError::Message("pipeline lock poisoned".into()))
    }

    /// Clone of the last captured WAV, if any.
    pub fn last_wav(&self) -> OtoResult<Option<Vec<u8>>> {
        let inner = self.lock_inner()?;
        Ok(inner.last_wav.clone())
    }

    /// Run STT on the last recorded buffer (settings "Test transcription").
    pub async fn transcribe_last(&self) -> OtoResult<String> {
        let wav = self
            .last_wav()?
            .ok_or_else(|| OtoError::Message("No audio yet — dictate first".into()))?;
        let cfg = load_config()?;
        let client = client_from_config(&cfg)?;
        client.transcribe(&wav, cfg.language.as_deref()).await
    }

    pub async fn ptt_down(&self) -> OtoResult<()> {
        {
            let mut inner = self.lock_inner()?;
            if inner.listening {
                return Ok(());
            }
            // Invalidate any pending error timeout from a previous take.
            inner.epoch = inner.epoch.wrapping_add(1);
        }

        self.emit_state(PipelineState::Listening);
        self.show_overlay();

        match AudioRecorder::start(self.app.clone()) {
            Ok(recorder) => {
                let mut inner = self.lock_inner()?;
                // Another concurrent start may have won; keep the first session.
                if inner.listening {
                    return Ok(());
                }
                inner.recorder = Some(recorder);
                inner.listening = true;
                Ok(())
            }
            Err(e) => {
                self.finish_error(e.to_string()).await;
                Err(e)
            }
        }
    }

    pub async fn ptt_up(&self) -> OtoResult<()> {
        let recorder = {
            let mut inner = self.lock_inner()?;
            if !inner.listening {
                return Ok(());
            }
            inner.listening = false;
            inner.recorder.take()
        };

        let wav = if let Some(rec) = recorder {
            match rec.stop() {
                Ok((wav, _sample_rate)) => {
                    let mut inner = self.lock_inner()?;
                    inner.last_wav = Some(wav.clone());
                    wav
                }
                Err(e) => {
                    self.finish_error(e.to_string()).await;
                    return Err(e);
                }
            }
        } else {
            self.finish_error("No audio captured".into()).await;
            return Ok(());
        };

        self.emit_state(PipelineState::Processing);

        let cfg = match load_config() {
            Ok(c) => c,
            Err(e) => {
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        let client = match client_from_config(&cfg) {
            Ok(c) => c,
            Err(e) => {
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        self.emit(PipelineEvent::Phase {
            phase: "transcribing".into(),
        });

        let mut text = match client.transcribe(&wav, cfg.language.as_deref()).await {
            Ok(t) => t,
            Err(e) => {
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        if text.trim().is_empty() {
            self.finish_error("No speech detected".into()).await;
            return Ok(());
        }

        if cfg.polish_enabled {
            self.emit(PipelineEvent::Phase {
                phase: "polishing".into(),
            });
            let ctx = PolishContext {
                language: cfg.language.clone(),
                dictionary: cfg.dictionary.clone(),
                tone_hint: cfg.tone_hint.clone(),
            };
            match client.polish(&text, &ctx).await {
                Ok(polished) => text = polished,
                Err(e) => {
                    // Spec: fall back to raw + toast (do not abort pipeline).
                    self.emit(PipelineEvent::state(
                        PipelineState::Processing,
                        Some(format!("Polish failed, using raw: {e}")),
                    ));
                }
            }
        }

        self.emit(PipelineEvent::Phase {
            phase: "injecting".into(),
        });

        let done_detail = match inject_text(&text, &cfg.injection_mode).await {
            Ok(InjectResult::ClipboardOnly) => {
                // Text is on clipboard; user pastes manually.
                "Copied — press Ctrl+V".to_string()
            }
            Ok(InjectResult::Pasted | InjectResult::Atspi) => {
                // Surface the injected text (truncate long transcripts for overlay).
                if text.chars().count() > 120 {
                    let short: String = text.chars().take(117).collect();
                    format!("{short}…")
                } else {
                    text
                }
            }
            Err(e) => {
                self.finish_error(format!("Injection failed: {e}")).await;
                return Err(e);
            }
        };

        self.emit(PipelineEvent::state(
            PipelineState::Done,
            Some(done_detail),
        ));
        // Done flash ~700ms then idle.
        sleep(Duration::from_millis(700)).await;

        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }

    pub async fn cancel(&self) -> OtoResult<()> {
        {
            let mut inner = self.lock_inner()?;
            inner.recorder = None;
            inner.listening = false;
            // Invalidate pending error auto-dismiss.
            inner.epoch = inner.epoch.wrapping_add(1);
        }
        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }
}

/// Apply saved overlay position, or place bottom-center on the current monitor.
pub fn position_overlay(w: &tauri::WebviewWindow) {
    let cfg = load_config().ok();
    if let Some(cfg) = cfg.as_ref() {
        if let (Some(x), Some(y)) = (cfg.overlay_x, cfg.overlay_y) {
            let _ = w.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }

    // Best-effort bottom-center of the monitor the window is on (or primary).
    let monitor = w
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| w.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let screen = monitor.size();
        let origin = monitor.position();
        let win = w.outer_size().unwrap_or(tauri::PhysicalSize::new(320, 72));
        let margin_bottom = 96i32;
        let x = origin.x + (screen.width as i32 - win.width as i32) / 2;
        let y = origin.y + screen.height as i32 - win.height as i32 - margin_bottom;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}
