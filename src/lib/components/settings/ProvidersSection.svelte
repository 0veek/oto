<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { IconChevronDown } from "@tabler/icons-svelte";
  import type { AppConfig, ProviderPreset, ProviderProfile } from "$lib/types";

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
      if (preset === "custom" && config.active_custom_provider_id) {
        const present = await invoke<boolean>("provider_api_key_present", {
          account: `custom:${config.active_custom_provider_id}`,
        });
        keyPresent = present;
        keyHint = present ? "••••" : null;
        return;
      }
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
    config.active_custom_provider_id;
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
      if (config.provider_preset === "custom" && config.active_custom_provider_id) {
        await invoke("set_provider_api_key", {
          account: `custom:${config.active_custom_provider_id}`,
          key: keyDraft,
        });
      } else {
        await invoke("set_api_key", {
          preset: config.provider_preset,
          key: keyDraft,
        });
      }
      keyDraft = "";
      await refreshKeyInfo(config.provider_preset);
      keyStatus = keyPresent ? "API key saved to keyring" : "API key cleared";
    } catch (e) {
      keyStatus = `Failed to save key: ${String(e)}`;
    } finally {
      keyBusy = false;
    }
  }

  function addProfile() {
    const id = globalThis.crypto?.randomUUID?.() ?? `provider-${Date.now()}`;
    config.custom_providers = [...config.custom_providers, {
      id,
      name: "New provider",
      base_url: "https://api.example.com/v1",
      stt_model: "whisper-1",
      polish_model: "gpt-4o-mini",
    }];
    config.provider_preset = "custom";
    config.active_custom_provider_id = id;
  }

  function patchProfile(id: string, patch: Partial<ProviderProfile>) {
    config.custom_providers = config.custom_providers.map((profile) => profile.id === id ? { ...profile, ...patch } : profile);
  }

  function removeProfile(id: string) {
    config.custom_providers = config.custom_providers.filter((profile) => profile.id !== id);
    if (config.active_custom_provider_id === id) {
      config.active_custom_provider_id = config.custom_providers[0]?.id ?? null;
    }
  }
</script>

<section class="provider-section">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Providers</h2>
    <p class="mt-1 text-sm text-slate-400">
      Choose an OpenAI-compatible provider and store your API key in the OS keyring.
    </p>
  </header>

  <div class="provider-form">
    <label class="provider-row">
      <span class="provider-row__label">Provider preset</span>
      <div class="provider-row__control select-wrap">
        <select
          class="provider-select"
          value={config.provider_preset}
          onchange={onPresetChange}
        >
          {#each PRESET_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
        <IconChevronDown aria-hidden="true" size={16} stroke={1.7} />
      </div>
    </label>

    {#if config.provider_preset === "custom"}
      <div class="provider-row provider-row--stacked">
        <div class="provider-row__label">Custom provider</div>
        <div class="provider-row__control space-y-3 rounded-xl border border-white/10 bg-slate-900/30 p-4">
        <div class="flex items-center justify-between gap-3">
          <div><div class="text-sm font-medium text-slate-200">Provider profiles</div><div class="text-xs text-slate-500">Declarative plugins for OpenAI-compatible endpoints.</div></div>
          <button type="button" class="rounded-lg bg-white/10 px-3 py-1.5 text-xs hover:bg-white/15" onclick={addProfile}>Add profile</button>
        </div>
        <div class="select-wrap">
          <select class="w-full rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm text-white" value={config.active_custom_provider_id ?? ""} onchange={(event) => config.active_custom_provider_id = event.currentTarget.value || null}>
            <option value="">Legacy custom fields below</option>
            {#each config.custom_providers as profile (profile.id)}<option value={profile.id}>{profile.name}</option>{/each}
          </select>
          <IconChevronDown aria-hidden="true" size={16} stroke={1.7} />
        </div>
          {#if config.active_custom_provider_id}
          {@const profile = config.custom_providers.find((item) => item.id === config.active_custom_provider_id)}
          {#if profile}
            <div class="grid gap-2 sm:grid-cols-2">
              <input aria-label="Profile name" class="rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm" value={profile.name} oninput={(event) => patchProfile(profile.id, { name: event.currentTarget.value })} />
              <input aria-label="Profile base URL" class="rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm" value={profile.base_url} oninput={(event) => patchProfile(profile.id, { base_url: event.currentTarget.value })} />
              <input aria-label="Profile STT model" class="rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm" value={profile.stt_model} oninput={(event) => patchProfile(profile.id, { stt_model: event.currentTarget.value })} />
              <input aria-label="Profile polish model" class="rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm" value={profile.polish_model} oninput={(event) => patchProfile(profile.id, { polish_model: event.currentTarget.value })} />
            </div>
            <button type="button" class="text-xs text-rose-300" onclick={() => removeProfile(profile.id)}>Remove this profile</button>
          {:else}
            <p class="text-xs text-amber-300">
              Active profile is missing. Choose another profile or use legacy fields below.
            </p>
          {/if}
          {/if}
        </div>
      </div>
    {/if}

    {#if config.provider_preset !== "custom" || !config.active_custom_provider_id}
      <label class="provider-row">
        <span class="provider-row__label">Base URL</span>
        <span class="provider-row__control">
          <input
            type="url"
            class="w-full"
            placeholder="https://api.example.com/v1"
            bind:value={config.base_url}
          />
          <span class="provider-row__hint">
            OpenAI-compatible API root (…/v1). Updated automatically for known presets.
          </span>
        </span>
      </label>
    {:else}
      <div class="provider-row">
        <span class="provider-row__label">Base URL</span>
        <p class="provider-row__control provider-row__hint">Base URL and model IDs come from the active provider profile above.</p>
      </div>
    {/if}

    <div class="provider-row">
      <span class="provider-row__label">API key</span>
      <div class="provider-row__control">
        <div class="provider-key-field">
          <input
            type="password"
            class="min-w-0 flex-1"
            placeholder={keyPresent ? "Enter new key to replace…" : "••••••••••••••••••••"}
            autocomplete="off"
            spellcheck="false"
            bind:value={keyDraft}
          />
          <button
            type="button"
            class="provider-key-field__button"
            disabled={keyBusy}
            onclick={saveKey}
          >
            {keyBusy ? "Saving…" : "Save Key"}
          </button>
        </div>
        <p class="provider-row__hint">
          Keys never write to config.json — only the OS keyring.
          {#if keyPresent && keyHint}
            <span class="text-emerald-400/90"> Stored: {keyHint}</span>
          {:else if !keyPresent}
            <span class="text-amber-400/90"> No key stored for this preset.</span>
          {/if}
        </p>
        {#if keyStatus}
          <p class="provider-row__hint text-slate-300">{keyStatus}</p>
        {/if}
      </div>
    </div>
  </div>
</section>
