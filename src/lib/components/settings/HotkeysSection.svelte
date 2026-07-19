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
      Global push-to-talk shortcut. Hold to dictate, then release to stop and process.
      Wayland uses the desktop GlobalShortcuts portal; X11 uses a native key grab.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <label class="block space-y-1.5">
      <span class="text-sm font-medium text-slate-300">Dictation hotkey</span>
      <input
        type="text"
        class="w-full rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 font-mono text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="Ctrl+Shift+Space"
        spellcheck="false"
        autocomplete="off"
        bind:value={config.hotkey}
      />
      <span class="block text-xs text-slate-500">
        Format: modifiers and a key joined by
        <code class="rounded bg-white/5 px-1">+</code>
        (e.g.
        <code class="rounded bg-white/5 px-1">Ctrl+Shift+Space</code>,
        <code class="rounded bg-white/5 px-1">Ctrl+Alt+D</code>).
      </span>
      <span class="block text-xs text-slate-500">
        Supported modifiers: Ctrl, Super/Meta/Win, Alt, Shift. Keys: Space, Enter, Tab, Escape,
        a–z.
      </span>
    </label>

    <div class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3 text-xs leading-relaxed text-amber-100/90">
      <p class="font-medium text-amber-200">If the overlay does not appear</p>
      <ul class="mt-1.5 list-disc space-y-1 pl-4 text-amber-100/80">
        <li>Click <strong>Save</strong> after changing the hotkey (bottom of Settings).</li>
        <li>
          On Hyprland, Oto creates a runtime <code class="rounded bg-white/5 px-1">global</code>
          bind for the configured chord. Make sure
          <code class="rounded bg-white/5 px-1">xdg-desktop-portal-hyprland</code> is running.
        </li>
        <li>
          Avoid reserved combos:
          <code class="rounded bg-white/5 px-1">Alt+Space</code>,
          <code class="rounded bg-white/5 px-1">Ctrl+Alt+Space</code> (IME),
          <code class="rounded bg-white/5 px-1">Super+…</code> (desktop shell).
        </li>
        <li>
          Prefer
          <code class="rounded bg-white/5 px-1">Ctrl+Shift+Space</code>
          or
          <code class="rounded bg-white/5 px-1">Ctrl+Shift+D</code>.
        </li>
        <li>
          Use tray → <strong>Start Listening</strong> as a fallback (always works without global
          grabs).
        </li>
      </ul>
    </div>
  </div>
</section>
