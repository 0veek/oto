<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig } from "$lib/types";
  import SettingsShell from "$lib/components/settings/SettingsShell.svelte";
  import ProvidersSection from "$lib/components/settings/ProvidersSection.svelte";
  import ModelsSection from "$lib/components/settings/ModelsSection.svelte";
  import HotkeysSection from "$lib/components/settings/HotkeysSection.svelte";
  import DictionarySection from "$lib/components/settings/DictionarySection.svelte";
  import SnippetsSection from "$lib/components/settings/SnippetsSection.svelte";
  import StylesSection from "$lib/components/settings/StylesSection.svelte";
  import HistorySection from "$lib/components/settings/HistorySection.svelte";
  import PrivacySection from "$lib/components/settings/PrivacySection.svelte";
  import AppearanceSection from "$lib/components/settings/AppearanceSection.svelte";
  import InjectionSection from "$lib/components/settings/InjectionSection.svelte";
  import AboutSection from "$lib/components/settings/AboutSection.svelte";
  import { applyTheme } from "$lib/theme";

  const SECTIONS = [
    { id: "providers", label: "Providers" },
    { id: "models", label: "Models" },
    { id: "hotkeys", label: "Hotkeys" },
    { id: "dictionary", label: "Dictionary" },
    { id: "snippets", label: "Snippets" },
    { id: "styles", label: "Styles & commands" },
    { id: "history", label: "History" },
    { id: "appearance", label: "Appearance" },
    { id: "privacy", label: "Privacy & sync" },
    { id: "injection", label: "Injection" },
    { id: "about", label: "About" },
  ] as const;

  type SectionId = (typeof SECTIONS)[number]["id"];

  let config = $state<AppConfig | null>(null);
  let active = $state<SectionId>("providers");
  let loadError = $state<string | null>(null);
  let saveStatus = $state<string | null>(null);
  let saving = $state(false);

  const SAVABLE: SectionId[] = [
    "providers",
    "models",
    "hotkeys",
    "dictionary",
    "snippets",
    "styles",
    "appearance",
    "privacy",
    "injection",
  ];

  async function loadConfig() {
    loadError = null;
    try {
      config = await invoke<AppConfig>("get_config");
    } catch (e) {
      // Browser/dev without Tauri: keep page usable for layout checks
      loadError = String(e);
      config = {
        provider_preset: "groq",
        base_url: "https://api.groq.com/openai/v1",
        stt_model: "whisper-large-v3",
        polish_model: "llama-3.1-8b-instant",
        polish_enabled: true,
        temperature: 0.2,
        tone_hint: "",
        hotkey: "Ctrl+Super+Space",
        language: null,
        dictionary: [],
        injection_mode: "auto",
        idle_behavior: "hide",
        overlay_x: null,
        overlay_y: null,
        stt_backend: "cloud",
        local_whisper_model_path: "",
        vocabulary_boost: true,
        snippets: [],
        styles: [
          { id: "professional", name: "Professional", prompt: "Professional, clear, and concise." },
          { id: "casual", name: "Casual", prompt: "Natural and friendly." },
        ],
        active_style_id: null,
        history_enabled: true,
        history_limit: 100,
        streaming_enabled: false,
        theme: "midnight",
        reduce_motion: false,
        font_scale: 1,
        custom_providers: [],
        active_custom_provider_id: null,
        sync: { enabled: false, endpoint: "" },
      };
    }
  }

  async function saveConfig() {
    if (!config) return;
    saving = true;
    saveStatus = null;
    try {
      // Never pass API keys through set_config — keys use set_api_key only
      await invoke("set_config", { cfg: config });
      saveStatus = "Saved";
      setTimeout(() => {
        if (saveStatus === "Saved") saveStatus = null;
      }, 2000);
    } catch (e) {
      saveStatus = `Save failed: ${String(e)}`;
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    void loadConfig();
  });

  $effect(() => {
    if (config) applyTheme(config.theme, config.reduce_motion, config.font_scale);
  });
</script>

{#if !config}
  <div class="flex h-screen items-center justify-center bg-slate-950 text-slate-400">
    Loading settings…
  </div>
{:else}
  <SettingsShell
    sections={[...SECTIONS]}
    {active}
    theme={config.theme}
    onselect={(id) => {
      active = id as SectionId;
      saveStatus = null;
    }}
  >
    <div class="mx-auto max-w-2xl space-y-6">
      {#if loadError}
        <div
          class="rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-100/90"
        >
          Could not load config from Tauri ({loadError}). Showing defaults for UI preview.
        </div>
      {/if}

      {#if active === "providers"}
        <ProvidersSection bind:config />
      {:else if active === "models"}
        <ModelsSection bind:config />
      {:else if active === "hotkeys"}
        <HotkeysSection bind:config />
      {:else if active === "dictionary"}
        <DictionarySection bind:config />
      {:else if active === "snippets"}
        <SnippetsSection bind:config />
      {:else if active === "styles"}
        <StylesSection bind:config />
      {:else if active === "history"}
        <HistorySection />
      {:else if active === "appearance"}
        <AppearanceSection bind:config />
      {:else if active === "privacy"}
        <PrivacySection bind:config />
      {:else if active === "injection"}
        <InjectionSection bind:config />
      {:else if active === "about"}
        <AboutSection />
      {/if}

      {#if SAVABLE.includes(active)}
        <div class="flex items-center justify-end gap-3 pt-2">
          {#if saveStatus}
            <span
              class="text-sm {saveStatus.startsWith('Save failed')
                ? 'text-rose-400'
                : 'text-emerald-400'}"
            >
              {saveStatus}
            </span>
          {/if}
          <button
            type="button"
            class="rounded-xl bg-white/10 px-5 py-2.5 text-sm font-medium text-white ring-1 ring-white/15 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={saving}
            onclick={saveConfig}
          >
            {saving ? "Saving…" : "Save settings"}
          </button>
        </div>
      {/if}
    </div>
  </SettingsShell>
{/if}
