//! Hybrid text injection: AT-SPI → direct typing → clipboard+paste → clipboard-only.

mod atspi_inject;
mod clipboard;
mod paste;

pub use clipboard::{get_clipboard_text, set_clipboard_text};
pub use paste::{
    detect_session, simulate_copy, simulate_paste, simulate_type, tool_exists, SessionKind,
};

use crate::config::InjectionMode;
use crate::error::{OtoError, OtoResult};
use atspi_inject::{try_atspi_insert, try_atspi_selection};

/// How text was delivered to the target application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectResult {
    /// Inserted via AT-SPI2 focused text interface.
    Atspi,
    /// Typed directly through ydotool/wtype/xdotool.
    DirectTyped,
    /// Clipboard written and paste key combo simulated.
    Pasted,
    /// Clipboard written only (user must paste manually).
    ClipboardOnly,
}

/// Inject `text` according to `mode`.
///
/// - **Auto:** AT-SPI → direct typing → clipboard+paste → clipboard-only fallback
/// - **DirectType:** ydotool/wtype/xdotool virtual-keyboard typing
/// - **ClipboardPaste:** set clipboard then simulate paste (errors if paste fails)
/// - **ClipboardOnly:** set clipboard only
pub async fn inject_text(text: &str, mode: &InjectionMode) -> OtoResult<InjectResult> {
    match mode {
        InjectionMode::ClipboardOnly => {
            set_clipboard_text(text)?;
            Ok(InjectResult::ClipboardOnly)
        }
        InjectionMode::DirectType => {
            simulate_type(text)?;
            Ok(InjectResult::DirectTyped)
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
            match simulate_type(text) {
                Ok(()) => return Ok(InjectResult::DirectTyped),
                Err(error) => eprintln!("oto injection: direct typing failed: {error}"),
            }
            set_clipboard_text(text)?;
            match simulate_paste() {
                Ok(()) => Ok(InjectResult::Pasted),
                Err(error) => {
                    eprintln!("oto injection: simulated paste failed: {error}");
                    Ok(InjectResult::ClipboardOnly)
                }
            }
        }
    }
}

/// Copy the focused application's selection for Command Mode. A sentinel makes
/// it possible to distinguish a real selection from a rejected synthetic key.
pub async fn capture_selected_text() -> OtoResult<String> {
    if let Some(selected) = try_atspi_selection().await? {
        return Ok(selected);
    }
    let previous = get_clipboard_text().ok();
    let sentinel = format!(
        "__oto_selection_{}__",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    set_clipboard_text(&sentinel)?;
    if let Err(error) = simulate_copy() {
        if let Some(previous) = previous {
            let _ = set_clipboard_text(&previous);
        }
        return Err(error);
    }
    tokio::time::sleep(std::time::Duration::from_millis(160)).await;
    let selected = get_clipboard_text()?;
    if selected == sentinel || selected.trim().is_empty() {
        if let Some(previous) = previous {
            let _ = set_clipboard_text(&previous);
        }
        return Err(OtoError::Message(
            "No selected text found — select text in the target app first".into(),
        ));
    }
    Ok(selected)
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
