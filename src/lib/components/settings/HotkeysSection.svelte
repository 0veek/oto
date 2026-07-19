<script lang="ts">
  import type { AppConfig } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Hotkeys</h2>
    <p class="mt-1 text-sm text-slate-400">
      Global push-to-talk shortcut. Hold to dictate, release to process.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Push-to-talk hotkey</span>
      <input
        type="text"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 font-mono text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="Ctrl+Super+Space"
        spellcheck="false"
        autocomplete="off"
        bind:value={config.hotkey}
      />
      <span class="block text-xs text-slate-500">
        Format: modifiers and a key joined by
        <code class="rounded bg-white/5 px-1">+</code>
        (e.g.
        <code class="rounded bg-white/5 px-1">Ctrl+Super+Space</code>,
        <code class="rounded bg-white/5 px-1">Alt+Shift+D</code>).
      </span>
      <span class="block text-xs text-slate-500">
        Supported modifiers: Ctrl/Control, Super/Meta/Win/Cmd, Alt, Shift. Keys: Space,
        Enter, Tab, Escape, a–z.
      </span>
    </label>
  </div>
</section>
