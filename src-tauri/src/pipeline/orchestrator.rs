//! PTT lifecycle: record → (STT placeholders) → events.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, Duration};

use crate::audio::AudioRecorder;
use crate::error::{OtoError, OtoResult};
use crate::pipeline::events::{PipelineEvent, PipelineState};

struct Inner {
    recorder: Option<AudioRecorder>,
    listening: bool,
    /// Last captured WAV bytes (for STT in Task 12).
    #[allow(dead_code)]
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

        if let Some(rec) = recorder {
            match rec.stop() {
                Ok((wav, _sample_rate)) => {
                    let mut inner = self.lock_inner()?;
                    inner.last_wav = Some(wav);
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
        }

        // STT placeholders (wired in Task 11+)
        self.emit_state(PipelineState::Processing);
        self.emit(PipelineEvent::Phase {
            phase: "transcribing".into(),
        });
        sleep(Duration::from_millis(100)).await;

        self.emit_state(PipelineState::Done);
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
