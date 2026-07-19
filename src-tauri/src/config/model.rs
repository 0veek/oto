use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPreset {
    OpenAi,
    Groq,
    OpenRouter,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMode {
    Auto,
    ClipboardPaste,
    ClipboardOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IdleBehavior {
    Hide,
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        }
    }
}
