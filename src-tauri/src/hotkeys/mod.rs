use crate::error::{OtoError, OtoResult};
use crate::state::AppState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg(target_os = "linux")]
use ashpd::{
    desktop::{
        global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut},
        CreateSessionOptions, Session,
    },
    register_host_app_with_connection, AppID, WindowIdentifier,
};
#[cfg(target_os = "linux")]
use futures_util::StreamExt;
#[cfg(target_os = "linux")]
use serde::Deserialize;
#[cfg(target_os = "linux")]
use std::{fs, path::PathBuf, process::Command};

const PORTAL_APP_ID: &str = "dev.oto.app";
const PORTAL_SHORTCUT_ID: &str = "dictation";
const PORTAL_ACTION: &str = "dev.oto.app:dictation";

/// Shared hotkey state. The event gate preserves press/release ordering even
/// though recording work runs asynchronously.
pub struct HotkeyManager {
    pressed: AtomicBool,
    event_gate: Arc<tokio::sync::Mutex<()>>,
    #[cfg(target_os = "linux")]
    wayland: tokio::sync::Mutex<Option<WaylandRegistration>>,
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self {
            pressed: AtomicBool::new(false),
            event_gate: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(target_os = "linux")]
            wayland: tokio::sync::Mutex::new(None),
        }
    }
}

#[cfg(target_os = "linux")]
struct WaylandRegistration {
    hotkey: String,
    session: Session<GlobalShortcuts>,
    activated_task: tauri::async_runtime::JoinHandle<()>,
    deactivated_task: tauri::async_runtime::JoinHandle<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HotkeyParts {
    modifiers: Vec<&'static str>,
    key: String,
}

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
            "space" => set_hotkey_key(&mut key, Code::Space, s)?,
            "enter" | "return" => set_hotkey_key(&mut key, Code::Enter, s)?,
            "tab" => set_hotkey_key(&mut key, Code::Tab, s)?,
            "escape" | "esc" => set_hotkey_key(&mut key, Code::Escape, s)?,
            other if other.len() == 1 => {
                let c = other.chars().next().unwrap();
                let code = match c {
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
                };
                set_hotkey_key(&mut key, code, s)?;
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

fn set_hotkey_key(slot: &mut Option<Code>, key: Code, hotkey: &str) -> OtoResult<()> {
    if slot.replace(key).is_some() {
        return Err(OtoError::Message(format!(
            "hotkey must contain exactly one non-modifier key: {hotkey}"
        )));
    }
    Ok(())
}

fn hotkey_parts(s: &str) -> OtoResult<HotkeyParts> {
    // Keep validation identical across the X11 and portal backends.
    parse_hotkey(s)?;

    let mut modifiers = Vec::new();
    let mut key = None;
    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" if !modifiers.contains(&"CTRL") => modifiers.push("CTRL"),
            "alt" | "option" if !modifiers.contains(&"ALT") => modifiers.push("ALT"),
            "shift" if !modifiers.contains(&"SHIFT") => modifiers.push("SHIFT"),
            "super" | "meta" | "win" | "cmd" | "command" if !modifiers.contains(&"SUPER") => {
                modifiers.push("SUPER");
            }
            "space" => key = Some("SPACE".to_string()),
            "enter" | "return" => key = Some("RETURN".to_string()),
            "tab" => key = Some("TAB".to_string()),
            "escape" | "esc" => key = Some("ESCAPE".to_string()),
            other if other.len() == 1 => key = Some(other.to_ascii_uppercase()),
            _ => {}
        }
    }

    Ok(HotkeyParts {
        modifiers,
        key: key.expect("parse_hotkey already required one key"),
    })
}

#[cfg(target_os = "linux")]
fn portal_trigger(s: &str) -> OtoResult<String> {
    let parts = hotkey_parts(s)?;
    let key = match parts.key.as_str() {
        "SPACE" => "space".to_string(),
        "RETURN" => "Return".to_string(),
        "TAB" => "Tab".to_string(),
        "ESCAPE" => "Escape".to_string(),
        other => other.to_ascii_lowercase(),
    };
    Ok(parts
        .modifiers
        .into_iter()
        .chain(std::iter::once(key.as_str()))
        .collect::<Vec<_>>()
        .join("+"))
}

/// Unregister all global shortcuts (no-op if none are registered).
pub fn unregister_all_hotkeys<R: Runtime>(app: &AppHandle<R>) -> OtoResult<()> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| OtoError::Message(e.to_string()))
}

