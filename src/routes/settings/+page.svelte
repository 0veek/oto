<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig } from "$lib/types";
  import SettingsShell from "$lib/components/settings/SettingsShell.svelte";
  import ProvidersSection from "$lib/components/settings/ProvidersSection.svelte";
  import ModelsSection from "$lib/components/settings/ModelsSection.svelte";
  import HotkeysSection from "$lib/components/settings/HotkeysSection.svelte";

  const SECTIONS = [
    { id: "providers", label: "Providers" },
    { id: "models", label: "Models" },
    { id: "hotkeys", label: "Hotkeys" },
    { id: "dictionary", label: "Dictionary" },
    { id: "appearance", label: "Appearance" },
    { id: "injection", label: "Injection" },
    { id: "about", label: "About" },
  ] as const;

  type SectionId = (typeof SECTIONS)[number]["id"];

  let config = $state<AppConfig | null>(null);
  let active = $state<SectionId>("providers");
  let loadError = $state<string | null>(null);
  let saveStatus = $state<string | null>(null);
  let saving = $state(false);

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
</script>

{#if !config}
  <div class="flex h-screen items-center justify-center bg-slate-950 text-slate-400">
    Loading settings…
  </div>
{:else}
  <SettingsShell
    sections={[...SECTIONS]}
    {active}
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
      {:else if active === "appearance"}
        <section class="space-y-4">
          <header>
            <h2 class="text-xl font-semibold tracking-tight">Appearance</h2>
            <p class="mt-1 text-sm text-slate-400">
              Preview the overlay UI without running dictation.
            </p>
          </header>
          <div
            class="rounded-2xl border border-white/10 bg-white/[0.03] p-6 space-y-4 backdrop-blur-xl"
          >
            <p class="text-sm text-slate-400">
              Emit mock listening + level events so you can check the glass pill and waveform.
            </p>
            <button
              type="button"
              class="rounded-xl bg-white/10 px-4 py-2 text-sm font-medium text-white ring-1 ring-white/15 transition hover:bg-white/15"
              onclick={async () => {
                try {
                  await invoke("debug_preview_listening");
                } catch (e) {
                  saveStatus = `Preview failed: ${String(e)}`;
                }
              }}
            >
              Preview listening UI
            </button>
          </div>
        </section>
      {:else}
        <section class="space-y-4">
          <header>
            <h2 class="text-xl font-semibold tracking-tight capitalize">{active}</h2>
            <p class="mt-1 text-sm text-slate-400">Coming soon — next task.</p>
          </header>
          <div
            class="rounded-2xl border border-dashed border-white/15 bg-white/[0.03] p-10 text-center text-sm text-slate-500 backdrop-blur-xl"
          >
            This section will be implemented in a later task.
          </div>
        </section>
      {/if}

      {#if active === "providers" || active === "models" || active === "hotkeys"}
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
