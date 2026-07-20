<script lang="ts">
  import { onMount } from "svelte";
  import { audioLevel } from "$lib/stores/pipeline";

  let { level }: { level?: number } = $props();
  let canvas: HTMLCanvasElement;
  let levels: number[] = [0.22, 0.58, 0.88, 0.46, 0.78, 0.62, 0.3];
  const bars = 7;

  onMount(() => {
    let previewFrame = 0;
    let animationFrame = 0;
    const unsubscribe = audioLevel.subscribe((value) => {
      if (level === undefined) addLevel(value);
    });
    const resizeObserver = new ResizeObserver(draw);
    resizeObserver.observe(canvas);

    if (level !== undefined) {
      const animatePreview = () => {
        previewFrame += 1;
        if (previewFrame % 8 === 0) {
          const movement = Math.sin(previewFrame / 12) * 0.14;
          addLevel(Math.max(0.12, Math.min(1, level + movement)));
        }
        animationFrame = requestAnimationFrame(animatePreview);
      };
      animationFrame = requestAnimationFrame(animatePreview);
    }

    draw();
    return () => {
      unsubscribe();
      resizeObserver.disconnect();
      if (animationFrame) cancelAnimationFrame(animationFrame);
    };
  });

  function addLevel(value: number) {
    levels = [...levels, Math.max(0.06, Math.min(1, value))].slice(-bars);
    draw();
  }

  function draw() {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;

    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const styles = getComputedStyle(canvas);
    const active = styles.getPropertyValue("--color-overlay-accent").trim();
    const quiet = styles.getPropertyValue("--color-overlay-wave-quiet").trim();

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, height);

    const gap = 4;
    const barWidth = (width - gap * (bars - 1)) / bars;
    for (let index = 0; index < bars; index += 1) {
      const value = levels[index] ?? 0.08;
      const barHeight = Math.max(4, value * height * 0.92);
      const x = index * (barWidth + gap);
      const y = (height - barHeight) / 2;
      context.fillStyle = value > 0.18 ? active : quiet;
      context.globalAlpha = 0.55 + value * 0.45;
      context.beginPath();
      context.roundRect(x, y, barWidth, barHeight, barWidth / 2);
      context.fill();
    }
    context.globalAlpha = 1;
  }
</script>

<canvas bind:this={canvas} class="oto-waveform" aria-hidden="true"></canvas>

<style>
  .oto-waveform {
    display: block;
    width: 4.75rem;
    height: 2rem;
    flex: 0 0 4.75rem;
  }
</style>
