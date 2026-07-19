pub mod openai_compat;
pub mod presets;
pub mod traits;

pub use openai_compat::{build_polish_system_prompt, client_from_config, OpenAiCompatClient};
pub use presets::{base_url_for, default_models, preset_account};
pub use traits::{PolishContext, SpeechToText, TextPolisher};
