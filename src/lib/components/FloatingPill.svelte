<script lang="ts">
  import {
    IconAlertTriangle,
    IconCheck,
    IconLoader2,
    IconX,
  } from "@tabler/icons-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { PipelineState } from "$lib/types";
  import {
    partialTranscript,
    pipelineDetail,
    pipelinePhase,
    pipelineState,
  } from "$lib/stores/pipeline";
  import Waveform from "./Waveform.svelte";

  type InteractionState = "hover" | "focus" | "active";

  type PreviewState = {
    state: PipelineState;
    detail?: string;
    phase?: string;
    partial?: string;
    level?: number;
  };

  let {
    preview = null,
    forceInteraction,
    actionDisabled = false,
    actionBusy = false,
    onPreviewAction,
  }: {
    preview?: PreviewState | null;
    forceInteraction?: InteractionState;
    actionDisabled?: boolean;
    actionBusy?: boolean;
    onPreviewAction?: () => void;
  } = $props();

  let cancelBusy = $state(false);

  const currentState = $derived(preview?.state ?? $pipelineState);
  const detail = $derived(preview?.detail ?? $pipelineDetail);
  const phase = $derived(preview?.phase ?? $pipelinePhase);
  const partial = $derived(preview?.partial ?? $partialTranscript);
  const busy = $derived(actionBusy || cancelBusy);
  const hasAction = $derived(currentState !== "idle");
  const actionLabel = $derived(
    currentState === "error"
      ? "Dismiss error"
      : currentState === "done"
        ? "Dismiss confirmation"
        : "Cancel dictation",
  );

  async function handleAction() {
    if (busy || actionDisabled) return;
    if (preview) {
      onPreviewAction?.();
      return;
    }

    cancelBusy = true;
    try {
      await invoke("cancel_dictation");
    } catch (error) {
      console.error("cancel_dictation failed", error);
    } finally {
      cancelBusy = false;
    }
  }

  function statusLabel(value: PipelineState) {
    switch (value) {
      case "listening":
        return "Listening";
      case "processing":
        return phase || "Processing";
      case "done":
        return "Inserted";
      case "error":
        return detail || "Couldn’t insert";
      default:
        return "Ready";
    }
  }
</script>

<div
  class:force-hover={forceInteraction === "hover"}
  class:force-focus={forceInteraction === "focus"}
  class:force-active={forceInteraction === "active"}
  class:is-disabled={actionDisabled}
  class:is-loading={busy}
  class="oto-pill state-{currentState}"
  role="status"
  aria-live="polite"
  aria-label={`Oto — ${statusLabel(currentState)}${partial ? `. ${partial}` : ""}`}
  title={detail || partial || statusLabel(currentState)}
