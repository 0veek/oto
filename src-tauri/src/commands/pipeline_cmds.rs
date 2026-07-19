use tauri::{AppHandle, Emitter, State};

use crate::error::OtoError;
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::state::AppState;

#[tauri::command]
pub async fn ptt_down(state: State<'_, AppState>) -> Result<(), OtoError> {
    *state.cancel_flag.lock().await = false;
    state.pipeline.ptt_down().await
}

#[tauri::command]
pub async fn ptt_up(state: State<'_, AppState>) -> Result<(), OtoError> {
    state.pipeline.ptt_up().await
}

#[tauri::command]
pub async fn cancel_dictation(state: State<'_, AppState>) -> Result<(), OtoError> {
    *state.cancel_flag.lock().await = true;
    state.pipeline.cancel().await
}

#[tauri::command]
pub async fn debug_preview_listening(app: AppHandle) -> Result<(), String> {
    let _ = app.emit(
        "pipeline://event",
        PipelineEvent::state(PipelineState::Listening, None),
    );
    // emit a few fake levels so the waveform animates
    for i in 0..10 {
        let level = 0.2 + (i as f32) * 0.05;
        let _ = app.emit("pipeline://event", PipelineEvent::Level { level });
    }
    Ok(())
}
