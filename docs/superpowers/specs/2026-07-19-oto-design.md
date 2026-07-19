# Oto — Phase 1 MVP Design Spec

**Date:** 2026-07-19  
**Status:** Approved for implementation planning  
**Product:** Oto — system-wide AI voice dictation for Linux  
**Reference:** `oto-details.md` (full product vision; this spec scopes Phase 1 only)

---

## 1. Goal

Build a native Linux desktop app that provides **system-wide, AI-powered voice dictation** with a polished floating overlay. The user holds a global hotkey, speaks, and receives cleaned text inserted into the focused field of any application.

**MVP success criteria**

- Push-to-talk global hotkey records audio → cloud STT → optional LLM polish → text injection (or clipboard fallback).
- Glassmorphism floating pill with live audio levels.
- Settings for providers, API keys, models, hotkey, language, dictionary, appearance, injection mode.
- System tray: open settings, fallback listen control, quit.
- API keys stored in the OS keyring (never plain-text in config).

---

## 2. Decisions (locked)

| Topic | Choice |
|-------|--------|
| Scope | Full Phase 1 MVP (not thin slice) |
| Name | **Oto** |
| Architecture | Single Tauri 2 app, modular Rust modules |
| Frontend | Svelte 5 + Tailwind CSS + TypeScript |
| AI | Cloud-first; OpenAI-compatible HTTP |
| Default listening mode | **Push-to-talk** (hold to talk, release to process) |
| Linux targeting | Multi-target hybrid injection (no single-DE privilege) |
| Overlay visual | **Glass pill** (frosted translucent bar) |
| Local Whisper / Ollama | Out of MVP (Phase 2) |
| Packaging (Flatpak/AppImage) | Out of MVP; document dev run first |

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri Frontend (Svelte 5)               │
│  Floating overlay · Settings window · System tray           │
└──────────────────────────┬──────────────────────────────────┘
                           │ commands + events
