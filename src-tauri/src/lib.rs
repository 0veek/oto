mod audio;
mod commands;
mod config;
mod error;
mod hotkeys;
mod injection;
mod pipeline;
mod providers;
mod state;

use std::sync::Arc;

use config::load_config;
use pipeline::Pipeline;
use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};
use tokio::sync::Mutex;

fn setup_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let start = MenuItem::with_id(app, "start_listening", "Start Listening", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop_listening", "Stop Listening", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open_settings", "Open Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&start, &stop, &open, &quit])?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| tauri::Error::FailedToReceiveMessage)?
        .clone();

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "start_listening" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let pipeline = state.pipeline.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = pipeline.ptt_down().await {
                            eprintln!("ptt_down (tray): {e}");
                        }
                    });
                }
            }
            "stop_listening" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let pipeline = state.pipeline.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = pipeline.ptt_up().await {
                            eprintln!("ptt_up (tray): {e}");
                        }
                    });
                }
            }
            "open_settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::config_cmds::get_config,
            commands::config_cmds::set_config,
            commands::config_cmds::set_api_key,
            commands::config_cmds::api_key_present,
            commands::config_cmds::api_key_hint,
            commands::pipeline_cmds::ptt_down,
            commands::pipeline_cmds::ptt_up,
            commands::pipeline_cmds::cancel_dictation,
            commands::pipeline_cmds::debug_preview_listening,
            commands::test_cmds::test_transcription,
            commands::test_cmds::test_injection,
        ])
        .setup(|app| {
            let pipeline = Arc::new(Pipeline::new(app.handle().clone()));
            app.manage(AppState {
                cancel_flag: Arc::new(Mutex::new(false)),
                pipeline,
            });

            setup_tray(app.handle())?;

            match load_config() {
                Ok(cfg) => {
                    if let Err(e) = hotkeys::register_ptt(app.handle(), &cfg.hotkey) {
                        eprintln!("hotkey registration failed (tray fallback available): {e}");
                    }
                }
                Err(e) => {
                    eprintln!("could not load config for hotkey: {e}");
                    if let Err(e) = hotkeys::register_ptt(app.handle(), "Ctrl+Super+Space") {
                        eprintln!("default hotkey registration failed: {e}");
                    }
                }
            }

            if let Some(settings) = app.get_webview_window("settings") {
                let settings_for_event = settings.clone();
                settings.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_for_event.hide();
                    }
                });
                let _ = settings.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
