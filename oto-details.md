**Comprehensive Technical Plan: Linux Voice Dictation App (Wispr Flow–style) using Rust + Tauri**

### 1. Product Vision & Core Goals
Build a native Linux desktop application that provides **system-wide, AI-powered voice dictation** with a beautiful floating overlay UI. Users press a global hotkey, speak naturally, and receive clean, polished text inserted into the currently focused text field of *any* application.

Key differentiators vs. basic Whisper tools:
- Beautiful, non-intrusive floating control bar / pill with real-time waveform.
- AI post-processing (remove fillers, fix grammar, apply tone/style, punctuation, formatting).
- Configurable AI backends (OpenAI, OpenRouter, Groq, local Whisper + local LLM, etc.).
- Settings GUI for providers, models, hotkeys, personal dictionary, snippets, styles.
- Privacy-first options (local STT possible).
- Excellent Linux support (X11 + Wayland awareness).

Target: Fast, reliable, visually polished tool that feels modern and “magical.”

### 2. Core Feature Set

**MVP (Phase 1)**
- Global hotkey to start/stop listening (push-to-talk or toggle).
- Beautiful always-on-top floating pill/bar with waveform visualization and status.
- Microphone capture → STT → optional LLM polishing → text injection into focused app.
- Settings window (separate or drawer) for:
  - API keys & provider selection (OpenAI, OpenRouter, Groq, custom OpenAI-compatible).
  - Model selection (Whisper variants, GPT-4o-mini, Llama-3.1, Gemma, etc.).
  - Hotkey configuration.
  - Language / auto-detect.
  - Basic personal dictionary.
- System tray icon with quick controls.
- Secure local storage of settings & keys.

**Phase 2**
- Snippets / voice macros (speak trigger → insert longer text).
- Styles / tone presets (professional, casual, code comments, etc.).
- Command Mode (select text → voice command to rewrite/edit).
- Local Whisper support (via whisper-rs or whisper.cpp bindings) for offline/privacy.
- Multi-language auto-detection + custom vocabulary boosting.
- History / recent dictations (optional scratchpad).

**Phase 3+**
- Streaming partial results (real-time transcription display).
- Accessibility improvements, custom themes, plugin system for extra providers.
- Optional cloud sync of dictionary/snippets (user-controlled).

### 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Tauri Frontend                       │
│  (Svelte/React + Tailwind + Framer Motion / Canvas)         │
│  - Floating Overlay Window (transparent, always-on-top)     │
│  - Settings Window                                          │
│  - System Tray                                              │
└──────────────────────────┬──────────────────────────────────┘
                           │ Tauri Commands / Events
┌──────────────────────────▼──────────────────────────────────┐
│                     Rust Backend (Core)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ Hotkey Mgr  │  │ Audio Engine │  │ Text Injector    │   │
│  │ (global)    │  │ (cpal)       │  │ (AT-SPI / tools) │   │
│  └─────────────┘  └──────┬───────┘  └────────┬─────────┘   │
│                          │                   │             │
│  ┌───────────────────────▼───────────────────▼──────────┐  │
│  │              Pipeline Orchestrator                   │  │
│  │  Audio → STT Provider → LLM Polisher → Inject        │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                          │                                 │
│  ┌──────────────┐  ┌─────▼──────┐  ┌──────────────────┐   │
│  │ Provider     │  │ Config &   │  │ Secure Storage   │   │
│  │ Abstraction  │  │ Dictionary │  │ (keyring)        │   │
│  └──────────────┘  └────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 4. Recommended Tech Stack

| Layer              | Choice                              | Reason |
|--------------------|-------------------------------------|------|
| Framework          | **Tauri 2.x**                       | Lightweight, secure, excellent Rust integration, good Linux support |
| Frontend           | **Svelte 5 + Tailwind CSS + TypeScript** (or React) | Fast, beautiful UI, easy waveform/animations |
| Language           | **Rust** (backend)                  | Performance, safety, ecosystem |
| Audio Capture      | `cpal` + optional `rodio`           | Cross-platform mic access |
| Global Hotkeys     | `tauri-plugin-global-shortcut`      | Official, works on X11/Wayland (with caveats) |
| STT                | Cloud: OpenAI / Groq Whisper API<br>Local: `whisper-rs` or `whisper-cpp-rs` | Flexibility |
| LLM Polishing      | OpenAI-compatible clients (`async-openai`, `reqwest`) | OpenRouter, Groq, Together, local Ollama, etc. |
| Text Injection     | Hybrid: AT-SPI2 + fallback (ydotool / wtype / clipboard) | Best reliability on modern Linux |
| Config Storage     | `serde` + TOML/JSON + `keyring` crate | Secure API keys |
| Waveform           | Web Audio API / Canvas in frontend  | Beautiful real-time visualization |
| Packaging          | Flatpak (preferred) + AppImage + .deb | Easy distribution on Linux |

### 5. Key Technical Components – Implementation Details

#### 5.1 Beautiful Floating Window
- Create a dedicated Tauri window:
  - `always_on_top(true)`
  - `decorations(false)`
  - `transparent(true)`
  - `skip_taskbar(true)`
  - Small size (e.g. 280×60 or expandable)
  - Positionable (remember last position, or bottom-center)
- Frontend: Pill-shaped glassmorphism UI with:
  - Animated waveform (Canvas or SVG + Web Audio analyser)
  - Mic icon / listening state / processing spinner
  - Optional cancel / done buttons
  - Subtle shadow, blur, rounded corners
