use crate::autostart;
use crate::config::{load_config, save_config, secrets, AppConfig, IdleBehavior, ProviderPreset};
use crate::error::OtoError;
use crate::hotkeys;
use crate::pipeline::orchestrator::position_overlay;
use crate::state::AppState;
use tauri::{AppHandle, Manager};

fn preset_account(p: &ProviderPreset) -> &'static str {
    match p {
        ProviderPreset::OpenAi => "openai",
        ProviderPreset::Groq => "groq",
        ProviderPreset::OpenRouter => "openrouter",
        ProviderPreset::Custom => "custom",
    }
}

#[tauri::command]
pub fn get_config() -> Result<AppConfig, OtoError> {
    load_config()
}

#[tauri::command]
pub async fn set_config(app: AppHandle, mut cfg: AppConfig) -> Result<(), OtoError> {
    // Normalize + re-register before saving so invalid hotkeys are rejected without writing.
    cfg.hotkey = hotkeys::normalize_hotkey(&cfg.hotkey);
    cfg.history_limit = cfg.history_limit.clamp(1, 1000);
    cfg.font_scale = cfg.font_scale.clamp(0.85, 1.25);
    cfg.temperature = cfg.temperature.clamp(0.0, 1.0);
    // Drop a dangling active profile pointer so the runtime never reads missing models.
    if let Some(active_id) = cfg.active_custom_provider_id.as_deref() {
        if !cfg
            .custom_providers
            .iter()
            .any(|profile| profile.id == active_id)
        {
            cfg.active_custom_provider_id = None;
        }
    }
    if let Some(style_id) = cfg.active_style_id.as_deref() {
        if !cfg.styles.iter().any(|style| style.id == style_id) {
            cfg.active_style_id = None;
        }
    }
    hotkeys::register_ptt(&app, &cfg.hotkey).await?;
    // Install/remove XDG autostart before writing config so a filesystem failure
    // does not leave the on-disk flag out of sync with the desktop entry.
    autostart::apply(cfg.autostart_enabled)?;
    eprintln!(
        "oto: config saved, hotkey active = {}, autostart = {}",
        cfg.hotkey, cfg.autostart_enabled
    );
    save_config(&cfg)?;
    // Apply idle appearance immediately when settings change.
    if let Some(overlay) = app.get_webview_window("overlay") {
        if cfg.idle_behavior == IdleBehavior::Minimal {
            position_overlay(&overlay);
            let _ = overlay.show();
        } else if app
            .try_state::<AppState>()
            .map(|s| s.pipeline.is_idle())
            .unwrap_or(true)
        {
            // Hide while idle when switching to Hide; leave visible mid-dictation.
            let _ = overlay.hide();
        }
    }
    Ok(())
}

/// Secret Service / keyring D-Bus calls can block while the daemon is locked.
/// Run them off the main IPC thread so the settings UI stays responsive.
#[tauri::command]
pub async fn set_api_key(preset: ProviderPreset, key: String) -> Result<(), OtoError> {
    let account = preset_account(&preset).to_string();
    tauri::async_runtime::spawn_blocking(move || secrets::set_api_key(&account, &key))
        .await
        .map_err(|error| OtoError::Message(format!("keyring task failed: {error}")))?
}

#[tauri::command]
pub async fn api_key_present(preset: ProviderPreset) -> Result<bool, OtoError> {
    let account = preset_account(&preset).to_string();
    tauri::async_runtime::spawn_blocking(move || Ok(secrets::has_api_key(&account)))
        .await
        .map_err(|error| OtoError::Message(format!("keyring task failed: {error}")))?
}

#[tauri::command]
pub async fn api_key_hint(preset: ProviderPreset) -> Result<Option<String>, OtoError> {
    let account = preset_account(&preset).to_string();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(secrets::get_api_key(&account)?.map(|k| {
            if k.len() <= 8 {
                "••••".into()
            } else {
                format!("{}…{}", &k[..4], &k[k.len() - 3..])
            }
        }))
    })
    .await
    .map_err(|error| OtoError::Message(format!("keyring task failed: {error}")))?
}

#[tauri::command]
pub async fn set_provider_api_key(account: String, key: String) -> Result<(), OtoError> {
    tauri::async_runtime::spawn_blocking(move || {
        secrets::validate_account(&account)?;
        secrets::set_api_key(&account, &key)
    })
    .await
    .map_err(|error| OtoError::Message(format!("keyring task failed: {error}")))?
}

#[tauri::command]
pub async fn provider_api_key_present(account: String) -> Result<bool, OtoError> {
    tauri::async_runtime::spawn_blocking(move || {
        secrets::validate_account(&account)?;
        Ok(secrets::has_api_key(&account))
    })
    .await
    .map_err(|error| OtoError::Message(format!("keyring task failed: {error}")))?
}

/// Cargo package version shown in About.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Session / portal capability report for the Hotkeys settings panel.
#[tauri::command]
pub async fn get_hotkey_desktop_status() -> hotkeys::HotkeyDesktopStatus {
    hotkeys::hotkey_desktop_status().await
}

/// Persist overlay window coordinates (physical pixels).
#[tauri::command]
pub fn set_overlay_position(x: i32, y: i32) -> Result<(), OtoError> {
    let mut cfg = load_config()?;
    cfg.overlay_x = Some(x);
    cfg.overlay_y = Some(y);
    save_config(&cfg)
}
