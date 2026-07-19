<script lang="ts">
  import Waveform from "./Waveform.svelte";
  import { pipelineState, pipelineDetail, pipelinePhase } from "../stores/pipeline";
  import { invoke } from "@tauri-apps/api/core";

  async function cancel() {
    await invoke("cancel_dictation");
  }
</script>

<div
  class="flex items-center gap-3 rounded-full border border-white/25 bg-slate-900/90 px-4 py-2.5 text-sm text-white shadow-2xl backdrop-blur-2xl"
  data-tauri-drag-region
>
  <span
    class="h-2.5 w-2.5 rounded-full
      {$pipelineState === 'listening' ? 'bg-rose-500 shadow-[0_0_8px_#f43f5e]' : ''}
      {$pipelineState === 'processing' ? 'bg-amber-400 animate-pulse' : ''}
      {$pipelineState === 'done' ? 'bg-emerald-400' : ''}
      {$pipelineState === 'error' ? 'bg-red-500' : ''}
      {$pipelineState === 'idle' ? 'bg-slate-400' : ''}
    "
  ></span>

  {#if $pipelineState === "listening"}
    <Waveform />
    <span class="opacity-90">Listening…</span>
  {:else if $pipelineState === "processing"}
    <span class="opacity-90">{$pipelineDetail || $pipelinePhase || "Processing…"}</span>
  {:else if $pipelineState === "error"}
    <span class="max-w-[200px] truncate text-rose-200">{$pipelineDetail}</span>
  {:else if $pipelineState === "done"}
    <span class="max-w-[200px] truncate opacity-90">{$pipelineDetail || "Done"}</span>
  {:else}
    <span class="opacity-70">Oto</span>
  {/if}

  {#if $pipelineState === "listening" || $pipelineState === "processing"}
    <button
      class="rounded-full bg-white/10 px-2 py-0.5 text-xs hover:bg-white/20"
      onclick={cancel}
    >
      Cancel
    </button>
  {:else if $pipelineState === "error"}
    <button
      class="rounded-full bg-white/10 px-2 py-0.5 text-xs hover:bg-white/20"
      onclick={cancel}
    >
      Dismiss
    </button>
  {/if}
</div>
