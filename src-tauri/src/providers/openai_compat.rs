use async_trait::async_trait;

use crate::config::{secrets, AppConfig};
use crate::error::{OtoError, OtoResult};

use super::presets;
use super::traits::{PolishContext, SpeechToText, TextPolisher, TranscriptionContext};

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
    let profile = if cfg.provider_preset == crate::config::ProviderPreset::Custom {
        cfg.active_custom_provider_id
            .as_deref()
            .and_then(|id| cfg.custom_providers.iter().find(|profile| profile.id == id))
    } else {
        None
    };
    let account = profile
        .map(|profile| format!("custom:{}", profile.id))
        .unwrap_or_else(|| presets::preset_account(&cfg.provider_preset).to_string());
    let configured_base = profile
        .map(|profile| profile.base_url.as_str())
        .unwrap_or(&cfg.base_url);
    let base = if configured_base.trim().is_empty() {
        presets::base_url_for(&cfg.provider_preset).to_string()
    } else {
        configured_base.to_string()
    };
    let key = match secrets::get_api_key(&account)? {
        Some(key) => key,
        None if base.starts_with("http://127.0.0.1") || base.starts_with("http://localhost") => {
            String::new()
        }
        None => return Err(OtoError::Message("API key not set".into())),
    };
    Ok(OpenAiCompatClient::new(
        base,
        key,
        profile
            .map(|profile| profile.stt_model.clone())
            .unwrap_or_else(|| cfg.stt_model.clone()),
        profile
            .map(|profile| profile.polish_model.clone())
            .unwrap_or_else(|| cfg.polish_model.clone()),
        cfg.temperature,
    ))
}

/// Map UI-friendly language names / tags to ISO-639-1 for Whisper-compatible APIs.
/// Returns `None` for empty input; otherwise a best-effort code (pass-through if unknown).
pub fn normalize_stt_language(language: &str) -> Option<String> {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    // Already an ISO-639-1 code, optionally with region (en, en-US, en_us).
    if let Some(base) = lower
        .split(['-', '_'])
        .next()
        .filter(|b| b.len() == 2 && b.chars().all(|c| c.is_ascii_alphabetic()))
    {
        return Some(base.to_string());
    }
    let code = match lower.as_str() {
        "english" => "en",
        "spanish" | "español" | "espanol" => "es",
        "french" | "français" | "francais" => "fr",
        "german" | "deutsch" => "de",
        "portuguese" | "português" | "portugues" => "pt",
        "italian" | "italiano" => "it",
        "dutch" | "nederlands" => "nl",
        "russian" => "ru",
        "japanese" => "ja",
        "korean" => "ko",
        "chinese" | "mandarin" => "zh",
        "arabic" => "ar",
        "hindi" => "hi",
        "bengali" | "bangla" => "bn",
        "turkish" => "tr",
        "polish" => "pl",
        "swedish" => "sv",
        "norwegian" => "no",
        "danish" => "da",
        "finnish" => "fi",
        "greek" => "el",
        "hebrew" => "he",
        "indonesian" => "id",
        "vietnamese" => "vi",
        "thai" => "th",
        "ukrainian" => "uk",
        "czech" => "cs",
        "romanian" => "ro",
        "hungarian" => "hu",
        _ => return Some(trimmed.to_string()),
    };
    Some(code.to_string())
}

async fn http_error_message(res: reqwest::Response, what: &str) -> OtoError {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .or_else(|| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                status.to_string()
            } else {
                // Char-safe truncation — byte slicing panics on multi-byte UTF-8.
                let mut iter = trimmed.chars();
                let head: String = iter.by_ref().take(300).collect();
                if iter.next().is_some() {
                    format!("{head}…")
                } else {
                    head
                }
            }
        });
    OtoError::Message(format!("{what} failed ({status}): {detail}"))
}

#[async_trait]
impl SpeechToText for OpenAiCompatClient {
    async fn transcribe(&self, audio_wav: &[u8], ctx: &TranscriptionContext) -> OtoResult<String> {
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
        if let Some(lang) = ctx
            .language
            .as_deref()
            .and_then(normalize_stt_language)
        {
            form = form.text("language", lang);
        }
        if let Some(prompt) = ctx.vocabulary_prompt.as_deref() {
            form = form.text("prompt", prompt.to_string());
        }
        let res = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(http_error_message(res, "STT").await);
        }
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
    if let Some(language) = ctx
        .language
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        p.push_str(" Write in language: ");
        p.push_str(language);
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
            .await?;
        if !res.status().is_success() {
            return Err(http_error_message(res, "Polish").await);
        }
        let v: serde_json::Value = res.json().await?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| OtoError::Message("polish response missing content".into()))?
            .trim()
            .to_string();
        Ok(text)
    }

    async fn rewrite(
        &self,
        selected: &str,
        instruction: &str,
        ctx: &PolishContext,
    ) -> OtoResult<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut system = String::from(
            "Edit the selected text by following the spoken instruction. Preserve facts and formatting unless the instruction asks you to change them. Return only the replacement text, with no explanation.",
        );
        if !ctx.dictionary.is_empty() {
            system.push_str(" Prefer these spellings when relevant: ");
            system.push_str(&ctx.dictionary.join(", "));
            system.push('.');
        }
        if !ctx.tone_hint.trim().is_empty() {
            system.push_str(" Style guidance: ");
            system.push_str(ctx.tone_hint.trim());
            system.push('.');
        }
        let body = serde_json::json!({
            "model": self.polish_model,
            "temperature": self.temperature,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": format!("Spoken instruction:\n{instruction}\n\nSelected text:\n{selected}")}
            ]
        });
        let res = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(http_error_message(res, "Rewrite").await);
        }
        let value: serde_json::Value = res.json().await?;
        Ok(value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| OtoError::Message("rewrite response missing content".into()))?
            .trim()
            .to_string())
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

    #[test]
    fn language_names_normalize_to_iso() {
        assert_eq!(normalize_stt_language("English").as_deref(), Some("en"));
        assert_eq!(normalize_stt_language("en-US").as_deref(), Some("en"));
        assert_eq!(normalize_stt_language("ES").as_deref(), Some("es"));
        assert_eq!(normalize_stt_language("").as_deref(), None);
        assert_eq!(normalize_stt_language("   ").as_deref(), None);
        // Unknown tokens pass through so custom tags remain visible in API errors.
        assert_eq!(
            normalize_stt_language("not-a-language").as_deref(),
            Some("not-a-language")
        );
    }
}
