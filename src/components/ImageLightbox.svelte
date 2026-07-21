<script lang="ts">
  import { t } from "../lib/i18n";
  import Icon from "./Icon.svelte";

  // Full-screen preview of a clipboard image. The parent owns the clip id and
  // supplies the resolved asset `src`; this component is purely presentational
  // plus a small action bar whose handlers the parent wires to the clipboard
  // API. Kept generic so both the palette and the manager can reuse it.
  interface Props {
    /** Resolved asset URL (via convertFileSrc) for the PNG. */
    src: string;
    /** Accessible label / caption (usually the clip preview, e.g. "Image 800×600"). */
    alt: string;
    onClose: () => void;
    /** Copy the image back to the clipboard. Optional — hidden if omitted. */
    onCopy?: () => void;
    /** Paste the image into the previously focused app. Optional. */
    onPaste?: () => void;
    /** Reveal the backing PNG on disk (to save/reuse it). Optional. */
    onReveal?: () => void;
  }
  let { src, alt, onClose, onCopy, onPaste, onReveal }: Props = $props();

  // Toggle a 1:1 "actual size" view; default fits the image within the viewport.
  let zoomed = $state(false);

  // Transient "Copied" confirmation on the copy button.
  let copied = $state(false);
  function copy() {
    onCopy?.();
    copied = true;
    setTimeout(() => (copied = false), 1200);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="dialog" aria-modal="true" aria-label={alt}>
  <!-- A dedicated backdrop layer handles click-to-dismiss, so the dialog
       container itself carries no interaction (keeps a11y roles clean). -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onClose}></div>

  <div class="toolbar">
    <span class="caption">{alt}</span>
    <div class="spacer"></div>
    <button
      class="tbtn"
      class:on={zoomed}
      title={zoomed ? t("lightbox.fit") : t("lightbox.actualSize")}
      aria-label={zoomed ? t("lightbox.fit") : t("lightbox.actualSize")}
      aria-pressed={zoomed}
      onclick={() => (zoomed = !zoomed)}
    >
      <Icon name="maximize" size={16} />
    </button>
    {#if onCopy}
      <button
        class="tbtn"
        title={t("clipboard.copy")}
        aria-label={t("clipboard.copyAria")}
        onclick={copy}
      >
        <Icon name={copied ? "check" : "copy"} size={16} />
      </button>
    {/if}
    {#if onPaste}
      <button
        class="tbtn"
        title={t("lightbox.paste")}
        aria-label={t("lightbox.paste")}
        onclick={onPaste}
      >
        <Icon name="clipboard" size={16} />
      </button>
    {/if}
    {#if onReveal}
      <button
        class="tbtn"
        title={t("lightbox.save")}
        aria-label={t("lightbox.save")}
        onclick={onReveal}
      >
        <Icon name="download" size={16} />
      </button>
    {/if}
    <button
      class="tbtn"
      title={t("lightbox.close")}
      aria-label={t("lightbox.close")}
      onclick={onClose}
    >
      <Icon name="close" size={16} />
    </button>
  </div>

  <div class="stage" class:zoomed>
    <!-- Click-to-zoom is a mouse convenience; the toolbar's "actual size" button
         is the keyboard-accessible equivalent. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_click_events_have_key_events -->
    <img class="image" class:zoomed {src} {alt} onclick={() => (zoomed = !zoomed)} />
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 2200;
    display: flex;
    flex-direction: column;
    padding: var(--space-4);
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    cursor: zoom-out;
  }
  .toolbar {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    margin-bottom: var(--space-3);
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow);
  }
  .caption {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
  .spacer {
    flex: 1;
  }
  .tbtn {
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
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .tbtn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .tbtn.on {
    color: var(--accent);
  }
  /* The stage fills the remaining space and centers the image. When zoomed it
     scrolls; when fitted the image is clamped to the stage. */
  .stage {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    pointer-events: none;
  }
  .stage.zoomed {
    overflow: auto;
  }
  /* Only the image itself is interactive within the stage, so clicks in the
     empty stage margins fall through to the backdrop and dismiss. */
  .image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    cursor: zoom-in;
    background: var(--bg-inset);
    pointer-events: auto;
  }
  .image.zoomed {
    max-width: none;
    max-height: none;
    cursor: zoom-out;
  }
</style>
