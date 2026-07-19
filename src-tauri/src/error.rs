use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum OtoError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("keyring: {0}")]
    Keyring(String),
}

impl Serialize for OtoError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type OtoResult<T> = Result<T, OtoError>;
