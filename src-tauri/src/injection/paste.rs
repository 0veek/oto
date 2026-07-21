//! Paste simulation via session-aware external tools (wtype / ydotool / xdotool).

use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::net::UnixDatagram,
    process::{Command, Stdio},
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

fn inject_log_path() -> std::path::PathBuf {
    // Per-user path avoids multi-user /tmp ownership collisions (EACCES).
    let mut path = std::env::temp_dir();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| format!("uid-{}", std::process::id()));
    path.push(format!("oto-inject-{user}.log"));
    path
}

fn inject_log(message: &str) {
    // GUI launches often discard stderr; keep a small on-disk trail for diagnosis.
    let path = inject_log_path();
    // Soft rotation so the diagnostic log cannot grow without bound.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 512 * 1024 {
            let _ = std::fs::remove_file(&path);
        }
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{} {message}", chrono_lite_timestamp());
    }
    eprintln!("oto injection: {message}");
}

fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn run_ok(mut cmd: Command, label: &str) -> OtoResult<()> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| OtoError::Message(format!("{label}: {e}")))?;
    let output = {
        let deadline = Instant::now() + Duration::from_secs(5);
        // Prefer wait_with_output but honor a hard timeout for stuck daemons.
        let mut child = child;
        loop {
            match child
                .try_wait()
                .map_err(|error| OtoError::Message(format!("{label}: {error}")))?
            {
                Some(status) => {
                    let stdout = {
                        let mut buf = Vec::new();
                        if let Some(mut out) = child.stdout.take() {
                            use std::io::Read;
                            let _ = out.read_to_end(&mut buf);
                        }
                        buf
                    };
                    let stderr = {
                        let mut buf = Vec::new();
                        if let Some(mut err) = child.stderr.take() {
                            use std::io::Read;
                            let _ = err.read_to_end(&mut buf);
                        }
                        buf
                    };
                    break std::process::Output {
                        status,
                        stdout,
                        stderr,
                    };
                }
                None if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(OtoError::Message(format!("{label} timed out after 5s")));
                }
            }
        }
    };
    if output.status.success() {
        inject_log(&format!("{label}: ok"));
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = match (stderr.is_empty(), stdout.is_empty()) {
        (false, _) => stderr,
        (true, false) => stdout,
        (true, true) => format!("exited with {}", output.status),
    };
    inject_log(&format!("{label}: failed — {detail}"));
    Err(OtoError::Message(format!("{label}: {detail}")))
}

fn ydotool_socket_paths() -> Vec<std::path::PathBuf> {
    let mut sockets = Vec::new();
    if let Ok(path) = std::env::var("YDOTOOL_SOCKET") {
        sockets.push(std::path::PathBuf::from(path));
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        sockets.push(std::path::Path::new(&runtime).join(".ydotool_socket"));
    }
    sockets.push(std::path::PathBuf::from("/tmp/.ydotool_socket"));
    sockets
}

fn ydotool_socket_alive(path: &std::path::Path) -> bool {
    // ydotoold uses a Unix *datagram* socket (not stream). Connecting with
    // SOCK_STREAM fails with EPROTOTYPE even when the daemon is healthy.
    let Ok(sock) = UnixDatagram::unbound() else {
        return false;
    };
    sock.connect(path).is_ok()
}

fn ydotool_ready() -> bool {
    if !tool_exists("ydotool") {
        return false;
    }
    // Older/system-wide setups may not expose ydotoold as a separate binary.
    if !tool_exists("ydotoold") {
        return true;
    }
    // Require a live daemon — a leftover socket file is not enough.
    ydotool_socket_paths()
        .into_iter()
        .any(|path| path.exists() && ydotool_socket_alive(&path))
}

/// Release common modifiers so PTT chords (Ctrl/Shift/Super) do not transform
/// generated typing into shortcuts. X11 gets this via xdotool --clearmodifiers;
/// on Wayland we synthesize key-up events through ydotool when available.
fn release_modifiers() {
    if ydotool_ready() {
        // 29/97 Ctrl, 42/54 Shift, 56/100 Alt, 125/126 Meta (left/right).
        let mut command = Command::new("ydotool");
        command.args([
            "key", "29:0", "97:0", "42:0", "54:0", "56:0", "100:0", "125:0", "126:0",
        ]);
        let _ = run_ok(command, "ydotool release modifiers");
        return;
    }
    if tool_exists("wtype") {
        // Best-effort: toggle modifiers off if a prior combo left them latched.
        let mut command = Command::new("wtype");
        command.args([
            "-m", "ctrl", "-m", "shift", "-m", "alt", "-m", "win",
        ]);
        let _ = run_ok(command, "wtype release modifiers");
    }
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
    run_ok(wtype_paste_command(), "wtype paste")
}

fn paste_with_ydotool() -> OtoResult<()> {
    // Same settle delay as wtype so arboard's selection is visible before Ctrl+V.
    thread::sleep(Duration::from_millis(50));
    // key codes: 29=LeftCtrl, 47=v (press:1 / release:0).
    let mut command = Command::new("ydotool");
    command.args(["key", "29:1", "47:1", "47:0", "29:0"]);
    run_ok(command, "ydotool paste")
}

/// Normalize line endings and strip control characters that typing tools treat
/// as accidental key actions, while preserving intentional newlines as `\n`.
fn normalize_typed_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\n' => out.push('\n'),
            // Vertical tab / form feed / NEL / line & paragraph separators → newline.
            '\u{000b}' | '\u{000c}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => out.push('\n'),
            other => out.push(other),
        }
    }
    out
}

/// Split into lines for virtual-keyboard tools that need explicit Return between rows.
fn typed_lines(text: &str) -> Vec<String> {
    normalize_typed_text(text)
        .split('\n')
        .map(|line| line.to_string())
        .collect()
}

