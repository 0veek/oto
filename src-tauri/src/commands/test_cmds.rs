use tauri::{AppHandle, Emitter, State};
use tokio::time::{sleep, Duration};

use crate::audio::AudioRecorder;
use crate::config::load_config;
use crate::error::OtoError;
use crate::injection::{inject_text, paste_tooling_summary, InjectResult};
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::state::AppState;

/// Transcribe the last PTT capture with the configured STT provider.
/// Returns an error if no audio has been recorded yet.
#[tauri::command]
pub async fn test_transcription(state: State<'_, AppState>) -> Result<String, OtoError> {
    state.pipeline.transcribe_last().await
}

/// Inject a fixed sample string using the configured injection mode.
/// After calling, focus a target app during the short delay below.
#[tauri::command]
pub async fn test_injection() -> Result<String, OtoError> {
    let cfg = load_config()?;
    let sample = "Oto injection test";
    // Clicking the settings button focuses Oto. Give the user a moment to
    // return focus to the target app before simulating Ctrl+V.
    sleep(Duration::from_millis(1200)).await;
    let result = inject_text(sample, &cfg.injection_mode).await?;
    let tooling = paste_tooling_summary();
    let msg = match result {
        InjectResult::Atspi => format!("Injected via AT-SPI ({tooling})"),
        InjectResult::DirectTyped => format!("Typed through a virtual keyboard ({tooling})"),
        InjectResult::Pasted => format!("Pasted via clipboard + simulation ({tooling})"),
        InjectResult::ClipboardOnly => {
            format!("Copied — press Ctrl+V ({tooling})")
        }
    };
    Ok(msg)
}

/// Capture ~2s of microphone audio and stream level events (no STT).
#[tauri::command]
pub async fn test_microphone(app: AppHandle) -> Result<(), OtoError> {
    let _ = app.emit(
        "pipeline://event",
        PipelineEvent::state(PipelineState::Listening, Some("Mic test".into())),
    );

    let recorder = match AudioRecorder::start(app.clone()) {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit(
                "pipeline://event",
                PipelineEvent::Error {
                    message: e.to_string(),
                },
            );
            return Err(e);
        }
    };

    sleep(Duration::from_secs(2)).await;

    // Drop stream (levels already streamed during capture).
    let _ = recorder.stop();

    let _ = app.emit(
        "pipeline://event",
        PipelineEvent::state(PipelineState::Idle, None),
    );
    Ok(())
}
