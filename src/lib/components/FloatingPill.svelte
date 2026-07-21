<script lang="ts">
  import {
    IconAlertTriangle,
    IconCheck,
    IconLoader2,
    IconPointFilled,
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
    <span class="oto-pill__status-mark" aria-hidden="true">
      {#if currentState === "listening"}
        <Waveform compact level={preview?.level} />
      {:else if currentState === "processing"}
        <Waveform compact level={preview?.level ?? 0.48} />
      {:else if currentState === "done"}
        <IconCheck size={17} stroke={2.2} />
      {:else if currentState === "error"}
        <IconAlertTriangle size={16} stroke={2} />
      {:else}
        <IconPointFilled size={14} />
      {/if}
    </span>

    {#key currentState}
      <span class:oto-pill__label--error={currentState === "error"} class="oto-pill__label">
        {statusLabel(currentState)}
      </span>
    {/key}

    {#if currentState === "listening" || currentState === "processing"}
      <span class="oto-pill__activity" aria-hidden="true">
        <IconPointFilled size={8} /><IconPointFilled size={8} /><IconPointFilled size={8} />
      </span>
    {/if}

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
          <IconLoader2 class="oto-pill__spinner" size={15} stroke={2} aria-hidden="true" />
        {:else}
          <IconX size={16} stroke={1.8} aria-hidden="true" />
        {/if}
      </button>
    {/if}
  </div>
</div>

<style>
  .oto-pill {
    width: min(15.75rem, calc(100vw - 0.5rem));
    height: min(2.75rem, calc(100vh - 0.5rem));
    color: var(--color-overlay-ink);
    font-family: var(--font-body);
    user-select: none;
  }

  .oto-pill__rail {
    display: flex;
    width: 100%;
    height: 100%;
    align-items: center;
    gap: 0.875rem;
    padding: 0.3125rem 0.375rem 0.3125rem 0.875rem;
    border: var(--rule-thin) solid var(--color-overlay-rule-strong);
    border-radius: var(--radius-round);
    background: color-mix(in oklch, var(--color-overlay-surface) 92%, transparent);
    box-shadow:
      0 0.5rem 1.5rem oklch(4% 0.015 235 / 0.38),
      inset 0 1px 0 oklch(100% 0 0 / 0.055);
    backdrop-filter: blur(18px) saturate(120%);
    box-sizing: border-box;
  }

  .oto-pill__status-mark {
    display: grid;
    width: 1.375rem;
    height: 1.375rem;
    flex: 0 0 1.375rem;
    place-items: center;
    color: var(--color-overlay-accent);
  }

  .state-error .oto-pill__status-mark,
  .oto-pill__label--error {
    color: var(--color-overlay-error);
  }

  .oto-pill__label {
    min-width: 0;
    overflow: hidden;
    flex: 1 1 auto;
    color: var(--color-overlay-ink);
    font-size: 0.8125rem;
    font-weight: 560;
    letter-spacing: -0.006em;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
    animation: oto-state-enter var(--dur-short) var(--ease-out) both;
  }

  .oto-pill__activity {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex: 0 0 auto;
    margin-inline: -0.75rem 0.75rem;
    color: var(--color-overlay-muted);
    animation: oto-activity 1.15s var(--ease-in-out) infinite;
  }

  .oto-pill__action {
    display: grid;
    width: 2rem;
    height: 2rem;
    min-height: 0;
    flex: 0 0 2rem;
    padding: 0;
    place-items: center;
    border: var(--rule-thin) solid var(--color-overlay-rule);
    border-radius: 50%;
    outline: 2px solid transparent;
    outline-offset: 1px;
    color: var(--color-overlay-ink-2);
    background: var(--color-overlay-action);
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
      transform: translateX(0.25rem);
    }
  }

  @keyframes oto-activity {
    0%,
    100% {
      opacity: 0.45;
    }
    50% {
      opacity: 1;
    }
  }

  @keyframes oto-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .oto-pill__label,
    .oto-pill__activity,
    .oto-pill__spinner {
      animation: none;
    }
  }

  :global(:root[data-reduce-motion="true"]) .oto-pill__label,
  :global(:root[data-reduce-motion="true"]) .oto-pill__activity,
  :global(:root[data-reduce-motion="true"]) .oto-pill__spinner {
    animation: none;
  }
</style>
