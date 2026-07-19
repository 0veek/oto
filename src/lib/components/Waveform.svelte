<script lang="ts">
  import { onMount } from "svelte";
  import { audioLevel } from "../stores/pipeline";

  let canvas: HTMLCanvasElement;
  let levels: number[] = [];
  const bars = 24;

  onMount(() => {
    const unsub = audioLevel.subscribe((l) => {
      levels = [...levels, l].slice(-bars);
      draw();
    });
    const ro = new ResizeObserver(() => draw());
    ro.observe(canvas);
    return () => {
      unsub();
      ro.disconnect();
    };
  });

  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const gap = 2;
    const bw = (w - gap * (bars - 1)) / bars;
    for (let i = 0; i < bars; i++) {
      const v = levels[i] ?? 0.05;
      const bh = Math.max(3, v * h * 0.9);
      const x = i * (bw + gap);
      const y = (h - bh) / 2;
      ctx.fillStyle = `rgba(165, 180, 252, ${0.35 + v * 0.65})`;
      ctx.beginPath();
      ctx.roundRect(x, y, bw, bh, 2);
      ctx.fill();
    }
  }
</script>

<canvas bind:this={canvas} class="h-6 w-28"></canvas>
