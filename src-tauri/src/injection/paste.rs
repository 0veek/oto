//! Paste simulation via session-aware external tools (wtype / ydotool / xdotool).

use std::process::Command;

use crate::error::{OtoError, OtoResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Wayland,
    X11,
    Unknown,
}

/// Detect display server from `XDG_SESSION_TYPE`.
pub fn detect_session() -> SessionKind {
    match std::env::var("XDG_SESSION_TYPE").ok().as_deref() {
        Some("wayland") => SessionKind::Wayland,
        Some("x11") => SessionKind::X11,
        _ => SessionKind::Unknown,
    }
}

/// True if `bin` is on PATH.
pub fn tool_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_ok(mut cmd: Command, label: &str) -> OtoResult<()> {
    let status = cmd
        .status()
        .map_err(|e| OtoError::Message(format!("{label}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(OtoError::Message(format!(
            "{label} exited with {status}"
        )))
    }
}

/// Simulate Ctrl+V (paste) using the best available tool for this session.
pub fn simulate_paste() -> OtoResult<()> {
    match detect_session() {
        SessionKind::Wayland => {
            if tool_exists("wtype") {
                // Hold Ctrl, press v, release Ctrl.
                return run_ok(
                    {
                        let mut c = Command::new("wtype");
                        c.args(["-M", "ctrl", "-P", "v", "-m", "ctrl"]);
                        c
                    },
                    "wtype",
                );
            }
            // ydotool: key codes 29=LeftCtrl, 47=v (press:1 / release:0).
            // Requires ydotoold running with appropriate permissions; may fail silently
            // on many systems — prefer wtype when available.
            if tool_exists("ydotool") {
                return run_ok(
                    {
                        let mut c = Command::new("ydotool");
                        c.args(["key", "29:1", "47:1", "47:0", "29:0"]);
                        c
                    },
                    "ydotool",
                );
            }
            Err(OtoError::Message(
                "No Wayland paste tool (install wtype or ydotool)".into(),
            ))
        }
        SessionKind::X11 | SessionKind::Unknown => {
            if tool_exists("xdotool") {
                return run_ok(
                    {
                        let mut c = Command::new("xdotool");
                        c.args(["key", "ctrl+v"]);
                        c
                    },
                    "xdotool",
                );
            }
            // Last-resort: try wtype even outside a declared Wayland session
            // (some hybrid setups omit XDG_SESSION_TYPE).
            if tool_exists("wtype") {
                return run_ok(
                    {
                        let mut c = Command::new("wtype");
                        c.args(["-M", "ctrl", "-P", "v", "-m", "ctrl"]);
                        c
                    },
                    "wtype",
                );
            }
            Err(OtoError::Message(
                "No paste tool found (install xdotool or wtype)".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_session_is_stable() {
        let a = detect_session();
        let b = detect_session();
        assert_eq!(a, b);
    }

    #[test]
    fn tool_exists_which_itself() {
        // `which` should find itself on a normal Linux PATH.
        assert!(tool_exists("which") || tool_exists("true"));
    }
}
