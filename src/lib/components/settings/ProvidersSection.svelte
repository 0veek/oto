<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, ProviderPreset } from "$lib/types";

  const PRESET_DEFAULTS: Record<Exclude<ProviderPreset, "custom">, string> = {
    open_ai: "https://api.openai.com/v1",
    groq: "https://api.groq.com/openai/v1",
    open_router: "https://openrouter.ai/api/v1",
  };

  const PRESET_OPTIONS: { value: ProviderPreset; label: string }[] = [
    { value: "open_ai", label: "OpenAI" },
    { value: "groq", label: "Groq" },
    { value: "open_router", label: "OpenRouter" },
    { value: "custom", label: "Custom" },
  ];

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let keyDraft = $state("");
  let keyHint = $state<string | null>(null);
  let keyPresent = $state(false);
  let keyStatus = $state<string | null>(null);
  let keyBusy = $state(false);

  async function refreshKeyInfo(preset: ProviderPreset) {
    try {
      const [present, hint] = await Promise.all([
        invoke<boolean>("api_key_present", { preset }),
        invoke<string | null>("api_key_hint", { preset }),
      ]);
      keyPresent = present;
      keyHint = hint;
    } catch {
      keyPresent = false;
      keyHint = null;
    }
  }

  $effect(() => {
    void refreshKeyInfo(config.provider_preset);
  });

  function onPresetChange(event: Event) {
    const value = (event.target as HTMLSelectElement).value as ProviderPreset;
    config.provider_preset = value;
    if (value !== "custom") {
      config.base_url = PRESET_DEFAULTS[value];
    }
    keyDraft = "";
    keyStatus = null;
  }

  async function saveKey() {
    keyBusy = true;
    keyStatus = null;
    try {
      await invoke("set_api_key", {
        preset: config.provider_preset,
        key: keyDraft,
      });
      keyDraft = "";
      await refreshKeyInfo(config.provider_preset);
      keyStatus = keyPresent ? "API key saved to keyring" : "API key cleared";
    } catch (e) {
      keyStatus = `Failed to save key: ${String(e)}`;
    } finally {
      keyBusy = false;
    }
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Providers</h2>
    <p class="mt-1 text-sm text-slate-400">
      Choose an OpenAI-compatible provider and store your API key in the OS keyring.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Provider preset</span>
      <select
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        value={config.provider_preset}
        onchange={onPresetChange}
      >
        {#each PRESET_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </label>

    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Base URL</span>
      <input
        type="url"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="https://api.example.com/v1"
        bind:value={config.base_url}
      />
      <span class="text-xs text-slate-500">
        OpenAI-compatible API root (…/v1). Updated automatically for known presets.
      </span>
    </label>

    <div class="space-y-1.5">
      <span class="text-sm font-medium text-slate-300">API key</span>
      <div class="flex flex-col gap-2 sm:flex-row">
        <input
          type="password"
          class="min-w-0 flex-1 rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
          placeholder={keyPresent ? "Enter new key to replace…" : "sk-…"}
          autocomplete="off"
          spellcheck="false"
          bind:value={keyDraft}
        />
        <button
          type="button"
          class="shrink-0 rounded-xl bg-sky-500/90 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={keyBusy}
          onclick={saveKey}
        >
          {keyBusy ? "Saving…" : "Save key"}
        </button>
      </div>
      <p class="text-xs text-slate-500">
        Keys never write to config.json — only the OS keyring.
        {#if keyPresent && keyHint}
          <span class="text-emerald-400/90"> Stored: {keyHint}</span>
        {:else if !keyPresent}
          <span class="text-amber-400/90"> No key stored for this preset.</span>
        {/if}
      </p>
      {#if keyStatus}
        <p class="text-xs text-slate-300">{keyStatus}</p>
      {/if}
    </div>
  </div>
</section>
