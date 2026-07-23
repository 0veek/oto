use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPreset {
    OpenAi,
    Groq,
    OpenRouter,
    Deepgram,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMode {
    Auto,
    DirectType,
    ClipboardPaste,
    ClipboardOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IdleBehavior {
    Hide,
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SttBackend {
    #[default]
    Cloud,
    LocalWhisper,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    System,
    #[default]
    Midnight,
    Light,
    HighContrast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub expansion: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StylePreset {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub stt_model: String,
    pub polish_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SyncConfig {
    pub enabled: bool,
    /// User-controlled HTTP(S) document URL. Credentials are stored in keyring.
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub provider_preset: ProviderPreset,
    pub base_url: String,
    pub stt_model: String,
    pub polish_model: String,
    pub polish_enabled: bool,
    pub temperature: f32,
    pub tone_hint: String,
    pub hotkey: String,
    pub language: Option<String>,
    pub dictionary: Vec<String>,
    pub injection_mode: InjectionMode,
    pub idle_behavior: IdleBehavior,
    pub overlay_x: Option<i32>,
    pub overlay_y: Option<i32>,
    pub stt_backend: SttBackend,
    pub local_whisper_model_path: String,
    pub vocabulary_boost: bool,
    pub snippets: Vec<Snippet>,
    pub styles: Vec<StylePreset>,
    pub active_style_id: Option<String>,
    pub history_enabled: bool,
    pub history_limit: usize,
    pub streaming_enabled: bool,
    pub theme: ThemePreset,
    pub reduce_motion: bool,
    pub font_scale: f32,
    pub custom_providers: Vec<ProviderProfile>,
    pub active_custom_provider_id: Option<String>,
    pub sync: SyncConfig,
    /// Launch Oto automatically when the desktop session starts (XDG autostart).
    pub autostart_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider_preset: ProviderPreset::Groq,
            base_url: "https://api.groq.com/openai/v1".into(),
            stt_model: "whisper-large-v3".into(),
            polish_model: "llama-3.1-8b-instant".into(),
            polish_enabled: true,
            temperature: 0.2,
            tone_hint: String::new(),
            hotkey: "Ctrl+Shift+Space".into(),
            language: None,
            dictionary: vec![],
            injection_mode: InjectionMode::Auto,
            idle_behavior: IdleBehavior::Hide,
            overlay_x: None,
            overlay_y: None,
            stt_backend: SttBackend::Cloud,
            local_whisper_model_path: String::new(),
            vocabulary_boost: true,
            snippets: vec![],
            styles: default_styles(),
            active_style_id: None,
            history_enabled: true,
            history_limit: 100,
            streaming_enabled: false,
            theme: ThemePreset::Midnight,
            reduce_motion: false,
            font_scale: 1.0,
            custom_providers: vec![],
            active_custom_provider_id: None,
            sync: SyncConfig::default(),
            autostart_enabled: false,
        }
    }
}

fn default_true() -> bool {
    true
}

pub fn default_styles() -> Vec<StylePreset> {
    vec![
        StylePreset {
            id: "professional".into(),
            name: "Professional".into(),
            prompt: "Professional, clear, and concise. Avoid filler and unnecessary flourish."
                .into(),
        },
        StylePreset {
            id: "casual".into(),
            name: "Casual".into(),
            prompt: "Natural and friendly while preserving the speaker's personality.".into(),
        },
        StylePreset {
            id: "email".into(),
            name: "Email".into(),
            prompt: "Polished email prose with sensible paragraphs and a courteous tone.".into(),
        },
        StylePreset {
            id: "code_comment".into(),
            name: "Code comment".into(),
            prompt: "Concise technical language suitable for code comments and documentation."
                .into(),
        },
    ]
}

impl AppConfig {
    pub fn active_style_prompt(&self) -> String {
        let preset = self
            .active_style_id
            .as_deref()
            .and_then(|id| self.styles.iter().find(|style| style.id == id))
            .map(|style| style.prompt.trim())
            .filter(|prompt| !prompt.is_empty());
        match (preset, self.tone_hint.trim()) {
            (Some(preset), "") => preset.to_string(),
            (Some(preset), custom) => format!("{preset} {custom}"),
            (None, custom) => custom.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_mvp_config_gets_phase_two_defaults() {
        let old = serde_json::json!({
            "provider_preset": "groq",
            "base_url": "https://api.groq.com/openai/v1",
            "stt_model": "whisper-large-v3",
            "polish_model": "llama-3.1-8b-instant",
            "polish_enabled": true,
            "temperature": 0.2,
            "tone_hint": "",
            "hotkey": "Ctrl+Shift+Space",
            "language": null,
            "dictionary": [],
            "injection_mode": "auto",
            "idle_behavior": "hide",
            "overlay_x": null,
            "overlay_y": null
        });
        let config: AppConfig = serde_json::from_value(old).unwrap();
        assert_eq!(config.stt_backend, SttBackend::Cloud);
        assert!(config.history_enabled);
        assert!(!config.styles.is_empty());
        assert!(!config.autostart_enabled);
    }

    #[test]
    fn active_style_and_custom_hint_are_combined() {
        let config = AppConfig {
            active_style_id: Some("professional".into()),
            tone_hint: "Use short paragraphs.".into(),
            ..AppConfig::default()
        };
        let prompt = config.active_style_prompt();
        assert!(prompt.contains("Professional"));
        assert!(prompt.contains("short paragraphs"));
    }
}
