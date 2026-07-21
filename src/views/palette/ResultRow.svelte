<script lang="ts">
  import type { Snippet, TransformKind } from "../../lib/types";
  import Icon from "../../components/Icon.svelte";
  import RowMenu from "./RowMenu.svelte";
  import { highlight } from "../../lib/highlight";

  interface Props {
    snippet: Snippet;
    active: boolean;
    /** Briefly true while this row is being inserted, for a confirm flash. */
    firing?: boolean;
    /** Whether this row's more-options menu is open (owned by the parent list). */
    menuOpen?: boolean;
    id: string;
    /** Current search query, for match highlighting (empty when browsing). */
    query?: string;
    /** Render position in the list, for the entrance stagger delay. Only the
        first few rows actually get a nonzero delay (see `staggerDelay`) so a
        long list doesn't visibly lag in one by one. */
    index?: number;
    onSelect: () => void;
    onHover: () => void;
    onMenuToggle?: () => void;
    onMenuClose?: () => void;
    /** Insert this snippet transformed ("paste as UPPERCASE/plain/…"). */
    onTransform?: (kind: TransformKind) => void;
  }
  let {
    snippet,
    active,
    firing = false,
    menuOpen = false,
    id,
    query = "",
    index = 0,
    onSelect,
    onHover,
    onMenuToggle,
    onMenuClose,
    onTransform,
  }: Props = $props();

  // Cap the stagger so a long list doesn't visibly crawl in — only the first
  // handful of rows get a delay, the rest enter together with the last one.
  const MAX_STAGGER_ITEMS = 8;
  const staggerDelay = $derived(`calc(var(--stagger) * ${Math.min(index, MAX_STAGGER_ITEMS)})`);

  // Render a snippet's first line for the list. Form variables are shown as a
  // friendly «Label» placeholder rather than the raw [[key:Label]] syntax, so
  // the list reads like text the user will get — not markup.
  function preview(body: string): string {
    const line = body
      .split("\n")[0]
      .replace(/\[\[\s*([^\]:]+?)\s*(?::\s*([^\]]*?))?\s*\]\]/g, (_m, key, label) =>
        `«${(label && label.trim()) || key.trim()}»`,
      );
    return line.length > 60 ? line.slice(0, 60) + "…" : line;
  }

  const nameChunks = $derived(highlight(snippet.name, query));
  const previewChunks = $derived(highlight(preview(snippet.body), query));
</script>

<!-- Listbox option: keyboard control lives on the parent combobox input
     (arrows/Enter), per the ARIA combobox pattern. Click/hover are mouse
     conveniences, so a per-row key handler would be redundant. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<li
  {id}
  class="row"
  class:active
  class:firing
  role="option"
  aria-selected={active}
  style:animation-delay={staggerDelay}
  onmousemove={onHover}
  onclick={onSelect}
>
  {#if snippet.isFavorite}
    <span class="star"><Icon name="star-filled" size={16} /></span>
  {:else}
    <span class="star placeholder"><Icon name="snippet" size={16} /></span>
  {/if}
  <div class="body">
    <div class="name-row">
      <span class="name"
        >{#each nameChunks as c}{#if c.match}<mark>{c.text}</mark>{:else}{c.text}{/if}{/each}</span
      >
      {#if snippet.trigger}<span class="trigger">{snippet.trigger}</span>{/if}
    </div>
    <span class="preview"
      >{#each previewChunks as c}{#if c.match}<mark>{c.text}</mark>{:else}{c.text}{/if}{/each}</span
    >
  </div>
  {#if onTransform}
    <RowMenu
      {active}
      open={menuOpen}
      headKey="palette.insertAs"
      onToggle={() => onMenuToggle?.()}
      onClose={() => onMenuClose?.()}
      onChoose={(kind) => onTransform(kind)}
    />
  {/if}
  <!-- Enter affordance: shows the primary action right where the eye is. -->
  <span class="enter" aria-hidden="true">↵</span>
</li>

<style>
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
    /* Staggered entrance: index-based delay comes from the `style:animation-delay`
       binding above; a ~0ms duration under reduced-motion lands directly on the
       100% (fully visible, untransformed) frame, so the guard alone is enough
       to make this instant without a separate override here. */
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
  /* "Stamp press" confirm flash: a crisp scale-down + solid accent fill the
     instant a row is picked, echoing a physical stamp hitting paper — quick
     enough to read as a snap, not a lingering highlight, before the window
     hides. The scale keyframe's rest state is 1 (its start, `from`), so under
     reduced-motion (duration -> ~0ms) it still lands on the flash's fill
     color instantly; only the press-in motion is skipped. */
  .row.firing {
    background: var(--accent);
    animation: row-stamp var(--transition-fast) var(--ease-out) both;
  }
  @keyframes row-stamp {
    0% {
      transform: scale(1);
    }
    35% {
      transform: scale(0.96);
    }
    100% {
      transform: scale(1);
    }
  }
  .row.firing .name,
  .row.firing .preview,
  .row.firing .trigger,
  .row.firing .enter {
    color: var(--accent-text);
  }
  .row.firing .trigger {
    background: color-mix(in srgb, var(--accent-text) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-text) 35%, transparent);
  }
  .row.firing mark {
    background: color-mix(in srgb, var(--accent-text) 30%, transparent);
    color: var(--accent-text);
  }
  /* Accent bar marks the active row unambiguously — same language as the
     manager sidebar's active nav item. */
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
  .row.firing::before {
    display: none;
  }
  mark {
    background: color-mix(in srgb, var(--accent) 24%, transparent);
    color: inherit;
    border-radius: 2px;
    padding: 0 1px;
  }
  .star {
    display: flex;
    color: var(--star);
    flex-shrink: 0;
  }
  .star.placeholder {
    color: var(--text-muted);
    opacity: 0.5;
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
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color var(--transition-fast);
  }
  .row.active .name {
    color: var(--text);
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
  /* "Press Enter" hint at the trailing edge, only on the active row. */
  .enter {
    flex-shrink: 0;
    margin-left: var(--space-1);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    color: var(--accent);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }
  .row.active .enter {
    opacity: 0.75;
  }
</style>
