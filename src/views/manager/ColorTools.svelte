<script lang="ts">
  import { untrack } from "svelte";
  import { listen, emit } from "@tauri-apps/api/event";
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n";
  import Icon from "../../components/Icon.svelte";
  import {
    contrastText,
    formatHsl,
    formatHsv,
    formatRgb,
    hexToRgb,
    hslToRgb,
    rgbToHex,
    rgbToHsl,
    toneScale,
    type Hsl,
    type Rgb,
  } from "../../lib/color";

  // A color handed in by the parent when the global color hotkey picks one (the
  // `seq` bumps on each pick so re-picking the same color still re-loads it).
  interface IncomingColor {
    r: number;
    g: number;
    b: number;
    seq: number;
  }
  let { incomingColor = null }: { incomingColor?: IncomingColor | null } =
    $props();

  // The strip: STEPS darker tones, the base color, STEPS lighter tones. The
  // base sits at index STEPS; picking/editing always re-centers there.
  const STEPS = 5;
  const RECENT_MAX = 10;

  const DEFAULT: Rgb = { r: 73, g: 119, b: 171 }; // the app's accent — steel-blue ink (#4977ab)

  let base = $state<Rgb>({ ...DEFAULT });
  let selIdx = $state(STEPS);
  // Slider source of truth. Kept separate from the derived rgb so dragging hue
  // at low saturation (etc.) doesn't jitter through lossy rgb->hsl round-trips.
  let hsl = $state<Hsl>(rgbToHsl(DEFAULT));
  let hexInput = $state(rgbToHex(DEFAULT));
  let recent = $state<string[]>([]);
  let picking = $state(false);
  // Blocks persisting until the saved color has been loaded back in.
  let loaded = false;

  const tones = $derived(toneScale(base, STEPS));
  const selected = $derived(tones[selIdx] ?? base);
  const selHex = $derived(rgbToHex(selected));

  const formats = $derived([
    { label: "HEX", value: selHex },
    { label: "RGB", value: formatRgb(selected) },
    { label: "HSL", value: formatHsl(selected) },
    { label: "HSV", value: formatHsv(selected) },
  ]);

  // Gradient tracks make the sliders self-describing: each shows what it will
  // do to the current color.
  const hueTrack =
    "linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)";
  const satTrack = $derived(
    `linear-gradient(to right, ${rgbToHex(hslToRgb({ ...hsl, s: 0 }))}, ${rgbToHex(hslToRgb({ ...hsl, s: 100 }))})`,
  );
  const lightTrack = $derived(
    `linear-gradient(to right, #000, ${rgbToHex(hslToRgb({ ...hsl, l: 50 }))}, #fff)`,
  );

  /** Make `c` the new base color (strip re-centers on it). */
  function setColor(c: Rgb) {
    base = c;
    selIdx = STEPS;
    hsl = rgbToHsl(c);
  }

  function selectTone(i: number) {
    selIdx = i;
    hsl = rgbToHsl(tones[i]);
  }

  // Sliders edit the color the user currently sees; the result becomes the new
  // base so the tone strip follows.
  function onSlider() {
    base = hslToRgb(hsl);
    selIdx = STEPS;
  }

  function onHexInput() {
    const parsed = hexToRgb(hexInput);
    if (parsed && rgbToHex(parsed) !== selHex) setColor(parsed);
  }

  // Reflect the selection back into the hex field. Only selHex is tracked —
  // reacting to hexInput itself would clobber half-typed (briefly invalid)
  // values — and a field that already parses to the same color (e.g. "#f00")
  // is left as the user wrote it.
  $effect(() => {
    const value = selHex;
    const parsed = hexToRgb(untrack(() => hexInput));
    if (!parsed || rgbToHex(parsed) !== value) hexInput = value;
  });

  function onChannelInput(channel: keyof Rgb, e: Event) {
    const n = (e.currentTarget as HTMLInputElement).valueAsNumber;
    if (Number.isNaN(n)) return;
    setColor({ ...selected, [channel]: Math.min(255, Math.max(0, Math.round(n))) });
  }

  // The HSL channels are edited via the `hsl` state (same source the sliders
  // use), so a direct number entry and a slider drag stay in sync.
  const HSL_MAX = { h: 360, s: 100, l: 100 } as const;

  function setHslChannel(channel: keyof Hsl, value: number) {
    if (Number.isNaN(value)) return;
    const clamped = Math.min(HSL_MAX[channel], Math.max(0, Math.round(value)));
    hsl = { ...hsl, [channel]: clamped };
    onSlider();
  }

  function onHslInput(channel: keyof Hsl, e: Event) {
    setHslChannel(channel, (e.currentTarget as HTMLInputElement).valueAsNumber);
  }

  // --- scroll-to-adjust ---------------------------------------------------
  // Opt-out toggle (persisted). When on, the wheel nudges a field/slider by ±1
  // (±10 with Shift), so fine-tuning a channel doesn't need click-drag or typing.
  let scrollAdjust = $state(true);

  function toggleScrollAdjust() {
    scrollAdjust = !scrollAdjust;
    api.setSetting("color.scrollAdjust", scrollAdjust);
  }

  // Wheel step: up = increase. Shift = coarse (10). Returns the delta to add.
  function wheelStep(e: WheelEvent): number {
    const dir = e.deltaY < 0 ? 1 : -1;
    return dir * (e.shiftKey ? 10 : 1);
  }

  function onRgbWheel(channel: keyof Rgb, e: WheelEvent) {
    if (!scrollAdjust) return;
    e.preventDefault();
    setColor({
      ...selected,
      [channel]: Math.min(255, Math.max(0, selected[channel] + wheelStep(e))),
    });
  }

  function onHslWheel(channel: keyof Hsl, e: WheelEvent) {
    if (!scrollAdjust) return;
    e.preventDefault();
    setHslChannel(channel, hsl[channel] + wheelStep(e));
  }

  async function pick() {
    if (picking) return;
    picking = true;
    try {
      await api.colorPickStart();
    } catch {
      picking = false;
    }
  }

  $effect(() => {
    const unPicked = listen<{ r: number; g: number; b: number }>(
      "color:picked",
      (e) => {
        picking = false;
        const c = { r: e.payload.r, g: e.payload.g, b: e.payload.b };
        setColor(c);
        addRecent(rgbToHex(c));
      },
    );
    const unCancelled = listen("color:cancelled", () => {
      picking = false;
    });
    return () => {
      unPicked.then((f) => f());
      unCancelled.then((f) => f());
    };
  });

  // A hotkey-picked color takes precedence over the restored "last color", even
  // if this effect's async settings read resolves after mount.
  let appliedIncomingSeq = 0;

  // Restore the last color + recents; persistence keeps the tool useful across
  // the hide-to-tray lifecycle.
  $effect(() => {
    api.getAllSettings().then((all) => {
      const cur = all["color.current"];
      const parsed = typeof cur === "string" ? hexToRgb(cur) : null;
      // Don't overwrite a color the hotkey already loaded.
      if (parsed && appliedIncomingSeq === 0) setColor(parsed);
      const rec = all["color.recent"];
      if (Array.isArray(rec)) {
        recent = rec
          .filter((x): x is string => typeof x === "string" && !!hexToRgb(x))
          .slice(0, RECENT_MAX);
      }
      if (typeof all["color.scrollAdjust"] === "boolean") {
        scrollAdjust = all["color.scrollAdjust"] as boolean;
      }
      loaded = true;
    });
  });

  // Load a color handed in by the global color hotkey. Reacts to `seq` so a
  // repeat pick of the same color re-loads it.
  $effect(() => {
    if (!incomingColor || incomingColor.seq === appliedIncomingSeq) return;
    appliedIncomingSeq = incomingColor.seq;
    const c = { r: incomingColor.r, g: incomingColor.g, b: incomingColor.b };
    untrack(() => {
      setColor(c);
      addRecent(rgbToHex(c));
    });
  });

  function addRecent(hex: string) {
    recent = [hex, ...recent.filter((h) => h !== hex)].slice(0, RECENT_MAX);
    api.setSetting("color.recent", $state.snapshot(recent));
  }

  // Persist the current selection (debounced — slider drags fire fast).
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const value = selHex;
    if (!loaded) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => api.setSetting("color.current", value), 400);
    return () => clearTimeout(saveTimer);
  });

  async function copy(value: string) {
    try {
      await api.colorCopy(value);
      await emit("carimbo:toast", t("colors.copied", { value }));
    } catch {
      await emit("carimbo:toast", t("colors.copyError"));
    }
  }
