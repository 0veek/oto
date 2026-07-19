<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig } from "$lib/types";

  let { config = $bindable() }: { config: AppConfig } = $props();
  let token = $state("");
  let tokenPresent = $state(false);
  let busy = $state(false);
  let status = $state<string | null>(null);
  $effect(() => { invoke<boolean>("sync_token_present").then((value) => tokenPresent = value).catch(() => tokenPresent = false); });

  async function saveToken() {
    status = null;
    try {
      await invoke("set_sync_token", { token });
      tokenPresent = token.trim().length > 0;
      token = "";
      status = tokenPresent ? "Sync token saved to the keyring." : "Sync token cleared.";
    } catch (error) {
      status = `Failed to save token: ${String(error)}`;
    }
  }
  async function syncNow() {
    busy = true;
    status = null;
    try {
      await invoke("set_config", { cfg: config });
      status = await invoke<string>("sync_now");
    } catch (error) {
      status = `Sync failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }
</script>

<section class="space-y-6">
  <header><h2 class="text-xl font-semibold tracking-tight">Privacy &amp; sync</h2><p class="mt-1 text-sm text-slate-400">History stays local. Sync is disabled until you configure and explicitly run it.</p></header>
  <div class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6">
    <label class="flex items-center justify-between gap-4"><span><span class="block text-sm font-medium">Save local history</span><span class="block text-xs text-slate-500">Stored under Oto’s XDG data directory.</span></span><input type="checkbox" bind:checked={config.history_enabled} /></label>
    <label class="block space-y-1.5" class:opacity-50={!config.history_enabled}>
      <span class="text-sm text-slate-300">Maximum entries</span>
      <input
        type="number"
        min="1"
        max="1000"
        disabled={!config.history_enabled}
        class="w-full rounded-xl border border-white/10 bg-slate-900 px-3 py-2.5 text-sm"
        value={config.history_limit}
        oninput={(event) => {
          const next = Number(event.currentTarget.value);
          config.history_limit = Number.isFinite(next)
            ? Math.min(1000, Math.max(1, Math.round(next)))
            : 100;
        }}
      />
    </label>
  </div>
  <div class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6">
    <label class="flex items-center justify-between gap-4"><span><span class="block text-sm font-medium">Enable user-controlled sync</span><span class="block text-xs text-slate-500">Merges dictionary, snippets, and styles through a JSON GET/PUT endpoint.</span></span><input type="checkbox" bind:checked={config.sync.enabled} /></label>
    <label class="block space-y-1.5" class:opacity-50={!config.sync.enabled}><span class="text-sm text-slate-300">HTTPS document endpoint</span><input type="url" disabled={!config.sync.enabled} placeholder="https://example.com/private/oto.json" class="w-full rounded-xl border border-white/10 bg-slate-900 px-3 py-2.5 text-sm" bind:value={config.sync.endpoint} /></label>
    <div class="flex gap-2" class:opacity-50={!config.sync.enabled}><input type="password" disabled={!config.sync.enabled} class="min-w-0 flex-1 rounded-xl border border-white/10 bg-slate-900 px-3 py-2.5 text-sm" placeholder={tokenPresent ? "Replace saved bearer token…" : "Optional bearer token"} bind:value={token} /><button type="button" disabled={!config.sync.enabled} class="rounded-xl bg-white/10 px-4 py-2.5 text-sm hover:bg-white/15" onclick={saveToken}>Save token</button></div>
    <button type="button" disabled={!config.sync.enabled || busy} class="rounded-xl bg-sky-500 px-4 py-2.5 text-sm font-medium text-white hover:bg-sky-400 disabled:opacity-50" onclick={syncNow}>{busy ? "Syncing…" : "Sync now"}</button>
    {#if status}<p aria-live="polite" class="text-sm {status.startsWith('Sync failed') ? 'text-rose-300' : 'text-slate-300'}">{status}</p>{/if}
    <p class="text-xs leading-relaxed text-slate-500">Local values win when an item has the same ID; remote-only items are added. Oto never syncs API keys, history, audio, or provider credentials.</p>
  </div>
</section>
