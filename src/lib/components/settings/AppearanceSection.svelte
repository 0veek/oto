<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { IconChevronDown } from "@tabler/icons-svelte";
  import type { AppConfig, IdleBehavior, ThemePreset } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let previewBusy = $state(false);
  let micBusy = $state(false);
  let status = $state<string | null>(null);

  const IDLE_OPTIONS: {
    value: IdleBehavior;
    label: string;
    hint: string;
  }[] = [
    {
      value: "hide",
      label: "Hide when idle",
      hint: "Overlay only appears during listening, processing, done, or error.",
    },
    {
      value: "minimal",
      label: "Minimal dormant pill",
      hint: "Keep a small dormant pill visible when idle so you can find Oto.",
    },
  ];

  const THEMES: { value: ThemePreset; label: string }[] = [
    { value: "system", label: "System" },
    { value: "midnight", label: "Midnight" },
    { value: "light", label: "Light" },
    { value: "high_contrast", label: "High contrast" },
  ];

  async function previewListening() {
    previewBusy = true;
    status = null;
    try {
      await invoke("debug_preview_listening");
      status = "Preview events sent — check the overlay.";
    } catch (e) {
      status = `Preview failed: ${String(e)}`;
    } finally {
      previewBusy = false;
    }
  }

  async function testMic() {
    micBusy = true;
    status = null;
    try {
      await invoke("test_microphone");
      status = "Mic test finished (~2s of levels).";
    } catch (e) {
      status = `Mic test failed: ${String(e)}`;
    } finally {
      micBusy = false;
    }
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Appearance</h2>
    <p class="mt-1 text-sm text-slate-400">
      Overlay idle behavior and quick UI previews without a full dictation run.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <div class="grid gap-4 sm:grid-cols-2">
      <label class="block space-y-1.5">
        <span class="text-sm font-medium text-slate-300">Theme</span>
        <div class="select-wrap">
          <select class="w-full rounded-xl border border-white/10 bg-slate-900 px-3 py-2.5 text-sm text-white" bind:value={config.theme}>
            {#each THEMES as theme}<option value={theme.value}>{theme.label}</option>{/each}
          </select>
          <IconChevronDown aria-hidden="true" size={16} stroke={1.7} />
        </div>
      </label>
      <label class="block space-y-1.5">
        <span class="text-sm font-medium text-slate-300">Text size · {Math.round(config.font_scale * 100)}%</span>
        <input class="w-full accent-sky-400" type="range" min="0.85" max="1.25" step="0.05" bind:value={config.font_scale} />
      </label>
    </div>

    <label class="flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3">
      <span><span class="block text-sm font-medium text-slate-200">Reduce motion</span><span class="block text-xs text-slate-500">Disable non-essential pulses and transitions.</span></span>
      <input type="checkbox" bind:checked={config.reduce_motion} />
    </label>

    <fieldset class="space-y-3">
      <legend class="text-sm font-medium text-slate-300">When idle</legend>
      {#each IDLE_OPTIONS as opt (opt.value)}
        <label
          class="flex cursor-pointer items-start gap-3 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3 transition hover:border-white/20 {config.idle_behavior ===
          opt.value
            ? 'ring-1 ring-sky-400/40'
            : ''}"
        >
          <input
            type="radio"
            name="idle_behavior"
            class="mt-1 h-4 w-4 border-white/20 bg-slate-900 text-sky-500 focus:ring-sky-400/30"
            value={opt.value}
            checked={config.idle_behavior === opt.value}
            onchange={() => {
              config.idle_behavior = opt.value;
            }}
          />
          <span>
            <span class="block text-sm font-medium text-slate-200">{opt.label}</span>
            <span class="block text-xs text-slate-500">{opt.hint}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="space-y-3 border-t border-white/10 pt-4">
      <div>
        <div class="text-sm font-medium text-slate-200">Preview &amp; mic test</div>
        <p class="mt-0.5 text-xs text-slate-500">
          Preview sends mock listening levels. Mic test opens the default input for ~2s and
          streams real levels to the overlay.
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          class="rounded-xl bg-white/10 px-4 py-2 text-sm font-medium text-white ring-1 ring-white/15 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={previewBusy}
          onclick={previewListening}
        >
          {previewBusy ? "Previewing…" : "Preview listening UI"}
        </button>
        <button
          type="button"
          class="rounded-xl bg-sky-500/90 px-4 py-2 text-sm font-medium text-white transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={micBusy}
          onclick={testMic}
        >
          {micBusy ? "Listening…" : "Test microphone"}
        </button>
      </div>
      {#if status}
        <p
          class="text-sm {status.includes('failed') || status.includes('Failed')
            ? 'text-rose-400'
            : 'text-slate-300'}"
        >
          {status}
        </p>
      {/if}
    </div>
  </div>
</section>
