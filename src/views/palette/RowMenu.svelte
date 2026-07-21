<script lang="ts">
  // Three-dot "more options" menu for a palette row. Offers the "paste as…"
  // text transforms (plain / UPPERCASE / slug / …) so a snippet or clip can be
  // inserted transformed without leaving the palette. The trigger sits at the
  // row's trailing edge, mirroring ClipRow's pin/save buttons; the popover
  // reuses the manager's transform-menu shape.
  import { TRANSFORMS } from "../../lib/clips";
  import type { ClipAction } from "../../lib/clips";
  import { t } from "../../lib/i18n";
  import type { TransformKind } from "../../lib/types";
  import Icon from "../../components/Icon.svelte";

  interface Props {
    /** Whether the parent row is the active one. Reveals the trigger (it's
        hidden on idle rows so the list stays clean). */
    active: boolean;
    /** Whether the menu is open. Owned by the row so only one is open at a time
        and arrow-navigating away can close it. */
    open: boolean;
    /** Verb shown in the menu head: "Paste as…" vs "Insert as…". When false, the
        transform list is hidden entirely (an actions-only menu, e.g. an image). */
    showTransforms?: boolean;
    /** Verb shown in the menu head: "Paste as…" vs "Insert as…". */
    headKey: "palette.pasteAs" | "palette.insertAs";
    /** Content-aware actions shown above the transforms (open link, reveal in
        Explorer, paste color as HEX/RGB/HSL, …). Empty/omitted → transforms
        only, exactly as before. */
    actions?: ClipAction[];
    onToggle: () => void;
    onClose: () => void;
    onChoose: (kind: TransformKind) => void;
    /** Run a content-aware action chosen from the actions section. */
    onAction?: (action: ClipAction) => void;
  }
  let {
    active,
    open,
    showTransforms = true,
    headKey,
    actions = [],
    onToggle,
    onClose,
    onChoose,
    onAction,
  }: Props = $props();

  // Whether the popover opens upward. Decided when the trigger is clicked, from
  // where the trigger sits in the scrolling results viewport: rows in the lower
  // portion open the menu above the trigger so it never spills past the palette
  // card's bottom edge (which clips it — see .results overflow / .palette-card
  // overflow: hidden).
  let up = $state(false);
  let wrap: HTMLDivElement | undefined = $state();

  function toggle() {
    if (!open) {
      // Opening: pick a direction that keeps the popover on-screen. Compare the
      // trigger's viewport position against the palette window height; if there
      // isn't room below for the menu, flip it upward.
      const rect = wrap?.getBoundingClientRect();
      // head + ~9 transform rows + any content actions (each ~32px + head).
      const actionsHeight = actions.length ? 28 + actions.length * 34 : 0;
      const menuHeight = 260 + actionsHeight;
      up = rect ? rect.bottom + menuHeight > window.innerHeight : false;
    }
    onToggle();
  }

  function pick(kind: TransformKind) {
    onChoose(kind);
    onClose();
  }

  function runAction(action: ClipAction) {
    // "soon" actions are inert placeholders — keep the menu open so the user
    // reads the hint rather than having it vanish on a dead click.
    if (action.run.t === "soon") return;
    onAction?.(action);
    onClose();
  }
  // Keyboard (Escape to close, arrows to navigate rows) is owned by the palette:
  // focus stays on the combobox input per the ARIA pattern, so it never lands on
  // this menu. The menu is a mouse affordance.
</script>

