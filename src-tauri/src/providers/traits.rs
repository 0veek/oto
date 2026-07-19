use async_trait::async_trait;
use crate::error::OtoResult;

#[derive(Debug, Clone)]
pub struct PolishContext {
    pub language: Option<String>,
    pub dictionary: Vec<String>,
    pub tone_hint: String,
}

#[async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, audio_wav: &[u8], language: Option<&str>) -> OtoResult<String>;
}

#[async_trait]
pub trait TextPolisher: Send + Sync {
    async fn polish(&self, raw: &str, ctx: &PolishContext) -> OtoResult<String>;
}