┌──────────────────────────▼──────────────────────────────────┐
│                     Rust backend                            │
│  hotkeys · audio · providers · injection · config · pipeline│
└─────────────────────────────────────────────────────────────┘
```

### 3.1 Windows & shell

1. **Overlay window** — transparent, always-on-top, undecorated, skip taskbar; small pill size; default bottom-center; draggable; position persisted.
2. **Settings window** — normal decorated window; opened from tray (and optionally from overlay).
3. **System tray** — Open settings · Start/stop listening (fallback if hotkey fails) · Quit.

### 3.2 Backend modules

| Module | Responsibility |
|--------|----------------|
| `hotkeys` | Register/unregister global shortcut via `tauri-plugin-global-shortcut`; PTT press/release → pipeline |
| `audio` | `cpal` capture (prefer 16 kHz mono); metering events; WAV/PCM buffer encode |
| `providers` | `SpeechToText` + `TextPolisher` traits; OpenAI-compatible client(s) |
| `injection` | Hybrid: AT-SPI → clipboard+paste sim → clipboard-only |
| `config` | XDG config JSON + keyring for secrets |
| `pipeline` | Orchestrate states, emit events, cancelation |

---

## 4. Dictation pipeline

### 4.1 Happy path (push-to-talk)

1. User **holds** configured global hotkey.
2. Overlay activates → state **Listening**.
3. `cpal` records; frontend receives **level** events for waveform.
4. User **releases** hotkey → state **Processing**.
5. Encode audio → **STT** → raw transcript.
6. If polish enabled → **LLM** with system prompt + dictionary context.
7. **Inject** final text into focused app (hybrid strategy).
8. Overlay shows brief **Done** or **Error**, then returns to idle/hidden.

### 4.2 UI states

| State | Overlay behavior |
|-------|------------------|
| Idle | Hidden (default) or minimal dormant indicator (appearance setting) |
| Listening | Glass pill + live waveform + mic/active indicator |
| Processing | Spinner + phase label (“Transcribing…”, “Polishing…”) |
| Done | Short success flash |
| Error | User-visible message + dismiss |

Overlay controls: **Cancel** (abort current take). If injection fails after all methods: keep text on clipboard and tell user to paste.

### 4.3 Default hotkey

Suggested default: `Ctrl+Super+Space` (configurable in settings). Document Wayland compositor limitations; tray provides fallback start/stop (toggle-style for tray only is acceptable as emergency control).

---

## 5. UI design

### 5.1 Floating pill (glass)

- Frosted translucent background, soft border, rounded full-pill shape, drop shadow.
- Contents: status dot / mic, animated waveform (canvas from level events), short status text, cancel control.
- Non-intrusive; appears during active sessions; hides when idle by default.
- Appearance settings may later tweak opacity/theme; MVP ships one polished glass dark theme that reads on light and dark desktops.

### 5.2 Settings window

Sidebar sections:

1. **Providers** — preset: OpenAI | Groq | OpenRouter | Custom; base URL; API key (masked → keyring); Test connection/transcription.
2. **Models** — STT model id; polish model id; polish on/off; optional temperature; optional free-text tone hint (fed into `PolishContext`).
3. **Hotkeys** — capture/edit shortcut string; note about PTT.
4. **Dictionary** — list of words/phrases to bias polish (and include in polish context).
5. **Appearance** — idle behavior (hide vs minimal dormant pill); optional overlay scale.
6. **Injection** — Auto | Clipboard + paste | Clipboard only; “Test insertion” into built-in text box.
7. **About** — version, links, what data is sent (privacy blurb).

Also: **Test microphone** (levels only), language / auto-detect setting.

**Provider profile (MVP):** one active cloud profile (preset + base URL + API key) is used for both STT and polish. STT and polish **model ids** may differ; separate keys/endpoints per capability are Phase 2.

### 5.3 Tray

- Open Settings  
- Start Listening / Stop (fallback)  
- Quit  

---

## 6. Providers & AI

### 6.1 Traits

```rust
#[async_trait]
trait SpeechToText {
    async fn transcribe(
        &self,
        audio_wav: &[u8],
        language: Option<&str>,
    ) -> Result<String>;
}

#[async_trait]
trait TextPolisher {
    async fn polish(&self, raw: &str, ctx: &PolishContext) -> Result<String>;
}
```

`PolishContext` (MVP): optional language, personal dictionary terms, optional free-text tone hint (single string). Full style preset library is Phase 2.

### 6.2 Implementation

- One (or thin-wrapper) **OpenAI-compatible** HTTP client parameterized by base URL, API key, model.
- Presets fill base URLs:
  - OpenAI: `https://api.openai.com/v1`
  - Groq: `https://api.groq.com/openai/v1`
  - OpenRouter: `https://openrouter.ai/api/v1`
  - Custom: user-provided
- STT: Whisper-compatible `/audio/transcriptions` (or provider-equivalent).
- Polish: chat completions with system prompt:

> You are an expert editor. Convert the following raw speech transcription into clean, natural written text. Remove filler words (um, uh, like…), fix grammar, add proper punctuation and capitalization. Preserve the original meaning and tone. If a personal dictionary is provided, prefer those spellings. Output only the final text.

- Polish may be disabled; then inject raw STT output (with light local trim of whitespace only).

### 6.3 Suggested defaults (user-overridable)

- Prefer Groq Whisper when using Groq; otherwise OpenAI `whisper-1` / documented default.
- Polish model: small/fast chat model appropriate to the chosen preset (e.g. `gpt-4o-mini` or Groq Llama).

Exact default model IDs may be adjusted at implementation time to match current provider catalogs; settings must never hardcode a single non-editable model.

---

## 7. Config & security

| Data | Storage |
|------|---------|
| Provider preset, base URL, model ids, polish flag, hotkey, language, dictionary, appearance, injection mode, window position | XDG config: `~/.config/oto/config.json` (serde JSON) |
| API keys | OS keyring via `keyring` crate; service name `oto`, accounts per provider/preset |

