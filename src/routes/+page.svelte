<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import FloatingPill from "$lib/components/FloatingPill.svelte";
  import { applyPipelineEvent, pipelineState } from "$lib/stores/pipeline";
  import type { PipelineEvent } from "$lib/types";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  onMount(() => {
    const unlistenPromise = listen<PipelineEvent>("pipeline://event", (e) => {
      applyPipelineEvent(e.payload);
    });

    const unsub = pipelineState.subscribe(async (s) => {
      try {
        const win = getCurrentWindow();
        if (s === "idle") {
          // hide when idle — appearance setting refined later
          await win.hide();
        } else {
          await win.show();
        }
      } catch {
        // Browser/dev without Tauri: ignore window API failures
      }
    });

    return () => {
      unsub();
      unlistenPromise.then((u) => u());
    };
  });
</script>

<div class="flex h-screen w-screen items-center justify-center bg-transparent">
  <FloatingPill />
</div>
