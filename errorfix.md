# Oto error fixes — Wayland / GNOME

Practical issues and fixes for running Oto under **GNOME Shell on Wayland** (Mutter), and for Wayland problems that also show up on GNOME.

> Desktop integration still varies by portal version, accessibility tree, and target app. Prefer the tray and **Test insertion** checks before chasing edge cases.

---

## Quick environment check

Confirm you are on GNOME Wayland, then verify the services Oto depends on:

```bash
echo "session=$XDG_SESSION_TYPE desktop=$XDG_CURRENT_DESKTOP"
echo "wayland=$WAYLAND_DISPLAY"

# Portal stack (GNOME uses xdg-desktop-portal-gnome)
systemctl --user status xdg-desktop-portal.service xdg-desktop-portal-gnome.service

# Accessibility bus (AT-SPI text insertion)
busctl --user status org.a11y.Bus 2>/dev/null || true

# Secret Service (API keys)
busctl --user status org.freedesktop.secrets 2>/dev/null || true

# Injection tools
which ydotool ydotoold wtype wl-copy wl-paste 2>/dev/null
systemctl --user status ydotool.service 2>/dev/null || true

# Diagnostic log written by the injector
ls -la "/tmp/oto-inject-${USER}.log" 2>/dev/null
```

| Component | Why Oto needs it on GNOME Wayland |
| --- | --- |
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | GlobalShortcuts portal for push-to-talk |
| AT-SPI / `org.a11y.Bus` | Direct text insert into accessible apps |
| `ydotool` + **user** `ydotoold` | Reliable synthetic keyboard for paste/type |
| `wtype` | Optional fallback typer (often weaker on GNOME) |
| `wl-clipboard` (`wl-copy` / `wl-paste`) | Helpful for clipboard diagnostics |
| GNOME Keyring / Secret Service | API keys (not stored in `config.json`) |
| `input` group + `/dev/uinput` | Lets `ydotoold` inject keys |

**Distro package hints**

```bash
# Fedora
sudo dnf install ydotool wtype wl-clipboard libsecret

# Debian / Ubuntu
sudo apt install ydotool wtype wl-clipboard libsecret-1-0

# Arch
sudo pacman -S ydotool wtype wl-clipboard libsecret
```

GNOME’s portal backend is normally installed with the desktop; if GlobalShortcuts fails, ensure `xdg-desktop-portal-gnome` is present and the user services are active.

---

## Issue index

