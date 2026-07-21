<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { settingsStore } from "../../lib/stores/settings.svelte";
  import { i18n, t } from "../../lib/i18n";

  interface Candidate {
    id: string;
    name: string;
    trigger: string;
    preview: string;
    distance: number;
  }

  // The list scrolls, but a disambiguation set is small by nature; the backend
  // already ranks by relevance, so capping keeps the window compact and the
  // number-key shortcuts (1-9) meaningful.
  const MAX_SHOWN = 9;

  let items = $state<Candidate[]>([]);
  let active = $state(0);
  let mounted = $state(false);

  // Match the manager's theme + language.
  $effect(() => {
    settingsStore.init();
    i18n.init();
  });

  // The backend emits the candidates right after showing the window.
  $effect(() => {
    const un = listen<Candidate[]>("radial:show", (e) => {
      items = (e.payload ?? []).slice(0, MAX_SHOWN);
      active = 0;
      mounted = true;
    });
    return () => {
      un.then((f) => f());
    };
  });

  async function pick(index: number) {
    const c = items[index];
    if (!c) {
      await dismiss();
      return;
    }
    await invoke("radial_pick", { id: c.id });
  }

  async function dismiss() {
    await invoke("radial_dismiss");
  }

  function onKeydown(e: KeyboardEvent) {
    // Number keys select directly (1-9).
    if (e.key >= "1" && e.key <= "9") {
      const n = parseInt(e.key, 10) - 1;
      if (n < items.length) {
        e.preventDefault();
        pick(n);
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
      case "Tab":
        e.preventDefault();
        if (items.length) active = (active + 1) % items.length;
        break;
      case "ArrowUp":
        e.preventDefault();
        if (items.length) active = (active - 1 + items.length) % items.length;
        break;
      case "Home":
        e.preventDefault();
        active = 0;
        break;
      case "End":
        e.preventDefault();
        active = items.length - 1;
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        pick(active);
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  }

  // Keep the active row scrolled into view when navigating by keyboard.
  $effect(() => {
    const el = document.getElementById(`radial-opt-${active}`);
    el?.scrollIntoView({ block: "nearest" });
  });
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Clicking the backdrop (outside the card) dismisses. -->
<div
  class="stage"
  class:ready={mounted}
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) dismiss();
  }}
>
  <div class="card">
    <!-- Header orients the user: these triggers look alike, here's how many.
         Mirrors the palette's search bar as the card's top strip. -->
    <div class="header">
      <span class="header-label">{t("radial.title")}</span>
      <span class="header-count">{items.length}</span>
    </div>

    <ul class="results" role="menu" aria-label={t("radial.chooseAria")}>
      {#each items as c, i (c.id)}
        <!-- Menu item: keyboard control lives on the window (arrows / 1-9 /
             Enter), like the palette's combobox, so a per-row key handler would
             be redundant. Click/hover are mouse conveniences. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
        <li
          id={`radial-opt-${i}`}
          class="row"
          class:active={i === active}
          role="menuitemradio"
          aria-checked={i === active}
          title={`${c.name} — ${c.preview}`}
          style:animation-delay={`calc(var(--stagger) * ${i})`}
          onmousemove={() => (active = i)}
          onclick={() => pick(i)}
        >
          <span class="num">{i + 1}</span>
          <div class="body">
            <div class="name-row">
              <span class="name">{c.name}</span>
              <span class="trigger">{c.trigger}</span>
            </div>
            {#if c.preview}
              <span class="preview">{c.preview}</span>
            {/if}
          </div>
        </li>
      {/each}
    </ul>

    <!-- Decluttered: kept the two hints that carry their own weight here (the
         numbers ARE the primary picking mechanism, arrows are the fallback);
         Enter/Esc are conventional enough not to need a callout on every
         open. Same restraint as the palette's footer. -->
    <div class="footer" aria-live="polite">
      <span class="hint"><kbd>↑</kbd><kbd>↓</kbd> {t("radial.navigate")}</span>
      <span class="hint"><kbd>1</kbd>–<kbd>9</kbd> {t("radial.choose")}</span>
    </div>
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
    transform: scale(0.97);
    transform-origin: top left;
    /* Entrance: subtle scale+opacity pop, same --ease-spring language as the
       palette's open animation, triggered once by the `mounted` flag flipping
       true on the first radial:show. A transition (not a keyframe animation)
       so the reduced-motion guards' `transition-duration: 0.001ms` collapse it
       straight to the `.ready` end state — no separate override needed. */
    transition:
      opacity var(--transition-slow) var(--ease-spring),
      transform var(--transition-slow) var(--ease-spring);
  }
  .stage.ready {
    opacity: 1;
    transform: scale(1);
  }

  /* Premium glass material, matching the palette card: this IS the window
     (transparent + shadow:false in tauri.conf). --elev-3 is the "floating
     window" tier of the shared elevation scale. Falls back to a solid surface
     where backdrop-filter isn't supported; --surface-glass itself already
     collapses to solid Canvas under forced-colors, and --elev-3 to a 1px
     border ring, so no separate forced-colors override is needed here. */
  .card {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--surface-glass);
    backdrop-filter: blur(20px) saturate(1.4);
    -webkit-backdrop-filter: blur(20px) saturate(1.4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--elev-3);
    overflow: hidden;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .card {
      background: var(--bg-elevated);
    }
  }

  .header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  .header-label {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .header-count {
    flex-shrink: 0;
    margin-left: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  .results {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    overflow-y: auto;
    flex: 1;
  }

  /* Same row idiom as ResultRow, with a leading number badge for the 1-9 keys. */
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    cursor: default;
    min-height: var(--hit-target);
    transition: background var(--transition-fast);
    /* Staggered entrance: index-based delay from the `style:animation-delay`
       binding above; a ~0ms duration under reduced-motion lands directly on
       the 100% (fully visible, untransformed) frame. */
    animation: row-in var(--transition) var(--ease-out) both;
  }
  @keyframes row-in {
    from {
      opacity: 0;
      transform: translateY(2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .row.active {
    background: var(--accent-weak);
  }
  /* Accent bar — same active language as the palette rows + sidebar. */
  .row.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 60%;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    background: var(--accent);
  }

  .num {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    transition:
      background var(--transition-fast),
      color var(--transition-fast),
      border-color var(--transition-fast);
  }
  .row.active .num {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .name-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trigger {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    padding: 1px 6px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }
  .preview {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Quieted to match the palette's restraint — a passive reference, not
     something competing with the candidate list for attention. */
  .footer {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-muted);
    opacity: 0.7;
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 1px 5px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
</style>