- Click-through support for transparent areas is desirable (platform-specific work or upcoming Tauri features).
- Show only while active or on hover; hide when idle (or keep minimal indicator).

#### 5.2 Global Hotkey System
- Use `tauri-plugin-global-shortcut`.
- Support push-to-talk and toggle modes.
- Configurable in settings (store as string, e.g. `"Ctrl+Super+Space"`).
- On Wayland some restrictions exist; document required compositor support or offer fallback (tray click).

#### 5.3 Audio Pipeline
1. On hotkey → start `cpal` stream (prefer 16 kHz mono for Whisper efficiency).
2. Buffer PCM data (or stream chunks).
3. Optional VAD (Voice Activity Detection) with `webrtc-vad` or simple energy threshold to auto-stop.
4. On stop → send to STT provider.
5. Show real-time volume/waveform in the floating UI via events.

#### 5.4 AI Provider Abstraction
Define clean traits:

```rust
#[async_trait]
trait SpeechToText {
    async fn transcribe(&self, audio: &[u8], language: Option<&str>) -> Result<String>;
}

#[async_trait]
trait TextPolisher {
    async fn polish(&self, raw: &str, context: &PolishContext) -> Result<String>;
}
```

- Concrete implementations for OpenAI, Groq, OpenRouter (OpenAI-compatible), local Whisper, Ollama, etc.
- Settings store base URL, API key (encrypted), model name, temperature, system prompt template.
- System prompt example for polishing:
  > “You are an expert editor. Convert the following raw speech transcription into clean, natural written text. Remove filler words (um, uh, like…), fix grammar, add proper punctuation and capitalization. Preserve the original meaning and tone. Output only the final text.”

- Support streaming responses where possible for lower perceived latency.

#### 5.5 Text Injection (Hardest Linux Part)
Priority order:
1. **AT-SPI2** (recommended primary path) – use `atspi` crate or D-Bus to find the focused accessible text object and insert/replace text.
2. Fallback: Copy polished text to clipboard → simulate Ctrl+V (via `ydotool`, `wtype`, or `xdotool` depending on session).
3. Additional fallback: `dotool` or compositor-specific portals.

Detect session type (X11 vs Wayland) at runtime and choose the best method. Provide clear user guidance for required permissions (accessibility, input simulation).

#### 5.6 Settings GUI
- Separate Tauri window or large modal.
- Sections: Providers, Models, Hotkeys, Dictionary, Snippets, Appearance, Privacy, About.
- Live test buttons (“Test Microphone”, “Test Transcription”).
- Secure key entry (masked + keyring backend).

#### 5.7 Data & Privacy
- All settings local by default.
- API keys never leave the machine except to the chosen provider.
- Optional local-only mode (Whisper.cpp + local LLM via Ollama/llama.cpp).
- Clear privacy policy and “what data is sent” indicators in UI.

### 6. Linux-Specific Considerations
- **Wayland vs X11**: Prefer AT-SPI and portals. Document limitations (global shortcuts, overlays).
- Permissions: Microphone, Accessibility (for AT-SPI), optionally Input simulation.
- Packaging: Flatpak is ideal (sandboxing + portals). Also ship AppImage for maximum compatibility.
- Dependencies: Keep runtime deps minimal; bundle what you can.
- Testing matrix: GNOME, KDE Plasma, Hyprland, Sway, Cosmic, etc.

### 7. Project Structure (Suggested)
```
voiceflow-linux/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── audio/
│   │   ├── hotkeys/
│   │   ├── injection/
│   │   ├── providers/   # stt.rs, llm.rs, openai.rs, groq.rs...
│   │   ├── config/
│   │   └── pipeline.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                 # Frontend (Svelte)
│   ├── lib/
│   ├── components/      # FloatingBar.svelte, Waveform.svelte, Settings...
│   └── App.svelte
├── package.json
└── README.md
```

### 8. Development Roadmap
1. **Week 1–2**: Scaffold Tauri 2 + Svelte, floating transparent window, basic UI, system tray.
2. **Week 3**: Audio capture + waveform visualization + global hotkey.
3. **Week 4**: Provider abstraction + OpenAI/Groq STT + simple polishing + clipboard injection.
4. **Week 5**: Proper text injection (AT-SPI primary), settings GUI, secure storage.
5. **Week 6**: Personal dictionary, basic snippets, polish UI/UX, error handling.
6. **Week 7–8**: Local Whisper support, packaging (Flatpak), testing across DEs, documentation.
7. Later: Command mode, streaming, advanced features.

### 9. Major Risks & Mitigations
- **Text injection reliability** → Hybrid approach + good user onboarding for accessibility permissions.
- **Wayland global hotkeys/overlays** → Graceful degradation + clear docs.
- **Latency** → Prefer fast providers (Groq Whisper is excellent), streaming where possible, local option.
- **Audio quality / noise** → Simple VAD + optional noise suppression later.
- **API key security** → Always use OS keyring.

### 10. Success Metrics
- Sub-2s end-to-end latency on good connections for short utterances.
- High insertion success rate across major apps (browsers, VS Code, LibreOffice, Slack, terminals, etc.).
- Users describe the floating UI as “beautiful” and non-intrusive.
- Easy provider switching and local mode for privacy-conscious users.

This architecture gives you a solid, maintainable, high-performance foundation that can evolve into a best-in-class Linux voice dictation tool. The combination of Rust’s performance/safety with Tauri’s modern web UI is ideal for a polished floating experience.

Would you like me to expand any section into more detailed code sketches, crate recommendations, system prompt templates, or a starter project structure?