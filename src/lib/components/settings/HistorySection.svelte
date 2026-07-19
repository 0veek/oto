<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { AppConfig, HistoryEntry } from "$lib/types";

  let entries = $state<HistoryEntry[]>([]);
  let historyEnabled = $state(true);
  let status = $state<string | null>(null);
  let busyId = $state<string | null>(null);

  async function refresh() {
    try {
      const [list, config] = await Promise.all([
        invoke<HistoryEntry[]>("get_history"),
        invoke<AppConfig>("get_config").catch(() => null),
      ]);
      entries = list;
      if (config) historyEnabled = config.history_enabled;
    } catch (error) {
      status = String(error);
    }
  }

  async function remove(id: string) {
    busyId = id;
    status = null;
    try {
      await invoke("delete_history_entry", { id });
      await refresh();
    } catch (error) {
      status = `Delete failed: ${String(error)}`;
    } finally {
      busyId = null;
    }
  }

  async function clearAll() {
    if (!confirm("Clear all saved dictations? This cannot be undone.")) return;
    status = null;
    try {
      await invoke("clear_history");
      entries = [];
      status = "History cleared.";
    } catch (error) {
      status = `Clear failed: ${String(error)}`;
    }
  }

  async function copy(text: string) {
    status = null;
    try {
      await invoke("copy_history_text", { text });
      status = "Copied to clipboard.";
    } catch (error) {
      status = `Copy failed: ${String(error)}`;
    }
  }

  onMount(() => {
    void refresh();
  });
</script>

<section class="space-y-6">
  <header class="flex items-end justify-between gap-4">
    <div>
      <h2 class="text-xl font-semibold tracking-tight">History</h2>
      <p class="mt-1 text-sm text-slate-400">A local scratchpad of recent dictations and command edits.</p>
    </div>
    {#if entries.length}
      <button type="button" class="text-xs text-rose-300 hover:text-rose-200" onclick={clearAll}>Clear all</button>
    {/if}
  </header>

  {#if !historyEnabled}
    <p class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3 text-sm text-amber-100/90">
      Saving new history is off. Existing entries below remain until deleted. Re-enable under
      <strong>Privacy &amp; sync</strong>.
    </p>
  {/if}

  {#if status}
    <p
      aria-live="polite"
      class="text-sm {status.includes('failed') || status.includes('Failed') ? 'text-rose-300' : 'text-slate-400'}"
    >
      {status}
    </p>
  {/if}

  {#if entries.length === 0}
    <div class="rounded-2xl border border-dashed border-white/15 px-6 py-14 text-center text-sm text-slate-500">
      No saved dictations yet.
    </div>
  {:else}
    <div class="space-y-3">
      {#each entries as entry (entry.id)}
        <article class="rounded-2xl border border-white/10 bg-white/[0.04] p-5">
          <div class="mb-3 flex items-center justify-between gap-3 text-xs text-slate-500">
            <span class="rounded-full bg-white/5 px-2 py-1 capitalize">{entry.mode}</span>
            <time datetime={new Date(entry.created_at_ms).toISOString()}>
              {new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(entry.created_at_ms)}
            </time>
          </div>
          <p class="whitespace-pre-wrap text-sm leading-relaxed text-slate-200">{entry.final_text}</p>
          {#if entry.raw_text !== entry.final_text}
            <details class="mt-3 text-xs text-slate-500">
              <summary class="cursor-pointer">Raw transcript</summary>
              <p class="mt-2 whitespace-pre-wrap">{entry.raw_text}</p>
            </details>
          {/if}
          <div class="mt-4 flex gap-2">
            <button type="button" class="rounded-lg bg-white/10 px-3 py-1.5 text-xs hover:bg-white/15" onclick={() => copy(entry.final_text)}>
              Copy
            </button>
            <button
              type="button"
              class="rounded-lg px-3 py-1.5 text-xs text-rose-300 hover:bg-white/10 disabled:opacity-50"
              disabled={busyId === entry.id}
              onclick={() => remove(entry.id)}
            >
              Delete
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

