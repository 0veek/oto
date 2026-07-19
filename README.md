# Oto

**Oto** is system-wide AI voice dictation for Linux. Hold a global push-to-talk hotkey, speak, and Oto transcribes via a cloud STT provider, optionally polishes the text with an LLM, then inserts it into the focused application (or copies it for paste).

Phase 1 MVP: Tauri 2 + Svelte 5 desktop app with a glass floating overlay, settings window, and system tray.

Design spec: [`docs/superpowers/specs/2026-07-19-oto-design.md`](docs/superpowers/specs/2026-07-19-oto-design.md)

---

## Requirements

- **Linux** (X11 or Wayland)
- **Rust** toolchain (stable; for `src-tauri`)
- **Node.js** 18+ and npm
- **Microphone** access
- **OS keyring** (libsecret / Secret Service) for API keys
- System libraries required by Tauri (WebKitGTK, etc.) — see [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/)

### Optional tools (text injection)

| Session | Tools |
|---------|--------|
| Wayland | `wtype` (preferred) or `ydotool` |
| X11 | `xdotool` |

Without a paste tool, **Auto** injection falls back to clipboard-only and prompts you to paste with Ctrl+V.

---

## Development

```bash
npm install
npm run tauri dev
```

Other useful scripts:

```bash
# Frontend typecheck
npm run check

# Frontend production build
npm run build

# Rust tests + compile check
cd src-tauri && cargo test && cargo check
```

Config (no API keys) is stored under your XDG config directory, typically:

`~/.config/oto/config.json`

API keys go only into the OS keyring (service `oto`, accounts per provider preset).

---

## Permissions & hotkeys

- **Global PTT** uses `tauri-plugin-global-shortcut`. Default: `Ctrl+Super+Space`.
- **Wayland**: global shortcuts depend on the compositor and portal support. If registration fails, use the **system tray** *Start Listening* / *Stop Listening* fallback.
- **Microphone**: grant capture when the OS prompts; mic test is under Settings → Appearance.
- **Keyring**: first save of an API key may prompt for keyring unlock.

---

## Provider setup

1. Open **Settings** from the tray (or the settings window on first launch).
2. **Providers** — choose OpenAI, Groq, OpenRouter, or Custom; set base URL if needed; save API key to the keyring.
3. **Models** — STT model id (e.g. `whisper-large-v3`), optional polish model, temperature, tone hint.
4. **Hotkeys** — set your push-to-talk chord.
5. **Dictionary** — terms polish should preserve (names, product jargon).
6. **Injection** — Auto | Clipboard + paste | Clipboard only; use **Test insertion** with another app focused.
7. **Appearance** — hide overlay when idle vs minimal dormant pill; preview UI / test microphone.
8. **About** — version and privacy summary.

Hold the hotkey to dictate; release to process. Tray start/stop is available if the hotkey is unavailable.

---

## Privacy

Audio is sent to your configured STT provider. When polish is enabled, transcript text is sent to your configured LLM provider. API keys stay in the OS keyring. No Oto cloud.

Oto does not operate a backend for your audio or text. Config JSON never stores API keys.

---

## Manual verification

- [ ] App starts; tray visible
- [ ] Settings opens; config saves to `~/.config/oto/config.json` without API keys
- [ ] API key stored; hint shows; transcription test works with real key
- [ ] PTT hotkey: press listens, release processes
- [ ] Tray start/stop works if hotkey unavailable
- [ ] Waveform moves with voice
- [ ] Polish on/off behaves; polish failure still injects raw
- [ ] Injection Auto pastes or leaves clipboard with notice
- [ ] Dictionary terms influence polish
- [ ] Cancel aborts listening
- [ ] Error state shows message; dismiss or ~4s returns to idle
- [ ] Done flash is brief (~700ms)
- [ ] Overlay position persists after drag
- [ ] Appearance “minimal” keeps a dormant pill when idle

---

## Project layout

```
src/                 SvelteKit frontend (overlay + settings)
src-tauri/           Rust backend (audio, hotkeys, providers, injection, pipeline)
docs/superpowers/    Design spec + implementation plan
```

## License

MIT
