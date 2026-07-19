use tauri::State;

use crate::config::load_config;
use crate::error::OtoError;
use crate::injection::{inject_text, paste_tooling_summary, InjectResult};
use crate::state::AppState;

/// Transcribe the last PTT capture with the configured STT provider.
/// Returns an error if no audio has been recorded yet.
#[tauri::command]
pub async fn test_transcription(state: State<'_, AppState>) -> Result<String, OtoError> {
    state.pipeline.transcribe_last().await
}

/// Inject a fixed sample string using the configured injection mode.
/// Focus a target app before calling (paste targets the focused window).
#[tauri::command]
pub async fn test_injection() -> Result<String, OtoError> {
    let cfg = load_config()?;
    let sample = "Oto injection test";
    let result = inject_text(sample, &cfg.injection_mode).await?;
    let tooling = paste_tooling_summary();
    let msg = match result {
        InjectResult::Atspi => format!("Injected via AT-SPI ({tooling})"),
        InjectResult::Pasted => format!("Pasted via clipboard + simulation ({tooling})"),
        InjectResult::ClipboardOnly => {
            format!("Copied — press Ctrl+V ({tooling})")
        }
    };
    Ok(msg)
}
