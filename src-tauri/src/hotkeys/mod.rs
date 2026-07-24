use crate::config::load_config;
use crate::error::{OtoError, OtoResult};
use crate::state::AppState;
use serde::Serialize;
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
use std::{fs, path::PathBuf, process::Command};

const PORTAL_APP_ID: &str = "dev.oto.app";
const PORTAL_SHORTCUT_ID: &str = "dictation";
const PORTAL_ACTION: &str = "dev.oto.app:dictation";

/// Desktop / portal capability snapshot for the Hotkeys settings panel.
#[derive(Debug, Clone, Serialize)]
pub struct HotkeyDesktopStatus {
    /// `wayland`, `x11`, or `unknown`.
    pub session: String,
    /// Raw `XDG_CURRENT_DESKTOP` (may be empty or colon-joined).
    pub desktop: String,
    /// True when push-to-talk can use a global shortcut backend.
    pub global_shortcuts_available: bool,
    /// Whether the current desktop is COSMIC.
    pub is_cosmic: bool,
    /// Whether the current session is Hyprland.
    pub is_hyprland: bool,
    /// Human-readable guidance when global shortcuts cannot work.
    pub warning: Option<String>,
    /// Short label of the recommended portal package for this desktop, if any.
    pub portal_hint: Option<String>,
}

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
    // If the new bind fails, try to re-apply the previously saved hotkey so PTT
    // is not left completely unbound for the rest of the session.
    let previous_hotkey = load_config()
        .ok()
        .map(|cfg| normalize_hotkey(&cfg.hotkey))
        .filter(|h| h != normalized);
    let _ = unregister_all_hotkeys(app);

    if let Err(error) = app.global_shortcut().on_shortcut(shortcut, |app, sc, event| {
        eprintln!(
            "oto hotkey event: {:?} state={:?} id={}",
            sc,
            event.state(),
            sc.id()
        );
        dispatch_ptt_event(app, event.state());
    }) {
        if let Some(previous) = previous_hotkey.as_deref() {
            if let Ok(prev_sc) = parse_hotkey(previous) {
                let _ = app.global_shortcut().on_shortcut(prev_sc, |app, _sc, event| {
                    dispatch_ptt_event(app, event.state());
                });
                eprintln!(
                    "oto hotkey: re-registered previous hotkey {previous} after failed bind of {normalized}"
                );
            }
        }
        return Err(OtoError::Message(format!(
            "failed to register hotkey '{normalized}': {error}"
        )));
    }

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
fn is_x11_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("x11"))
        || std::env::var_os("DISPLAY").is_some()
}

