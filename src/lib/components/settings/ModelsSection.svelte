<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, SttBackend } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let testBusy = $state(false);
  let testResult = $state<string | null>(null);
  let testError = $state<string | null>(null);

  async function testTranscription() {
    testBusy = true;
    testResult = null;
    testError = null;
    try {
      testResult = await invoke<string>("test_transcription");
    } catch (e) {
      testError = String(e);
    } finally {
      testBusy = false;
    }
  }
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
    <fieldset class="space-y-3">
      <legend class="text-sm font-medium text-slate-300">Transcription engine</legend>
      {#each [
        { value: "cloud", label: "Cloud provider", hint: "Use the configured OpenAI-compatible transcription API." },
        { value: "local_whisper", label: "Local Whisper", hint: "Offline transcription with a local ggml Whisper model." },
      ] as backend}
        <label class="flex cursor-pointer items-start gap-3 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3 {config.stt_backend === backend.value ? 'ring-1 ring-sky-400/40' : ''}">
          <input type="radio" name="stt_backend" value={backend.value} checked={config.stt_backend === backend.value} onchange={() => config.stt_backend = backend.value as SttBackend} />
          <span>
            <span class="block text-sm font-medium text-slate-200">{backend.label}</span>
            <span class="block text-xs text-slate-500">{backend.hint}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    {#if config.stt_backend === "local_whisper"}
      <label class="block space-y-1.5">
        <span class="text-sm font-medium text-slate-300">Local ggml model path</span>
        <input
          type="text"
          class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 font-mono text-sm text-white outline-none focus:border-sky-400/50"
          placeholder="/home/you/.local/share/oto/ggml-base.en.bin"
          bind:value={config.local_whisper_model_path}
        />
        <span class="text-xs text-slate-500">Use a whisper.cpp-compatible ggml model file. Audio and transcript remain on-device.</span>
      </label>
      {#if config.polish_enabled}
        <p class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3 text-xs text-amber-100/90">
          Transcription stays local, but the resulting text is still sent to the configured polish provider. Disable polish or use a localhost profile for a fully local pipeline.
        </p>
      {/if}
    {:else}
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
    {/if}

    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Language</span>
      <input
        type="text"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="auto-detect"
        value={config.language ?? ""}
        oninput={(e) => {
          const v = e.currentTarget.value;
          config.language = v.trim() === "" ? null : v.trim();
        }}
      />
      <span class="text-xs text-slate-500">
        Optional ISO code (e.g. en, es). Leave empty for auto-detect.
      </span>
    </label>

    <label class="flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3">
      <div>
        <div class="text-sm font-medium text-slate-200">Vocabulary boosting</div>
        <div class="text-xs text-slate-500">Pass dictionary terms as an STT prompt to improve names and technical spelling.</div>
      </div>
      <input type="checkbox" bind:checked={config.vocabulary_boost} />
    </label>

    <label class="flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3">
      <div>
        <div class="text-sm font-medium text-slate-200">Show partial results</div>
        <div class="text-xs text-slate-500">Display intermediate text whenever the active engine provides it.</div>
      </div>
      <input type="checkbox" bind:checked={config.streaming_enabled} />
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

    <div class="space-y-2 border-t border-white/10 pt-5">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-sm font-medium text-slate-200">Test transcription</div>
          <div class="text-xs text-slate-500">
            Run STT on the last dictation capture (dictate once first).
          </div>
        </div>
        <button
          type="button"
          class="shrink-0 rounded-xl bg-sky-500/90 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={testBusy}
          onclick={testTranscription}
        >
          {testBusy ? "Transcribing…" : "Test transcription"}
        </button>
      </div>
      {#if testResult !== null}
        <p class="rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-100">
          {testResult || "(empty transcript)"}
        </p>
      {/if}
      {#if testError}
        <p class="rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-sm text-amber-100">
          {testError}
        </p>
      {/if}
    </div>
  </div>
</section>
