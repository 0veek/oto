<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    sections,
    active,
    theme,
    onselect,
    children,
  }: {
    sections: { id: string; label: string }[];
    active: string;
    theme: string;
    onselect: (id: string) => void;
    children: Snippet;
  } = $props();
</script>

<div class="oto-settings flex h-screen bg-slate-950 text-slate-100" data-theme={theme}>
  <aside
    class="flex w-56 shrink-0 flex-col border-r border-white/10 bg-white/[0.03] p-4 backdrop-blur-xl"
  >
    <div class="mb-8 px-2">
      <div class="text-lg font-semibold tracking-tight">Oto</div>
      <div class="mt-0.5 text-xs text-slate-500">Settings</div>
    </div>
    <nav class="flex flex-col gap-1">
      {#each sections as s (s.id)}
        <button
          type="button"
          class="rounded-lg px-3 py-2 text-left text-sm transition
            {active === s.id
              ? 'bg-white/10 text-white shadow-sm ring-1 ring-white/10'
              : 'text-slate-400 hover:bg-white/5 hover:text-white'}"
          onclick={() => onselect(s.id)}
        >
          {s.label}
        </button>
      {/each}
    </nav>
  </aside>
  <main class="flex-1 overflow-y-auto p-8">
    {@render children()}
  </main>
</div>
