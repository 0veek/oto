use crate::config::{load_config, save_config, secrets, AppConfig, ProviderPreset};
use crate::error::OtoError;
use crate::hotkeys;
use tauri::AppHandle;

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
pub fn set_config(app: AppHandle, cfg: AppConfig) -> Result<(), OtoError> {
    // Re-register before saving so invalid hotkeys are rejected without writing config.
    hotkeys::register_ptt(&app, &cfg.hotkey)?;
    save_config(&cfg)
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
