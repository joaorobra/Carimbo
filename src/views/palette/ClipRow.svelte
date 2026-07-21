<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ClipEntry, TransformKind } from "../../lib/types";
  import { clipActions, contentTypeMeta, hexColor } from "../../lib/clips";
  import type { ClipAction } from "../../lib/clips";
  import { t } from "../../lib/i18n";
  import { timeAgo } from "../../lib/time";
  import { highlight } from "../../lib/highlight";
  import Icon from "../../components/Icon.svelte";
  import RowMenu from "./RowMenu.svelte";

  interface Props {
    entry: ClipEntry;
    active: boolean;
    /** Briefly true while this row is being pasted, for a confirm flash. */
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
    onTogglePin: () => void;
    /** Save this (text) clip as a reusable snippet. */
    onSaveSnippet?: () => void;
    /** Open the image lightbox (image rows only). */
    onPreview?: () => void;
    onMenuToggle?: () => void;
    onMenuClose?: () => void;
    /** Paste this (text) clip transformed ("paste as UPPERCASE/plain/…"). */
    onTransform?: (kind: TransformKind) => void;
    /** Run a content-aware action (open link, reveal path, paste color as…). */
    onAction?: (action: ClipAction) => void;
  }
  let {
    entry,
    active,
    firing = false,
    menuOpen = false,
    id,
    query = "",
    index = 0,
    onSelect,
    onHover,
    onTogglePin,
    onSaveSnippet,
    onPreview,
    onMenuToggle,
    onMenuClose,
    onTransform,
    onAction,
  }: Props = $props();

  // Cap the stagger so a long list doesn't visibly crawl in — only the first
  // handful of rows get a delay, the rest enter together with the last one.
  const MAX_STAGGER_ITEMS = 8;
  const staggerDelay = $derived(`calc(var(--stagger) * ${Math.min(index, MAX_STAGGER_ITEMS)})`);

  const previewChunks = $derived(highlight(entry.preview, query));

  const isImage = $derived(entry.kind === "image" && !!entry.imagePath);
  const isText = $derived(entry.kind === "text");

  // Brief "saved" confirmation on the save-as-snippet button.
  let saved = $state(false);
  function save() {
    onSaveSnippet?.();
    saved = true;
    setTimeout(() => (saved = false), 1200);
  }

  const meta = $derived(contentTypeMeta(entry.contentType));
  const swatch = $derived(
    entry.contentType === "color" ? hexColor(entry.content) : null,
  );
  // Content-aware actions (open link, reveal in Explorer, paste color as
  // HEX/RGB/HSL, edit image). Drives the more-options menu's actions section.
  const actions = $derived(clipActions(entry));
</script>

<!-- Listbox option: arrows/Enter live on the parent combobox input; click/hover
     are mouse conveniences (same pattern as ResultRow). The pin button is a real
     control, so its click is stopped from bubbling into row selection. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<li
  {id}
  class="row"
  class:active
  class:firing
  class:pinned={entry.isPinned}
  role="option"
  aria-selected={active}
  style:animation-delay={staggerDelay}
  onmousemove={onHover}
  onclick={onSelect}
