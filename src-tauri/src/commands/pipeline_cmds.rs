use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time::{sleep, Duration};

use crate::config::{load_config, IdleBehavior};
use crate::error::OtoError;
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::pipeline::orchestrator::position_overlay;
use crate::state::AppState;

#[tauri::command]
pub async fn ptt_down(state: State<'_, AppState>) -> Result<(), OtoError> {
    state.pipeline.ptt_down().await
}

#[tauri::command]
pub async fn ptt_up(state: State<'_, AppState>) -> Result<(), OtoError> {
    state.pipeline.ptt_up().await
}

/// Begin select-and-rewrite Command Mode. The delay gives a settings-window
/// caller time to refocus the app containing the selection.
#[tauri::command]
pub async fn start_command_mode(
    state: State<'_, AppState>,
    focus_delay_ms: Option<u64>,
) -> Result<(), OtoError> {
    state
        .pipeline
        .command_down(focus_delay_ms.unwrap_or(0))
        .await
}

#[tauri::command]
pub async fn cancel_dictation(state: State<'_, AppState>) -> Result<(), OtoError> {
    state.pipeline.cancel().await
}

/// Brief mock listening UI for Appearance settings. Shows the overlay even when
/// idle behavior is set to hide.
#[tauri::command]
pub async fn debug_preview_listening(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.pipeline.is_idle() {
        return Err("Finish or cancel the current dictation before previewing".into());
    }

    if let Some(window) = app.get_webview_window("overlay") {
        position_overlay(&window);
        let _ = window.set_always_on_top(true);
        let _ = window.set_skip_taskbar(true);
        let _ = window.show();
        let _ = window.unminimize();
    }

    let _ = app.emit(
        "pipeline://event",
        PipelineEvent::state(PipelineState::Listening, Some("Preview".into())),
    );
    // emit a few fake levels so the waveform animates
    for i in 0..12 {
        let level = 0.15 + ((i % 6) as f32) * 0.12;
        let _ = app.emit("pipeline://event", PipelineEvent::Level { level });
        sleep(Duration::from_millis(90)).await;
    }

    let _ = app.emit(
        "pipeline://event",
        PipelineEvent::state(PipelineState::Idle, None),
    );

    let keep = load_config()
        .map(|config| config.idle_behavior == IdleBehavior::Minimal)
        .unwrap_or(false);
    if !keep {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.hide();
        }
    }
    Ok(())
}