1. [Hotkey does nothing / overlay never appears](#1-hotkey-does-nothing--overlay-never-appears)
2. [Portal or “could not bind Wayland shortcut” on save](#2-portal-or-could-not-bind-wayland-shortcut-on-save)
3. [Hotkey conflicts with GNOME Shell or IME](#3-hotkey-conflicts-with-gnome-shell-or-ime)
4. [Text is transcribed but not inserted](#4-text-is-transcribed-but-not-inserted)
5. [`ydotool.service` not found or daemon not running](#5-ydotoolservice-not-found-or-daemon-not-running)
6. [Permission denied on `/dev/uinput` (input group)](#6-permission-denied-on-devuinput-input-group)
7. [`wtype` “succeeds” but nothing is typed](#7-wtype-succeeds-but-nothing-is-typed)
8. [AT-SPI insertion fails on GNOME](#8-at-spi-insertion-fails-on-gnome)
9. [Clipboard has text but Ctrl+V never fires](#9-clipboard-has-text-but-ctrlv-never-fires)
10. [Focus lands in the wrong window after dictation](#10-focus-lands-in-the-wrong-window-after-dictation)
11. [API key / keyring errors under GNOME](#11-api-key--keyring-errors-under-gnome)
12. [Microphone permission or empty capture](#12-microphone-permission-or-empty-capture)
13. [Flatpak on GNOME: insertion or host tools missing](#13-flatpak-on-gnome-insertion-or-host-tools-missing)
14. [Dev builds: portal app identity / desktop file](#14-dev-builds-portal-app-identity--desktop-file)
15. [How to read Oto’s injection log](#15-how-to-read-otos-injection-log)

---

## 1. Hotkey does nothing / overlay never appears

### Symptoms

- Holding the configured shortcut does nothing.
- Tray **Start Listening** works (mic + overlay OK).
- Settings save may succeed, but press/release never start recording.

### Cause

On Wayland, Oto does **not** grab keys natively. It binds through the **XDG GlobalShortcuts** portal (`org.freedesktop.portal.GlobalShortcuts`). If the portal session is missing, denied, or the chord is reserved by GNOME Shell, press events never reach Oto.

### Solution

1. Confirm session type:
   ```bash
   echo $XDG_SESSION_TYPE   # should be wayland
   ```
2. Ensure portal services:
   ```bash
   systemctl --user restart xdg-desktop-portal.service xdg-desktop-portal-gnome.service
   systemctl --user status xdg-desktop-portal-gnome.service
   ```
3. In Oto: set hotkey to `Ctrl+Shift+Space` or `Ctrl+Shift+D`, click **Save**, restart Oto.
4. When GNOME prompts to allow a global shortcut for Oto, **accept** it (Settings → *Keyboard* / *Apps* may also list portal shortcuts on newer GNOME).
5. Use tray → **Start Listening** / **Stop Listening** as an always-available fallback.
6. Preview the overlay alone: **Appearance → Preview listening**.

If tray works but the hotkey does not, treat it as a **shortcut registration** problem, not mic or STT.

---

## 2. Portal or “could not bind Wayland shortcut” on save

### Symptoms

- Saving Hotkeys fails with messages such as:
  - `Wayland portal connection failed`
  - `GlobalShortcuts portal unavailable`
  - `could not bind Wayland shortcut`
  - `Wayland portal did not accept the dictation shortcut`
- Oto restores the last working shortcut automatically.

### Cause

- `xdg-desktop-portal` / `xdg-desktop-portal-gnome` not running.
- Portal implementation too old for GlobalShortcuts.
- App ID / desktop entry not visible to the portal (common in raw `cargo`/`tauri dev` launches).
- User dismissed the GNOME permission dialog.

### Solution

```bash
# Restart portal stack after package updates
systemctl --user restart xdg-desktop-portal.service xdg-desktop-portal-gnome.service

# Confirm the GNOME backend is the one answering
systemctl --user status xdg-desktop-portal-gnome.service
```

- Install/update `xdg-desktop-portal` and `xdg-desktop-portal-gnome` from your distro.
- Prefer a packaged install (`.deb` / AppImage / Flatpak) so `dev.oto.app.desktop` is on the XDG data path.
- For `tauri dev`, Oto installs a development desktop entry under `~/.local/share/applications/dev.oto.app.desktop` when needed — re-run the app once, then retry Save.
- Log out and back in after installing portal packages.
- Keep using tray Start/Stop until the portal bind succeeds.

---

## 3. Hotkey conflicts with GNOME Shell or IME

### Symptoms

- Shortcut triggers a GNOME action (Activities, window menu, layout switch) instead of Oto.
- Chord works in isolation but fails when an IME is active.

### Cause

GNOME Shell reserves many **Super** combos. Input methods often own chords like `Ctrl+Alt+Space`. Oto will not steal keys already taken by the compositor or IME.

### Solution

Avoid:

| Chord | Typical conflict |
| --- | --- |
| `Super+…` | GNOME Shell overview, app grid, workspace |
| `Alt+Space` | Window menu |
| `Ctrl+Alt+Space` | Input method / layout |

Prefer:

- `Ctrl+Shift+Space` (default)
- `Ctrl+Shift+D`
- Other free `Ctrl+Shift+…` chords

Also check **Settings → Keyboard → Keyboard Shortcuts** (and any extension that remaps keys) and free the chord you want Oto to use.

---

## 4. Text is transcribed but not inserted

### Symptoms

- Overlay shows Listening → Processing → Done.
- Transcript appears (or history shows it), but the focused app stays empty.
- Sometimes only clipboard updates; paste never happens.

### Cause

GNOME Wayland blocks unrestricted global key injection. Oto’s chain is:

1. **AT-SPI** `EditableText` on the focused accessible object  
2. **Clipboard + simulated Ctrl+V** (`ydotool` first, then `wtype`)  
3. **Direct typing** (same tools)  
4. **Clipboard only** (user pastes manually)

If tools/daemons are missing, or the target app rejects synthetic input and AT-SPI, only the last step remains.

### Solution

1. Focus a normal text field (gedit, Browser address bar, LibreOffice) — **not** Oto Settings.
2. Settings → **Injection → Test insertion**.
3. Install and start Wayland typing tools (see issues 5–7).
4. Try modes in order:
   - **Auto** (default)
   - **Clipboard + paste** when typing is flaky
   - **Direct type** for clipboard-hostile apps
   - **Clipboard only** when the app blocks all synthetic input — then press Ctrl+V yourself
5. Inspect `/tmp/oto-inject-$USER.log` for which step failed (see [§15](#15-how-to-read-otos-injection-log)).

Keep focus on the target field for the whole hold-to-talk cycle. On GNOME, Oto does **not** restore window focus the way Hyprland can via `hyprctl` — if you click elsewhere during Processing, keys go to the wrong place.

---

## 5. `ydotool.service` not found or daemon not running

### Symptoms

- Insertion falls through to clipboard-only.
- Log shows ydotool failures or timeouts.
- `systemctl start ydotool.service` → **Unit ydotool.service not found**.

### Cause

On Arch and most distros, ydotool ships a **systemd user unit**, not a system unit. Installing the package does **not** start `ydotoold`. Oto expects a live Unix datagram socket (often `$XDG_RUNTIME_DIR/.ydotool_socket`).

### Solution

```bash
# Correct: user unit
systemctl --user enable --now ydotool.service
systemctl --user status ydotool.service

# Manual smoke test (focus a text field first)
ydotool type -- 'hello '

# Wrong: system unit (fails with "not found")
# systemctl start ydotool.service
```

Notes:

- Unit name is one word: `ydotool.service` (no space after the dot).
- Optional: `export YDOTOOL_SOCKET=$XDG_RUNTIME_DIR/.ydotool_socket` if your package uses a custom path.
- After enabling, re-run **Test insertion**.

---

## 6. Permission denied on `/dev/uinput` (input group)

### Symptoms

- `ydotoold` fails to start or dies immediately.
- `ydotool type` errors with permission / uinput issues.
- User is not in the `input` group.

### Cause

`ydotoold` injects through `/dev/uinput`. Without group access, the daemon cannot open the device even if the package is installed.

### Solution

```bash
# Add yourself to the input group
sudo usermod -aG input "$USER"

# Apply the new group (must re-login; newgrp is a temporary check)
newgrp input
groups   # should list "input"

# Then start the user daemon again
systemctl --user restart ydotool.service
ydotool type -- 'uinput ok '
```

Log out of GNOME fully (or reboot) so graphical sessions pick up the new group membership.

---

## 7. `wtype` “succeeds” but nothing is typed

### Symptoms

- `wtype` exits 0.
- No characters appear in GTK, Electron, or browser fields.
- Clipboard mode works when you paste manually.

### Cause

On many Wayland compositors (including GNOME/Mutter), `wtype` can complete without delivering text into the focused client. Oto prefers **`ydotool` first**, then `wtype`, for this reason.

### Solution

1. Prefer a healthy `ydotoold` (issues 5–6) over `wtype` alone.
2. Keep `wtype` installed only as a secondary fallback.
3. For stubborn apps, use **Clipboard only** + manual Ctrl+V, or rely on AT-SPI when the app exposes `EditableText`.

```bash
# Compare tools yourself
ydotool type -- 'via ydotool '
wtype 'via wtype '
```

---

## 8. AT-SPI insertion fails on GNOME

### Symptoms

- Test insertion reports clipboard/paste/type paths, never AT-SPI.
- Works in some apps (e.g. gedit) but not others (custom toolkits, games, terminals).

### Cause

AT-SPI only works when:

- The accessibility bus is up (`org.a11y.Bus`).
- The target exposes a focused object with `EditableText` (or similar).
- The app is not sandboxed away from the a11y tree.

Terminals, some Electron apps, privileged UIs, and custom-rendered editors often do not offer a usable accessible text interface.

### Solution

```bash
# Ensure accessibility bus is available
busctl --user status org.a11y.Bus

# On some systems, enabling accessibility improves tree population
gsettings get org.gnome.desktop.interface toolkit-accessibility
# If false and AT-SPI never works, try:
# gsettings set org.gnome.desktop.interface toolkit-accessibility true
```

- Prefer **Auto** so Oto falls back to clipboard+paste automatically.
- Use **Test insertion** per target application — success in gedit does not guarantee success in every Electron app.
- For Command Mode (rewrite selection), AT-SPI selection is best-effort; otherwise Oto uses simulated copy.

---

## 9. Clipboard has text but Ctrl+V never fires

### Symptoms

- After dictation, `wl-paste` shows the transcript.
- The focused app never receives paste.
- Overlay may still show Done (Auto can end on clipboard-only).

### Cause

Clipboard write succeeded (Oto keeps a long-lived clipboard owner on Wayland so data is not dropped). Simulated **Ctrl+V** failed because `ydotool`/`wtype` were unavailable, unfocused, or blocked. Leftover modifiers from the PTT chord can also turn a paste into a different shortcut if they are not released.

### Solution

1. Fix ydotool (issues 5–6).
2. Hold focus on the target field; do not click Oto during Processing.
3. Prefer **Clipboard + paste** mode so paste failures surface clearly.
4. As a last resort: **Clipboard only**, then paste with Ctrl+V yourself.
5. Install `wl-clipboard` for diagnosis:
   ```bash
   wl-paste --no-newline
   ```

Oto attempts to release Ctrl/Shift/Alt/Super after PTT via ydotool key-ups so the paste chord is not remapped.

---

## 10. Focus lands in the wrong window after dictation

### Symptoms

- Text inserts into the wrong app (or into Oto).
- You switched windows while the overlay showed Processing.

### Cause

Speech-to-text can take seconds. On **Hyprland**, Oto can re-focus the window captured at PTT press via `hyprctl`. On **GNOME Wayland**, that restore path is not available — injection targets whatever is focused at inject time.

### Solution

- Keep the target text field focused until Done.
- Avoid clicking Settings or other windows during Processing.
- Use **Clipboard only** if you need to switch windows, then paste when ready.
- Cancel with the overlay control if you changed intent mid-flight.

---

## 11. API key / keyring errors under GNOME

### Symptoms

- Provider save fails; STT returns auth errors.
- Messages about Secret Service / keyring.
- `~/.config/oto/config.json` has no API key fields (by design).

### Cause

Keys live in the OS keyring (service `dev.oto.app`), usually **GNOME Keyring** via Secret Service. A locked, missing, or non-running keyring prevents store/load.

### Solution

```bash
# Secret Service should be on the session bus
busctl --user list | grep -i secret

# Unlock keyring (login password / Seahorse)
# Applications → Passwords and Keys, or re-login
```

1. Unlock the login keyring after boot if GNOME prompts.
2. In Oto → **Providers**, re-save the API key for the active preset.
3. Do not paste keys into `config.json`; Oto rejects serializing secrets there.
4. Ensure `gnome-keyring` (or another Secret Service) is installed and started with the session.

---

## 12. Microphone permission or empty capture

### Symptoms

- **Test microphone** fails or level meters stay flat.
- Portal/PipeWire denial dialogs.
- Empty transcript after a long hold.

### Cause

GNOME + PipeWire gate mic access per app. Wrong device, muted source, or denied portal permission yields silence. Empty audio produces empty STT, not insertion.

### Solution

1. GNOME **Settings → Privacy → Microphone** (or system sound settings): allow Oto / the runtime.
2. Confirm input level in GNOME Sound settings while speaking.
3. Use Oto **Test microphone** before full dictation.
4. For Flatpak: ensure the app has mic permission (`flatpak permission-set` / Flatseal).
5. Hold the PTT long enough to capture real speech; release only after finishing.

---

## 13. Flatpak on GNOME: insertion or host tools missing

### Symptoms

- Flatpak Oto transcribes fine but rarely types into other apps.
- Host `ydotool` / `wtype` / `hyprctl` are installed system-wide but unused.

### Cause

The Flatpak (`packaging/dev.oto.app.yml`, GNOME Platform runtime) allows Wayland, portals, Secret Service, AT-SPI, network, and devices — but **does not bundle** compositor host tools (`ydotool`, `wtype`). Sandbox isolation blocks casual host PATH use.

### Solution

- Prefer **AT-SPI** (Auto) or **Clipboard only** inside Flatpak.
- For full ydotool-based injection, run the native package (`.deb` / AppImage / local build) instead of Flatpak.
- Keep local Whisper models under `~/.local/share/oto` (manifest grants read-only access).
- Flatpak config/data: `~/.var/app/dev.oto.app`.

See [`packaging/README.md`](packaging/README.md).

---

## 14. Portal app identity / desktop file (AppImage, dev, unpackaged)

### Symptoms

- `Wayland portal app registration failed: … App info not found for 'dev.oto.app'`
- GlobalShortcuts works for Flatpak/deb but not AppImage or `npm run tauri dev`.

### Cause

The portal Registry resolves app ID `dev.oto.app` via a desktop entry named exactly:

`~/.local/share/applications/dev.oto.app.desktop`

(or the same name under `$XDG_DATA_DIRS/applications`).

Tauri AppImages often install a differently named launcher (e.g. `oto_0.1.0_amd64.desktop`), which does **not** satisfy the portal. Cosmic/GNOME then reject host-app registration.

Oto now writes `dev.oto.app.desktop` on first Wayland hotkey bind if it is missing or points at an old binary.

### Solution

1. Confirm the file exists and `Exec=` points at the binary or AppImage you actually run:

   ```bash
   cat ~/.local/share/applications/dev.oto.app.desktop
   update-desktop-database ~/.local/share/applications
   ```

2. Restart portals and re-save the hotkey:

   ```bash
   systemctl --user restart xdg-desktop-portal.service
   ```

3. On **COSMIC**, even with a correct desktop file, **GlobalShortcuts is not implemented** by `xdg-desktop-portal-cosmic` (only Access/FileChooser/Screenshot/Settings/ScreenCast). Use tray **Start/Stop** for dictation until a GlobalShortcuts backend exists, or run under GNOME/KDE/Hyprland with the matching portal package.

---

## 15. How to read Oto’s injection log

When insertion misbehaves, check:

```bash
tail -n 80 "/tmp/oto-inject-${USER}.log"
```

Typical lines:

| Log fragment | Meaning |
| --- | --- |
| `inject_text mode=Auto …` | Start of an inject attempt |
| `result=Atspi` | Inserted via accessibility tree |
| `result=Pasted` | Clipboard + simulated paste OK |
| `result=DirectTyped` | Character typing path used |
| `result=ClipboardOnly` | Fallback; paste manually |
| `clipboard+paste failed:` | ydotool/wtype paste path failed |
| `direct typing failed:` | Typing tools failed; clipboard backup likely set |
| `ydotool … failed` / `timed out` | Daemon/socket/permission problem |
| `wtype …` | Fallback tool used or failed |

The log rotates soft-capped around 512 KiB. GUI launches often hide stderr; this file is the durable trail.

---

## Recommended GNOME Wayland baseline

Do this once on a new GNOME Wayland machine:

```bash
# 1) Tools
sudo usermod -aG input "$USER"
# log out / log in, then:
systemctl --user enable --now ydotool.service

# 2) Portals
systemctl --user enable --now xdg-desktop-portal.service
systemctl --user enable --now xdg-desktop-portal-gnome.service

# 3) Smoke tests
ydotool type -- 'oto ydotool ok '
# In Oto: Providers key → Models → Hotkeys Ctrl+Shift+Space → Save
# Injection → Test insertion (focus another app first)
```

Then verify:

| Check | Expected |
| --- | --- |
| Tray Start / Stop | Overlay + mic work without global shortcut |
| PTT hold/release | Portal shortcut starts/stops listening |
| Test insertion | AT-SPI or Pasted (not permanent ClipboardOnly) |
| Real dictation into gedit/browser | Transcript appears in the field |

---

## Related docs

- Project troubleshooting: [`README.md`](README.md) (Hotkeys, Text insertion, Troubleshooting)
- Flatpak sandbox notes: [`packaging/README.md`](packaging/README.md)
- Design background (portal limits, hybrid injection): [`docs/superpowers/specs/2026-07-19-oto-design.md`](docs/superpowers/specs/2026-07-19-oto-design.md)

When filing a bug, include: **GNOME version**, **Wayland**, portal backend status, injection mode, whether tray PTT works, and a redacted snippet of `/tmp/oto-inject-$USER.log`.