fn dispatch_ptt_event<R: Runtime>(app: &AppHandle<R>, event: ShortcutState) {
    let Some(hotkey_state) = app.try_state::<HotkeyManager>() else {
        eprintln!("oto hotkey: HotkeyManager missing");
        return;
    };

    // Native backends can repeat Pressed while a key is held. Only accept real
    // up/down transitions so repeat events cannot immediately stop recording.
    let should_dispatch = match event {
        ShortcutState::Pressed => !hotkey_state.pressed.swap(true, Ordering::SeqCst),
        ShortcutState::Released => hotkey_state.pressed.swap(false, Ordering::SeqCst),
    };
    if !should_dispatch {
        return;
    }

    let Some(state) = app.try_state::<AppState>() else {
        eprintln!("oto hotkey: AppState missing");
        return;
    };
    let pipeline = state.pipeline.clone();
    let gate = hotkey_state.event_gate.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = gate.lock().await;
        let result = match event {
            ShortcutState::Pressed => {
                eprintln!("oto hotkey: Pressed → ptt_down");
                pipeline.ptt_down().await
            }
            ShortcutState::Released => {
                eprintln!("oto hotkey: Released → ptt_up");
                pipeline.ptt_up().await
            }
        };
        if let Err(error) = result {
            eprintln!("oto hotkey pipeline event failed: {error}");
        }
    });
}

/// Register the push-to-talk hotkey using the Wayland GlobalShortcuts portal
/// on Wayland and the Tauri/X11 backend elsewhere.
pub async fn register_ptt<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> OtoResult<()> {
    let normalized = normalize_hotkey(hotkey);
    parse_hotkey(&normalized)?;

    #[cfg(target_os = "linux")]
    if is_wayland_session() {
        return register_wayland_ptt(app, &normalized).await;
    }

    register_native_ptt(app, &normalized)
}

fn register_native_ptt<R: Runtime>(app: &AppHandle<R>, normalized: &str) -> OtoResult<()> {
    if let Some(state) = app.try_state::<HotkeyManager>() {
        state.pressed.store(false, Ordering::SeqCst);
    }
    let shortcut = parse_hotkey(normalized)?;
    let shortcut_for_check = parse_hotkey(normalized)?;

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
            dispatch_ptt_event(app, event.state());
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

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg(target_os = "linux")]
fn is_hyprland_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|value| value.to_ascii_lowercase().contains("hyprland"))
        || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

#[cfg(target_os = "linux")]
fn ensure_development_desktop_file() -> OtoResult<()> {
    if !cfg!(debug_assertions) {
        return Ok(());
    }

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(dirs::data_dir)
        .ok_or_else(|| OtoError::Message("could not locate XDG data directory".into()))?;
    let applications = data_home.join("applications");
    fs::create_dir_all(&applications)?;
    let desktop_file = applications.join(format!("{PORTAL_APP_ID}.desktop"));
    if desktop_file.exists() {
        return Ok(());
    }

    let executable = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Oto\nComment=System-wide voice dictation\nExec={}\nTerminal=false\nCategories=Utility;AudioVideo;\n",
        executable.display()
    );
    fs::write(&desktop_file, entry)?;
    eprintln!(
        "oto hotkey: installed development portal identity at {}",
        desktop_file.display()
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Debug, Deserialize)]
struct HyprlandBind {
    modmask: u32,
    key: String,
    dispatcher: String,
    arg: String,
}

#[cfg(target_os = "linux")]
fn hyprland_modmask(parts: &HotkeyParts) -> u32 {
    parts.modifiers.iter().fold(0, |mask, modifier| {
        mask | match *modifier {
            "SHIFT" => 1,
            "CTRL" => 4,
            "ALT" => 8,
            "SUPER" => 64,
            _ => 0,
        }
    })
}

#[cfg(target_os = "linux")]
fn hyprland_modifiers_from_mask(mask: u32) -> OtoResult<String> {
    if mask & !(1 | 4 | 8 | 64) != 0 {
        return Err(OtoError::Message(format!(
            "cannot safely replace Oto's existing Hyprland binding (unknown modifier mask {mask})"
        )));
    }
    let mut modifiers = Vec::new();
    if mask & 4 != 0 {
        modifiers.push("CTRL");
    }
    if mask & 8 != 0 {
        modifiers.push("ALT");
    }
    if mask & 1 != 0 {
        modifiers.push("SHIFT");
    }
    if mask & 64 != 0 {
        modifiers.push("SUPER");
    }
    Ok(modifiers.join(" "))
}

