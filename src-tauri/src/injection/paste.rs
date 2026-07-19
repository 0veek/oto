//! Paste simulation via session-aware external tools (wtype / ydotool / xdotool).

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

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
    let mut child = cmd
        .spawn()
        .map_err(|e| OtoError::Message(format!("{label}: {e}")))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child
            .try_wait()
            .map_err(|error| OtoError::Message(format!("{label}: {error}")))?
        {
            Some(status) if status.success() => return Ok(()),
            Some(status) => return Err(OtoError::Message(format!("{label} exited with {status}"))),
            None if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(OtoError::Message(format!("{label} timed out after 5s")));
            }
        }
    }
}

fn ydotool_ready() -> bool {
    if !tool_exists("ydotool") {
        return false;
    }
    // Older/system-wide setups may not expose ydotoold as a separate binary.
    if !tool_exists("ydotoold") {
        return true;
    }
    let mut sockets = Vec::new();
    if let Ok(path) = std::env::var("YDOTOOL_SOCKET") {
        sockets.push(std::path::PathBuf::from(path));
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        sockets.push(std::path::Path::new(&runtime).join(".ydotool_socket"));
    }
    sockets.push(std::path::PathBuf::from("/tmp/.ydotool_socket"));
    sockets.into_iter().any(|path| path.exists())
}

fn wtype_paste_command() -> Command {
    let mut command = Command::new("wtype");
    // `-k` emits both key-down and key-up. Leaving V pressed can make some
    // Wayland clients ignore the shortcut even though wtype exits successfully.
    command.args(["-M", "ctrl", "-k", "v", "-m", "ctrl"]);
    command
}

fn wtype_copy_command() -> Command {
    let mut command = Command::new("wtype");
    command.args(["-M", "ctrl", "-k", "c", "-m", "ctrl"]);
    command
}

fn paste_with_wtype() -> OtoResult<()> {
    // Give the compositor a moment to publish the new clipboard selection
    // before the focused client requests it in response to Ctrl+V.
    thread::sleep(Duration::from_millis(50));
    run_ok(wtype_paste_command(), "wtype")
}

fn text_without_line_actions(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\r' | '\n' | '\u{000b}' | '\u{000c}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => ' ',
            other => other,
        })
        .collect()
}

fn wtype_text_command(text: &str) -> Command {
    let mut command = Command::new("wtype");
    command.arg("--").arg(text_without_line_actions(text));
    command
}

fn ydotool_text_command(text: &str) -> Command {
    let mut command = Command::new("ydotool");
    command
        .args(["type", "--"])
        .arg(text_without_line_actions(text));
    command
}

/// Type text through a virtual keyboard. This mirrors the reliable Hyprland
/// chain used by Hyprvoice: ydotool first, then wtype on Wayland.
pub fn simulate_type(text: &str) -> OtoResult<()> {
    if text.is_empty() {
        return Err(OtoError::Message("cannot type empty text".into()));
    }
    match detect_session() {
        SessionKind::Wayland => {
            let mut failures = Vec::new();
            if ydotool_ready() {
                match run_ok(ydotool_text_command(text), "ydotool type") {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            if tool_exists("wtype") {
                match run_ok(wtype_text_command(text), "wtype text") {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            Err(OtoError::Message(if failures.is_empty() {
                "No Wayland typing tool (install ydotool or wtype)".into()
            } else {
                format!("Wayland typing failed: {}", failures.join("; "))
            }))
        }
        SessionKind::X11 | SessionKind::Unknown => {
            if tool_exists("xdotool") {
                let mut command = Command::new("xdotool");
                command
                    .args(["type", "--clearmodifiers", "--"])
                    .arg(text_without_line_actions(text));
                return run_ok(command, "xdotool type");
            }
            if tool_exists("wtype") {
                return run_ok(wtype_text_command(text), "wtype text");
            }
            Err(OtoError::Message(
                "No typing tool found (install xdotool or wtype)".into(),
            ))
        }
    }
}

/// Simulate Ctrl+V (paste) using the best available tool for this session.
pub fn simulate_paste() -> OtoResult<()> {
    match detect_session() {
        SessionKind::Wayland => {
            if tool_exists("wtype") {
                return paste_with_wtype();
            }
            // ydotool: key codes 29=LeftCtrl, 47=v (press:1 / release:0).
            // Requires ydotoold running with appropriate permissions; may fail silently
            // on many systems — prefer wtype when available.
            if ydotool_ready() {
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
                return paste_with_wtype();
            }
            Err(OtoError::Message(
                "No paste tool found (install xdotool or wtype)".into(),
            ))
        }
    }
}

/// Simulate Ctrl+C without changing focus. Used by Command Mode to read the
/// selection before recording the spoken edit instruction.
pub fn simulate_copy() -> OtoResult<()> {
    match detect_session() {
        SessionKind::Wayland => {
            if tool_exists("wtype") {
                return run_ok(wtype_copy_command(), "wtype");
            }
            if tool_exists("ydotool") {
                return run_ok(
                    {
                        let mut command = Command::new("ydotool");
                        command.args(["key", "29:1", "46:1", "46:0", "29:0"]);
                        command
                    },
                    "ydotool",
                );
            }
            Err(OtoError::Message(
                "No Wayland copy tool (install wtype or ydotool)".into(),
            ))
        }
        SessionKind::X11 | SessionKind::Unknown => {
            if tool_exists("xdotool") {
                return run_ok(
                    {
                        let mut command = Command::new("xdotool");
                        command.args(["key", "ctrl+c"]);
                        command
                    },
                    "xdotool",
                );
            }
            if tool_exists("wtype") {
                return run_ok(wtype_copy_command(), "wtype");
            }
            Err(OtoError::Message(
                "No copy tool found (install xdotool or wtype)".into(),
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

    #[test]
    fn wtype_paste_is_a_complete_ctrl_v_keystroke() {
        let command = wtype_paste_command();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["-M", "ctrl", "-k", "v", "-m", "ctrl"]);
    }

    #[test]
    fn wtype_copy_is_a_complete_ctrl_c_keystroke() {
        let command = wtype_copy_command();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["-M", "ctrl", "-k", "c", "-m", "ctrl"]);
    }

    #[test]
    fn direct_typing_commands_use_literal_text_after_double_dash() {
        let wtype = wtype_text_command("hello\nworld");
        let wtype_args: Vec<_> = wtype
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(wtype_args, ["--", "hello world"]);

        let ydotool = ydotool_text_command("-not-an-option");
        let ydotool_args: Vec<_> = ydotool
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(ydotool_args, ["type", "--", "-not-an-option"]);
    }
}
