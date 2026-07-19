<script lang="ts">
  import {
    IconCursorText,
    IconMicrophone,
    IconSparkles,
    IconWaveSine,
  } from "@tabler/icons-svelte";

  const stages = [
    { id: "listen", label: "1. Listen", hint: "Capture microphone audio", icon: IconMicrophone },
    { id: "transcribe", label: "2. Transcribe", hint: "Convert speech to text", icon: null },
    { id: "polish", label: "3. Polish", hint: "Refine grammar and tone", icon: IconSparkles },
    { id: "insert", label: "4. Insert", hint: "Place text at the cursor", icon: IconCursorText },
  ];
</script>

<div class="pipeline-map" aria-label="Oto dictation pipeline">
  {#each stages as stage, index (stage.id)}
    <div class="pipeline-stage" class:pipeline-stage--active={stage.id === "transcribe"}>
      <div class="pipeline-stage__icon">
        {#if stage.icon}
          {@const StageIcon = stage.icon}
          <StageIcon aria-hidden="true" size={26} stroke={1.45} />
        {:else}
          <img src="/favicon.png" alt="" width="46" height="46" />
        {/if}
      </div>
      <strong>{stage.label}</strong>
      <span>{stage.hint}</span>
      {#if index < stages.length - 1}
        <IconWaveSine class="pipeline-stage__arrow" aria-hidden="true" size={20} stroke={1.25} />
      {/if}
    </div>
  {/each}
</div>
