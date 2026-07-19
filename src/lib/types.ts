export type PipelineState =
  | "idle"
  | "listening"
  | "processing"
  | "done"
  | "error";

export type PipelineEvent =
  | { type: "state"; state: PipelineState; detail?: string | null }
  | { type: "level"; level: number }
  | { type: "phase"; phase: string }
  | { type: "partial"; text: string }
  | { type: "error"; message: string };

export type ProviderPreset = "open_ai" | "groq" | "open_router" | "custom";
export type InjectionMode = "auto" | "direct_type" | "clipboard_paste" | "clipboard_only";
export type IdleBehavior = "hide" | "minimal";
export type SttBackend = "cloud" | "local_whisper";
export type ThemePreset = "system" | "midnight" | "light" | "high_contrast";

export interface Snippet {
  id: string;
  trigger: string;
  expansion: string;
  enabled: boolean;
}

export interface StylePreset {
  id: string;
  name: string;
  prompt: string;
}

export interface ProviderProfile {
  id: string;
  name: string;
  base_url: string;
  stt_model: string;
  polish_model: string;
}

export interface SyncConfig {
  enabled: boolean;
  endpoint: string;
}

export interface HistoryEntry {
  id: string;
  created_at_ms: number;
  raw_text: string;
  final_text: string;
  mode: "dictation" | "command";
  language: string | null;
}

export interface AppConfig {
  provider_preset: ProviderPreset;
  base_url: string;
  stt_model: string;
  polish_model: string;
  polish_enabled: boolean;
  temperature: number;
  tone_hint: string;
  hotkey: string;
  language: string | null;
  dictionary: string[];
  injection_mode: InjectionMode;
  idle_behavior: IdleBehavior;
  overlay_x: number | null;
  overlay_y: number | null;
  stt_backend: SttBackend;
  local_whisper_model_path: string;
  vocabulary_boost: boolean;
  snippets: Snippet[];
  styles: StylePreset[];
  active_style_id: string | null;
  history_enabled: boolean;
  history_limit: number;
  streaming_enabled: boolean;
  theme: ThemePreset;
  reduce_motion: boolean;
  font_scale: number;
  custom_providers: ProviderProfile[];
  active_custom_provider_id: string | null;
  sync: SyncConfig;
}
