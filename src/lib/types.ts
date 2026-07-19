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
  | { type: "error"; message: string };

export type ProviderPreset = "open_ai" | "groq" | "open_router" | "custom";
export type InjectionMode = "auto" | "clipboard_paste" | "clipboard_only";
export type IdleBehavior = "hide" | "minimal";

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
}