>
  <div class="oto-pill__rail" data-tauri-drag-region>
    <div class="oto-pill__content">
      {#key currentState}
        <div class="oto-pill__state">
          {#if currentState === "listening"}
            <span class="oto-pill__listening">
              <span class="oto-pill__label">Listening</span>
              <span class="oto-pill__live" aria-hidden="true">
                <i></i><i></i><i></i>
              </span>
            </span>
            <Waveform level={preview?.level} />
          {:else if currentState === "processing"}
            <span class="oto-pill__label">{phase || "Processing"}</span>
            <span class="oto-pill__processing" aria-hidden="true">
              <i></i><i></i><i></i>
            </span>
          {:else if currentState === "done"}
            <span class="oto-pill__label">Inserted</span>
            <span class="oto-pill__result oto-pill__result--success" aria-hidden="true">
              <IconCheck size={22} stroke={2.4} />
            </span>
          {:else if currentState === "error"}
            <span class="oto-pill__label oto-pill__label--error">{detail || "Couldn’t insert"}</span>
            <span class="oto-pill__result oto-pill__result--error" aria-hidden="true">
              <IconAlertTriangle size={20} stroke={2.2} />
            </span>
          {:else}
            <span class="oto-pill__label">Ready</span>
            <span class="oto-pill__ready-dot" aria-hidden="true"></span>
          {/if}
        </div>
      {/key}
    </div>

    {#if hasAction}
      <button
        type="button"
        class="oto-pill__action"
        aria-label={actionLabel}
        title={actionLabel}
        disabled={actionDisabled || busy}
        onclick={handleAction}
      >
        {#if busy}
          <IconLoader2 class="oto-pill__spinner" size={21} stroke={2.1} aria-hidden="true" />
        {:else}
          <IconX size={22} stroke={2.15} aria-hidden="true" />
        {/if}
      </button>
    {/if}
  </div>

  <div class="oto-pill__brand" data-tauri-drag-region aria-hidden="true">
    <img src="/favicon.png" alt="" draggable="false" />
  </div>
</div>

<style>
  /* Hallmark · component: overlay pill · genre: playful · theme: Oto Midnight
   * states: default · hover · focus · active · disabled · loading · error · success
   * pre-emit critique: P5 H5 E5 S5 R5 V5 · contrast: pass (40–41)
   * slop: pass (1–58) · tokens: pass (48) · responsive/mobile: pass (34, 49–57)
   */
  .oto-pill {
    position: relative;
    width: min(21.25rem, 100vw);
    height: min(5rem, 100vh);
    color: var(--color-overlay-ink);
    font-family: var(--font-body);
    user-select: none;
    isolation: isolate;
  }

  .oto-pill__rail {
    position: absolute;
    inset: 0.5rem 0 0.5rem 3.5rem;
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0.5rem 0.5rem 0.5rem 1.75rem;
    border: var(--rule-thin) solid var(--color-overlay-rule);
    border-radius: var(--radius-round);
    background: var(--color-overlay-surface);
    box-shadow: var(--shadow-overlay);
  }

  .oto-pill__brand {
    position: absolute;
    inset-block-start: 0.25rem;
    inset-inline-start: 0;
    display: grid;
    width: 4.5rem;
    height: 4.5rem;
    place-items: center;
    border: var(--rule-thin) solid var(--color-overlay-rule-strong);
    border-radius: 50%;
    background: var(--color-overlay-brand);
    box-shadow: var(--shadow-overlay-brand);
  }

  .oto-pill__brand img {
    position: relative;
    z-index: 1;
    width: 3rem;
    height: 3rem;
    image-rendering: auto;
    pointer-events: none;
  }

  .oto-pill__content {
    min-width: 0;
    flex: 1 1 auto;
  }

  .oto-pill__state {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.5rem;
    animation: oto-state-enter var(--dur-short) var(--ease-out) both;
  }

  .oto-pill__label {
    min-width: 4.25rem;
    max-width: 7.25rem;
    overflow: hidden;
    flex: 0 1 auto;
    color: var(--color-overlay-ink);
    font-size: 0.9375rem;
    font-weight: 600;
    letter-spacing: -0.012em;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .oto-pill__label--error {
    min-width: 6.75rem;
    color: var(--color-overlay-error);
  }

  .oto-pill__listening {
    display: grid;
    width: 4.25rem;
    flex: 0 0 4.25rem;
    gap: 0.25rem;
  }

  .oto-pill__listening .oto-pill__label {
    min-width: 0;
  }

  .oto-pill__live {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .oto-pill__live i {
    display: block;
    width: 0.25rem;
    height: 0.25rem;
    border-radius: 50%;
    background: var(--color-overlay-warm);
    animation: oto-live 1.05s var(--ease-in-out) infinite;
  }

  .oto-pill__live i:nth-child(2) {
    animation-delay: 120ms;
  }

  .oto-pill__live i:nth-child(3) {
    animation-delay: 240ms;
  }

  .oto-pill__processing {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-inline-start: auto;
    margin-inline-end: 0.75rem;
  }

  .oto-pill__processing i {
    display: block;
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 0.25rem;
    background: var(--color-overlay-accent);
    animation: oto-processing 0.9s var(--ease-in-out) infinite alternate;
  }

  .oto-pill__processing i:nth-child(2) {
    animation-delay: 130ms;
  }

  .oto-pill__processing i:nth-child(3) {
    animation-delay: 260ms;
  }

  .oto-pill__result {
    display: grid;
    width: 2rem;
    height: 2rem;
    margin-inline-start: auto;
    margin-inline-end: 0.5rem;
    place-items: center;
    border: var(--rule-thin) solid currentColor;
    border-radius: 50%;
  }

  .oto-pill__result--success {
    color: var(--color-overlay-accent);
  }

  .oto-pill__result--error {
    color: var(--color-overlay-error);
  }

  .oto-pill__ready-dot {
    width: 0.5rem;
    height: 0.5rem;
    margin-inline-start: auto;
    margin-inline-end: 0.75rem;
    border-radius: 50%;
    background: var(--color-overlay-muted);
  }

  .oto-pill__action {
    display: grid;
    width: 2.75rem;
    height: 2.75rem;
    flex: 0 0 auto;
    place-items: center;
    border: var(--rule-thin) solid var(--color-overlay-rule-strong);
    border-radius: 50%;
    outline: 2px solid transparent;
    outline-offset: 2px;
    color: var(--color-overlay-ink-2);
    background: var(--color-overlay-action);
    box-shadow: none;
    transition:
      color var(--dur-micro) var(--ease-out),
      background-color var(--dur-micro) var(--ease-out),
      border-color var(--dur-micro) var(--ease-out),
      transform var(--dur-micro) var(--ease-out),
      opacity var(--dur-micro) var(--ease-out);
  }

  .oto-pill__action:hover,
  .force-hover .oto-pill__action {
    border-color: var(--color-overlay-rule-hover);
    color: var(--color-overlay-ink);
    background: var(--color-overlay-action-hover);
  }

  .oto-pill__action:focus-visible,
  .force-focus .oto-pill__action {
    border-color: var(--color-overlay-accent);
    outline-color: var(--color-overlay-focus);
  }

  .oto-pill__action:active,
  .force-active .oto-pill__action {
    transform: scale(0.92);
    color: var(--color-overlay-ink);
    background: var(--color-overlay-action-active);
  }

  .oto-pill__action:disabled,
  .is-disabled .oto-pill__action {
    cursor: not-allowed;
    opacity: 0.42;
  }

  .oto-pill__spinner {
    animation: oto-spin 0.85s linear infinite;
  }

  .state-error .oto-pill__rail {
    border-color: var(--color-overlay-error-rule);
  }

  .state-done .oto-pill__rail {
    border-color: var(--color-overlay-accent-rule);
  }

  @keyframes oto-state-enter {
    from {
      opacity: 0;
      transform: translateX(0.375rem);
    }
  }

  @keyframes oto-live {
    0%,
    100% {
      opacity: 0.35;
      transform: translateY(0);
    }
    50% {
      opacity: 1;
      transform: translateY(-0.1875rem);
    }
  }

  @keyframes oto-processing {
    from {
      opacity: 0.38;
      transform: scale(0.78);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes oto-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .oto-pill *,
    .oto-pill *::before,
    .oto-pill *::after {
      scroll-behavior: auto !important;
      animation-duration: 1ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 1ms !important;
    }
  }
</style>
