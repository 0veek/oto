<script lang="ts">
  import type { AppConfig, Snippet } from "$lib/types";

  let { config = $bindable() }: { config: AppConfig } = $props();
  let trigger = $state("");
  let expansion = $state("");
  let error = $state<string | null>(null);

  function id() {
    return globalThis.crypto?.randomUUID?.() ?? `snippet-${Date.now()}`;
  }

  function addSnippet() {
    error = null;
    if (!trigger.trim() || !expansion.trim()) {
      error = "Add both a spoken trigger and its expansion.";
      return;
    }
    if (config.snippets.some((item) => item.trigger.toLowerCase() === trigger.trim().toLowerCase())) {
      error = "That trigger already exists.";
      return;
    }
    config.snippets = [
      ...config.snippets,
      { id: id(), trigger: trigger.trim(), expansion: expansion.trim(), enabled: true },
    ];
    trigger = "";
    expansion = "";
  }

  function patchSnippet(id: string, patch: Partial<Snippet>) {
    config.snippets = config.snippets.map((item) => item.id === id ? { ...item, ...patch } : item);
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Snippets</h2>
    <p class="mt-1 text-sm text-slate-400">
      Speak an exact trigger to insert a longer block verbatim. “Snippet” before the trigger is optional.
    </p>
  </header>

  <div class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl">
    <div class="grid gap-3 sm:grid-cols-[0.8fr_1.4fr_auto]">
      <input
        aria-label="Spoken trigger"
        class="rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none focus:border-sky-400/50"
        placeholder="my signature"
        bind:value={trigger}
        onkeydown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            addSnippet();
          }
        }}
      />
      <textarea
        aria-label="Snippet expansion"
        class="min-h-11 resize-y rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none focus:border-sky-400/50"
        rows="1"
        placeholder="Best,&#10;Your name"
        bind:value={expansion}
      ></textarea>
      <button type="button" class="rounded-xl bg-sky-500/90 px-4 py-2.5 text-sm font-medium text-white hover:bg-sky-400" onclick={addSnippet}>Add</button>
    </div>
    {#if error}<p class="text-xs text-amber-300">{error}</p>{/if}

    {#if config.snippets.length === 0}
      <p class="rounded-xl border border-dashed border-white/15 px-4 py-8 text-center text-sm text-slate-500">No voice macros yet.</p>
    {:else}
      <div class="space-y-3">
        {#each config.snippets as snippet (snippet.id)}
          <article class="grid gap-3 rounded-xl border border-white/10 bg-slate-900/40 p-4 sm:grid-cols-[1fr_1.5fr_auto]">
            <input aria-label="Spoken trigger" class="rounded-lg border border-white/10 bg-slate-950/70 px-3 py-2 text-sm" value={snippet.trigger} oninput={(event) => patchSnippet(snippet.id, { trigger: event.currentTarget.value })} />
            <textarea aria-label="Snippet expansion" class="resize-y rounded-lg border border-white/10 bg-slate-950/70 px-3 py-2 text-sm" rows="2" value={snippet.expansion} oninput={(event) => patchSnippet(snippet.id, { expansion: event.currentTarget.value })}></textarea>
            <div class="flex items-start gap-2">
              <label class="flex items-center gap-1.5 text-xs text-slate-400"><input type="checkbox" checked={snippet.enabled} onchange={(event) => patchSnippet(snippet.id, { enabled: event.currentTarget.checked })} /> On</label>
              <button type="button" class="rounded-lg px-2 py-1 text-xs text-rose-300 hover:bg-white/10" onclick={() => config.snippets = config.snippets.filter((item) => item.id !== snippet.id)}>Remove</button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
    <p class="text-xs text-slate-500">Macros only match a complete utterance, preventing a phrase inside normal dictation from expanding unexpectedly.</p>
  </div>
</section>

