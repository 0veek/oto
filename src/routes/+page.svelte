<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import FloatingPill from "$lib/components/FloatingPill.svelte";
  import { applyPipelineEvent } from "$lib/stores/pipeline";
  import type { PipelineEvent } from "$lib/types";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";

  // Window show/hide is owned by the Rust pipeline. The frontend must NOT
  // call hide() on mount — a cold-start overlay loads with state "idle" and
  // would immediately hide itself, racing the Listening event.

  let posTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    const unlistenPromise = listen<PipelineEvent>("pipeline://event", (e) => {
      applyPipelineEvent(e.payload);
    });

    // Persist overlay position after user drag (skip 0,0 noise from first map).
    let unlistenMoved: (() => void) | undefined;
    void (async () => {
      try {
        const win = getCurrentWindow();
        unlistenMoved = await win.onMoved(({ payload }) => {
          if (payload.x === 0 && payload.y === 0) return;
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
      unlistenPromise.then((u) => u());
      unlistenMoved?.();
      if (posTimer) clearTimeout(posTimer);
    };
  });
</script>

<div class="flex h-screen w-screen items-center justify-center bg-transparent">
  <FloatingPill />
</div>
