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

/// Exclusive pipeline phase — only one session may run at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Listening,
    Processing,
}

struct Inner {
    recorder: Option<AudioRecorder>,
    phase: Phase,
    /// Bumped on new sessions / cancel so delayed work doesn't clobber later sessions.
    epoch: u64,
    /// Set on cancel; checked after awaits during processing.
    cancel_flag: bool,
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
                phase: Phase::Idle,
                epoch: 0,
                cancel_flag: false,
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
            let _ = w.set_always_on_top(true);
            let _ = w.set_skip_taskbar(true);
            // Do not steal keyboard focus from the app the user is dictating into.
            if let Err(e) = w.show() {
                eprintln!("oto: overlay.show failed: {e}");
            } else {
                eprintln!("oto: overlay shown");
            }
            let _ = w.unminimize();
        } else {
            eprintln!("oto: overlay window missing");
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

    /// True when phase is Idle (safe for appearance changes / new PTT).
    pub fn is_idle(&self) -> bool {
        self.lock_inner()
            .map(|g| g.phase == Phase::Idle)
            .unwrap_or(true)
    }

    /// True while actively capturing audio (between ptt_down and ptt_up).
    pub fn is_listening(&self) -> bool {
        self.lock_inner()
            .map(|g| g.phase == Phase::Listening)
            .unwrap_or(false)
    }

    /// True if this processing session was cancelled or superseded.
    fn session_aborted(&self, session_epoch: u64) -> bool {
        self.lock_inner()
            .map(|g| g.epoch != session_epoch || g.cancel_flag)
            .unwrap_or(true)
    }

    /// Mark phase Idle (best-effort) without bumping epoch.
    fn set_phase_idle(&self) {
        if let Ok(mut inner) = self.lock_inner() {
            inner.phase = Phase::Idle;
            inner.recorder = None;
        }
    }

    /// Error state stays ~4s (or until cancel/dismiss), then idle.
    async fn finish_error(&self, message: String) {
        // Allow a new PTT immediately; error flash is non-exclusive.
        self.set_phase_idle();
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
            .map(|g| g.epoch == epoch && g.phase == Phase::Idle)
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
            // Only start a new listen from Idle — reject if already Listening or Processing.
            if inner.phase != Phase::Idle {
                return Ok(());
            }
            // Invalidate any pending error timeout / leftover cancel from a previous take.
            inner.epoch = inner.epoch.wrapping_add(1);
            inner.cancel_flag = false;
            inner.phase = Phase::Listening;
        }

        // Show first so a cold webview can load, then emit (and re-emit shortly
        // so late event listeners still enter Listening UI).
        self.show_overlay();
        self.emit_state(PipelineState::Listening);
        {
            let app = self.app.clone();
            tauri::async_runtime::spawn(async move {
                sleep(Duration::from_millis(80)).await;
                let _ = app.emit(
                    "pipeline://event",
                    PipelineEvent::state(PipelineState::Listening, None),
                );
            });
        }

        match AudioRecorder::start(self.app.clone()) {
            Ok(recorder) => {
                let mut inner = self.lock_inner()?;
                // Cancel or supersede may have happened while starting the device.
                if inner.phase != Phase::Listening || inner.cancel_flag {
                    return Ok(());
                }
                inner.recorder = Some(recorder);
                Ok(())
            }
            Err(e) => {
                self.set_phase_idle();
                self.finish_error(e.to_string()).await;
                Err(e)
            }
        }
    }

    pub async fn ptt_up(&self) -> OtoResult<()> {
        let (recorder, session_epoch) = {
            let mut inner = self.lock_inner()?;
            if inner.phase != Phase::Listening {
                return Ok(());
            }
            // Capture epoch for this session; further work aborts if cancel bumps it.
            let session_epoch = inner.epoch;
            inner.phase = Phase::Processing;
            (inner.recorder.take(), session_epoch)
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

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

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
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

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
                Ok(polished) => {
                    if self.session_aborted(session_epoch) {
                        self.set_phase_idle();
                        return Ok(());
                    }
                    text = polished;
                }
                Err(e) => {
                    if self.session_aborted(session_epoch) {
                        self.set_phase_idle();
                        return Ok(());
                    }
                    // Spec: fall back to raw + toast (do not abort pipeline).
                    self.emit(PipelineEvent::state(
                        PipelineState::Processing,
                        Some(format!("Polish failed, using raw: {e}")),
                    ));
                }
            }
        }

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        self.emit(PipelineEvent::Phase {
            phase: "injecting".into(),
        });

        let done_detail = match inject_text(&text, &cfg.injection_mode).await {
            Ok(InjectResult::ClipboardOnly) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                // Text is on clipboard; user pastes manually.
                "Copied — press Ctrl+V".to_string()
            }
            Ok(InjectResult::Pasted | InjectResult::Atspi) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                // Surface the injected text (truncate long transcripts for overlay).
                if text.chars().count() > 120 {
                    let short: String = text.chars().take(117).collect();
                    format!("{short}…")
                } else {
                    text
                }
            }
            Err(e) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                self.finish_error(format!("Injection failed: {e}")).await;
                return Err(e);
            }
        };

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        self.emit(PipelineEvent::state(
            PipelineState::Done,
            Some(done_detail),
        ));
        // Done flash ~700ms then idle.
        sleep(Duration::from_millis(700)).await;

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        {
            let mut inner = self.lock_inner()?;
            inner.phase = Phase::Idle;
        }
        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }

    pub async fn cancel(&self) -> OtoResult<()> {
        {
            let mut inner = self.lock_inner()?;
            inner.recorder = None;
            inner.phase = Phase::Idle;
            inner.cancel_flag = true;
            // Invalidate pending error auto-dismiss and in-flight processing.
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
    // Treat (0, 0) as unset — Moved events often fire with that before layout.
    if let Some(cfg) = cfg.as_ref() {
        if let (Some(x), Some(y)) = (cfg.overlay_x, cfg.overlay_y) {
            if !(x == 0 && y == 0) {
                let _ = w.set_position(PhysicalPosition::new(x, y));
                return;
            }
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
