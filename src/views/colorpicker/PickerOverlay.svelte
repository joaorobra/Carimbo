<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { settingsStore } from "../../lib/stores/settings.svelte";
  import { i18n, t } from "../../lib/i18n";
  import { rgbToHex, contrastText, type Rgb } from "../../lib/color";

  // Payload streamed by the backend while picking (see commands/color.rs).
  interface MovePayload {
    x: number;
    y: number;
    size: number;
    rgb: number[];
  }

  // Cell size for the magnifier: 11px cells x 11 cells = 121px canvas.
  const CELL = 11;

  let canvas: HTMLCanvasElement | undefined = $state();
  let current = $state<Rgb>({ r: 0, g: 0, b: 0 });
  let ready = $state(false);
  const hex = $derived(rgbToHex(current));

  // Match the manager's theme + language.
  $effect(() => {
    settingsStore.init();
    i18n.init();
  });

  $effect(() => {
    const un = listen<MovePayload>("color:move", (e) => {
      ready = true;
      draw(e.payload);
    });
    return () => {
      un.then((f) => f());
    };
  });

  function draw(p: MovePayload) {
    const n = p.size;
    const center = Math.floor(n / 2);
    const ci = (center * n + center) * 3;
    current = { r: p.rgb[ci], g: p.rgb[ci + 1], b: p.rgb[ci + 2] };

    const ctx = canvas?.getContext("2d");
    if (!ctx || !canvas) return;
    const px = canvas.width / n;
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) {
        const i = (y * n + x) * 3;
        ctx.fillStyle = `rgb(${p.rgb[i]}, ${p.rgb[i + 1]}, ${p.rgb[i + 2]})`;
        ctx.fillRect(x * px, y * px, px, px);
      }
    }
    // Double outline on the cursor cell so it reads on any background.
    ctx.lineWidth = 1;
    ctx.strokeStyle = "#000000";
    ctx.strokeRect(center * px + 0.5, center * px + 0.5, px - 1, px - 1);
    ctx.strokeStyle = "#ffffff";
    ctx.strokeRect(center * px + 1.5, center * px + 1.5, px - 3, px - 3);
  }
</script>

<div class="stage" class:ready>
  <div class="card">
    <canvas bind:this={canvas} width={CELL * 11} height={CELL * 11}></canvas>
    <div class="readout" style:background={hex} style:color={contrastText(current)}>
      {hex}
    </div>
    <div class="hint">{t("colors.overlayHint")}</div>
  </div>
</div>

<style>
  .stage {
    height: 100%;
    display: flex;
    align-items: flex-start;
    justify-content: flex-start;
    padding: var(--space-2);
    opacity: 0;
    transition: opacity var(--transition);
  }
  .stage.ready {
    opacity: 1;
  }

  /* Same floating-card surface as the radial. */
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2);
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
  }

  canvas {
    display: block;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    image-rendering: pixelated;
  }

  .readout {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
  }

  .hint {
    font-size: var(--fs-xs);
    line-height: 1.3;
    text-align: center;
    color: var(--text-muted);
  }
</style>