- Config file must not store secret key material.
- README documents what leaves the machine: audio to STT provider; text to polish provider when polish is on.

---

## 8. Text injection (Linux)

**Auto mode priority:**

1. **AT-SPI2** — focused accessible text object; insert/replace text when possible.
2. **Clipboard + paste simulation** — write clipboard, then:
   - Wayland: prefer `wtype` or `ydotool` if available
   - X11: `xdotool` (or equivalent)
   - Key combo: Ctrl+V and/or Shift+Insert as appropriate
3. **Clipboard only** — leave polished text on clipboard; overlay/tray notifies user to paste.

**Settings override:** Auto | Clipboard + paste | Clipboard only.

**Runtime:** detect session type; probe available tools; log which method succeeded. Provide first-run / Help checklist for mic, accessibility, and optional input tools.

---

## 9. Error handling

| Failure | User-facing behavior |
|---------|----------------------|
| Mic missing / permission denied | Error state; link to help/settings |
| Hotkey registration failed | Tray notification; use tray fallback; document Wayland limits |
| STT auth/network/rate limit | Show clear error; no inject |
| Empty transcript | Soft message; no inject |
| Polish failure | Fall back to raw STT transcript, inject that, and toast that polish failed |
| Injection step fails | Try next method in chain; final clipboard-only + paste instruction |
| Cancel | Abort capture/in-flight request best-effort; return to idle |

Pipeline emits typed events (state, levels, errors) for the frontend.

---

## 10. Project structure

```
oto/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── audio/
│   │   ├── hotkeys/
│   │   ├── injection/
│   │   ├── providers/
│   │   ├── config/
│   │   └── pipeline.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                      # Svelte 5 frontend
│   ├── lib/
│   │   └── components/       # FloatingPill, Waveform, settings sections
│   ├── ...
│   └── app.css
├── package.json
├── README.md
├── oto-details.md
└── docs/superpowers/specs/
```

---

## 11. Out of scope (Phase 2+)

- Local Whisper / whisper.cpp / Ollama
- Snippets / voice macros
- Style preset library (beyond single tone hint)
- Command mode (select text → voice rewrite)
- Streaming partial transcription display
- Cloud sync of dictionary/settings
- Flatpak / AppImage / .deb packaging (dev instructions only in MVP)
- Click-through transparent regions (nice-to-have if easy; not blocking)

---

## 12. Testing strategy (MVP)

- **Unit:** config load/save (no secrets in file); polish prompt assembly; provider URL/preset mapping; injection method selection logic (mocked probes).
- **Manual:** mic test; PTT record → STT with real key; inject into browser, VS Code, terminal, LibreOffice where available; Wayland and X11 if available.
- **Settings:** test mic levels; test transcription; test insertion into settings text box.

Automated E2E for global hotkeys/injection is best-effort only (environment-dependent).

---

## 13. Implementation order (for planning skill)

1. Scaffold Tauri 2 + Svelte 5 + Tailwind; app name Oto; two windows + tray.
2. Glass overlay UI + state machine wiring (mock pipeline events).
3. Config + keyring + settings shell.
4. Audio capture + level events + waveform.
5. Global PTT hotkey.
6. OpenAI-compatible STT + polish providers.
7. Hybrid text injection + fallbacks.
8. Dictionary + polish context; error polish; README permissions.
9. End-to-end manual validation checklist.

---

## 14. Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Text injection unreliable | Hybrid chain + clipboard-only always works |
| Wayland global shortcuts | Document limits; tray fallback |
| Latency | Prefer fast cloud STT (e.g. Groq); polish optional |
| API key leakage | Keyring only; never log secrets |
| Provider API drift | Configurable base URL + model ids |

---

## 15. Non-goals for this spec

- Competitive feature parity with Wispr Flow on day one  
- Multi-user or team features  
- Windows/macOS support (Linux-first; don’t block portable crates, but don’t ship or test non-Linux in MVP)
