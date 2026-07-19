use crate::error::{OtoError, OtoResult};
use crate::state::AppState;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

/// Normalize user input like `ctrl + alt + space` → `Ctrl+Alt+Space`.
pub fn normalize_hotkey(s: &str) -> String {
    s.split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|part| -> String {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => "Ctrl".into(),
                "alt" | "option" => "Alt".into(),
                "shift" => "Shift".into(),
                "super" | "meta" | "win" | "cmd" | "command" => "Super".into(),
                "space" => "Space".into(),
                "enter" | "return" => "Enter".into(),
                "tab" => "Tab".into(),
                "escape" | "esc" => "Escape".into(),
                other if other.len() == 1 => other.to_ascii_uppercase(),
                other => {
                    // Preserve unknown tokens as-is (parse will error later).
                    let mut c = other.chars();
                    match c.next() {
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Parse a human-readable hotkey string like `Ctrl+Alt+Space` into a [`Shortcut`].
pub fn parse_hotkey(s: &str) -> OtoResult<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;

    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "meta" | "win" | "cmd" | "command" => mods |= Modifiers::SUPER,
            "space" => key = Some(Code::Space),
            "enter" | "return" => key = Some(Code::Enter),
            "tab" => key = Some(Code::Tab),
            "escape" | "esc" => key = Some(Code::Escape),
            other if other.len() == 1 => {
                let c = other.chars().next().unwrap();
                key = Some(match c {
                    'a' => Code::KeyA,
                    'b' => Code::KeyB,
                    'c' => Code::KeyC,
                    'd' => Code::KeyD,
                    'e' => Code::KeyE,
                    'f' => Code::KeyF,
                    'g' => Code::KeyG,
                    'h' => Code::KeyH,
                    'i' => Code::KeyI,
                    'j' => Code::KeyJ,
                    'k' => Code::KeyK,
                    'l' => Code::KeyL,
                    'm' => Code::KeyM,
                    'n' => Code::KeyN,
                    'o' => Code::KeyO,
                    'p' => Code::KeyP,
                    'q' => Code::KeyQ,
                    'r' => Code::KeyR,
                    's' => Code::KeyS,
                    't' => Code::KeyT,
                    'u' => Code::KeyU,
                    'v' => Code::KeyV,
                    'w' => Code::KeyW,
                    'x' => Code::KeyX,
                    'y' => Code::KeyY,
                    'z' => Code::KeyZ,
                    _ => {
                        return Err(OtoError::Message(format!(
                            "unsupported key in hotkey: {part}"
                        )));
                    }
                });
            }
            other => {
                return Err(OtoError::Message(format!(
                    "unsupported hotkey token: {other}"
                )));
            }
        }
    }

    let key = key.ok_or_else(|| OtoError::Message(format!("no key in hotkey: {s}")))?;
    Ok(Shortcut::new(Some(mods), key))
}

/// Unregister all global shortcuts (no-op if none are registered).
pub fn unregister_all_hotkeys<R: Runtime>(app: &AppHandle<R>) -> OtoResult<()> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| OtoError::Message(e.to_string()))
}

/// Register the push-to-talk hotkey.
///
/// Behavior (Wayland-friendly):
/// - **Press** while idle → start listening
/// - **Press** while already listening → stop & process (toggle fallback)
/// - **Release** while listening → stop & process (classic hold-to-talk)
///
/// Clears any previously registered shortcuts first so re-registration on config
/// save replaces the old binding.
pub fn register_ptt<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> OtoResult<()> {
    let normalized = normalize_hotkey(hotkey);
    let shortcut = parse_hotkey(&normalized)?;
    let shortcut_for_check = parse_hotkey(&normalized)?;

    // Best-effort clear so changing the binding does not leave stale shortcuts.
    let _ = unregister_all_hotkeys(app);

    app.global_shortcut()
        .on_shortcut(shortcut, |app, sc, event| {
            eprintln!(
                "oto hotkey event: {:?} state={:?} id={}",
                sc,
                event.state(),
                sc.id()
            );
            let Some(state) = app.try_state::<AppState>() else {
                eprintln!("oto hotkey: AppState missing");
                return;
            };
            match event.state() {
                ShortcutState::Pressed => {
                    let p = state.pipeline.clone();
                    tauri::async_runtime::spawn(async move {
                        // Toggle-friendly: second press stops if still listening
                        // (release events are unreliable on many Wayland setups).
                        if p.is_listening() {
                            eprintln!("oto hotkey: Pressed while listening → ptt_up");
                            if let Err(e) = p.ptt_up().await {
                                eprintln!("ptt_up (hotkey press): {e}");
                            }
                        } else {
                            eprintln!("oto hotkey: Pressed → ptt_down");
                            if let Err(e) = p.ptt_down().await {
                                eprintln!("ptt_down (hotkey): {e}");
                            }
                        }
                    });
                }
                ShortcutState::Released => {
                    let p = state.pipeline.clone();
                    tauri::async_runtime::spawn(async move {
                        if p.is_listening() {
                            eprintln!("oto hotkey: Released → ptt_up");
                            if let Err(e) = p.ptt_up().await {
                                eprintln!("ptt_up (hotkey release): {e}");
                            }
                        }
                    });
                }
            }
        })
        .map_err(|e| OtoError::Message(format!("failed to register hotkey '{normalized}': {e}")))?;

    // Verify registration where the backend supports it.
    if app.global_shortcut().is_registered(shortcut_for_check) {
        eprintln!("hotkey registered and active: {normalized}");
    } else {
        eprintln!(
            "hotkey register returned OK but is_registered=false for {normalized} \
             (compositor may block global shortcuts — use tray Start/Stop)"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_hotkey() {
        let sc = parse_hotkey("Ctrl+Super+Space").unwrap();
        assert_eq!(sc.key, Code::Space);
        assert!(sc.mods.contains(Modifiers::CONTROL));
        assert!(sc.mods.contains(Modifiers::SUPER));
    }

    #[test]
    fn parses_ctrl_alt_space_lowercase() {
        let sc = parse_hotkey("ctrl+alt+space").unwrap();
        assert_eq!(sc.key, Code::Space);
        assert!(sc.mods.contains(Modifiers::CONTROL));
        assert!(sc.mods.contains(Modifiers::ALT));
    }

    #[test]
    fn normalize_hotkey_formats() {
        assert_eq!(normalize_hotkey("ctrl + alt + space"), "Ctrl+Alt+Space");
        assert_eq!(normalize_hotkey("CTRL+ALT+SPACE"), "Ctrl+Alt+Space");
    }

    #[test]
    fn parses_letter_with_modifiers() {
        let sc = parse_hotkey("Alt+Shift+A").unwrap();
        assert_eq!(sc.key, Code::KeyA);
        assert!(sc.mods.contains(Modifiers::ALT));
        assert!(sc.mods.contains(Modifiers::SHIFT));
    }

    #[test]
    fn rejects_empty_and_unknown() {
        assert!(parse_hotkey("").is_err());
        assert!(parse_hotkey("Ctrl+F1").is_err());
        assert!(parse_hotkey("Ctrl").is_err());
    }
}
