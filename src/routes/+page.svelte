<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import FloatingPill from "$lib/components/FloatingPill.svelte";
  import { applyPipelineEvent, pipelineState } from "$lib/stores/pipeline";
  import type { AppConfig, IdleBehavior, PipelineEvent } from "$lib/types";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";

  let idleBehavior = $state<IdleBehavior>("hide");
  let posTimer: ReturnType<typeof setTimeout> | null = null;

  async function refreshIdleBehavior() {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      idleBehavior = cfg.idle_behavior;
    } catch {
      idleBehavior = "hide";
    }
  }

  onMount(() => {
    void refreshIdleBehavior();

    const unlistenPromise = listen<PipelineEvent>("pipeline://event", (e) => {
      applyPipelineEvent(e.payload);
      // Config may have changed in settings; refresh when returning to idle.
      if (e.payload.type === "state" && e.payload.state === "idle") {
        void refreshIdleBehavior();
      }
    });

    const unsub = pipelineState.subscribe(async (s) => {
      try {
        const win = getCurrentWindow();
        if (s === "idle") {
          if (idleBehavior === "hide") {
            await win.hide();
          } else {
            await win.show();
          }
        } else {
          await win.show();
        }
      } catch {
        // Browser/dev without Tauri: ignore window API failures
      }
    });

    // Persist overlay position after drag (debounced Moved events).
    let unlistenMoved: (() => void) | undefined;
    void (async () => {
      try {
        const win = getCurrentWindow();
        unlistenMoved = await win.onMoved(({ payload }) => {
          if (posTimer) clearTimeout(posTimer);
          posTimer = setTimeout(() => {
            void invoke("set_overlay_position", {
              x: payload.x,
              y: payload.y,
            });
          }, 350);
        });
      } catch {
        // no Tauri
      }
    })();

    return () => {
      unsub();
      unlistenPromise.then((u) => u());
      unlistenMoved?.();
      if (posTimer) clearTimeout(posTimer);
    };
  });
</script>

<div class="flex h-screen w-screen items-center justify-center bg-transparent">
  <FloatingPill />
</div>
