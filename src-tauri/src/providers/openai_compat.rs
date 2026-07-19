use async_trait::async_trait;

use crate::config::{secrets, AppConfig};
use crate::error::{OtoError, OtoResult};

use super::presets;
use super::traits::{PolishContext, SpeechToText, TextPolisher};

pub struct OpenAiCompatClient {
    pub base_url: String,
    pub api_key: String,
    pub stt_model: String,
    pub polish_model: String,
    pub temperature: f32,
    client: reqwest::Client,
}

impl OpenAiCompatClient {
    pub fn new(
        base_url: String,
        api_key: String,
        stt_model: String,
        polish_model: String,
        temperature: f32,
    ) -> Self {
        Self {
            base_url,
            api_key,
            stt_model,
            polish_model,
            temperature,
            client: reqwest::Client::new(),
        }
    }
}

pub fn client_from_config(cfg: &AppConfig) -> OtoResult<OpenAiCompatClient> {
    let account = presets::preset_account(&cfg.provider_preset);
    let key = secrets::get_api_key(account)?
        .ok_or_else(|| OtoError::Message("API key not set".into()))?;
    let base = if cfg.base_url.trim().is_empty() {
        presets::base_url_for(&cfg.provider_preset).to_string()
    } else {
        cfg.base_url.clone()
    };
    Ok(OpenAiCompatClient::new(
        base,
        key,
        cfg.stt_model.clone(),
        cfg.polish_model.clone(),
        cfg.temperature,
    ))
}

#[async_trait]
impl SpeechToText for OpenAiCompatClient {
    async fn transcribe(&self, audio_wav: &[u8], language: Option<&str>) -> OtoResult<String> {
        let url = format!(
            "{}/audio/transcriptions",
            self.base_url.trim_end_matches('/')
        );
        let part = reqwest::multipart::Part::bytes(audio_wav.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| OtoError::Message(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.stt_model.clone())
            .text("response_format", "json".to_string());
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }
        let res = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;
        let body: serde_json::Value = res.json().await?;
        let text = body
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OtoError::Message("STT response missing text".into()))?
            .trim()
            .to_string();
        Ok(text)
    }
}

pub fn build_polish_system_prompt(ctx: &PolishContext) -> String {
    let mut p = String::from(
        "You are an expert editor. Convert the following raw speech transcription into clean, natural written text. \
         Remove filler words (um, uh, like…), fix grammar, add proper punctuation and capitalization. \
         Preserve the original meaning and tone. Output only the final text.",
    );
    if !ctx.dictionary.is_empty() {
        p.push_str(" Prefer these spellings/terms when relevant: ");
        p.push_str(&ctx.dictionary.join(", "));
        p.push('.');
    }
    if !ctx.tone_hint.trim().is_empty() {
        p.push_str(" Tone/style: ");
        p.push_str(ctx.tone_hint.trim());
        p.push('.');
    }
    p
}

#[async_trait]
impl TextPolisher for OpenAiCompatClient {
    async fn polish(&self, raw: &str, ctx: &PolishContext) -> OtoResult<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.polish_model,
            "temperature": self.temperature,
            "messages": [
                {"role": "system", "content": build_polish_system_prompt(ctx)},
                {"role": "user", "content": raw}
            ]
        });
        let res = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let v: serde_json::Value = res.json().await?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| OtoError::Message("polish response missing content".into()))?
            .trim()
            .to_string();
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polish_prompt_includes_dictionary() {
        let ctx = PolishContext {
            language: Some("en".into()),
            dictionary: vec!["Kubernetes".into(), "Oto".into()],
            tone_hint: String::new(),
        };
        let prompt = build_polish_system_prompt(&ctx);
        assert!(prompt.contains("Kubernetes"));
        assert!(prompt.contains("Oto"));
        assert!(prompt.contains("Prefer these spellings/terms"));
    }

    #[test]
    fn polish_prompt_includes_tone_hint() {
        let ctx = PolishContext {
            language: None,
            dictionary: vec![],
            tone_hint: "professional and concise".into(),
        };
        let prompt = build_polish_system_prompt(&ctx);
        assert!(prompt.contains("professional and concise"));
        assert!(prompt.contains("Tone/style:"));
    }

    #[test]
    fn polish_prompt_empty_extras() {
        let ctx = PolishContext {
            language: None,
            dictionary: vec![],
            tone_hint: "   ".into(),
        };
        let prompt = build_polish_system_prompt(&ctx);
        assert!(!prompt.contains("Prefer these spellings"));
        assert!(!prompt.contains("Tone/style:"));
        assert!(prompt.contains("expert editor"));
    }
}
