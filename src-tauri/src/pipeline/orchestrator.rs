//! PTT lifecycle: record → STT → optional polish → inject → events.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, Duration};

use crate::audio::AudioRecorder;
use crate::config::load_config;
use crate::error::{OtoError, OtoResult};
use crate::injection::{inject_text, InjectResult};
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::providers::{
    client_from_config, PolishContext, SpeechToText, TextPolisher,
};

struct Inner {
    recorder: Option<AudioRecorder>,
    listening: bool,
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

    fn show_overlay(&self) {
        if let Some(w) = self.app.get_webview_window("overlay") {
            let _ = w.show();
        }
    }

    fn hide_overlay(&self) {
        if let Some(w) = self.app.get_webview_window("overlay") {
            let _ = w.hide();
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
            let inner = self.lock_inner()?;
            if inner.listening {
                return Ok(());
            }
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
                self.emit(PipelineEvent::Error {
                    message: e.to_string(),
                });
                self.emit_state(PipelineState::Idle);
                self.hide_overlay();
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
                    self.emit(PipelineEvent::Error {
                        message: e.to_string(),
                    });
                    self.emit_state(PipelineState::Idle);
                    self.hide_overlay();
                    return Err(e);
                }
            }
        } else {
            self.emit(PipelineEvent::Error {
                message: "No audio captured".into(),
            });
            self.emit_state(PipelineState::Idle);
            self.hide_overlay();
            return Ok(());
        };

        self.emit_state(PipelineState::Processing);

        let cfg = match load_config() {
            Ok(c) => c,
            Err(e) => {
                self.emit(PipelineEvent::Error {
                    message: e.to_string(),
                });
                self.emit_state(PipelineState::Idle);
                self.hide_overlay();
                return Err(e);
            }
        };

        let client = match client_from_config(&cfg) {
            Ok(c) => c,
            Err(e) => {
                self.emit(PipelineEvent::Error {
                    message: e.to_string(),
                });
                self.emit_state(PipelineState::Idle);
                self.hide_overlay();
                return Err(e);
            }
        };

        self.emit(PipelineEvent::Phase {
            phase: "transcribing".into(),
        });

        let mut text = match client.transcribe(&wav, cfg.language.as_deref()).await {
            Ok(t) => t,
            Err(e) => {
                self.emit(PipelineEvent::Error {
                    message: e.to_string(),
                });
                self.emit_state(PipelineState::Idle);
                self.hide_overlay();
                return Err(e);
            }
        };

        if text.trim().is_empty() {
            self.emit(PipelineEvent::Error {
                message: "No speech detected".into(),
            });
            self.emit_state(PipelineState::Idle);
            self.hide_overlay();
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
                self.emit(PipelineEvent::Error {
                    message: format!("Injection failed: {e}"),
                });
                self.emit_state(PipelineState::Idle);
                self.hide_overlay();
                return Err(e);
            }
        };

        self.emit(PipelineEvent::state(
            PipelineState::Done,
            Some(done_detail),
        ));
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
        }
        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }
}