fn wtype_text_command(text: &str) -> Command {
    let mut command = Command::new("wtype");
    let lines = typed_lines(text);
    if lines.is_empty() {
        command.arg("--").arg("");
        return command;
    }
    for (index, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            command.arg("--").arg(line);
        }
        if index + 1 < lines.len() {
            command.args(["-k", "Return"]);
        }
    }
    // Pure empty line with no trailing newline still needs a type arg for run_ok.
    if lines.len() == 1 && lines[0].is_empty() {
        command.arg("--").arg("");
    }
    command
}

fn ydotool_type_line_command(line: &str) -> Command {
    let mut command = Command::new("ydotool");
    // -e 0: type the transcript literally (escape sequences would mangle paths/code).
    // -d 12: slightly slower than default so busy apps do not drop key events.
    command
        .args(["type", "-e", "0", "-d", "12", "--"])
        .arg(line);
    command
}

fn ydotool_return_command() -> Command {
    let mut command = Command::new("ydotool");
    // KEY_ENTER = 28
    command.args(["key", "28:1", "28:0"]);
    command
}

fn xdotool_type_line_command(line: &str) -> Command {
    let mut command = Command::new("xdotool");
    command
        .args(["type", "--clearmodifiers", "--"])
        .arg(line);
    command
}

fn xdotool_return_command() -> Command {
    let mut command = Command::new("xdotool");
    command.args(["key", "Return"]);
    command
}

fn type_lines_with_ydotool(text: &str) -> OtoResult<()> {
    let lines = typed_lines(text);
    for (index, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            run_ok(ydotool_type_line_command(line), "ydotool type")?;
        }
        if index + 1 < lines.len() {
            run_ok(ydotool_return_command(), "ydotool Return")?;
        }
    }
    Ok(())
}

fn type_lines_with_xdotool(text: &str) -> OtoResult<()> {
    let lines = typed_lines(text);
    for (index, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            run_ok(xdotool_type_line_command(line), "xdotool type")?;
        }
        if index + 1 < lines.len() {
            run_ok(xdotool_return_command(), "xdotool Return")?;
        }
    }
    Ok(())
}

/// Type text through a virtual keyboard. This mirrors the reliable Hyprland
/// chain used by Hyprvoice: ydotool first, then wtype on Wayland.
pub fn simulate_type(text: &str) -> OtoResult<()> {
    if text.is_empty() {
        return Err(OtoError::Message("cannot type empty text".into()));
    }
    let session = detect_session();
    inject_log(&format!(
        "simulate_type session={session:?} ydotool_ready={} wtype={} chars={}",
        ydotool_ready(),
        tool_exists("wtype"),
        text.chars().count()
    ));
    // PTT chords leave Ctrl/Shift held long enough to turn "hello" into shortcuts.
    release_modifiers();
    thread::sleep(Duration::from_millis(40));
    match session {
        SessionKind::Wayland => {
            let mut failures = Vec::new();
            if ydotool_ready() {
                match type_lines_with_ydotool(text) {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            } else {
                inject_log("ydotool not ready (daemon/socket); trying wtype");
            }
            if tool_exists("wtype") {
                match run_ok(wtype_text_command(text), "wtype text") {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            Err(OtoError::Message(if failures.is_empty() {
                "No Wayland typing tool (install ydotool or wtype; enable: systemctl --user enable --now ydotool.service)".into()
            } else {
                format!("Wayland typing failed: {}", failures.join("; "))
            }))
        }
        SessionKind::X11 | SessionKind::Unknown => {
            if tool_exists("xdotool") {
                return type_lines_with_xdotool(text);
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
/// Prefer ydotool on Wayland (same reliability rationale as typing), then wtype.
pub fn simulate_paste() -> OtoResult<()> {
    release_modifiers();
    thread::sleep(Duration::from_millis(40));
    match detect_session() {
        SessionKind::Wayland => {
            let mut failures = Vec::new();
            // ydotool first: uinput injection is more reliable than wtype on
            // Hyprland/Electron; mirrors simulate_type priority.
            if ydotool_ready() {
                match paste_with_ydotool() {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            if tool_exists("wtype") {
                match paste_with_wtype() {
                    Ok(()) => return Ok(()),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            Err(OtoError::Message(if failures.is_empty() {
                "No Wayland paste tool (install ydotool or wtype)".into()
            } else {
                format!("Wayland paste failed: {}", failures.join("; "))
            }))
        }
        SessionKind::X11 | SessionKind::Unknown => {
            if tool_exists("xdotool") {
                return run_ok(
                    {
                        let mut c = Command::new("xdotool");
                        c.args(["key", "ctrl+v"]);
                        c
                    },
                    "xdotool paste",
                );
            }
            // Last-resort: try wtype even outside a declared Wayland session
            // (some hybrid setups omit XDG_SESSION_TYPE).
            if tool_exists("wtype") {
                return paste_with_wtype();
            }
            if ydotool_ready() {
                return paste_with_ydotool();
            }
            Err(OtoError::Message(
                "No paste tool found (install xdotool, wtype, or ydotool)".into(),
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
            if ydotool_ready() {
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
    fn direct_typing_commands_preserve_newlines_as_return() {
        let wtype = wtype_text_command("hello\nworld");
        let wtype_args: Vec<_> = wtype
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            wtype_args,
            ["--", "hello", "-k", "Return", "--", "world"]
        );

        let ydotool = ydotool_type_line_command("-not-an-option");
        let ydotool_args: Vec<_> = ydotool
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            ydotool_args,
            ["type", "-e", "0", "-d", "12", "--", "-not-an-option"]
        );

        assert_eq!(typed_lines("Best,\r\nAveek"), vec!["Best,", "Aveek"]);
    }
}
