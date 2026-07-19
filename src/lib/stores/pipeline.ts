import { writable } from "svelte/store";
import type { PipelineState } from "../types";

export const pipelineState = writable<PipelineState>("idle");
export const pipelineDetail = writable<string>("");
export const audioLevel = writable<number>(0);
export const pipelinePhase = writable<string>("");

export function applyPipelineEvent(ev: import("../types").PipelineEvent) {
  switch (ev.type) {
    case "state":
      pipelineState.set(ev.state);
      pipelineDetail.set(ev.detail ?? "");
      break;
    case "level":
      audioLevel.set(ev.level);
      break;
    case "phase":
      pipelinePhase.set(ev.phase);
      break;
    case "error":
      pipelineState.set("error");
      pipelineDetail.set(ev.message);
      break;
  }
}
