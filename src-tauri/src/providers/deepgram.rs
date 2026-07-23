//! Deepgram pre-recorded speech-to-text client.
//!
//! API docs: https://developers.deepgram.com/docs/pre-recorded-audio
//! Auth uses `Authorization: Token <API_KEY>` (not Bearer).
//! Default model: `nova-3` with `smart_format=true`.

use async_trait::async_trait;

use crate::config::{secrets, AppConfig};
use crate::error::{OtoError, OtoResult};

use super::presets;
use super::traits::{SpeechToText, TranscriptionContext};

const MAX_KEYTERMS: usize = 100;

pub struct DeepgramClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    client: reqwest::Client,
}

impl DeepgramClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

pub fn client_from_config(cfg: &AppConfig) -> OtoResult<DeepgramClient> {
    let account = presets::preset_account(&crate::config::ProviderPreset::Deepgram);
    let key = secrets::get_api_key(account)?
        .ok_or_else(|| OtoError::Message("API key not set".into()))?;
    let configured = cfg.base_url.trim();
    let base = if configured.is_empty() {
        presets::base_url_for(&crate::config::ProviderPreset::Deepgram).to_string()
    } else {
        configured.to_string()
    };
    let model = if cfg.stt_model.trim().is_empty() {
        "nova-3".to_string()
    } else {
        cfg.stt_model.clone()
    };
    Ok(DeepgramClient::new(base, key, model))
}

fn listen_url(base_url: &str, model: &str, ctx: &TranscriptionContext) -> OtoResult<String> {
    let root = base_url.trim_end_matches('/');
    // Accept either the API root (https://api.deepgram.com) or a path already ending in /v1.
    let endpoint = if root.ends_with("/v1") {
        format!("{root}/listen")
    } else {
        format!("{root}/v1/listen")
    };

    let mut url = reqwest::Url::parse(&endpoint)
        .map_err(|e| OtoError::Message(format!("invalid Deepgram base URL: {e}")))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("model", model);
        pairs.append_pair("smart_format", "true");
        if let Some(lang) = ctx.language.as_deref().filter(|l| !l.trim().is_empty()) {
            pairs.append_pair("language", lang);
        } else {
            // Without an explicit language, let Deepgram detect it rather than forcing English.
            pairs.append_pair("detect_language", "true");
        }
        // Nova-3 uses keyterm prompting (not the legacy keywords intensifier syntax).
        for term in ctx.keyterms.iter().take(MAX_KEYTERMS) {
            let trimmed = term.trim();
            if !trimmed.is_empty() {
                pairs.append_pair("keyterm", trimmed);
            }
        }
    }
    Ok(url.to_string())
}

async fn http_error_message(res: reqwest::Response) -> OtoError {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("err_msg")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    v.get("error")
                        .and_then(|e| e.as_str().map(|s| s.to_string()).or_else(|| {
                            e.get("message")
                                .and_then(|m| m.as_str())
                                .map(|s| s.to_string())
                        }))
                })
                .or_else(|| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                status.to_string()
            } else if trimmed.len() > 300 {
                format!("{}…", &trimmed[..300])
            } else {
                trimmed.to_string()
            }
        });
    OtoError::Message(format!("STT failed ({status}): {detail}"))
}

/// Extract transcript from Deepgram listen response JSON.
fn transcript_from_body(body: &serde_json::Value) -> OtoResult<String> {
    body.get("results")
        .and_then(|r| r.get("channels"))
        .and_then(|c| c.as_array())
        .and_then(|channels| channels.first())
        .and_then(|ch| ch.get("alternatives"))
        .and_then(|a| a.as_array())
        .and_then(|alts| alts.first())
        .and_then(|alt| alt.get("transcript"))
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| OtoError::Message("STT response missing transcript".into()))
}

#[async_trait]
impl SpeechToText for DeepgramClient {
    async fn transcribe(&self, audio_wav: &[u8], ctx: &TranscriptionContext) -> OtoResult<String> {
        let url = listen_url(&self.base_url, &self.model, ctx)?;
        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "audio/wav")
            .body(audio_wav.to_vec())
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(http_error_message(res).await);
        }
        let body: serde_json::Value = res.json().await?;
        transcript_from_body(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_url_includes_model_and_smart_format() {
        let ctx = TranscriptionContext::default();
        let url = listen_url("https://api.deepgram.com", "nova-3", &ctx).unwrap();
        assert!(url.starts_with("https://api.deepgram.com/v1/listen?"));
        assert!(url.contains("model=nova-3"));
        assert!(url.contains("smart_format=true"));
        assert!(url.contains("detect_language=true"));
        // Query pairs should not include a dedicated `language=` param when detecting.
        let has_language_param = url
            .split('?')
            .nth(1)
            .unwrap_or("")
            .split('&')
            .any(|pair| pair.starts_with("language="));
        assert!(!has_language_param);
    }

    #[test]
    fn listen_url_uses_language_when_set() {
        let ctx = TranscriptionContext {
            language: Some("es".into()),
            ..Default::default()
        };
        let url = listen_url("https://api.deepgram.com", "nova-3", &ctx).unwrap();
        assert!(url.contains("language=es"));
        assert!(!url.contains("detect_language"));
    }

    #[test]
    fn listen_url_appends_keyterms() {
        let ctx = TranscriptionContext {
            keyterms: vec!["Kubernetes".into(), "Oto".into()],
            ..Default::default()
        };
        let url = listen_url("https://api.deepgram.com", "nova-3", &ctx).unwrap();
        assert!(url.contains("keyterm=Kubernetes"));
        assert!(url.contains("keyterm=Oto"));
    }

    #[test]
    fn listen_url_accepts_v1_suffix() {
        let ctx = TranscriptionContext::default();
        let url = listen_url("https://api.deepgram.com/v1", "nova-3", &ctx).unwrap();
        assert!(url.starts_with("https://api.deepgram.com/v1/listen?"));
        assert!(!url.contains("/v1/v1/"));
    }

    #[test]
    fn transcript_parses_nested_channels() {
        let body = serde_json::json!({
            "results": {
                "channels": [{
                    "alternatives": [{
                        "transcript": "  Hello world.  "
                    }]
                }]
            }
        });
        assert_eq!(transcript_from_body(&body).unwrap(), "Hello world.");
    }

    #[test]
    fn transcript_missing_is_error() {
        let body = serde_json::json!({ "results": { "channels": [] } });
        assert!(transcript_from_body(&body).is_err());
    }
}
