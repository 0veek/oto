//! Login autostart via the XDG Autostart specification.
//!
//! Enabled state is stored in [`crate::config::AppConfig::autostart_enabled`].
//! When on, Oto installs `~/.config/autostart/dev.oto.app.desktop` pointing at
//! the currently running executable so AppImage/dev builds keep working.

use crate::error::{OtoError, OtoResult};

const AUTOSTART_DESKTOP_ID: &str = "dev.oto.app.desktop";
/// CLI flag written into the XDG autostart `Exec=` line so login launches stay
/// tray-only (settings stays hidden until the user opens it from the tray).
pub const AUTOSTART_FLAG: &str = "--autostart";

/// True when this process was started by the login autostart entry.
pub fn launched_from_autostart() -> bool {
    std::env::args().any(|arg| arg == AUTOSTART_FLAG || arg == "--background")
}

/// Create or remove the XDG autostart entry to match `enabled`.
pub fn apply(enabled: bool) -> OtoResult<()> {
    #[cfg(target_os = "linux")]
    {
        if enabled {
            enable()
        } else {
            disable()
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = enabled;
        Ok(())
    }
}

/// Refresh the desktop file path when autostart is already enabled (e.g. after
/// the binary moved). No-op when disabled or the entry is missing.
pub fn refresh_if_enabled(enabled: bool) {
    if !enabled {
        return;
    }
    if let Err(error) = apply(true) {
        eprintln!("oto autostart: could not refresh desktop entry: {error}");
    }
}

#[cfg(target_os = "linux")]
fn autostart_dir() -> OtoResult<std::path::PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or_else(|| OtoError::Message("could not locate XDG config directory".into()))?;
    Ok(config_home.join("autostart"))
}

#[cfg(target_os = "linux")]
fn autostart_desktop_path() -> OtoResult<std::path::PathBuf> {
    Ok(autostart_dir()?.join(AUTOSTART_DESKTOP_ID))
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
fn desktop_entry_for(executable: &std::path::Path) -> String {
    let exec = shell_quote_desktop_exec(executable);
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Oto\n\
         Comment=System-wide push-to-talk voice dictation\n\
         Exec={exec} {AUTOSTART_FLAG}\n\
         Terminal=false\n\
         Categories=Utility;Accessibility;\n\
         Keywords=dictation;voice;speech;transcription;accessibility;\n\
         StartupNotify=false\n\
         StartupWMClass=oto\n\
         X-GNOME-Autostart-enabled=true\n\
         X-KDE-autostart-after=panel\n"
    )
}

#[cfg(target_os = "linux")]
fn enable() -> OtoResult<()> {
    use std::fs;

    let executable = std::env::current_exe().map_err(|error| {
        OtoError::Message(format!("could not resolve Oto executable path: {error}"))
    })?;
    // Prefer the canonical path so symlink launches still restart the real binary.
    let executable = executable.canonicalize().unwrap_or(executable);

    let dir = autostart_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(AUTOSTART_DESKTOP_ID);
    let entry = desktop_entry_for(&executable);

    let needs_write = match fs::read_to_string(&path) {
        Ok(existing) => existing != entry,
        Err(_) => true,
    };
    if needs_write {
        fs::write(&path, entry)?;
        eprintln!("oto autostart: enabled at {}", path.display());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable() -> OtoResult<()> {
    use std::fs;

    let path = autostart_desktop_path()?;
    match fs::remove_file(&path) {
        Ok(()) => {
            eprintln!("oto autostart: removed {}", path.display());
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(OtoError::Message(format!(
            "could not remove autostart entry {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn desktop_entry_includes_exec_and_autostart_flag() {
        let entry = desktop_entry_for(Path::new("/usr/bin/oto"));
        assert!(entry.contains(&format!("Exec=/usr/bin/oto {AUTOSTART_FLAG}\n")));
        assert!(entry.contains("X-GNOME-Autostart-enabled=true"));
        assert!(entry.contains("Type=Application"));
        assert!(entry.contains("Name=Oto"));
    }

    #[test]
    fn quotes_paths_with_spaces() {
        let entry = desktop_entry_for(Path::new("/opt/My Apps/oto"));
        assert!(entry.contains(&format!("Exec=\"/opt/My Apps/oto\" {AUTOSTART_FLAG}\n")));
    }

    #[test]
    fn quotes_appimage_style_paths() {
        let path = Path::new("/home/user/.local/bin/Oto_0.1.0_amd64.AppImage");
        let entry = desktop_entry_for(path);
        assert!(entry.contains(&format!(
            "Exec=/home/user/.local/bin/Oto_0.1.0_amd64.AppImage {AUTOSTART_FLAG}\n"
        )));
    }
}
