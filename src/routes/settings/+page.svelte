<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { IconCircleCheck } from "@tabler/icons-svelte";
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
  import PermissionsSection from "$lib/components/settings/PermissionsSection.svelte";
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
    { id: "permissions", label: "Permissions" },
    { id: "appearance", label: "Appearance" },
    { id: "privacy", label: "Privacy" },
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

  function browserPreviewDefaults(): AppConfig {
    return {
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

  async function loadConfig() {
    loadError = null;
    try {
      config = await invoke<AppConfig>("get_config");
    } catch (e) {
      // Browser/dev without Tauri: keep page usable for layout checks only.
      // Never install defaults into a live Tauri session — Save would wipe disk config.
      const browserPreview = ["http:", "https:"].includes(window.location.protocol);
      if (browserPreview) {
        loadError = null;
        config = browserPreviewDefaults();
      } else {
        loadError = String(e);
        config = null;
      }
    }
  }

  async function saveConfig() {
    if (!config || saving || loadError) return;
    saving = true;
    saveStatus = null;
    try {
      // Normalize values the backend also clamps so the form stays consistent.
      config.history_limit = Math.min(1000, Math.max(1, Math.round(Number(config.history_limit) || 100)));
      config.font_scale = Math.min(1.25, Math.max(0.85, Number(config.font_scale) || 1));
      config.temperature = Math.min(1, Math.max(0, Number(config.temperature) || 0));
      // Never pass API keys through set_config — keys use set_api_key only
      await invoke("set_config", { cfg: config });
      // Reload so server-side normalization (hotkey formatting, etc.) is reflected.
      try {
        config = await invoke<AppConfig>("get_config");
      } catch {
        // Keep local draft if reload fails.
      }
      saveStatus = "Saved";
      setTimeout(() => {
        if (saveStatus === "Saved") saveStatus = null;
      }, 2000);
    } catch (e) {
      const message = String(e);
      // Hotkey / portal registration failures leave the draft invalid — restore
      // the last good backend config so the UI does not keep a broken chord.
      try {
        config = await invoke<AppConfig>("get_config");
        if (message.toLowerCase().includes("hotkey") || message.toLowerCase().includes("portal") || message.toLowerCase().includes("shortcut")) {
          saveStatus = `Hotkey not registered: ${message}. Restored previous shortcut.`;
        } else {
          saveStatus = `Save failed: ${message}. Reloaded last saved settings.`;
        }
      } catch {
        saveStatus = `Save failed: ${message}`;
      }
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    const requestedSection = new URLSearchParams(window.location.search).get("section");
    if (requestedSection && SECTIONS.some((section) => section.id === requestedSection)) {
      active = requestedSection as SectionId;
    }
    void loadConfig();
    const onKeydown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s" && SAVABLE.includes(active)) {
        event.preventDefault();
        void saveConfig();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  $effect(() => {
    if (config) applyTheme(config.theme, config.reduce_motion, config.font_scale);
  });
</script>

{#if !config}
  <div class="flex h-screen flex-col items-center justify-center gap-3 bg-slate-950 px-6 text-center text-slate-400">
    {#if loadError}
      <p class="max-w-md text-sm text-amber-100/90">
        Could not load settings from Oto ({loadError}). Your on-disk configuration was not modified.
      </p>
      <button
        type="button"
        class="rounded-xl border border-white/15 bg-white/5 px-4 py-2 text-sm text-white transition hover:bg-white/10"
        onclick={() => void loadConfig()}
      >
        Retry
      </button>
    {:else}
      Loading settings…
    {/if}
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
    <div class="settings-stage">

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
      {:else if active === "permissions"}
        <PermissionsSection />
      {:else if active === "privacy"}
        <PrivacySection bind:config />
      {:else if active === "injection"}
        <InjectionSection bind:config />
      {:else if active === "about"}
        <AboutSection />
      {/if}

      {#if SAVABLE.includes(active)}
        <div class="settings-actionbar">
          <span class="settings-actionbar__note">
            Changes are stored locally and take effect after saving.
          </span>
          <div class="flex items-center gap-3">
            {#if saveStatus?.startsWith("Save failed")}
              <span class="text-sm text-rose-400" role="alert">{saveStatus}</span>
            {/if}
          <button
            type="button"
            class="settings-actionbar__button"
            disabled={saving}
            onclick={saveConfig}
          >
            {#if saveStatus === "Saved"}
              <IconCircleCheck aria-hidden="true" size={18} stroke={1.8} />
              Saved
            {:else}
              {saving ? "Saving…" : "Save Changes"}
            {/if}
          </button>
          </div>
        </div>
      {/if}
    </div>
  </SettingsShell>
{/if}
