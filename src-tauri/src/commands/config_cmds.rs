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
    hotkeys::register_ptt(&app, &cfg.hotkey).await?;
    eprintln!("oto: config saved, hotkey active = {}", cfg.hotkey);
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

#[tauri::command]
pub fn set_api_key(preset: ProviderPreset, key: String) -> Result<(), OtoError> {
    secrets::set_api_key(preset_account(&preset), &key)
}

#[tauri::command]
pub fn api_key_present(preset: ProviderPreset) -> Result<bool, OtoError> {
    Ok(secrets::has_api_key(preset_account(&preset)))
}

#[tauri::command]
pub fn api_key_hint(preset: ProviderPreset) -> Result<Option<String>, OtoError> {
    Ok(secrets::get_api_key(preset_account(&preset))?.map(|k| {
        if k.len() <= 8 {
            "••••".into()
        } else {
            format!("{}…{}", &k[..4], &k[k.len() - 3..])
        }
    }))
}

#[tauri::command]
pub fn set_provider_api_key(account: String, key: String) -> Result<(), OtoError> {
    secrets::validate_account(&account)?;
    secrets::set_api_key(&account, &key)
}

#[tauri::command]
pub fn provider_api_key_present(account: String) -> Result<bool, OtoError> {
    secrets::validate_account(&account)?;
    Ok(secrets::has_api_key(&account))
}

/// Cargo package version shown in About.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Persist overlay window coordinates (physical pixels).
#[tauri::command]
pub fn set_overlay_position(x: i32, y: i32) -> Result<(), OtoError> {
    let mut cfg = load_config()?;
    cfg.overlay_x = Some(x);
    cfg.overlay_y = Some(y);
    save_config(&cfg)
}
