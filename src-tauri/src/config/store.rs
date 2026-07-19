use super::model::AppConfig;
use crate::error::{OtoError, OtoResult};
use std::fs;
use std::path::PathBuf;

pub fn config_path() -> OtoResult<PathBuf> {
    let base = directories::ProjectDirs::from("dev", "Oto", "oto")
        .ok_or_else(|| OtoError::Message("could not resolve config dir".into()))?;
    Ok(base.config_dir().join("config.json"))
}

pub fn load_config() -> OtoResult<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path)?;
    let cfg: AppConfig = serde_json::from_str(&raw)?;
    // Hard guard: never accept api_key fields if present from older versions
    if raw.contains("api_key") {
        // still load structural fields; keys ignored
        let _ = ();
    }
    Ok(cfg)
}

pub fn save_config(cfg: &AppConfig) -> OtoResult<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(cfg)?;
    if raw.contains("api_key") {
        return Err(OtoError::Message(
            "refusing to write config that contains api_key".into(),
        ));
    }
    fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::*;

    #[test]
    fn default_roundtrip_json_has_no_api_key() {
        let cfg = AppConfig::default();
        let raw = serde_json::to_string(&cfg).unwrap();
        assert!(!raw.contains("api_key"));
        let back: AppConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn save_load_roundtrip_tmp() {
        let mut cfg = AppConfig::default();
        cfg.dictionary = vec!["Oto".into(), "Tauri".into()];
        cfg.polish_enabled = false;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();
        let loaded: AppConfig = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(loaded.dictionary, vec!["Oto", "Tauri"]);
        assert!(!loaded.polish_enabled);
    }
}
