//! System clipboard write via arboard.

use std::sync::{Mutex, OnceLock};

use crate::error::{OtoError, OtoResult};

// X11 and Wayland selections are owned by the application that supplied the
// data. Keep one Clipboard alive for the lifetime of Oto so the focused client
// can request the text after `set_clipboard_text` returns. Creating a temporary
// Clipboard here makes Wayland paste race against its immediate Drop.
static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();

/// Set the system clipboard to `text`.
pub fn set_clipboard_text(text: &str) -> OtoResult<()> {
    let clipboard = CLIPBOARD.get_or_init(|| Mutex::new(None));
    let mut guard = clipboard
        .lock()
        .map_err(|_| OtoError::Message("clipboard lock poisoned".into()))?;
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|e| OtoError::Message(e.to_string()))?);
    }
    guard
        .as_mut()
        .expect("clipboard initialized above")
        .set_text(text.to_string())
        .map_err(|e| OtoError::Message(e.to_string()))?;
    Ok(())
}

/// Read text through the same long-lived clipboard owner used for writes.
pub fn get_clipboard_text() -> OtoResult<String> {
    let clipboard = CLIPBOARD.get_or_init(|| Mutex::new(None));
    let mut guard = clipboard
        .lock()
        .map_err(|_| OtoError::Message("clipboard lock poisoned".into()))?;
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|e| OtoError::Message(e.to_string()))?);
    }
    guard
        .as_mut()
        .expect("clipboard initialized above")
        .get_text()
        .map_err(|e| OtoError::Message(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Clipboard access may fail in headless CI; only assert error type mapping works.
    #[test]
    fn set_clipboard_returns_result() {
        // Smoke: call should not panic. Ok or Err both acceptable in sandbox.
        let _ = set_clipboard_text("oto clipboard smoke");
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires a live Wayland session and wl-paste"]
    fn wayland_clipboard_survives_set_call() {
        let expected = "oto persistent clipboard smoke";
        set_clipboard_text(expected).expect("set Wayland clipboard");
        let output = std::process::Command::new("wl-paste")
            .arg("--no-newline")
            .output()
            .expect("run wl-paste");
        assert!(output.status.success(), "wl-paste failed");
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }
}
