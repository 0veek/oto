<script lang="ts">
  import type { AppConfig } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Models</h2>
    <p class="mt-1 text-sm text-slate-400">
      Speech-to-text and optional polish model settings.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">STT model</span>
      <input
        type="text"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="whisper-large-v3"
        bind:value={config.stt_model}
      />
      <span class="text-xs text-slate-500">
        Model id for transcription (e.g. whisper-large-v3, whisper-1).
      </span>
    </label>

    <label class="flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3">
      <div>
        <div class="text-sm font-medium text-slate-200">Enable polish</div>
        <div class="text-xs text-slate-500">
          Clean up transcripts with an LLM before injection.
        </div>
      </div>
      <input
        type="checkbox"
        class="h-4 w-4 rounded border-white/20 bg-slate-900 text-sky-500 focus:ring-sky-400/30"
        bind:checked={config.polish_enabled}
      />
    </label>

    <label class="block space-y-1.5" class:opacity-50={!config.polish_enabled}>
      <span class="text-sm font-medium text-slate-300">Polish model</span>
      <input
        type="text"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20 disabled:cursor-not-allowed"
        placeholder="llama-3.1-8b-instant"
        disabled={!config.polish_enabled}
        bind:value={config.polish_model}
      />
    </label>

    <label class="block space-y-1.5" class:opacity-50={!config.polish_enabled}>
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-slate-300">Temperature</span>
        <span class="tabular-nums text-xs text-slate-400">{config.temperature.toFixed(2)}</span>
      </div>
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        class="w-full accent-sky-400 disabled:cursor-not-allowed"
        disabled={!config.polish_enabled}
        bind:value={config.temperature}
      />
      <span class="text-xs text-slate-500">Lower is more deterministic (default 0.2).</span>
    </label>

    <label class="block space-y-1.5" class:opacity-50={!config.polish_enabled}>
      <span class="text-sm font-medium text-slate-300">Tone hint</span>
      <textarea
        rows="3"
        class="w-full resize-y rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20 disabled:cursor-not-allowed"
        placeholder="e.g. professional, concise, no fluff"
        disabled={!config.polish_enabled}
        bind:value={config.tone_hint}
      ></textarea>
      <span class="text-xs text-slate-500">
        Optional guidance for how polished text should sound.
      </span>
    </label>
  </div>
</section>
