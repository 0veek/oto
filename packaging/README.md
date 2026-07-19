# Packaging Oto

The Flatpak manifest wraps the Debian artifact produced by Tauri, keeping the Rust and frontend build identical across direct packages and Flatpak.

```bash
npm run tauri build -- --bundles deb
flatpak-builder --force-clean --install-deps-from=flathub .flatpak-build packaging/dev.oto.app.yml
flatpak-builder --user --install --force-clean .flatpak-build packaging/dev.oto.app.yml
flatpak run dev.oto.app
```

Local Whisper models used by the Flatpak should be placed under `~/.local/share/oto`; the manifest grants that directory read-only access. Oto’s Flatpak data and configuration live under `~/.var/app/dev.oto.app`.

The sandbox permits microphone devices, portals, Secret Service, AT-SPI, Wayland/X11, and network access for configured cloud providers. Compositor-specific host tools such as `wtype`, `ydotool`, and `hyprctl` are not bundled, so Flatpak insertion should prefer AT-SPI or clipboard-only fallback.

