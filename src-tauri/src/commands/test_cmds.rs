use tauri::State;

use crate::error::OtoError;
use crate::state::AppState;

/// Transcribe the last PTT capture with the configured STT provider.
/// Returns an error if no audio has been recorded yet.
#[tauri::command]
pub async fn test_transcription(state: State<'_, AppState>) -> Result<String, OtoError> {
    state.pipeline.transcribe_last().await
}
