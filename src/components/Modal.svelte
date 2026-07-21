<script lang="ts">
  import type { Snippet as SvelteSnippet } from "svelte";

  // Reusable centered dialog on a --scrim backdrop. Owns focus management only —
  // callers own the body content and the primary/secondary actions' behavior.
  // Modeled as a proper aria-modal dialog: focus moves in on open, Tab cycles
  // within it, Esc cancels, and focus restores to whatever triggered it on close.
  interface Props {
    /** Accessible dialog name — rendered as the visible heading too. */
    title: string;
    /** Body content. */
    children: SvelteSnippet;
    /** Primary action label (e.g. "Delete", "Add link"). */
    confirmLabel: string;
    /** Secondary/dismiss action label (usually "Cancel"). */
    cancelLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
    /** Danger-styled primary action (destructive confirms, e.g. delete). */
    danger?: boolean;
    /** Disable the primary action (e.g. empty required input). */
    confirmDisabled?: boolean;
  }
  let {
    title,
    children,
    confirmLabel,
    cancelLabel,
    onConfirm,
    onCancel,
    danger = false,
    confirmDisabled = false,
  }: Props = $props();

  let dialogEl: HTMLDivElement | undefined = $state();
  // The element focused right before the dialog opened — restored on close so
  // keyboard/screen-reader users land back where they were, not at document top.
  let previouslyFocused: HTMLElement | null = null;

  function focusables(): HTMLElement[] {
    if (!dialogEl) return [];
    const nodes = dialogEl.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
    );
    return Array.from(nodes).filter(
      (el) => el.offsetParent !== null || el === document.activeElement,
    );
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
      return;
    }
    if (e.key !== "Tab") return;
    const els = focusables();
    if (els.length === 0) return;
    const first = els[0];
    const last = els[els.length - 1];
    const active = document.activeElement;
    // Wrap the tab order at the dialog's edges instead of letting focus
    // escape into the page behind the scrim.
    if (e.shiftKey && active === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  }

  // Move focus into the dialog on mount; restore it to the trigger on unmount.
  // Svelte action form (rather than $effect) so it runs exactly once against
  // the dialog root regardless of how the parent conditionally mounts us.
  function trap(node: HTMLDivElement) {
    previouslyFocused = document.activeElement as HTMLElement | null;
    const target = node.querySelector<HTMLElement>("[data-autofocus]");
    (target ?? focusables()[0] ?? node).focus();
    return {
      destroy() {
        previouslyFocused?.focus?.();
      },
    };
  }
</script>

<div class="scrim">
  <!-- The backdrop is a separate layer purely for click-to-dismiss; the dialog
       itself stays free of non-dialog interaction semantics. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onCancel}></div>

  <div
    bind:this={dialogEl}
    use:trap
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
    tabindex="-1"
    onkeydown={onKeydown}
  >
    <h2 id="modal-title" class="title">{title}</h2>
    <div class="body">
      {@render children()}
    </div>
    <div class="actions">
      <button class="secondary" onclick={onCancel}>{cancelLabel}</button>
      <button
        class="primary"
        class:danger
        disabled={confirmDisabled}
        onclick={onConfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 2400;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: var(--scrim);
  }
  .dialog {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: 100%;
    max-width: 400px;
    padding: var(--space-5);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    box-shadow: var(--elev-3);
    /* Subtle scale/opacity pop on entrance. Duration/iteration are neutralized
       globally under prefers-reduced-motion (see base.css), so no JS branch
       is needed here to degrade to instant. */
    animation: dialog-in var(--transition-slow);
  }
  .dialog:focus-visible {
    outline: none;
  }
  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  .title {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    color: var(--text);
  }
  .body {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  .actions button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    padding: 0 var(--space-4);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition:
      filter var(--transition-fast),
      background var(--transition-fast),
      border-color var(--transition-fast),
      color var(--transition-fast);
  }
  .secondary {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--text);
  }
  .secondary:hover {
    background: var(--bg-hover);
  }
  .primary {
    background: var(--accent);
    color: var(--accent-text);
  }
  .primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .primary.danger {
    background: var(--danger);
    color: var(--accent-text);
  }
</style>