#[cfg(target_os = "linux")]
fn current_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn is_hyprland_session() -> bool {
    current_desktop()
        .to_ascii_lowercase()
        .contains("hyprland")
        || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

#[cfg(target_os = "linux")]
fn is_cosmic_session() -> bool {
    current_desktop().to_ascii_lowercase().contains("cosmic")
}

#[cfg(target_os = "linux")]
fn is_gnome_session() -> bool {
    // Avoid treating COSMIC (or other non-GNOME desktops) as GNOME when both
    // appear in a colon-joined XDG_CURRENT_DESKTOP value.
    if is_cosmic_session() {
        return false;
    }
    current_desktop().to_ascii_lowercase().contains("gnome")
}

#[cfg(target_os = "linux")]
fn is_kde_session() -> bool {
    let desktop = current_desktop().to_ascii_lowercase();
    desktop.contains("kde") || desktop.contains("plasma")
}

/// Probe whether `org.freedesktop.portal.GlobalShortcuts` is exposed on the
/// session bus. Presence of the interface is what binds need — not merely a
/// running `xdg-desktop-portal` process.
#[cfg(target_os = "linux")]
async fn portal_has_global_shortcuts() -> bool {
    let Ok(connection) = ashpd::zbus::Connection::session().await else {
        return false;
    };
    let reply = connection
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.DBus.Introspectable"),
            "Introspect",
            &(),
        )
        .await;
    match reply {
        Ok(message) => message
            .body()
            .deserialize::<String>()
            .map(|xml| xml.contains("org.freedesktop.portal.GlobalShortcuts"))
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// Build the warning shown when Wayland GlobalShortcuts cannot bind.
#[cfg(target_os = "linux")]
fn global_shortcuts_warning(is_cosmic: bool, is_hyprland: bool, is_gnome: bool, is_kde: bool) -> String {
    if is_cosmic {
        return "COSMIC does not implement the GlobalShortcuts portal yet, so Oto cannot bind a system-wide push-to-talk key. Use tray → Start Listening / Stop Listening to dictate. Global shortcuts work on GNOME, KDE Plasma, and Hyprland with their matching portal packages.".into();
    }
    if is_hyprland {
        return "GlobalShortcuts portal is missing. On Hyprland install and run xdg-desktop-portal-hyprland, then restart portals and re-save this hotkey. Until then use tray → Start Listening.".into();
    }
    if is_gnome {
        return "GlobalShortcuts portal is missing. On GNOME install xdg-desktop-portal-gnome, restart the portal user service, and re-save this hotkey. Until then use tray → Start Listening.".into();
    }
    if is_kde {
        return "GlobalShortcuts portal is missing. On KDE install xdg-desktop-portal-kde, restart portals, and re-save this hotkey. Until then use tray → Start Listening.".into();
    }
    "GlobalShortcuts portal is unavailable on this desktop. Install the portal backend for your compositor (GNOME: xdg-desktop-portal-gnome, KDE: xdg-desktop-portal-kde, Hyprland: xdg-desktop-portal-hyprland), or use tray → Start Listening / Stop Listening.".into()
}

#[cfg(target_os = "linux")]
fn portal_package_hint(is_cosmic: bool, is_hyprland: bool, is_gnome: bool, is_kde: bool) -> Option<String> {
    if is_cosmic {
        // No supported package yet — tell the UI not to recommend a wrong install.
        return None;
    }
    if is_hyprland {
        return Some("xdg-desktop-portal-hyprland".into());
    }
    if is_gnome {
        return Some("xdg-desktop-portal-gnome".into());
    }
    if is_kde {
        return Some("xdg-desktop-portal-kde".into());
    }
    None
}

/// Report session type, desktop identity, and GlobalShortcuts availability for
/// the settings UI. Safe to call often (one D-Bus introspect on Wayland).
pub async fn hotkey_desktop_status() -> HotkeyDesktopStatus {
    #[cfg(not(target_os = "linux"))]
    {
        return HotkeyDesktopStatus {
            session: "unknown".into(),
            desktop: String::new(),
            global_shortcuts_available: true,
            is_cosmic: false,
            is_hyprland: false,
            warning: None,
            portal_hint: None,
        };
    }

    #[cfg(target_os = "linux")]
    {
        let wayland = is_wayland_session();
        let session = if wayland {
            "wayland"
        } else if is_x11_session() {
            "x11"
        } else {
            "unknown"
        }
        .to_string();
        let desktop = current_desktop();
        let is_cosmic = is_cosmic_session();
        let is_hyprland = is_hyprland_session();
        let is_gnome = is_gnome_session();
        let is_kde = is_kde_session();

        // X11 uses the native grab path — portal GlobalShortcuts is not required.
        let global_shortcuts_available = if wayland {
            portal_has_global_shortcuts().await
        } else {
            true
        };

        let warning = if wayland && !global_shortcuts_available {
            Some(global_shortcuts_warning(
                is_cosmic,
                is_hyprland,
                is_gnome,
                is_kde,
            ))
        } else {
            None
        };
        let portal_hint = if wayland && !global_shortcuts_available {
            portal_package_hint(is_cosmic, is_hyprland, is_gnome, is_kde)
        } else {
            None
        };

        HotkeyDesktopStatus {
            session,
            desktop,
            global_shortcuts_available,
            is_cosmic,
            is_hyprland,
            warning,
            portal_hint,
        }
    }
}

/// Ensure `dev.oto.app.desktop` is visible to xdg-desktop-portal.
///
/// Host apps must call the portal Registry with an app ID that resolves via
/// `Gio.DesktopAppInfo` (filename `{app-id}.desktop`). Tauri AppImages install
/// a differently named launcher (`Oto_…desktop`), and unpackaged binaries have
/// no launcher at all — both fail with "App info not found for 'dev.oto.app'".
#[cfg(target_os = "linux")]
fn ensure_portal_desktop_file() -> OtoResult<()> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(dirs::data_dir)
        .ok_or_else(|| OtoError::Message("could not locate XDG data directory".into()))?;
    let applications = data_home.join("applications");
    fs::create_dir_all(&applications)?;
    let desktop_file = applications.join(format!("{PORTAL_APP_ID}.desktop"));

    let executable = resolve_host_executable()?;
    // Quote the path so spaces/AppImage paths survive desktop-entry parsing.
    let exec = shell_quote_desktop_exec(&executable);
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Oto\n\
         Comment=System-wide push-to-talk voice dictation\n\
         Exec={exec}\n\
         Terminal=false\n\
         Categories=Utility;Accessibility;\n\
         Keywords=dictation;voice;speech;transcription;accessibility;\n\
         StartupNotify=true\n\
         StartupWMClass=oto\n\
         NoDisplay=true\n"
    );

    let needs_write = match fs::read_to_string(&desktop_file) {
        Ok(existing) => !existing.contains(executable.to_string_lossy().as_ref()),
        Err(_) => true,
    };
    if needs_write {
        fs::write(&desktop_file, entry)?;
        eprintln!(
            "oto hotkey: installed portal app identity at {}",
            desktop_file.display()
        );
        // Best-effort: refresh the MIME/app cache so portals see the new file.
        let _ = Command::new("update-desktop-database")
            .arg(&applications)
            .status();
    }
    Ok(())
}

