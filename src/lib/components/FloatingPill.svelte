<script lang="ts">
  import Waveform from "./Waveform.svelte";
  import { pipelineState, pipelineDetail, pipelinePhase, partialTranscript } from "../stores/pipeline";
  import { invoke } from "@tauri-apps/api/core";

  let cancelBusy = $state(false);

  async function cancel() {
    if (cancelBusy) return;
    cancelBusy = true;
    try {
      await invoke("cancel_dictation");
    } catch (error) {
      console.error("cancel_dictation failed", error);
    } finally {
      cancelBusy = false;
    }
  }

  function statusLabel(state: string) {
    switch (state) {
      case "listening":
        return "Listening";
      case "processing":
        return "Processing";
      case "done":
        return "Done";
      case "error":
        return "Error";
      default:
        return "Idle";
    }
  }
</script>

<div
  class="flex max-w-[min(92vw,28rem)] items-center gap-2 rounded-full border border-white/25 bg-slate-900/90 py-2.5 pl-4 pr-2 text-sm text-white shadow-2xl backdrop-blur-2xl"
  role="status"
  aria-live="polite"
  aria-label={`Oto ${statusLabel($pipelineState)}`}
>
  <!-- Drag only the status region so Cancel/Dismiss stay clickable under Tauri. -->
  <div class="flex min-w-0 flex-1 items-center gap-3" data-tauri-drag-region>
    <span
      class="h-2.5 w-2.5 shrink-0 rounded-full
        {$pipelineState === 'listening' ? 'bg-rose-500 shadow-[0_0_8px_#f43f5e]' : ''}
        {$pipelineState === 'processing' ? 'bg-amber-400 animate-pulse' : ''}
        {$pipelineState === 'done' ? 'bg-emerald-400' : ''}
        {$pipelineState === 'error' ? 'bg-red-500' : ''}
        {$pipelineState === 'idle' ? 'bg-slate-400' : ''}
      "
      aria-hidden="true"
    ></span>

    {#if $pipelineState === "listening"}
      <Waveform />
      <span class="min-w-0 truncate opacity-90">{$partialTranscript || ($pipelineDetail ? `${$pipelineDetail} · Listening…` : "Listening…")}</span>
    {:else if $pipelineState === "processing"}
      <span class="min-w-0 truncate opacity-90">{$partialTranscript || $pipelineDetail || $pipelinePhase || "Processing…"}</span>
    {:else if $pipelineState === "error"}
      <span class="min-w-0 truncate text-rose-200" title={$pipelineDetail}>{$pipelineDetail || "Something went wrong"}</span>
    {:else if $pipelineState === "done"}
      <span class="min-w-0 truncate opacity-90" title={$pipelineDetail}>{$pipelineDetail || "Done"}</span>
    {:else}
      <span class="opacity-70">Oto</span>
    {/if}
  </div>

  {#if $pipelineState === "listening" || $pipelineState === "processing"}
    <button
      type="button"
      class="shrink-0 rounded-full bg-white/10 px-2.5 py-1 text-xs hover:bg-white/20 disabled:opacity-50"
      disabled={cancelBusy}
      onclick={cancel}
    >
      Cancel
    </button>
  {:else if $pipelineState === "error"}
    <button
      type="button"
      class="shrink-0 rounded-full bg-white/10 px-2.5 py-1 text-xs hover:bg-white/20 disabled:opacity-50"
      disabled={cancelBusy}
      onclick={cancel}
    >
      Dismiss
    </button>
  {/if}
</div>
