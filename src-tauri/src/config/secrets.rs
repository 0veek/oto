use crate::error::{OtoError, OtoResult};
use keyring::Entry;

const SERVICE: &str = "dev.oto.app";

fn entry_for(preset: &str) -> OtoResult<Entry> {
    Entry::new(SERVICE, preset).map_err(|e| OtoError::Keyring(e.to_string()))
}

pub fn set_api_key(preset: &str, key: &str) -> OtoResult<()> {
    if key.trim().is_empty() {
        return delete_api_key(preset);
    }
    entry_for(preset)?
        .set_password(key)
        .map_err(|e| OtoError::Keyring(e.to_string()))
}

pub fn get_api_key(preset: &str) -> OtoResult<Option<String>> {
    match entry_for(preset)?.get_password() {
        Ok(p) if !p.is_empty() => Ok(Some(p)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(OtoError::Keyring(e.to_string())),
    }
}

pub fn delete_api_key(preset: &str) -> OtoResult<()> {
    match entry_for(preset)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(OtoError::Keyring(e.to_string())),
    }
}

pub fn has_api_key(preset: &str) -> bool {
    matches!(get_api_key(preset), Ok(Some(_)))
}
