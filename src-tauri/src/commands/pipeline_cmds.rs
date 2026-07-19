use tauri::{AppHandle, Emitter};

use crate::pipeline::events::{PipelineEvent, PipelineState};

#[tauri::command]
pub async fn cancel_dictation() -> Result<(), String> {
    Ok(())
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
