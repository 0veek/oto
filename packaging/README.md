# Packaging Oto

The Flatpak manifest wraps the Debian artifact produced by Tauri, keeping the Rust and frontend build identical across direct packages and Flatpak.

Host requirement: install **libayatana-appindicator** (or your distro’s
`libayatana-appindicator3-dev` / equivalent) before `npm run tauri build`.
Without it Tauri aborts after compile with
`Can't detect any appindicator library` and never writes the `.deb`. See the
root [development prerequisites](../README.md#development-prerequisites).

The root installer automates the complete process, including copying the
versioned Debian payload to the stable path consumed by the manifest:

```bash
./install.sh --local . --bundle flatpak
flatpak run dev.oto.app
```

For a manual build:

```bash
npm run tauri build -- --bundles deb
cp src-tauri/target/release/bundle/deb/*.deb packaging/oto.deb
flatpak remote-add --user --if-not-exists flathub \
  https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak-builder --force-clean --user --install-deps-from=flathub \
  --repo=.flatpak-repo .flatpak-build packaging/dev.oto.app.yml
flatpak build-bundle .flatpak-repo Oto.flatpak dev.oto.app \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user Oto.flatpak
```

Local Whisper models used by the Flatpak should be placed under `~/.local/share/oto`; the manifest grants that directory read-only access. Oto’s Flatpak data and configuration live under `~/.var/app/dev.oto.app`.

The sandbox permits microphone devices, portals, Secret Service, AT-SPI, Wayland/X11, and network access for configured cloud providers. Compositor-specific host tools such as `wtype`, `ydotool`, and `hyprctl` are not bundled, so Flatpak insertion should prefer AT-SPI or clipboard-only fallback.
