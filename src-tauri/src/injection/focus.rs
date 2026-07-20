//! Save/restore the focused window so injection hits the dictation target.
//!
//! STT can take seconds. During that time the user may switch windows, or the
//! compositor may give focus to Oto's settings/overlay. On Hyprland we pin the
//! original client with `hyprctl` before generating key events.

use std::process::Command;

use super::paste::{detect_session, tool_exists, SessionKind};

/// Snapshot of the window that should receive injected text.
#[derive(Debug, Clone, Default)]
pub struct FocusTarget {
    /// Hyprland client address, e.g. `0x5648f2dad7c0`.
    pub hyprland_address: Option<String>,
    pub class: Option<String>,
    #[allow(dead_code)]
    pub title: Option<String>,
}

fn run_hyprctl(args: &[&str]) -> Option<String> {
    if !tool_exists("hyprctl") {
        return None;
    }
    let output = Command::new("hyprctl").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_oto_window(class: Option<&str>, title: Option<&str>) -> bool {
    class.is_some_and(|c| c.eq_ignore_ascii_case("oto"))
        || title.is_some_and(|t| t.starts_with("Oto"))
}

fn target_from_fields(
    address: Option<String>,
    class: Option<String>,
    title: Option<String>,
) -> Option<FocusTarget> {
    if address.as_deref().unwrap_or("").is_empty() {
        return None;
    }
    if is_oto_window(class.as_deref(), title.as_deref()) {
        return None;
    }
    Some(FocusTarget {
        hyprland_address: address,
        class,
        title,
    })
}

/// Capture the client that should receive dictation text (Hyprland).
///
/// Prefers the active window, but if Oto Settings/overlay is focused, falls
/// back to the previous non-Oto client in Hyprland's focus history.
pub fn capture_focus_target() -> FocusTarget {
    if detect_session() != SessionKind::Wayland {
        return FocusTarget::default();
    }
    if let Some(raw) = run_hyprctl(&["activewindow", "-j"]) {
        if let Some(target) = target_from_fields(
            json_string_field(&raw, "address"),
            json_string_field(&raw, "class"),
            json_string_field(&raw, "title"),
        ) {
            return target;
        }
    }
    // Oto is focused — pick the most recently focused non-Oto client.
    if let Some(raw) = run_hyprctl(&["clients", "-j"]) {
        if let Some(target) = best_non_oto_from_clients_json(&raw) {
            return target;
        }
    }
    FocusTarget::default()
}

/// Parse `hyprctl clients -j` and pick the best non-Oto target.
/// Prefer lower `focusHistoryID` (0 = current, 1 = previous, …).
fn best_non_oto_from_clients_json(json: &str) -> Option<FocusTarget> {
    // Split on top-level objects roughly by `"address":`.
    let mut best: Option<(i64, FocusTarget)> = None;
    for chunk in json.split("\"address\"").skip(1) {
        // chunk starts with `: "0x...", ...`
        let Some(addr) = json_string_field(&format!("\"address\"{chunk}"), "address") else {
            continue;
        };
        let class = json_string_field(&format!("\"address\"{chunk}"), "class");
        let title = json_string_field(&format!("\"address\"{chunk}"), "title");
        if is_oto_window(class.as_deref(), title.as_deref()) {
            continue;
        }
        let hist = json_number_field(&format!("\"address\"{chunk}"), "focusHistoryID").unwrap_or(99);
        // Skip the currently focused client (usually Oto) — we want the previous app.
        if hist == 0 {
            continue;
        }
        let target = FocusTarget {
            hyprland_address: Some(addr),
            class,
            title,
        };
        match &best {
            None => best = Some((hist, target)),
            Some((h, _)) if hist < *h => best = Some((hist, target)),
            _ => {}
        }
    }
    best.map(|(_, t)| t)
}

fn json_number_field(json: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after_key = &json[idx + needle.len()..];
    let colon = after_key.find(':')?;
    let rest = after_key[colon + 1..].trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    num.parse().ok()
}

/// Restore focus to a previously captured target. Returns true if a restore ran.
pub fn restore_focus_target(target: &FocusTarget) -> bool {
    let Some(address) = target.hyprland_address.as_deref() else {
        return false;
    };
    if address.is_empty() {
        return false;
    }
    // Re-check current focus; skip if already correct.
    if let Some(raw) = run_hyprctl(&["activewindow", "-j"]) {
        if json_string_field(&raw, "address").as_deref() == Some(address) {
            return true;
        }
    }
    let arg = format!("address:{address}");
    run_hyprctl(&["dispatch", "focuswindow", &arg]).is_some()
}

/// Log-friendly summary of the currently focused client.
pub fn active_focus_summary() -> String {
    let Some(raw) = run_hyprctl(&["activewindow", "-j"]) else {
        return "unknown".into();
    };
    let class = json_string_field(&raw, "class").unwrap_or_else(|| "?".into());
    let title = json_string_field(&raw, "title").unwrap_or_else(|| "?".into());
    let address = json_string_field(&raw, "address").unwrap_or_else(|| "?".into());
    format!("{class} | {title} | {address}")
}

fn json_string_field(json: &str, key: &str) -> Option<String> {
    // Match `"key": "value"` with minimal escaping support for common titles.
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after_key = &json[idx + needle.len()..];
    let colon = after_key.find(':')?;
    let mut rest = after_key[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        // null or non-string
        return None;
    }
    rest = &rest[1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(match next {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                }
            }
            other => out.push(other),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hyprctl_activewindow_json() {
        let sample = r#"{
    "address": "0x5648f2dad7c0",
    "mapped": true,
    "class": "kitty",
    "title": "hello \"world\"",
    "pid": 1
}"#;
        assert_eq!(
            json_string_field(sample, "address").as_deref(),
            Some("0x5648f2dad7c0")
        );
        assert_eq!(json_string_field(sample, "class").as_deref(), Some("kitty"));
        assert_eq!(
            json_string_field(sample, "title").as_deref(),
            Some("hello \"world\"")
        );
    }
}
