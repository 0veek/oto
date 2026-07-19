use crate::config::ProviderPreset;

pub fn base_url_for(preset: &ProviderPreset) -> &'static str {
    match preset {
        ProviderPreset::OpenAi => "https://api.openai.com/v1",
        ProviderPreset::Groq => "https://api.groq.com/openai/v1",
        ProviderPreset::OpenRouter => "https://openrouter.ai/api/v1",
        ProviderPreset::Custom => "",
    }
}

#[allow(dead_code)]
pub fn default_models(preset: &ProviderPreset) -> (&'static str, &'static str) {
    match preset {
        ProviderPreset::OpenAi => ("whisper-1", "gpt-4o-mini"),
        ProviderPreset::Groq => ("whisper-large-v3", "llama-3.1-8b-instant"),
        ProviderPreset::OpenRouter => ("openai/whisper-1", "openai/gpt-4o-mini"),
        ProviderPreset::Custom => ("whisper-1", "gpt-4o-mini"),
    }
}

pub fn preset_account(preset: &ProviderPreset) -> &'static str {
    match preset {
        ProviderPreset::OpenAi => "openai",
        ProviderPreset::Groq => "groq",
        ProviderPreset::OpenRouter => "openrouter",
        ProviderPreset::Custom => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groq_url() {
        assert!(base_url_for(&ProviderPreset::Groq).contains("groq.com"));
    }

    #[test]
    fn openai_url() {
        assert_eq!(
            base_url_for(&ProviderPreset::OpenAi),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn openrouter_url() {
        assert!(base_url_for(&ProviderPreset::OpenRouter).contains("openrouter.ai"));
    }

    #[test]
    fn custom_url_empty() {
        assert_eq!(base_url_for(&ProviderPreset::Custom), "");
    }

    #[test]
    fn groq_default_models() {
        let (stt, polish) = default_models(&ProviderPreset::Groq);
        assert_eq!(stt, "whisper-large-v3");
        assert_eq!(polish, "llama-3.1-8b-instant");
    }
}
