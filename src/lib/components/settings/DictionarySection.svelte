<script lang="ts">
  import type { AppConfig } from "$lib/types";

  let {
    config = $bindable(),
  }: {
    config: AppConfig;
  } = $props();

  let draft = $state("");
  let addError = $state<string | null>(null);

  function addTerm() {
    addError = null;
    const term = draft.trim();
    if (!term) {
      addError = "Enter a word or phrase.";
      return;
    }
    const exists = config.dictionary.some(
      (t) => t.toLowerCase() === term.toLowerCase(),
    );
    if (exists) {
      addError = "Already in the dictionary.";
      return;
    }
    config.dictionary = [...config.dictionary, term];
    draft = "";
  }

  function removeTerm(index: number) {
    config.dictionary = config.dictionary.filter((_, i) => i !== index);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      addTerm();
    }
  }
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Dictionary</h2>
    <p class="mt-1 text-sm text-slate-400">
      Words and phrases included in polish context so the model keeps preferred spellings
      (names, product terms, jargon).
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <div class="flex flex-col gap-2 sm:flex-row">
      <input
        type="text"
        class="min-w-0 flex-1 rounded-xl border border-white/10 bg-slate-900/80 px-3 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-sky-400/50 focus:ring-2 focus:ring-sky-400/20"
        placeholder="Add a term…"
        spellcheck="false"
        bind:value={draft}
        onkeydown={onKeydown}
      />
      <button
        type="button"
        class="shrink-0 rounded-xl bg-sky-500/90 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-sky-400"
        onclick={addTerm}
      >
        Add
      </button>
    </div>
    {#if addError}
      <p class="text-xs text-amber-300/90">{addError}</p>
    {/if}

    {#if config.dictionary.length === 0}
      <p
        class="rounded-xl border border-dashed border-white/15 bg-slate-900/30 px-4 py-8 text-center text-sm text-slate-500"
      >
        No terms yet. Add names or domain vocabulary you want polish to preserve.
      </p>
    {:else}
      <ul class="space-y-2">
        {#each config.dictionary as term, i (term + i)}
          <li
            class="flex items-center justify-between gap-3 rounded-xl border border-white/10 bg-slate-900/40 px-4 py-2.5"
          >
            <span class="truncate text-sm text-slate-200">{term}</span>
            <button
              type="button"
              class="shrink-0 rounded-lg px-2 py-1 text-xs text-slate-400 transition hover:bg-white/10 hover:text-rose-300"
              onclick={() => removeTerm(i)}
            >
              Remove
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <p class="text-xs leading-relaxed text-slate-500">
      Dictionary terms are sent to your LLM only when polish is enabled. They are not used by
      STT in this MVP.
    </p>
  </div>
</section>