</script>

<div class="colors">
  <section class="card">
    <div class="card-head">
      <h2>{t("colors.title")}</h2>
      <button
        class="pick-btn"
        onclick={pick}
        disabled={picking}
        aria-label={t("colors.pickAria")}
      >
        <Icon name="pipette" size={16} />
        <span>{picking ? t("colors.picking") : t("colors.pick")}</span>
      </button>
    </div>

    <div class="hero">
      <div
        class="swatch-big"
        style:background={selHex}
        style:color={contrastText(selected)}
      >
        <!-- "Aa" contrast-text sample: a quick, honest preview of how the
             chosen color's own contrastText() reads as foreground text on
             itself — useful when picking a color destined to sit behind
             light or dark text. -->
        <span class="swatch-sample" aria-hidden="true">Aa</span>
        <span class="swatch-hex">{selHex}</span>
      </div>

      <div class="formats" aria-label={t("colors.copyFormats")}>
        {#each formats as f (f.label)}
          <button
            class="format-row"
            onclick={() => copy(f.value)}
            aria-label={t("colors.copyAria", { format: f.label })}
          >
            <span class="format-name">{f.label}</span>
            <code class="format-value">{f.value}</code>
            <span class="format-copy"><Icon name="copy" size={14} /></span>
          </button>
        {/each}
      </div>
    </div>

    <div class="tones">
      <div class="section-label">{t("colors.tones")}</div>
      <div class="strip">
        {#each tones as tone, i (i)}
          {@const hx = rgbToHex(tone)}
          <button
            class="tone"
            class:selected={i === selIdx}
            class:center={i === STEPS}
            style:background={hx}
            title={i === STEPS ? `${t("colors.baseTone")} — ${hx}` : hx}
            aria-label={t("colors.toneAria", { value: hx })}
            aria-pressed={i === selIdx}
            onclick={() => selectTone(i)}
          >
            {#if i === STEPS}
              <span
                class="center-dot"
                style:background={contrastText(tone)}
              ></span>
            {/if}
          </button>
        {/each}
      </div>
      <div class="strip-labels">
        <span>← {t("colors.darker")}</span>
        <span>{t("colors.lighter")} →</span>
      </div>
    </div>
  </section>

  <section class="card">
    <div class="card-head">
      <h2>{t("colors.edit")}</h2>
      <label class="scroll-toggle" title={t("colors.scrollAdjustNote")}>
        <input
          type="checkbox"
          checked={scrollAdjust}
          onchange={toggleScrollAdjust}
        />
        <span>{t("colors.scrollAdjust")}</span>
      </label>
    </div>

    <div class="fields">
      <label class="field hex-field">
        <span class="label">{t("colors.hex")}</span>
        <input
          type="text"
          class="hex-input"
          bind:value={hexInput}
          oninput={onHexInput}
          spellcheck="false"
          autocomplete="off"
        />
      </label>
      {#each [["r", t("colors.red")], ["g", t("colors.green")], ["b", t("colors.blue")]] as [ch, label] (ch)}
        <label class="field">
          <span class="label">{label}</span>
          <input
            type="number"
            min="0"
            max="255"
            value={selected[ch as keyof Rgb]}
            oninput={(e) => onChannelInput(ch as keyof Rgb, e)}
            onwheel={(e) => onRgbWheel(ch as keyof Rgb, e)}
          />
        </label>
      {/each}
    </div>

    <!-- HSL numeric entry, mirroring the RGB row. These edit the same `hsl`
         state the sliders bind to, so typing and dragging stay in sync. -->
    <div class="fields hsl-fields">
      {#each [["h", t("colors.hue"), "°"], ["s", t("colors.saturation"), "%"], ["l", t("colors.lightness"), "%"]] as [ch, label, unit] (ch)}
        <label class="field">
          <span class="label hsl-label">{label} <span class="unit">{unit}</span></span>
          <input
            type="number"
            min="0"
            max={HSL_MAX[ch as keyof Hsl]}
            value={hsl[ch as keyof Hsl]}
            oninput={(e) => onHslInput(ch as keyof Hsl, e)}
            onwheel={(e) => onHslWheel(ch as keyof Hsl, e)}
          />
        </label>
      {/each}
    </div>

    <div class="sliders">
      <div class="slider-row">
        <span class="label">{t("colors.hue")}</span>
        <input
          type="range"
          class="range"
          min="0"
          max="360"
          bind:value={hsl.h}
          oninput={onSlider}
          onwheel={(e) => onHslWheel("h", e)}
          style:--track={hueTrack}
          aria-label={t("colors.hue")}
        />
        <span class="slider-value">{hsl.h}°</span>
      </div>
      <div class="slider-row">
        <span class="label">{t("colors.saturation")}</span>
        <input
          type="range"
          class="range"
          min="0"
          max="100"
          bind:value={hsl.s}
          oninput={onSlider}
          onwheel={(e) => onHslWheel("s", e)}
          style:--track={satTrack}
          aria-label={t("colors.saturation")}
        />
        <span class="slider-value">{hsl.s}%</span>
      </div>
      <div class="slider-row">
        <span class="label">{t("colors.lightness")}</span>
        <input
          type="range"
          class="range"
          min="0"
          max="100"
          bind:value={hsl.l}
          oninput={onSlider}
          onwheel={(e) => onHslWheel("l", e)}
          style:--track={lightTrack}
          aria-label={t("colors.lightness")}
        />
        <span class="slider-value">{hsl.l}%</span>
      </div>
    </div>
  </section>

  {#if recent.length}
    <section class="card">
      <h2>{t("colors.recent")}</h2>
      <div class="recent-row">
        {#each recent as hx (hx)}
          <button
            class="recent-swatch"
            style:background={hx}
            title={hx}
            aria-label={t("colors.recentAria", { value: hx })}
            onclick={() => {
              const c = hexToRgb(hx);
              if (c) setColor(c);
            }}
          ></button>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  /* Same scrolling column-of-cards shell as Settings. */
  .colors {
    height: 100%;
    padding: var(--space-6) var(--space-5);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    overflow-y: auto;
  }
  .card {
    width: 100%;
    max-width: 620px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    /* A resting-card lift (elev-1) rather than a flat hairline card — this is
       a "designer tool" surface, so it earns a touch more depth than a plain
       settings list. */
    box-shadow: var(--elev-1);
  }
  h2 {
    font-size: var(--fs-lg);
    margin: 0;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .card-head h2 {
    padding-bottom: 0;
    border-bottom: none;
  }

  .pick-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    padding: 0 var(--space-4);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    transition:
      filter var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .pick-btn:hover:not(:disabled) {
    filter: brightness(1.08);
    box-shadow: var(--elev-1);
  }
  .pick-btn:disabled {
    opacity: 0.7;
    cursor: default;
  }

  .hero {
    display: grid;
    grid-template-columns: 216px 1fr;
    gap: var(--space-5);
    align-items: stretch;
  }
  /* The hero swatch is the designer-tool centerpiece: bigger than a plain
     preview chip, elev-2 depth so it reads as the "floating" focal surface,
     the Aa sample up top and the hex chip anchored bottom-left like a
     watermark. */
  .swatch-big {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: flex-start;
    padding: var(--space-4);
    min-height: 216px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--elev-2);
    transition: background var(--transition-fast);
  }
  .swatch-sample {
    font-size: var(--fs-2xl);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    line-height: 1;
    opacity: 0.92;
  }
  .swatch-hex {
    display: inline-flex;
    align-items: center;
    padding: var(--space-1) var(--space-3);
    background: color-mix(in srgb, currentColor 14%, transparent);
    border-radius: 999px;
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .formats {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-2);
  }
  /* Whole row is the copy button — value + affordance in one hit target. */
  .format-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--hit-target);
    padding: 0 var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    text-align: left;
    transition:
      background var(--transition-fast),
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .format-row:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
    box-shadow: var(--elev-1);
  }
  .format-name {
    width: 36px;
    flex-shrink: 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .format-value {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .format-copy {
    display: flex;
    color: var(--text-muted);
    transition: color var(--transition-fast);
  }
  .format-row:hover .format-copy {
    color: var(--accent);
  }

  .section-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .tones {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .strip {
    display: flex;
    gap: var(--space-1);
  }
  .tone {
    position: relative;
    flex: 1;
    height: 40px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      transform var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .tone:hover {
    transform: translateY(-2px);
    box-shadow: var(--elev-1);
  }
  /* Selection ring sits outside the swatch so the color itself stays honest. */
  .tone.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .center-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    opacity: 0.9;
  }
  .strip-labels {
    display: flex;
    justify-content: space-between;
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }

  .fields {
    display: grid;
    grid-template-columns: minmax(120px, 1fr) 72px 72px 72px;
    gap: var(--space-3);
  }
  /* HSL row: three equal-width inputs across the full row, so the longer labels
     (Saturation / Lightness) have room to sit on one line with their unit. */
  .hsl-fields {
    grid-template-columns: repeat(3, 1fr);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .label {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-muted);
  }
  .unit {
    font-weight: 400;
    opacity: 0.7;
  }
  .hsl-label {
    white-space: nowrap;
  }
  .fields input {
    min-height: var(--hit-target);
    padding: 0 var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    transition: border-color var(--transition-fast);
  }
  .fields input:focus {
    border-color: var(--accent);
  }
  .hex-input {
    font-family: var(--font-mono);
  }

  /* Scroll-to-adjust opt-out, sitting in the Adjust card header. */
  .scroll-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--fs-sm);
    color: var(--text-muted);
    cursor: pointer;
    user-select: none;
  }
  .scroll-toggle input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

  .sliders {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .slider-row {
    display: grid;
    grid-template-columns: 110px 1fr 52px;
    align-items: center;
    gap: var(--space-3);
  }
  /* Custom track so each slider previews its own effect on the color. */
  .range {
    appearance: none;
    -webkit-appearance: none;
    height: 14px;
    border-radius: 999px;
    background: var(--track);
    border: 1px solid var(--border);
  }
  .range::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #ffffff;
    border: 2px solid var(--border-strong);
    box-shadow: var(--shadow-sm);
  }
  .slider-value {
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    text-align: right;
    color: var(--text-muted);
  }

  .recent-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .recent-swatch {
    width: 32px;
    height: 32px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    transition:
      transform var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .recent-swatch:hover {
    transform: translateY(-2px);
    box-shadow: var(--elev-1);
  }
</style>
