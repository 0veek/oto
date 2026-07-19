import { writable } from "svelte/store";
import type { PipelineState } from "../types";

export const pipelineState = writable<PipelineState>("idle");
export const pipelineDetail = writable<string>("");
export const audioLevel = writable<number>(0);
export const pipelinePhase = writable<string>("");
export const partialTranscript = writable<string>("");

export function applyPipelineEvent(ev: import("../types").PipelineEvent) {
  switch (ev.type) {
    case "state":
      pipelineState.set(ev.state);
      pipelineDetail.set(ev.detail ?? "");
      // Clear ephemeral UI so a new session never flashes stale phase/partial/level.
      if (ev.state === "listening" || ev.state === "idle") {
        partialTranscript.set("");
        pipelinePhase.set("");
      }
      if (ev.state === "idle" || ev.state === "done" || ev.state === "error") {
        audioLevel.set(0);
      }
      if (ev.state === "listening") {
        audioLevel.set(0);
      }
      break;
    case "level":
      audioLevel.set(ev.level);
      break;
    case "phase":
      pipelinePhase.set(ev.phase);
      break;
    case "partial":
      partialTranscript.set(ev.text);
      break;
    case "error":
      pipelineState.set("error");
      pipelineDetail.set(ev.message);
      pipelinePhase.set("");
      partialTranscript.set("");
      audioLevel.set(0);
      break;
  }
}
