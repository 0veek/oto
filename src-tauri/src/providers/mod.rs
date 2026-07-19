pub mod local_whisper;
pub mod openai_compat;
pub mod presets;
pub mod traits;

pub use local_whisper::LocalWhisperClient;
pub use openai_compat::{client_from_config, OpenAiCompatClient};
pub use traits::{PolishContext, SpeechToText, TextPolisher, TranscriptionContext};