>
  <span class="glyph">
    {#if entry.kind === "image" && entry.imagePath}
      <img class="thumb" src={convertFileSrc(entry.imagePath)} alt="" loading="lazy" />
    {:else if swatch}
      <span class="swatch" style="background:{swatch}" aria-hidden="true"></span>
    {:else if meta.icon}
      <Icon name={meta.icon} size={16} />
    {:else}
      <Icon name="clipboard" size={16} />
    {/if}
  </span>

  <span class="preview"
    >{#each previewChunks as c}{#if c.match}<mark>{c.text}</mark>{:else}{c.text}{/if}{/each}</span
  >

  {#if meta.badgeKey}
    <span class="badge">{t(meta.badgeKey)}</span>
  {/if}

  <span class="time">{timeAgo(entry.createdAt)}</span>

  {#if isText && onSaveSnippet}
    <button
      class="pin"
      title={t("clipboard.promote")}
      aria-label={t("clipboard.promoteAria")}
      onclick={(e) => {
        e.stopPropagation();
        save();
      }}
    >
      <Icon name={saved ? "check" : "file-plus"} size={16} />
    </button>
  {/if}

  {#if isText && onTransform}
    <RowMenu
      {active}
      open={menuOpen}
      {actions}
      headKey="palette.pasteAs"
      onToggle={() => onMenuToggle?.()}
      onClose={() => onMenuClose?.()}
      onChoose={(kind) => onTransform(kind)}
      onAction={(a) => onAction?.(a)}
    />
  {/if}

  <!-- Image clips have no text transforms, but may carry actions (e.g. Edit,
       coming soon). Show an actions-only more-options menu when they do. -->
  {#if isImage && actions.length}
    <RowMenu
      {active}
      open={menuOpen}
      {actions}
      showTransforms={false}
      headKey="palette.pasteAs"
      onToggle={() => onMenuToggle?.()}
      onClose={() => onMenuClose?.()}
      onChoose={() => {}}
      onAction={(a) => onAction?.(a)}
    />
  {/if}

  {#if isImage && onPreview}
    <button
      class="pin expand"
      title={t("lightbox.open")}
      aria-label={t("lightbox.open")}
      onclick={(e) => {
        e.stopPropagation();
        onPreview();
      }}
    >
      <Icon name="maximize" size={16} />
    </button>
  {/if}

  <!-- Pushpin glyph (not star) — pinning a clip is a distinct concept from
       favoriting a snippet (ResultRow's star), so the two controls never read
       as "the same action" at a glance. Accent-colored when active per the
       shared accent-for-active-toggle language; pin's hover-reveal behavior
       (opacity via .row.active/:focus-visible, inherited from the shared
       .pin rules below) is unchanged. -->
  <button
    class="pin pin-toggle"
    class:on={entry.isPinned}
    title={entry.isPinned ? t("palette.pinRemove") : t("palette.pinAdd")}
    aria-label={entry.isPinned ? t("palette.pinRemove") : t("palette.pinAdd")}
    aria-pressed={entry.isPinned}
    onclick={(e) => {
      e.stopPropagation();
      onTogglePin();
    }}
  >
    <Icon name="pin" size={16} />
  </button>
</li>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    cursor: default;
    min-height: var(--hit-target);
    transition: background var(--transition-fast);
    /* Staggered entrance: index-based delay comes from the `style:animation-delay`
       binding above; a ~0ms duration under reduced-motion lands directly on the
       100% (fully visible, untransformed) frame. */
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
     instant a clip is chosen — same idiom as ResultRow's firing state, so a
     pick reads identically whichever tab it happens on. */
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
  .row.firing .preview,
  .row.firing .time,
  .row.firing .badge {
    color: var(--accent-text);
  }
  .row.firing .badge {
    background: color-mix(in srgb, var(--accent-text) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent-text) 35%, transparent);
  }
  .row.firing mark {
    background: color-mix(in srgb, var(--accent-text) 30%, transparent);
    color: var(--accent-text);
  }
  /* Accent bar marks the active row — same language as ResultRow + sidebar. */
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
  .glyph {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 20px;
    color: var(--text-muted);
  }
  .thumb {
    max-height: 24px;
    max-width: 40px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
  }
  .preview {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-sm);
  }
  .badge {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-inset);
    border: 1px solid var(--border);
  }
  .time {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  /* Pin sits at the row's trailing edge. Hidden until the row is hovered/active
     or already pinned, so a clean list of unpinned clips stays uncluttered. */
  .pin {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--hit-target);
    height: var(--hit-target);
    flex-shrink: 0;
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
  .row.active .pin,
  .pin:focus-visible {
    opacity: 1;
  }
  .pin:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  /* Pinned state: accent-colored (not --star) — the pin glyph already reads as
     distinct from ResultRow's amber star, and using the same accent as the
     rest of the app's "active toggle" language keeps pinned≠favorite legible
     by shape (pushpin vs star) rather than relying on color alone. */
  .pin-toggle.on {
    color: var(--accent);
    opacity: 1;
  }
</style>
