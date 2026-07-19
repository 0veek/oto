<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, StylePreset } from "$lib/types";

  let { config = $bindable() }: { config: AppConfig } = $props();
  let commandBusy = $state(false);
  let commandStatus = $state<string | null>(null);

  function patchStyle(id: string, patch: Partial<StylePreset>) {
    config.styles = config.styles.map((style) => style.id === id ? { ...style, ...patch } : style);
  }

  function addStyle() {
    const id = globalThis.crypto?.randomUUID?.() ?? `style-${Date.now()}`;
    config.styles = [...config.styles, { id, name: "Custom style", prompt: "" }];
    config.active_style_id = id;
  }

  async function startCommandMode() {
    commandBusy = true;
    commandStatus = "Refocus the app and keep text selected — capture starts in 2 seconds.";
    try {
      await invoke("set_config", { cfg: config });
      await invoke("start_command_mode", { focusDelayMs: 2000 });
      commandStatus = "Listening. Say the edit, then tap/release your dictation hotkey or use tray → Stop Listening.";
    } catch (error) {
      commandStatus = `Command Mode failed: ${String(error)}`;
    } finally {
      commandBusy = false;
    }
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Styles &amp; commands</h2>
    <p class="mt-1 text-sm text-slate-400">Choose reusable writing guidance or edit selected text with a spoken instruction.</p>
  </header>

  <div class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl">
    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Active style</span>
      <select class="w-full rounded-xl border border-white/10 bg-slate-900 px-3 py-2.5 text-sm text-white" value={config.active_style_id ?? ""} onchange={(event) => config.active_style_id = event.currentTarget.value || null}>
        <option value="">No preset</option>
        {#each config.styles as style (style.id)}<option value={style.id}>{style.name}</option>{/each}
      </select>
    </label>

    <div class="space-y-3">
      {#each config.styles as style (style.id)}
        <article class="rounded-xl border border-white/10 bg-slate-900/40 p-4">
          <div class="flex gap-2">
            <input aria-label="Style name" class="min-w-0 flex-1 rounded-lg border border-white/10 bg-slate-950/70 px-3 py-2 text-sm font-medium" value={style.name} oninput={(event) => patchStyle(style.id, { name: event.currentTarget.value })} />
            <button type="button" class="rounded-lg px-2 text-xs text-rose-300 hover:bg-white/10" onclick={() => { config.styles = config.styles.filter((item) => item.id !== style.id); if (config.active_style_id === style.id) config.active_style_id = null; }}>Remove</button>
          </div>
          <textarea aria-label="Style prompt" class="mt-2 w-full resize-y rounded-lg border border-white/10 bg-slate-950/70 px-3 py-2 text-sm text-slate-300" rows="2" value={style.prompt} oninput={(event) => patchStyle(style.id, { prompt: event.currentTarget.value })}></textarea>
        </article>
      {/each}
      <button type="button" class="rounded-xl bg-white/10 px-4 py-2 text-sm ring-1 ring-white/15 hover:bg-white/15" onclick={addStyle}>Add style</button>
    </div>
  </div>

  <div class="space-y-4 rounded-2xl border border-violet-400/20 bg-violet-400/[0.06] p-6">
    <div>
      <h3 class="font-medium text-violet-100">Command Mode</h3>
      <p class="mt-1 text-sm text-slate-400">Select text in any app, start Command Mode, then say something like “make this concise” or “translate this to Spanish.” The replacement uses your configured polish model.</p>
    </div>
    <button type="button" class="rounded-xl bg-violet-500 px-4 py-2.5 text-sm font-medium text-white hover:bg-violet-400 disabled:opacity-50" disabled={commandBusy} onclick={startCommandMode}>{commandBusy ? "Preparing…" : "Start Command Mode"}</button>
    {#if commandStatus}<p aria-live="polite" class="text-sm text-violet-100/90">{commandStatus}</p>{/if}
  </div>
</section>

