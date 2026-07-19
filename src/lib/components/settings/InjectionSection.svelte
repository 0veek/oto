<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, InjectionMode } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let testBusy = $state(false);
  let testResult = $state<string | null>(null);
  let testError = $state<string | null>(null);

  const MODES: { value: InjectionMode; label: string; hint: string }[] = [
    {
      value: "auto",
      label: "Auto",
      hint: "AT-SPI, direct ydotool/wtype typing, clipboard + paste, then clipboard only.",
    },
    {
      value: "direct_type",
      label: "Direct type",
      hint: "Type through ydotool, wtype, or xdotool without touching the clipboard.",
    },
    {
      value: "clipboard_paste",
      label: "Clipboard + paste",
      hint: "Always copy, then simulate Ctrl+V via wtype/xdotool.",
    },
    {
      value: "clipboard_only",
      label: "Clipboard only",
      hint: "Copy text and prompt you to paste (Ctrl+V).",
    },
  ];

  async function testInjection() {
    testBusy = true;
    testResult = null;
    testError = null;
    try {
      // Persist mode first so the command reads the selection.
      await invoke("set_config", { cfg: config });
      testResult = await invoke<string>("test_injection");
    } catch (e) {
      testError = String(e);
    } finally {
      testBusy = false;
    }
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Injection</h2>
    <p class="mt-1 text-sm text-slate-400">
      How Oto delivers dictated text into the focused application.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <fieldset class="space-y-3">
      <legend class="text-sm font-medium text-slate-300">Mode</legend>
      {#each MODES as mode (mode.value)}
        <label
          class="flex cursor-pointer items-start gap-3 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3 transition hover:border-white/20 {config.injection_mode ===
          mode.value
            ? 'ring-1 ring-sky-400/40'
            : ''}"
        >
          <input
            type="radio"
            name="injection_mode"
            class="mt-1 h-4 w-4 border-white/20 bg-slate-900 text-sky-500 focus:ring-sky-400/30"
            value={mode.value}
            checked={config.injection_mode === mode.value}
            onchange={() => {
              config.injection_mode = mode.value;
            }}
          />
          <span>
            <span class="block text-sm font-medium text-slate-200">{mode.label}</span>
            <span class="block text-xs text-slate-500">{mode.hint}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div
      class="rounded-xl border border-white/10 bg-slate-900/30 px-4 py-3 text-xs leading-relaxed text-slate-400"
    >
      <p class="font-medium text-slate-300">Injection tools</p>
      <p class="mt-1">
        Wayland: install <code class="rounded bg-white/5 px-1">ydotool</code>
        (best for Chromium/Electron when its daemon is running) and/or
        <code class="rounded bg-white/5 px-1">wtype</code>.
        X11: install <code class="rounded bg-white/5 px-1">xdotool</code>.
        Without a tool, Auto falls back to clipboard-only.
      </p>
    </div>

    <div class="space-y-3 border-t border-white/10 pt-4">
      <div>
        <div class="text-sm font-medium text-slate-200">Test insertion</div>
        <p class="mt-0.5 text-xs text-slate-500">
          Click Test, then immediately focus a text field in another app. Oto waits briefly, then
          injects
          <code class="rounded bg-white/5 px-1">Oto injection test</code>
          using the mode above.
        </p>
      </div>
      <button
        type="button"
        class="rounded-xl bg-white/10 px-4 py-2 text-sm font-medium text-white ring-1 ring-white/15 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={testBusy}
        onclick={testInjection}
      >
        {testBusy ? "Testing…" : "Test insertion"}
      </button>
      {#if testResult}
        <p aria-live="polite" class="text-sm text-emerald-400">{testResult}</p>
      {/if}
      {#if testError}
        <p role="alert" class="text-sm text-rose-400">{testError}</p>
      {/if}
    </div>
  </div>
</section>
