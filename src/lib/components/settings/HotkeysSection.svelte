<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { IconAlertTriangle, IconKeyboard } from "@tabler/icons-svelte";
  import type { AppConfig, HotkeyDesktopStatus } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let status = $state<HotkeyDesktopStatus | null>(null);
  let statusError = $state<string | null>(null);

  async function loadStatus() {
    statusError = null;
    try {
      status = await invoke<HotkeyDesktopStatus>("get_hotkey_desktop_status");
    } catch (error) {
      status = null;
      statusError = String(error);
    }
  }

  onMount(() => {
    void loadStatus();
  });
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Hotkeys</h2>
    <p class="mt-1 text-sm text-slate-400">
      Global push-to-talk shortcut. Hold to dictate, then release to stop and process.
      Wayland uses the desktop GlobalShortcuts portal; X11 uses a native key grab.
    </p>
  </header>

  {#if status?.warning}
    <div
      role="alert"
      class="rounded-2xl border border-amber-400/35 bg-amber-400/10 px-5 py-4 text-sm leading-relaxed text-amber-50 shadow-xl backdrop-blur-xl"
    >
      <div class="flex items-start gap-3">
        <span class="mt-0.5 shrink-0 text-amber-300">
          <IconAlertTriangle aria-hidden="true" size={20} stroke={1.8} />
        </span>
        <div class="min-w-0 space-y-2">
          <p class="font-semibold tracking-tight text-amber-100">
            {#if status.is_cosmic}
              Global push-to-talk is unavailable on COSMIC
            {:else}
              Global push-to-talk is unavailable on this desktop
            {/if}
          </p>
          <p class="text-amber-50/90">{status.warning}</p>
          <ul class="list-disc space-y-1 pl-4 text-amber-50/80">
            <li>
              Use tray → <strong class="text-amber-50">Start Listening</strong> /
              <strong class="text-amber-50">Stop Listening</strong> for dictation.
            </li>
            {#if status.portal_hint}
              <li>
                Install <code class="rounded bg-black/25 px-1.5 py-0.5 font-mono text-xs">{status.portal_hint}</code>,
                restart portals, then re-save the hotkey.
              </li>
            {:else if status.is_cosmic}
              <li>
                COSMIC’s portal does not expose
                <code class="rounded bg-black/25 px-1.5 py-0.5 font-mono text-xs">GlobalShortcuts</code>
                yet — there is no package to install for this.
              </li>
            {/if}
            {#if status.desktop}
              <li class="text-amber-100/60">
                Detected session: <code class="rounded bg-black/20 px-1 font-mono text-xs">{status.session}</code>
                · desktop <code class="rounded bg-black/20 px-1 font-mono text-xs">{status.desktop}</code>
              </li>
            {/if}
          </ul>
          <button
            type="button"
            class="mt-1 rounded-lg border border-amber-300/25 bg-amber-300/10 px-3 py-1.5 text-xs font-medium text-amber-50 transition hover:bg-amber-300/20"
            onclick={() => void loadStatus()}
          >
            Recheck portal
          </button>
        </div>
      </div>
    </div>
  {:else if status && status.global_shortcuts_available && status.session === "wayland"}
    <div
      class="flex items-start gap-3 rounded-2xl border border-emerald-400/20 bg-emerald-400/5 px-5 py-3 text-xs leading-relaxed text-emerald-100/90"
    >
      <span class="mt-0.5 shrink-0 text-emerald-300">
        <IconKeyboard aria-hidden="true" size={18} stroke={1.7} />
      </span>
      <p>
        GlobalShortcuts portal is available
        {#if status.desktop}
          on <code class="rounded bg-white/5 px-1 font-mono">{status.desktop}</code>
        {/if}.
        Saving the hotkey below registers it system-wide.
      </p>
    </div>
  {:else if statusError}
    <p class="text-xs text-slate-500">Could not probe desktop hotkey support ({statusError}).</p>
  {/if}

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
      {#if status?.warning}
        <span class="block text-xs text-amber-200/80">
          The chord is still saved for when a GlobalShortcuts backend is available, but it will not
          bind on this desktop — use tray Start/Stop for dictation.
        </span>
      {/if}
    </label>

    <div class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3 text-xs leading-relaxed text-amber-100/90">
      <p class="font-medium text-amber-200">If the overlay does not appear</p>
      <ul class="mt-1.5 list-disc space-y-1 pl-4 text-amber-100/80">
        <li>Click <strong>Save</strong> after changing the hotkey (bottom of Settings).</li>
        <li>
          On Wayland, registration uses the GlobalShortcuts portal. If save fails with a portal
          or compositor error, Oto restores the last working shortcut automatically.
        </li>
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

  <div
    class="space-y-4 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <div>
      <h3 class="text-sm font-semibold tracking-tight text-slate-200">Startup</h3>
      <p class="mt-1 text-xs text-slate-500">
        Launch Oto with your desktop session so the tray and hotkey are ready without opening
        Settings first.
      </p>
    </div>
    <label
      class="flex cursor-pointer items-center justify-between gap-4 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-3 transition hover:border-white/20"
    >
      <span>
        <span class="block text-sm font-medium text-slate-200">Launch at login</span>
        <span class="block text-xs text-slate-500">
          Installs an XDG autostart entry for this binary
          (<code class="rounded bg-white/5 px-1">~/.config/autostart/dev.oto.app.desktop</code>).
          Save to apply. Hyprland needs a session that runs XDG autostart (or add the same Exec
          to your Hyprland startup).
        </span>
      </span>
      <input
        type="checkbox"
        class="h-4 w-4 shrink-0 rounded border-white/20 bg-slate-900 text-sky-500 focus:ring-sky-400/30"
        bind:checked={config.autostart_enabled}
      />
    </label>
  </div>
</section>
