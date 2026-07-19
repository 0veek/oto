use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{OtoError, OtoResult};

static HISTORY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: String,
    pub created_at_ms: u64,
    pub raw_text: String,
    pub final_text: String,
    /// `dictation` or `command`.
    pub mode: String,
    pub language: Option<String>,
}

fn history_path() -> OtoResult<PathBuf> {
    let base = directories::ProjectDirs::from("dev", "Oto", "oto")
        .ok_or_else(|| OtoError::Message("could not resolve data dir".into()))?;
    Ok(base.data_local_dir().join("history.json"))
}

fn load_from(path: &Path) -> OtoResult<Vec<HistoryEntry>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&raw)?)
}

fn save_to(path: &Path, entries: &[HistoryEntry]) -> OtoResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(entries)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub fn list() -> OtoResult<Vec<HistoryEntry>> {
    let _guard = HISTORY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| OtoError::Message("history lock poisoned".into()))?;
    load_from(&history_path()?)
}

pub fn append(
    raw_text: String,
    final_text: String,
    mode: &str,
    language: Option<String>,
    limit: usize,
) -> OtoResult<HistoryEntry> {
    let _guard = HISTORY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| OtoError::Message("history lock poisoned".into()))?;
    let path = history_path()?;
    let mut entries = load_from(&path)?;
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let entry = HistoryEntry {
        // Include a short unique suffix so two saves in the same millisecond never collide.
        id: format!(
            "{created_at_ms}-{:x}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ),
        created_at_ms,
        raw_text,
        final_text,
        mode: mode.into(),
        language,
    };
    entries.insert(0, entry.clone());
    entries.truncate(limit.clamp(1, 1000));
    save_to(&path, &entries)?;
    Ok(entry)
}

pub fn delete(id: &str) -> OtoResult<()> {
    let _guard = HISTORY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| OtoError::Message("history lock poisoned".into()))?;
    let path = history_path()?;
    let mut entries = load_from(&path)?;
    entries.retain(|entry| entry.id != id);
    save_to(&path, &entries)
}

pub fn clear() -> OtoResult<()> {
    let _guard = HISTORY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| OtoError::Message("history lock poisoned".into()))?;
    save_to(&history_path()?, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_json_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let entries = vec![HistoryEntry {
            id: "one".into(),
            created_at_ms: 1,
            raw_text: "raw".into(),
            final_text: "final".into(),
            mode: "dictation".into(),
            language: Some("en".into()),
        }];
        save_to(&path, &entries).unwrap();
        assert_eq!(load_from(&path).unwrap(), entries);
    }
}