#[cfg(target_os = "linux")]
fn hyprland_binds() -> OtoResult<Vec<HyprlandBind>> {
    let output = Command::new("hyprctl")
        .args(["binds", "-j"])
        .output()
        .map_err(|error| OtoError::Message(format!("failed to run hyprctl: {error}")))?;
    if !output.status.success() {
        return Err(OtoError::Message(format!(
            "hyprctl binds failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| OtoError::Message(format!("invalid hyprctl binds response: {error}")))
}

#[cfg(target_os = "linux")]
fn run_hyprland_keyword(keyword: &str, value: &str) -> OtoResult<()> {
    let output = Command::new("hyprctl")
        .args(["keyword", keyword, value])
        .output()
        .map_err(|error| OtoError::Message(format!("failed to run hyprctl: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(OtoError::Message(format!(
            "hyprctl {keyword} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(target_os = "linux")]
fn configure_hyprland_binding(hotkey: &str) -> OtoResult<()> {
    let parts = hotkey_parts(hotkey)?;
    let target_mask = hyprland_modmask(&parts);
    let target_key = parts.key.to_ascii_uppercase();
    let binds = hyprland_binds()?;
    let is_oto = |bind: &HyprlandBind| {
        bind.dispatcher.eq_ignore_ascii_case("global") && bind.arg == PORTAL_ACTION
    };
    let same_chord = |bind: &HyprlandBind, mask: u32, key: &str| {
        bind.modmask == mask && bind.key.eq_ignore_ascii_case(key)
    };

    let target_binds: Vec<_> = binds
        .iter()
        .filter(|bind| same_chord(bind, target_mask, &target_key))
        .collect();
    if let Some(conflict) = target_binds.iter().find(|bind| !is_oto(bind)) {
        return Err(OtoError::Message(format!(
            "hotkey '{hotkey}' is already used by Hyprland ({})",
            conflict.dispatcher
        )));
    }

    let stale_oto: Vec<_> = binds
        .iter()
        .filter(|bind| is_oto(bind) && !same_chord(bind, target_mask, &target_key))
        .collect();
    for stale in &stale_oto {
        let chord_users = binds
            .iter()
            .filter(|bind| same_chord(bind, stale.modmask, &stale.key))
            .count();
        if chord_users != 1 {
            return Err(OtoError::Message(
                "cannot replace Oto's old Hyprland hotkey because that chord has another binding"
                    .into(),
            ));
        }
    }

    let target_exists = target_binds.iter().any(|bind| is_oto(bind));
    let modifier_text = parts.modifiers.join(" ");
    if !target_exists {
        run_hyprland_keyword(
            "bind",
            &format!("{modifier_text},{target_key},global,{PORTAL_ACTION}"),
        )?;
    }

    for stale in stale_oto {
        let stale_modifiers = hyprland_modifiers_from_mask(stale.modmask)?;
        if let Err(error) =
            run_hyprland_keyword("unbind", &format!("{stale_modifiers},{}", stale.key))
        {
            if !target_exists {
                let _ = run_hyprland_keyword("unbind", &format!("{modifier_text},{target_key}"));
            }
            return Err(error);
        }
    }

    eprintln!("oto hotkey: Hyprland binding active: {hotkey} → {PORTAL_ACTION}");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn register_wayland_ptt<R: Runtime>(app: &AppHandle<R>, normalized: &str) -> OtoResult<()> {
    let state = app
        .try_state::<HotkeyManager>()
        .ok_or_else(|| OtoError::Message("HotkeyManager missing".into()))?;
    let mut registration = state.wayland.lock().await;

    // Settings autosave can submit the unchanged config while the shortcut is
    // held. Do not reset the pressed gate or touch the live portal session.
    if registration
        .as_ref()
        .is_some_and(|current| current.hotkey == normalized)
    {
        return Ok(());
    }

    // Hyprland ignores preferred_trigger, so changing the user-facing chord
    // only requires updating its runtime bind; keep the portal session alive.
    if is_hyprland_session() && registration.is_some() {
        configure_hyprland_binding(normalized)?;
        state.pressed.store(false, Ordering::SeqCst);
        if let Some(current) = registration.as_mut() {
            current.hotkey = normalized.to_string();
        }
        return Ok(());
    }

    state.pressed.store(false, Ordering::SeqCst);
    if let Some(old) = registration.take() {
        old.activated_task.abort();
        old.deactivated_task.abort();
        let _ = old.session.close().await;
    }

    ensure_development_desktop_file()?;
    let connection = ashpd::zbus::Connection::session()
        .await
        .map_err(|error| OtoError::Message(format!("Wayland portal connection failed: {error}")))?;
    let app_id = AppID::try_from(PORTAL_APP_ID)
        .map_err(|error| OtoError::Message(format!("invalid portal app ID: {error}")))?;
    register_host_app_with_connection(connection.clone(), app_id)
        .await
        .map_err(|error| {
            OtoError::Message(format!("Wayland portal app registration failed: {error}"))
        })?;

    let portal = GlobalShortcuts::with_connection(connection)
        .await
        .map_err(|error| {
            OtoError::Message(format!("GlobalShortcuts portal unavailable: {error}"))
        })?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(|error| {
            OtoError::Message(format!("could not create shortcut session: {error}"))
        })?;
    let session_handle = serde_json::to_value(&session)
        .map_err(|error| OtoError::Message(format!("invalid portal session: {error}")))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| OtoError::Message("portal returned a non-string session handle".into()))?;
    let mut activated = portal.receive_activated().await.map_err(|error| {
        OtoError::Message(format!("could not listen for shortcut press: {error}"))
    })?;
    let mut deactivated = portal.receive_deactivated().await.map_err(|error| {
        OtoError::Message(format!("could not listen for shortcut release: {error}"))
    })?;

    let preferred_trigger = portal_trigger(normalized)?;
    let shortcut = NewShortcut::new(PORTAL_SHORTCUT_ID, "Start or stop Oto dictation")
        .preferred_trigger(preferred_trigger.as_str());
    let response = portal
        .bind_shortcuts(
            &session,
            &[shortcut],
            None::<&WindowIdentifier>,
            BindShortcutsOptions::default(),
        )
        .await
        .and_then(|request| request.response())
        .map_err(|error| OtoError::Message(format!("could not bind Wayland shortcut: {error}")))?;
    if !response
        .shortcuts()
        .iter()
        .any(|shortcut| shortcut.id() == PORTAL_SHORTCUT_ID)
    {
        let _ = session.close().await;
        return Err(OtoError::Message(
            "Wayland portal did not accept the dictation shortcut".into(),
        ));
    }

    if is_hyprland_session() {
        if let Err(error) = configure_hyprland_binding(normalized) {
            let _ = session.close().await;
            return Err(error);
        }
    }

    let pressed_app = app.clone();
    let pressed_session = session_handle.clone();
    let activated_task = tauri::async_runtime::spawn(async move {
        while let Some(event) = activated.next().await {
            if event.session_handle().as_str() == pressed_session
                && event.shortcut_id() == PORTAL_SHORTCUT_ID
            {
                dispatch_ptt_event(&pressed_app, ShortcutState::Pressed);
            }
        }
        eprintln!("oto hotkey: Wayland activation stream ended");
    });
    let released_app = app.clone();
    let released_session = session_handle;
    let deactivated_task = tauri::async_runtime::spawn(async move {
        while let Some(event) = deactivated.next().await {
            if event.session_handle().as_str() == released_session
                && event.shortcut_id() == PORTAL_SHORTCUT_ID
            {
                dispatch_ptt_event(&released_app, ShortcutState::Released);
            }
        }
        eprintln!("oto hotkey: Wayland deactivation stream ended");
    });

    *registration = Some(WaylandRegistration {
        hotkey: normalized.to_string(),
        session,
        activated_task,
        deactivated_task,
    });
    eprintln!("oto hotkey: Wayland portal shortcut active: {normalized}");
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
        assert!(parse_hotkey("Ctrl+A+B").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn formats_portal_trigger() {
        assert_eq!(
            portal_trigger("Ctrl+Shift+Space").unwrap(),
            "CTRL+SHIFT+space"
        );
        assert_eq!(portal_trigger("Alt+D").unwrap(), "ALT+d");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn maps_hyprland_modifiers() {
        let parts = hotkey_parts("Ctrl+Alt+Shift+Super+D").unwrap();
        assert_eq!(hyprland_modmask(&parts), 77);
        assert_eq!(
            hyprland_modifiers_from_mask(77).unwrap(),
            "CTRL ALT SHIFT SUPER"
        );
    }
}
