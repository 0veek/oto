use crate::error::OtoResult;
use async_trait::async_trait;

#[derive(Debug, Clone, Default)]
pub struct TranscriptionContext {
    pub language: Option<String>,
    /// Free-form vocabulary prompt for Whisper-compatible APIs (`prompt` field).
    pub vocabulary_prompt: Option<String>,
    /// Individual terms for providers that accept keyword/keyterm lists (e.g. Deepgram).
    pub keyterms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PolishContext {
    pub language: Option<String>,
    pub dictionary: Vec<String>,
    pub tone_hint: String,
}

#[async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, audio_wav: &[u8], ctx: &TranscriptionContext) -> OtoResult<String>;
}

#[async_trait]
pub trait TextPolisher: Send + Sync {
    async fn polish(&self, raw: &str, ctx: &PolishContext) -> OtoResult<String>;
    async fn rewrite(
        &self,
        selected: &str,
        instruction: &str,
        ctx: &PolishContext,
    ) -> OtoResult<String>;
}
