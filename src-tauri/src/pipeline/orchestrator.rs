//! PTT lifecycle: record → STT → optional polish → inject → events.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::time::{sleep, Duration};

use crate::audio::AudioRecorder;
use crate::config::{load_config, AppConfig, IdleBehavior, SttBackend};
use crate::error::{OtoError, OtoResult};
use crate::features::{history, snippets::expand_snippet};
use crate::injection::{
    capture_focus_target, capture_selected_text, inject_text_to, FocusTarget, InjectResult,
};
use crate::pipeline::events::{PipelineEvent, PipelineState};
use crate::providers::{
    client_from_config, LocalWhisperClient, OpenAiCompatClient, PolishContext, SpeechToText,
    TextPolisher, TranscriptionContext,
};
use crate::state::AppState;

async fn client_from_config_async(cfg: &crate::config::AppConfig) -> OtoResult<OpenAiCompatClient> {
    // keyring's Secret Service backend is synchronous and internally blocks on
    // Tokio. Run it outside the async worker to avoid nested-runtime panics.
    let cfg = cfg.clone();
    tauri::async_runtime::spawn_blocking(move || client_from_config(&cfg))
        .await
        .map_err(|error| OtoError::Message(format!("credential lookup task failed: {error}")))?
}

fn command_mode_client_error(error: &OtoError) -> String {
    let message = error.to_string();
    if message.contains("API key not set") {
        "Command Mode needs an LLM API key to rewrite the selection. Local Whisper only handles speech-to-text — add a key under Providers, or use a Custom provider pointed at a local OpenAI-compatible server (localhost needs no key).".into()
    } else {
        message
    }
}

fn transcription_context(cfg: &AppConfig) -> TranscriptionContext {
    TranscriptionContext {
        language: cfg
            .language
            .as_deref()
            .and_then(crate::providers::openai_compat::normalize_stt_language),
        vocabulary_prompt: if cfg.vocabulary_boost && !cfg.dictionary.is_empty() {
            Some(format!(
                "Preferred names, spellings, and domain terms: {}",
                cfg.dictionary.join(", ")
            ))
        } else {
            None
        },
    }
}

async fn transcribe_from_config(cfg: &AppConfig, wav: &[u8]) -> OtoResult<String> {
    let context = transcription_context(cfg);
    match cfg.stt_backend {
        SttBackend::Cloud => {
            let client = client_from_config_async(cfg).await?;
            client.transcribe(wav, &context).await
        }
        SttBackend::LocalWhisper => {
            let client = LocalWhisperClient::new(cfg.local_whisper_model_path.clone())?;
            client.transcribe(wav, &context).await
        }
    }
}

/// Exclusive pipeline phase — only one session may run at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Listening,
    Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionMode {
    Dictation,
    Command,
}

impl SessionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dictation => "dictation",
            Self::Command => "command",
        }
    }
}

struct Inner {
    recorder: Option<AudioRecorder>,
    phase: Phase,
    /// Bumped on new sessions / cancel so delayed work doesn't clobber later sessions.
    epoch: u64,
    /// Set on cancel; checked after awaits during processing.
    cancel_flag: bool,
    /// Last captured WAV bytes (for STT / test_transcription).
    last_wav: Option<Vec<u8>>,
    mode: SessionMode,
    selected_text: Option<String>,
    /// Window that should receive injected text (captured at PTT press).
    focus_target: Option<FocusTarget>,
}

pub struct Pipeline {
    app: AppHandle,
    inner: Mutex<Inner>,
}