/// Prefer `$APPIMAGE` when running from an AppImage so desktop entries point at
/// the persistent launcher, not the transient `/tmp/.mount_*` runtime path.
#[cfg(target_os = "linux")]
fn resolve_host_executable() -> OtoResult<PathBuf> {
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        let path = PathBuf::from(appimage);
        if path.is_file() {
            return Ok(path);
        }
    }
    let executable = std::env::current_exe()?;
    Ok(executable.canonicalize().unwrap_or(executable))
}

/// Quote a path for a desktop-entry `Exec=` line.
#[cfg(target_os = "linux")]
fn shell_quote_desktop_exec(path: &std::path::Path) -> String {
    let raw = path.display().to_string();
    if raw.chars().any(|c| c.is_whitespace() || "\"'\\$`".contains(c)) {
        format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        raw
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
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
    // Ignore unknown modifier bits rather than aborting hotkey registration.
    // Hyprland may set extra flags we do not model; known bits still unbind correctly.
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

/// Format a Hyprland `unbind` argument. Modifier-less chords must be `key`
/// (not `,key`) — a leading comma is invalid hyprctl syntax.
#[cfg(target_os = "linux")]
fn hyprland_unbind_spec(modifiers: &str, key: &str) -> String {
    if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{modifiers},{key}")
    }
}

/// Parse plain-text `hyprctl binds` output.
///
/// Hyprland 0.56 ships a broken `binds -j` encoder (field values are shifted and
/// strings are often unquoted), so JSON is unusable. The text format remains stable:
///
/// ```text
/// bindd
/// modmask: 64
/// submap:
/// key: D
/// keycode: 0
/// catchall: false
/// description: App launcher
/// dispatcher: exec
/// arg: rofi -show drun
/// ```
#[cfg(target_os = "linux")]
fn parse_hyprland_binds_plain(text: &str) -> Vec<HyprlandBind> {
    let mut binds = Vec::new();
    let mut modmask: Option<u32> = None;
    let mut key: Option<String> = None;
    let mut dispatcher: Option<String> = None;
    let mut arg: Option<String> = None;
    let mut in_bind = false;

    let flush = |binds: &mut Vec<HyprlandBind>,
                 modmask: &mut Option<u32>,
                 key: &mut Option<String>,
                 dispatcher: &mut Option<String>,
                 arg: &mut Option<String>| {
        if let (Some(mask), Some(k), Some(disp), Some(a)) =
            (modmask.take(), key.take(), dispatcher.take(), arg.take())
        {
            binds.push(HyprlandBind {
                modmask: mask,
                key: k,
                dispatcher: disp,
                arg: a,
            });
        } else {
            *modmask = None;
            *key = None;
            *dispatcher = None;
            *arg = None;
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        // Bind headers look like `bind`, `bindd`, `bindlnd`, … and have no `:`.
        if trimmed.starts_with("bind") && !trimmed.contains(':') {
            if in_bind {
                flush(
                    &mut binds,
                    &mut modmask,
                    &mut key,
                    &mut dispatcher,
                    &mut arg,
                );
            }
            in_bind = true;
            continue;
        }
        if !in_bind {
            continue;
        }
        let Some((field, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match field.trim() {
            "modmask" => {
                modmask = value.parse().ok();
            }
            "key" => {
                key = Some(value.to_string());
            }
            "dispatcher" => {
                dispatcher = Some(value.to_string());
            }
            "arg" => {
                // Keep empty args as Some("") so incomplete records can flush.
                arg = Some(value.to_string());
            }
            _ => {}
        }
    }
    if in_bind {
        flush(
            &mut binds,
            &mut modmask,
            &mut key,
            &mut dispatcher,
            &mut arg,
        );
    }
    binds
}

#[cfg(target_os = "linux")]
fn hyprland_binds() -> OtoResult<Vec<HyprlandBind>> {
    // Prefer plain text: Hyprland 0.56's `binds -j` emits invalid JSON.
    let output = Command::new("hyprctl")
        .args(["binds"])
        .output()
        .map_err(|error| OtoError::Message(format!("failed to run hyprctl: {error}")))?;
    if !output.status.success() {
        return Err(OtoError::Message(format!(
            "hyprctl binds failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let binds = parse_hyprland_binds_plain(&text);
    if binds.is_empty() && !text.trim().is_empty() {
        return Err(OtoError::Message(
            "invalid hyprctl binds response: could not parse any binds from plain-text output"
                .into(),
        ));
    }
    Ok(binds)
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
        let unbind_spec = hyprland_unbind_spec(&stale_modifiers, &stale.key);
        if let Err(error) = run_hyprland_keyword("unbind", &unbind_spec) {
            if !target_exists {
                let rollback = hyprland_unbind_spec(&modifier_text, &target_key);
                let _ = run_hyprland_keyword("unbind", &rollback);
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

    // Desktops without GlobalShortcuts (notably COSMIC today) cannot bind a
    // system-wide PTT key. Soft-skip so settings save still works — but never
    // tear down a live session first (that permanently disables PTT mid-run).
    if !portal_has_global_shortcuts().await {
        let warning = global_shortcuts_warning(
            is_cosmic_session(),
            is_hyprland_session(),
            is_gnome_session(),
            is_kde_session(),
        );
        if registration.is_some() {
            eprintln!(
                "oto hotkey: portal unavailable; keeping existing binding for {}",
                registration
                    .as_ref()
                    .map(|r| r.hotkey.as_str())
                    .unwrap_or("?")
            );
            // Surface failure when the user is trying to change the chord.
            return Err(OtoError::Message(format!(
                "GlobalShortcuts portal unavailable; kept previous hotkey. {warning}"
            )));
        }
        eprintln!("oto hotkey: skipping bind — {warning}");
        return Ok(());
    }

    ensure_portal_desktop_file()?;
    let connection = ashpd::zbus::Connection::session()
        .await
        .map_err(|error| OtoError::Message(format!("Wayland portal connection failed: {error}")))?;
    let app_id = AppID::try_from(PORTAL_APP_ID)
        .map_err(|error| OtoError::Message(format!("invalid portal app ID: {error}")))?;
    register_host_app_with_connection(connection.clone(), app_id)
        .await
        .map_err(|error| {
            OtoError::Message(format!(
                "Wayland portal app registration failed: {error}. \
                 Ensure ~/.local/share/applications/{PORTAL_APP_ID}.desktop exists \
                 (AppImage/dev installs need this so the portal can resolve the app ID), \
                 then restart xdg-desktop-portal and re-save the hotkey."
            ))
        })?;

    let portal = GlobalShortcuts::with_connection(connection)
        .await
        .map_err(|error| {
            OtoError::Message(format!(
                "GlobalShortcuts portal unavailable: {error}. \
                 Your desktop portal backend must implement org.freedesktop.portal.GlobalShortcuts \
                 (GNOME: xdg-desktop-portal-gnome; KDE: xdg-desktop-portal-kde; \
                 Hyprland: xdg-desktop-portal-hyprland). \
                 COSMIC currently has no GlobalShortcuts backend — use the tray Start/Stop controls."
            ))
        })?;

    // Open the new portal session before closing the old one so a failed bind
    // can leave the previous registration intact (when the portal allows two
    // sequential sessions). We still must close the old session after success
    // because most portals only keep one host-app binding set active.
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

    // New bind succeeded — now retire the previous portal session.
    if let Some(old) = registration.take() {
        old.activated_task.abort();
        old.deactivated_task.abort();
        let _ = old.session.close().await;
    }

    state.pressed.store(false, Ordering::SeqCst);

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
    fn quotes_desktop_exec_paths() {
        assert_eq!(
            shell_quote_desktop_exec(std::path::Path::new("/usr/bin/oto")),
            "/usr/bin/oto"
        );
        assert_eq!(
            shell_quote_desktop_exec(std::path::Path::new(
                "/home/user/.local/bin/Oto_0.1.0_amd64.AppImage"
            )),
            "/home/user/.local/bin/Oto_0.1.0_amd64.AppImage"
        );
        assert_eq!(
            shell_quote_desktop_exec(std::path::Path::new("/opt/My Apps/oto")),
            "\"/opt/My Apps/oto\""
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cosmic_warning_mentions_tray_fallback() {
        let warning = global_shortcuts_warning(true, false, false, false);
        assert!(warning.to_ascii_lowercase().contains("cosmic"));
        assert!(warning.contains("Start Listening") || warning.contains("tray"));
        assert!(portal_package_hint(true, false, false, false).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn hyprland_hint_names_portal_package() {
        let warning = global_shortcuts_warning(false, true, false, false);
        assert!(warning.contains("xdg-desktop-portal-hyprland"));
        assert_eq!(
            portal_package_hint(false, true, false, false).as_deref(),
            Some("xdg-desktop-portal-hyprland")
        );
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
        // SUPER + SHIFT (65) must not abort; unknown high bits are ignored.
        assert_eq!(hyprland_modifiers_from_mask(65).unwrap(), "SHIFT SUPER");
        assert_eq!(hyprland_unbind_spec("", "F12"), "F12");
        assert_eq!(hyprland_unbind_spec("CTRL", "Space"), "CTRL,Space");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_hyprland_binds_plain_text() {
        let sample = "\
bindd
modmask: 64
submap: 
key: D
keycode: 0
catchall: false
description: App launcher
dispatcher: exec
arg: rofi -show drun

bind
modmask: 5
submap: 
key: Space
keycode: 0
catchall: false
description: 
dispatcher: global
arg: dev.oto.app:dictation

bindmd
modmask: 64
submap: 
key: mouse:272
keycode: 0
catchall: false
description: Move window
dispatcher: mouse
arg: movewindow
";
        let binds = parse_hyprland_binds_plain(sample);
        assert_eq!(binds.len(), 3);
        assert_eq!(
            binds[0],
            HyprlandBind {
                modmask: 64,
                key: "D".into(),
                dispatcher: "exec".into(),
                arg: "rofi -show drun".into(),
            }
        );
        assert_eq!(
            binds[1],
            HyprlandBind {
                modmask: 5,
                key: "Space".into(),
                dispatcher: "global".into(),
                arg: "dev.oto.app:dictation".into(),
            }
        );
        assert_eq!(binds[2].key, "mouse:272");
        assert_eq!(binds[2].dispatcher, "mouse");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_hyprland_binds_empty_arg_and_skips_incomplete() {
        let sample = "\
bindd
modmask: 8
submap: 
key: Tab
keycode: 0
catchall: false
description: Bring active to top
dispatcher: bringactivetotop
arg: 

bind
modmask: not-a-number
key: X
dispatcher: exec
arg: fail

";
        let binds = parse_hyprland_binds_plain(sample);
        assert_eq!(binds.len(), 1);
        assert_eq!(binds[0].key, "Tab");
        assert_eq!(binds[0].arg, "");
        assert_eq!(binds[0].dispatcher, "bringactivetotop");
    }
}
