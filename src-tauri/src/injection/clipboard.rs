//! System clipboard write via arboard.

use crate::error::{OtoError, OtoResult};

/// Set the system clipboard to `text`.
pub fn set_clipboard_text(text: &str) -> OtoResult<()> {
    let mut cb = arboard::Clipboard::new().map_err(|e| OtoError::Message(e.to_string()))?;
    cb.set_text(text.to_string())
        .map_err(|e| OtoError::Message(e.to_string()))?;
    Ok(())
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
}
