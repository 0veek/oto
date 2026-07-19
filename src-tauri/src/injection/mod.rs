//! Hybrid text injection: AT-SPI → clipboard+paste → clipboard-only.

mod atspi_inject;
mod clipboard;
mod paste;

pub use clipboard::set_clipboard_text;
pub use paste::{detect_session, simulate_paste, tool_exists, SessionKind};

use crate::config::InjectionMode;
use crate::error::OtoResult;
use atspi_inject::try_atspi_insert;

/// How text was delivered to the target application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectResult {
    /// Inserted via AT-SPI2 focused text interface.
    Atspi,
    /// Clipboard written and paste key combo simulated.
    Pasted,
    /// Clipboard written only (user must paste manually).
    ClipboardOnly,
}

/// Inject `text` according to `mode`.
///
/// - **Auto:** AT-SPI → clipboard+paste → clipboard-only fallback
/// - **ClipboardPaste:** set clipboard then simulate paste (errors if paste fails)
/// - **ClipboardOnly:** set clipboard only
pub async fn inject_text(text: &str, mode: &InjectionMode) -> OtoResult<InjectResult> {
    match mode {
        InjectionMode::ClipboardOnly => {
            set_clipboard_text(text)?;
            Ok(InjectResult::ClipboardOnly)
        }
        InjectionMode::ClipboardPaste => {
            set_clipboard_text(text)?;
            simulate_paste()?;
            Ok(InjectResult::Pasted)
        }
        InjectionMode::Auto => {
            if try_atspi_insert(text).await? {
                return Ok(InjectResult::Atspi);
            }
            set_clipboard_text(text)?;
            match simulate_paste() {
                Ok(()) => Ok(InjectResult::Pasted),
                Err(_) => Ok(InjectResult::ClipboardOnly),
            }
        }
    }
}

/// Lightweight diagnostics for settings / logs (touches public paste helpers).
pub fn paste_tooling_summary() -> String {
    let session = match detect_session() {
        SessionKind::Wayland => "wayland",
        SessionKind::X11 => "x11",
        SessionKind::Unknown => "unknown",
    };
    let tools: Vec<&str> = ["wtype", "ydotool", "xdotool"]
        .into_iter()
        .filter(|b| tool_exists(b))
        .collect();
    format!(
        "session={session}; tools={}",
        if tools.is_empty() {
            "none".into()
        } else {
            tools.join(",")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn clipboard_only_mode() {
        let result = inject_text("oto unit", &InjectionMode::ClipboardOnly).await;
        // May fail in headless CI without a clipboard server — accept either.
        match result {
            Ok(r) => assert_eq!(r, InjectResult::ClipboardOnly),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                assert!(
                    msg.contains("clipboard")
                        || msg.contains("display")
                        || msg.contains("wayland")
                        || msg.contains("x11")
                        || msg.contains("not available"),
                    "unexpected error: {e}"
                );
            }
        }
    }

    #[test]
    fn paste_tooling_summary_nonempty() {
        let s = paste_tooling_summary();
        assert!(s.contains("session="));
        assert!(s.contains("tools="));
    }
}
