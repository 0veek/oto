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