<!-- Wrapper is positioned so the popover anchors to the trigger. Clicks inside
     are stopped so they never reach the row's onclick (which would insert). -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="menu-wrap" bind:this={wrap} onclick={(e) => e.stopPropagation()}>
  <button
    class="more"
    class:visible={active}
    class:on={open}
    title={t("palette.moreOptions")}
    aria-label={t("palette.moreOptions")}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={(e) => {
      e.stopPropagation();
      toggle();
    }}
  >
    <Icon name="more" size={16} />
  </button>

  {#if open}
    <div class="menu" class:up role="menu">
      {#if actions.length}
        <p class="menu-head">{t("clipboard.actions")}</p>
        {#each actions as a (a.id)}
          {@const soon = a.run.t === "soon"}
          <button
            class="menu-item action"
            class:soon
            role="menuitem"
            disabled={soon}
            aria-disabled={soon}
            onclick={(e) => {
              e.stopPropagation();
              runAction(a);
            }}
          >
            <Icon name={a.icon} size={15} />
            <span class="label">{t(a.labelKey)}</span>
            {#if soon}<span class="soon-tag">{t("clipboard.soon")}</span>{/if}
          </button>
        {/each}
      {/if}

      {#if showTransforms}
        {#if actions.length}<span class="sep" role="separator"></span>{/if}
        <p class="menu-head">{t(headKey)}</p>
        {#each TRANSFORMS as tr (tr.kind)}
          <button
            class="menu-item"
            role="menuitem"
            onclick={(e) => {
              e.stopPropagation();
              pick(tr.kind);
            }}
          >
            {t(tr.labelKey)}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .menu-wrap {
    position: relative;
    flex-shrink: 0;
  }
  /* Same affordance as ClipRow's pin: hidden until the row is active or the
     menu is open, so an idle list stays clean. */
  .more {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--hit-target);
    height: var(--hit-target);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    opacity: 0;
    transition:
      opacity var(--transition-fast),
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .more.visible,
  .more:focus-visible {
    opacity: 1;
  }
  .more:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .more.on {
    opacity: 1;
    background: var(--bg-hover);
    color: var(--text);
  }
  /* Popover anchored to the trigger, opening from the row's trailing edge.
     Mirrors the manager's ClipActions menu. */
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 30;
    /* Cap height + scroll so a downward menu never grows past the card either. */
    max-height: 300px;
    overflow-y: auto;
    min-width: 168px;
    padding: var(--space-1);
    /* Fully opaque menu surface. The parent .palette-card is frosted glass
       (--surface-glass, whose alpha is user-tunable), so this popover must NOT
       inherit any translucency or the result rows behind it ghost through the
       menu text. --bg-elevated is a solid color in every theme; `isolation`
       gives the menu its own compositing context so it can never blend with the
       glass pane beneath it. */
    background: var(--bg-elevated);
    isolation: isolate;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    /* Popover tier of the shared elevation scale (1 = resting card, 2 = this
       popover, 3 = the floating palette/radial window itself). */
    box-shadow: var(--elev-2);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  /* Lower rows open the menu upward, anchored to the trigger's top edge, so it
     never spills past the palette card's clipped bottom. */
  .menu.up {
    top: auto;
    bottom: calc(100% + 4px);
  }
  .menu-head {
    margin: 0;
    padding: var(--space-1) var(--space-2);
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .menu-item {
    text-align: left;
    padding: var(--space-2);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background var(--transition-fast);
  }
  .menu-item:hover {
    background: var(--accent-weak);
  }
  /* Content-aware actions carry a leading icon; the label + optional "soon" tag
     lay out in a row (transforms stay plain text, so they're unaffected). */
  .menu-item.action {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .menu-item.action .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .menu-item.action :global(svg) {
    flex-shrink: 0;
    color: var(--text-muted);
  }
  .menu-item.action:hover :global(svg) {
    color: var(--text);
  }
  /* Deferred action: dimmed, non-interactive, with a small "soon" pill so it
     reads as a preview of what's coming rather than a broken button. */
  .menu-item.soon {
    color: var(--text-muted);
    cursor: default;
  }
  .menu-item.soon:hover {
    background: transparent;
  }
  .soon-tag {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-inset);
    border: 1px solid var(--border);
  }
  /* Hairline between the actions section and the transforms below it. */
  .sep {
    height: 1px;
    margin: var(--space-1) var(--space-1);
    background: var(--border);
  }
</style>
