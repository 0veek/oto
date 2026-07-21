<script lang="ts">
  import { onMount } from "svelte";
  import FloatingPill from "$lib/components/FloatingPill.svelte";

  const initialParams = typeof window === "undefined" ? null : new URL(window.location.href).searchParams;
  let actions = $state(0);
  let referenceMode = $state(["reference", "single"].includes(initialParams?.get("mode") ?? ""));
  let singleState = $state(initialParams?.get("mode") === "single" ? initialParams.get("state") : null);

  onMount(() => {
    const params = new URL(window.location.href).searchParams;
    referenceMode = ["reference", "single"].includes(params.get("mode") ?? "");
    singleState = params.get("mode") === "single" ? params.get("state") : null;
  });

  const examples = [
    {
      name: "Default",
      note: "Live input",
      preview: { state: "listening" as const, level: 0.68 },
    },
    {
      name: "Hover",
      note: "Action affordance",
      preview: { state: "listening" as const, level: 0.54 },
      forceInteraction: "hover" as const,
    },
    {
      name: "Focus",
      note: "Keyboard focus",
      preview: { state: "listening" as const, level: 0.74 },
      forceInteraction: "focus" as const,
    },
    {
      name: "Active",
      note: "Pressed action",
      preview: { state: "listening" as const, level: 0.46 },
      forceInteraction: "active" as const,
    },
    {
      name: "Disabled",
      note: "Action unavailable",
      preview: { state: "listening" as const, level: 0.28 },
      actionDisabled: true,
    },
    {
      name: "Loading",
      note: "Processing audio",
      preview: { state: "processing" as const },
      actionBusy: true,
    },
    {
      name: "Error",
      note: "Insertion failed",
      preview: { state: "error" as const, detail: "Couldn’t insert" },
    },
    {
      name: "Success",
      note: "Text inserted",
      preview: { state: "done" as const, detail: "Inserted into the focused app" },
    },
  ];

  const referenceExamples = [
    { state: "listening" as const, level: 0.68 },
    { state: "processing" as const },
    { state: "done" as const, detail: "Inserted into the focused app" },
    { state: "error" as const, detail: "Couldn’t insert" },
  ];
</script>

<svelte:head>
  <title>Oto overlay states</title>
</svelte:head>

<main class:reference-mode={referenceMode} class:single-mode={singleState !== null} class="overlay-preview">
  {#if referenceMode}
    <section class="overlay-preview__reference" aria-label="Primary overlay states">
      {#each referenceExamples.filter((preview) => !singleState || preview.state === singleState) as preview (preview.state)}
        <FloatingPill {preview} onPreviewAction={() => (actions += 1)} />
      {/each}
    </section>
  {:else}
    <header>
      <div>
        <p class="overlay-preview__eyebrow">Oto · overlay system</p>
        <h1>One little pod, every state.</h1>
      </div>
      <p class="overlay-preview__meta">252 × 44 · {actions} test {actions === 1 ? "action" : "actions"}</p>
    </header>

    <section aria-label="Overlay component states">
      {#each examples as example (example.name)}
        <article>
          <div class="overlay-preview__caption">
            <span>{example.name}</span>
            <small>{example.note}</small>
          </div>
          <FloatingPill
            preview={example.preview}
            forceInteraction={example.forceInteraction}
            actionDisabled={example.actionDisabled}
            actionBusy={example.actionBusy}
            onPreviewAction={() => (actions += 1)}
          />
        </article>
      {/each}
    </section>
  {/if}
</main>

<style>
  .overlay-preview {
    min-height: 100dvh;
    padding: clamp(2rem, 6vw, 5rem);
    color: var(--color-ink);
    background: var(--color-paper);
  }

  .overlay-preview > header {
    display: flex;
    width: min(100%, 58rem);
    align-items: end;
    justify-content: space-between;
    gap: var(--space-lg);
    margin: 0 auto var(--space-xl);
    padding-block-end: var(--space-md);
    border-block-end: var(--rule-thin) solid var(--color-rule);
  }

  .overlay-preview h1,
  .overlay-preview p {
    margin: 0;
  }

  .overlay-preview h1 {
    min-width: 0;
    font-size: clamp(1.75rem, 4vw, 2.75rem);
    font-weight: 720;
    letter-spacing: -0.04em;
    line-height: 1;
    overflow-wrap: anywhere;
  }

  .overlay-preview__eyebrow,
  .overlay-preview__meta {
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .overlay-preview__eyebrow {
    margin-block-end: var(--space-xs) !important;
    color: var(--color-accent) !important;
  }

  .overlay-preview > section {
    display: grid;
    width: min(100%, 58rem);
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-xs) var(--space-lg);
    margin-inline: auto;
  }

  .overlay-preview article {
    display: grid;
    min-width: 0;
    grid-template-columns: 5.5rem minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
    padding-block: var(--space-sm);
  }

  .overlay-preview__caption {
    display: grid;
    gap: var(--space-2xs);
  }

  .overlay-preview__caption span {
    font-size: var(--text-sm);
    font-weight: 650;
  }

  .overlay-preview__caption small {
    color: var(--color-subtle);
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    line-height: 1.35;
  }

  .overlay-preview.reference-mode {
    display: grid;
    width: 100vw;
    height: 100vh;
    min-height: 0;
    place-items: center;
    overflow: hidden;
    padding: 0;
  }

  .overlay-preview > .overlay-preview__reference {
    display: grid;
    width: 15.75rem;
    grid-template-columns: 1fr;
    gap: 0.5rem;
    transform: scale(2.35);
  }

  .overlay-preview.single-mode > .overlay-preview__reference {
    transform: none;
  }

  @media (max-width: 58rem) {
    .overlay-preview > section {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 34rem) {
    .overlay-preview {
      padding-inline: var(--space-sm);
    }

    .overlay-preview > header {
      align-items: start;
      flex-direction: column;
    }

    .overlay-preview article {
      grid-template-columns: 1fr;
    }

    .overlay-preview article :global(.oto-pill) {
      transform: scale(0.9);
      transform-origin: left center;
    }
  }
</style>
