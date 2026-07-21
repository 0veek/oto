//! Hybrid text injection: AT-SPI → direct typing → clipboard+paste → clipboard-only.

mod atspi_inject;
mod clipboard;
mod focus;
mod paste;

pub use clipboard::{get_clipboard_text, set_clipboard_text};
pub use focus::{
    active_focus_summary, capture_focus_target, restore_focus_target, FocusTarget,
};
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
/// - **Auto:** AT-SPI → clipboard+paste → direct typing → clipboard-only fallback
/// - **DirectType:** ydotool/wtype/xdotool virtual-keyboard typing
/// - **ClipboardPaste:** set clipboard then simulate paste (errors if paste fails)
/// - **ClipboardOnly:** set clipboard only
pub async fn inject_text(text: &str, mode: &InjectionMode) -> OtoResult<InjectResult> {
    inject_text_to(text, mode, None).await
}

fn append_inject_log(message: &str) {
    use std::io::Write;
    // Per-user path avoids multi-user /tmp ownership collisions (EACCES).
    let mut path = std::env::temp_dir();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| format!("uid-{}", std::process::id()));
    path.push(format!("oto-inject-{user}.log"));
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 512 * 1024 {
            let _ = std::fs::remove_file(&path);
        }
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{message}");
    }
    eprintln!("oto injection: {message}");
}

/// Write `text` to the clipboard and simulate Ctrl+V.
fn paste_via_clipboard(text: &str) -> OtoResult<()> {
    set_clipboard_text(text)?;
    simulate_paste()
}

/// Inject `text`, optionally restoring a previously captured focus target first.
pub async fn inject_text_to(
    text: &str,
    mode: &InjectionMode,
    focus: Option<&FocusTarget>,
) -> OtoResult<InjectResult> {
    append_inject_log(&format!(
        "inject_text mode={mode:?} chars={} focus_before={}",
        text.chars().count(),
        active_focus_summary()
    ));
    if let Some(target) = focus {
        let restored = restore_focus_target(target);
        append_inject_log(&format!(
            "restore_focus ok={restored} target_class={:?} address={:?}",
            target.class, target.hyprland_address
        ));
        if restored {
            // Give the compositor a beat to apply focus before key events.
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    }
    append_inject_log(&format!("focus_at_type={}", active_focus_summary()));
    let result = match mode {
        InjectionMode::ClipboardOnly => {
            set_clipboard_text(text)?;
            Ok(InjectResult::ClipboardOnly)
        }
        InjectionMode::DirectType => {
            // Keep a clipboard backup so the user can Ctrl+V if focus was wrong.
            let _ = set_clipboard_text(text);
            simulate_type(text)?;
            Ok(InjectResult::DirectTyped)
        }
        InjectionMode::ClipboardPaste => {
            paste_via_clipboard(text)?;
            Ok(InjectResult::Pasted)
        }
        InjectionMode::Auto => {
            // Prefer instant bulk insertion over character-by-character typing
            // (ydotool type with per-key delay feels laggy on long transcripts).
            if try_atspi_insert(text).await? {
                return Ok(InjectResult::Atspi);
            }
            match paste_via_clipboard(text) {
                Ok(()) => return Ok(InjectResult::Pasted),
                Err(error) => {
                    append_inject_log(&format!("clipboard+paste failed: {error}"));
                }
            }
            match simulate_type(text) {
                Ok(()) => Ok(InjectResult::DirectTyped),
                Err(error) => {
                    append_inject_log(&format!("direct typing failed: {error}"));
                    // Best-effort: ensure text is on the clipboard if earlier paste failed mid-way.
                    let _ = set_clipboard_text(text);
                    Ok(InjectResult::ClipboardOnly)
                }
            }
        }
    };
    match &result {
        Ok(kind) => append_inject_log(&format!("result={kind:?}")),
        Err(error) => append_inject_log(&format!("error={error}")),
    }
    result
}

/// Copy the focused application's selection for Command Mode. A sentinel makes
/// it possible to distinguish a real selection from a rejected synthetic key.
/// When the clipboard path is used, the previous clipboard contents are restored
/// after the selection is read so Command Mode does not permanently clobber it.
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
    // Selection is held in memory; put the user's prior clipboard back.
    if let Some(previous) = previous {
        let _ = set_clipboard_text(&previous);
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