impl Pipeline {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Mutex::new(Inner {
                recorder: None,
                phase: Phase::Idle,
                epoch: 0,
                cancel_flag: false,
                last_wav: None,
                mode: SessionMode::Dictation,
                selected_text: None,
                focus_target: None,
            }),
        }
    }

    fn emit(&self, event: PipelineEvent) {
        let _ = self.app.emit("pipeline://event", event);
    }

    fn emit_state(&self, state: PipelineState) {
        self.emit(PipelineEvent::state(state, None));
    }

    /// Position overlay from config or bottom-center of the current monitor, then show.
    fn show_overlay(&self) {
        if let Some(w) = self.app.get_webview_window("overlay") {
            position_overlay(&w);
            let _ = w.set_always_on_top(true);
            let _ = w.set_skip_taskbar(true);
            // Never accept keyboard focus — synthetic typing must hit the dictation target.
            let _ = w.set_focusable(false);
            // Do not steal keyboard focus from the app the user is dictating into.
            if let Err(e) = w.show() {
                eprintln!("oto: overlay.show failed: {e}");
            } else {
                eprintln!("oto: overlay shown");
            }
            let _ = w.unminimize();
        } else {
            eprintln!("oto: overlay window missing");
        }
    }

    /// Hide overlay unless appearance is set to minimal dormant pill.
    fn hide_overlay(&self) {
        let keep = load_config()
            .map(|c| c.idle_behavior == IdleBehavior::Minimal)
            .unwrap_or(false);
        if keep {
            return;
        }
        if let Some(w) = self.app.get_webview_window("overlay") {
            let _ = w.hide();
        }
    }

    fn bump_epoch(&self) -> OtoResult<u64> {
        let mut inner = self.lock_inner()?;
        inner.epoch = inner.epoch.wrapping_add(1);
        Ok(inner.epoch)
    }

    /// True when phase is Idle (safe for appearance changes / new PTT).
    pub fn is_idle(&self) -> bool {
        self.lock_inner()
            .map(|g| g.phase == Phase::Idle)
            .unwrap_or(true)
    }

    /// True while actively capturing audio (between ptt_down and ptt_up).
    pub fn is_listening(&self) -> bool {
        self.lock_inner()
            .map(|g| g.phase == Phase::Listening)
            .unwrap_or(false)
    }

    fn listening_snapshot(&self, session_epoch: u64) -> OtoResult<Option<Vec<u8>>> {
        let inner = self.lock_inner()?;
        if inner.phase != Phase::Listening || inner.epoch != session_epoch {
            return Ok(None);
        }
        inner
            .recorder
            .as_ref()
            .map(AudioRecorder::snapshot_wav)
            .transpose()
            .map(Option::flatten)
    }

    /// True if this processing session was cancelled or superseded.
    fn session_aborted(&self, session_epoch: u64) -> bool {
        self.lock_inner()
            .map(|g| g.epoch != session_epoch || g.cancel_flag)
            .unwrap_or(true)
    }

    /// Mark phase Idle (best-effort) without bumping epoch.
    fn set_phase_idle(&self) {
        if let Ok(mut inner) = self.lock_inner() {
            inner.phase = Phase::Idle;
            inner.recorder = None;
            inner.selected_text = None;
        }
    }

    /// Error state stays ~4s (or until cancel/dismiss), then idle.
    async fn finish_error(&self, message: String) {
        // Allow a new PTT immediately; error flash is non-exclusive.
        self.set_phase_idle();
        let epoch = self.bump_epoch().unwrap_or(0);
        self.emit(PipelineEvent::Error {
            message: message.clone(),
        });
        // Ensure overlay is visible for the error flash.
        self.show_overlay();
        sleep(Duration::from_secs(4)).await;
        // Skip if user dismissed or a new session started.
        let still = self
            .lock_inner()
            .map(|g| g.epoch == epoch && g.phase == Phase::Idle)
            .unwrap_or(false);
        if still {
            self.emit_state(PipelineState::Idle);
            self.hide_overlay();
        }
    }

    fn lock_inner(&self) -> OtoResult<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| OtoError::Message("pipeline lock poisoned".into()))
    }

    /// Clone of the last captured WAV, if any.
    pub fn last_wav(&self) -> OtoResult<Option<Vec<u8>>> {
        let inner = self.lock_inner()?;
        Ok(inner.last_wav.clone())
    }

    /// Run STT on the last recorded buffer (settings "Test transcription").
    pub async fn transcribe_last(&self) -> OtoResult<String> {
        let wav = self
            .last_wav()?
            .ok_or_else(|| OtoError::Message("No audio yet — dictate first".into()))?;
        let cfg = load_config()?;
        transcribe_from_config(&cfg, &wav).await
    }

    pub async fn ptt_down(&self) -> OtoResult<()> {
        self.start_listening(SessionMode::Dictation, None).await
    }

    /// Start Command Mode after capturing the selected text in the focused app.
    /// Settings uses a short delay so the user can restore focus; tray uses zero.
    pub async fn command_down(&self, focus_delay_ms: u64) -> OtoResult<()> {
        if focus_delay_ms > 0 {
            sleep(Duration::from_millis(focus_delay_ms.min(5000))).await;
        }
        let selected = capture_selected_text().await?;
        self.start_listening(SessionMode::Command, Some(selected))
            .await
    }

    async fn start_listening(
        &self,
        mode: SessionMode,
        selected_text: Option<String>,
    ) -> OtoResult<()> {
        // Capture focus *before* showing the overlay so injection can restore it
        // after multi-second STT, even if Settings/overlay steals keyboard focus.
        let focus_target = capture_focus_target();
        eprintln!(
            "oto focus: captured class={:?} address={:?}",
            focus_target.class, focus_target.hyprland_address
        );
        {
            let mut inner = self.lock_inner()?;
            // Only start a new listen from Idle — reject if already Listening or Processing.
            if inner.phase != Phase::Idle {
                return Ok(());
            }
            // Invalidate any pending error timeout / leftover cancel from a previous take.
            inner.epoch = inner.epoch.wrapping_add(1);
            inner.cancel_flag = false;
            inner.phase = Phase::Listening;
            inner.mode = mode;
            inner.selected_text = selected_text;
            inner.focus_target = Some(focus_target);
        }

        // Set the visible state at the press boundary. The overlay is prewarmed,
        // so emitting before show prevents a stale Processing frame on map.
        self.emit(PipelineEvent::state(
            PipelineState::Listening,
            (mode == SessionMode::Command).then(|| "Command mode".into()),
        ));
        self.show_overlay();

        // A first-run webview may still attach its listener after show. Retry only
        // while this session is listening so a quick release cannot be overwritten
        // by a late Listening event.
        {
            let app = self.app.clone();
            tauri::async_runtime::spawn(async move {
                sleep(Duration::from_millis(80)).await;
                let still_listening = app
                    .try_state::<AppState>()
                    .map(|state| state.pipeline.is_listening())
                    .unwrap_or(false);
                if still_listening {
                    let _ = app.emit(
                        "pipeline://event",
                        PipelineEvent::state(
                            PipelineState::Listening,
                            (mode == SessionMode::Command).then(|| "Command mode".into()),
                        ),
                    );
                }
            });
        }

        match AudioRecorder::start(self.app.clone()) {
            Ok(recorder) => {
                let mut inner = self.lock_inner()?;
                // Cancel or supersede may have happened while starting the device.
                if inner.phase != Phase::Listening || inner.cancel_flag {
                    return Ok(());
                }
                inner.recorder = Some(recorder);
                let session_epoch = inner.epoch;
                drop(inner);
                self.spawn_partial_loop(session_epoch);
                Ok(())
            }
            Err(e) => {
                self.set_phase_idle();
                self.finish_error(e.to_string()).await;
                Err(e)
            }
        }
    }

    fn spawn_partial_loop(&self, session_epoch: u64) {
        let Ok(config) = load_config() else {
            return;
        };
        if !config.streaming_enabled || config.stt_backend != SttBackend::LocalWhisper {
            return;
        }
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            let mut previous = String::new();
            loop {
                sleep(Duration::from_millis(1800)).await;
                let Some(pipeline) = app
                    .try_state::<AppState>()
                    .map(|state| state.pipeline.clone())
                else {
                    break;
                };
                let wav = match pipeline.listening_snapshot(session_epoch) {
                    Ok(Some(wav)) => wav,
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!("oto: live preview snapshot failed: {error}");
                        break;
                    }
                };
                match transcribe_from_config(&config, &wav).await {
                    Ok(text) if !text.trim().is_empty() && text != previous => {
                        // Drop stale partials if PTT released / canceled while inference ran.
                        if pipeline
                            .listening_snapshot(session_epoch)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            break;
                        }
                        if !pipeline.is_listening() {
                            break;
                        }
                        previous = text.clone();
                        let _ = app.emit("pipeline://event", PipelineEvent::Partial { text });
                    }
                    Ok(_) => {}
                    Err(error) => {
                        // Preview failure must never abort the actual dictation.
                        eprintln!("oto: live local preview failed: {error}");
                        break;
                    }
                }
            }
        });
    }

    pub async fn ptt_up(&self) -> OtoResult<()> {
        let (recorder, session_epoch, mode, selected_text) = {
            let mut inner = self.lock_inner()?;
            if inner.phase != Phase::Listening {
                return Ok(());
            }
            // Capture epoch for this session; further work aborts if cancel bumps it.
            let session_epoch = inner.epoch;
            inner.phase = Phase::Processing;
            (
                inner.recorder.take(),
                session_epoch,
                inner.mode,
                inner.selected_text.take(),
            )
        };

        // Switch the overlay as soon as the hotkey is released. Finalizing the
        // recorder and network processing happen after the UI leaves Listening.
        self.emit_state(PipelineState::Processing);

        let wav = if let Some(rec) = recorder {
            match rec.stop() {
                Ok((wav, _sample_rate)) => {
                    // Tiny captures are almost always accidental taps (no speech).
                    // Surface a clearer error than a failed remote STT call.
                    if wav.len() < 1024 {
                        self.finish_error(
                            "Recording was too short — hold the hotkey while speaking".into(),
                        )
                        .await;
                        return Ok(());
                    }
                    let mut inner = self.lock_inner()?;
                    inner.last_wav = Some(wav.clone());
                    wav
                }
                Err(e) => {
                    self.finish_error(e.to_string()).await;
                    return Err(e);
                }
            }
        } else {
            self.finish_error("No audio captured".into()).await;
            return Ok(());
        };

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        let cfg = match load_config() {
            Ok(c) => c,
            Err(e) => {
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        self.emit(PipelineEvent::Phase {
            phase: "transcribing".into(),
        });

        let mut text = match transcribe_from_config(&cfg, &wav).await {
            Ok(t) => t,
            Err(e) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                self.finish_error(e.to_string()).await;
                return Err(e);
            }
        };

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        if text.trim().is_empty() {
            self.finish_error("No speech detected".into()).await;
            return Ok(());
        }

        let raw_text = text.clone();
        if cfg.streaming_enabled {
            self.emit(PipelineEvent::Partial { text: text.clone() });
        }

        let snippet_expanded = if mode == SessionMode::Dictation {
            if let Some(expansion) = expand_snippet(&text, &cfg.snippets).map(str::to_owned) {
                text = expansion;
                true
            } else {
                false
            }
        } else {
            false
        };

        if mode == SessionMode::Command {
            self.emit(PipelineEvent::Phase {
                phase: "rewriting selection".into(),
            });
            let selected = selected_text
                .as_deref()
                .ok_or_else(|| OtoError::Message("Command Mode lost the selected text".into()))?;
            let client = match client_from_config_async(&cfg).await {
                Ok(client) => client,
                Err(error) => {
                    // Command Mode rewrites via the LLM provider; Local Whisper only
                    // covers STT. Surface a clearer message when the polish key is missing.
                    let message = command_mode_client_error(&error);
                    self.finish_error(message).await;
                    return Err(error);
                }
            };
            let ctx = PolishContext {
                language: cfg.language.clone(),
                dictionary: cfg.dictionary.clone(),
                tone_hint: cfg.active_style_prompt(),
            };
            text = match client.rewrite(selected, &text, &ctx).await {
                Ok(rewritten) => rewritten,
                Err(error) => {
                    self.finish_error(error.to_string()).await;
                    return Err(error);
                }
            };
        } else if cfg.polish_enabled && !snippet_expanded {
            self.emit(PipelineEvent::Phase {
                phase: "polishing".into(),
            });
            let client = match client_from_config_async(&cfg).await {
                Ok(client) => client,
                Err(error) => {
                    if self.session_aborted(session_epoch) {
                        self.set_phase_idle();
                        return Ok(());
                    }
                    self.emit(PipelineEvent::state(
                        PipelineState::Processing,
                        Some(format!("Polish unavailable, using raw: {error}")),
                    ));
                    // Continue with raw transcription, matching polish-failure behavior.
                    return self
                        .finish_with_text(text, raw_text, mode, &cfg, session_epoch)
                        .await;
                }
            };
            let ctx = PolishContext {
                language: cfg.language.clone(),
                dictionary: cfg.dictionary.clone(),
                tone_hint: cfg.active_style_prompt(),
            };
            match client.polish(&text, &ctx).await {
                Ok(polished) => {
                    if self.session_aborted(session_epoch) {
                        self.set_phase_idle();
                        return Ok(());
                    }
                    text = polished;
                }
                Err(e) => {
                    if self.session_aborted(session_epoch) {
                        self.set_phase_idle();
                        return Ok(());
                    }
                    // Spec: fall back to raw + toast (do not abort pipeline).
                    self.emit(PipelineEvent::state(
                        PipelineState::Processing,
                        Some(format!("Polish failed, using raw: {e}")),
                    ));
                }
            }
        }

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        self.finish_with_text(text, raw_text, mode, &cfg, session_epoch)
            .await
    }

    async fn finish_with_text(
        &self,
        text: String,
        raw_text: String,
        mode: SessionMode,
        cfg: &AppConfig,
        session_epoch: u64,
    ) -> OtoResult<()> {
        // Cancel during polish/credential lookup must never inject canceled text.
        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        if cfg.history_enabled {
            if let Err(error) = history::append(
                raw_text,
                text.clone(),
                mode.as_str(),
                cfg.language.clone(),
                cfg.history_limit,
            ) {
                eprintln!("oto: could not save history: {error}");
            }
        }

        self.emit(PipelineEvent::Phase {
            phase: "injecting".into(),
        });

        // Portal/global-shortcut release can arrive while Ctrl/Shift/Super are
        // still physically held. Wait long enough for the chord to settle, then
        // inject_text also synthesizes explicit key-up events before typing.
        sleep(Duration::from_millis(400)).await;

        let focus_target = {
            let mut inner = self.lock_inner()?;
            inner.focus_target.take()
        };

        let done_detail = match inject_text_to(
            &text,
            &cfg.injection_mode,
            focus_target.as_ref(),
        )
        .await
        {
            Ok(InjectResult::ClipboardOnly) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                // Text is on clipboard; user pastes manually.
                "Copied — press Ctrl+V".to_string()
            }
            Ok(InjectResult::Pasted | InjectResult::Atspi | InjectResult::DirectTyped) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                // Surface the injected text (truncate long transcripts for overlay).
                if text.chars().count() > 120 {
                    let short: String = text.chars().take(117).collect();
                    format!("{short}…")
                } else {
                    text
                }
            }
            Err(e) => {
                if self.session_aborted(session_epoch) {
                    self.set_phase_idle();
                    return Ok(());
                }
                self.finish_error(format!("Injection failed: {e}")).await;
                return Err(e);
            }
        };

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        self.emit(PipelineEvent::state(PipelineState::Done, Some(done_detail)));
        // Done flash ~700ms then idle.
        sleep(Duration::from_millis(700)).await;

        if self.session_aborted(session_epoch) {
            self.set_phase_idle();
            return Ok(());
        }

        {
            let mut inner = self.lock_inner()?;
            inner.phase = Phase::Idle;
        }
        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }

    pub async fn cancel(&self) -> OtoResult<()> {
        {
            let mut inner = self.lock_inner()?;
            inner.recorder = None;
            inner.phase = Phase::Idle;
            inner.selected_text = None;
            inner.focus_target = None;
            inner.cancel_flag = true;
            // Invalidate pending error auto-dismiss and in-flight processing.
            inner.epoch = inner.epoch.wrapping_add(1);
        }
        self.emit_state(PipelineState::Idle);
        self.hide_overlay();
        Ok(())
    }
}

/// Apply saved overlay position, or place bottom-center on the current monitor.
pub fn position_overlay(w: &tauri::WebviewWindow) {
    let cfg = load_config().ok();
    // Treat (0, 0) as unset — Moved events often fire with that before layout.
    if let Some(cfg) = cfg.as_ref() {
        if let (Some(x), Some(y)) = (cfg.overlay_x, cfg.overlay_y) {
            if !(x == 0 && y == 0) {
                let _ = w.set_position(PhysicalPosition::new(x, y));
                return;
            }
        }
    }

    // Best-effort bottom-center of the monitor the window is on (or primary).
    let monitor = w
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| w.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let screen = monitor.size();
        let origin = monitor.position();
        let win = w.outer_size().unwrap_or(tauri::PhysicalSize::new(260, 54));
        let margin_bottom = 96i32;
        let x = origin.x + (screen.width as i32 - win.width as i32) / 2;
        let y = origin.y + screen.height as i32 - win.height as i32 - margin_bottom;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}
