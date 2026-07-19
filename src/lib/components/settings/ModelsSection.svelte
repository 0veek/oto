<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { IconCloud, IconCpu } from "@tabler/icons-svelte";
  import type { AppConfig, SttBackend } from "$lib/types";
  import PipelineMap from "./PipelineMap.svelte";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let testBusy = $state(false);
  let testResult = $state<string | null>(null);
  let testError = $state<string | null>(null);

  const activeProfile = $derived(
    config.provider_preset === "custom" && config.active_custom_provider_id
      ? config.custom_providers.find((profile) => profile.id === config.active_custom_provider_id) ?? null
      : null,
  );

  function patchActiveProfile(patch: { stt_model?: string; polish_model?: string }) {
    if (!activeProfile) return;
    config.custom_providers = config.custom_providers.map((profile) =>
      profile.id === activeProfile.id ? { ...profile, ...patch } : profile,
    );
  }

  async function testTranscription() {
    testBusy = true;
    testResult = null;
    testError = null;
    try {
      // Persist current model settings so the test uses what the form shows.
      await invoke("set_config", { cfg: config });
      testResult = await invoke<string>("test_transcription");
    } catch (e) {
      testError = String(e);
    } finally {
      testBusy = false;
    }
  }
</script>

<section class="settings-section">
  <header>
    <h2>Models</h2>
    <p>Configure the speech-to-text and post-processing pipeline.</p>
  </header>

  <PipelineMap />

  <div class="models-grid">
    <div class="settings-panel model-column">
      <div class="model-column__head">
        <h3>Transcription model</h3>
        <p>Choose where speech becomes text.</p>
      </div>

      <fieldset class="setting-field">
        <legend class="setting-field__label">Engine</legend>
        <div class="engine-switch">
          <label data-active={config.stt_backend === "cloud"}>
            <input type="radio" name="stt_backend" value="cloud" checked={config.stt_backend === "cloud"} onchange={() => config.stt_backend = "cloud" as SttBackend} />
            <IconCloud aria-hidden="true" size={17} stroke={1.6} />
            Cloud
          </label>
          <label data-active={config.stt_backend === "local_whisper"}>
            <input type="radio" name="stt_backend" value="local_whisper" checked={config.stt_backend === "local_whisper"} onchange={() => config.stt_backend = "local_whisper" as SttBackend} />
            <IconCpu aria-hidden="true" size={17} stroke={1.6} />
            Local Whisper
          </label>
        </div>
      </fieldset>

      {#if config.stt_backend === "local_whisper"}
        <label class="setting-field">
          <span class="setting-field__label">Local ggml model path</span>
          <input class="font-mono" type="text" placeholder="/home/you/.local/share/oto/ggml-base.en.bin" bind:value={config.local_whisper_model_path} />
          <span class="setting-field__hint">Use a whisper.cpp-compatible ggml file. Transcription remains on-device.</span>
        </label>
        {#if config.polish_enabled}
          <p class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3 text-xs text-amber-100/90">
            Polish still sends the resulting text to its configured provider. Disable it or use a localhost profile for a fully local pipeline.
          </p>
        {/if}
      {:else if activeProfile}
        <label class="setting-field">
          <span class="setting-field__label">STT model · {activeProfile.name}</span>
          <input
            class="font-mono"
            type="text"
            placeholder="whisper-large-v3"
            value={activeProfile.stt_model}
            oninput={(event) => patchActiveProfile({ stt_model: event.currentTarget.value })}
          />
          <span class="setting-field__hint">Stored on the active custom provider profile (not the legacy global field).</span>
        </label>
      {:else}
        <label class="setting-field">
          <span class="setting-field__label">STT model</span>
          <input class="font-mono" type="text" placeholder="whisper-large-v3" bind:value={config.stt_model} />
          <span class="setting-field__hint">The model identifier accepted by your active provider.</span>
        </label>
      {/if}

      <label class="setting-field">
        <span class="setting-field__label">Language</span>
        <input
          type="text"
          placeholder="Auto detect"
          value={config.language ?? ""}
          oninput={(event) => {
            const value = event.currentTarget.value;
            config.language = value.trim() === "" ? null : value.trim();
          }}
        />
        <span class="setting-field__hint">Optional ISO code such as en or es. Leave empty for automatic detection.</span>
      </label>

      <label class="setting-row">
        <span class="setting-row__copy">
          <strong>Vocabulary boost</strong>
          <span>Bias transcription toward terms in your dictionary.</span>
        </span>
        <input type="checkbox" bind:checked={config.vocabulary_boost} />
      </label>
    </div>

    <div class="settings-panel model-column">
      <div class="model-column__head">
        <h3>Decoding &amp; polish</h3>
        <p>Control interim text and final phrasing.</p>
      </div>

      <label class="setting-row">
        <span class="setting-row__copy">
          <strong>Partial results</strong>
          <span>Show intermediate text when the engine provides it.</span>
        </span>
        <input type="checkbox" bind:checked={config.streaming_enabled} />
      </label>

      <label class="setting-row">
        <span class="setting-row__copy">
          <strong>Enable polish</strong>
          <span>Refine grammar, punctuation, and tone before insertion.</span>
        </span>
        <input type="checkbox" bind:checked={config.polish_enabled} />
      </label>

      <label class="setting-field" class:opacity-50={!config.polish_enabled}>
        <span class="setting-field__label">
          Polish model{activeProfile ? ` · ${activeProfile.name}` : ""}
        </span>
        {#if activeProfile}
          <input
            class="font-mono"
            type="text"
            placeholder="llama-3.1-8b-instant"
            disabled={!config.polish_enabled}
            value={activeProfile.polish_model}
            oninput={(event) => patchActiveProfile({ polish_model: event.currentTarget.value })}
          />
        {:else}
          <input class="font-mono" type="text" placeholder="llama-3.1-8b-instant" disabled={!config.polish_enabled} bind:value={config.polish_model} />
        {/if}
      </label>

      <label class="setting-field" class:opacity-50={!config.polish_enabled}>
        <span class="flex items-center justify-between gap-3">
          <span class="setting-field__label">Temperature</span>
          <span class="font-mono tabular-nums text-xs text-slate-400">{config.temperature.toFixed(2)}</span>
        </span>
        <input type="range" min="0" max="1" step="0.05" disabled={!config.polish_enabled} bind:value={config.temperature} />
        <span class="setting-field__hint">Lower values keep edits more deterministic.</span>
      </label>

      <label class="setting-field" class:opacity-50={!config.polish_enabled}>
        <span class="setting-field__label">Tone hint</span>
        <textarea rows="3" placeholder="Technical, precise, no filler" disabled={!config.polish_enabled} bind:value={config.tone_hint}></textarea>
        <span class="setting-field__hint">Optional guidance for how final text should sound.</span>
      </label>

      <div class="setting-row">
        <span class="setting-row__copy">
          <strong>Test transcription</strong>
          <span>Run STT on your last completed dictation.</span>
        </span>
        <button type="button" class="bg-sky-500 px-4 text-sm font-medium" disabled={testBusy} onclick={testTranscription}>
          {testBusy ? "Transcribing…" : "Run test"}
        </button>
      </div>

      {#if testResult !== null}
        <p aria-live="polite" class="rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-100">
          {testResult || "No speech was detected in the last capture."}
        </p>
      {/if}
      {#if testError}
        <p role="alert" class="rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-sm text-amber-100">{testError}</p>
      {/if}
    </div>
  </div>
</section>
