<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { t } from "../lib/i18n";
  import Icon from "./Icon.svelte";

  interface Toast {
    id: number;
    message: string;
  }
  let toasts = $state<Toast[]>([]);
  let nextId = 0;

  function push(message: string) {
    const id = nextId++;
    toasts = [...toasts, { id, message }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
    }, 4000);
  }

  $effect(() => {
    const un = listen("expansion:blocked-elevated", () => {
      push(t("toast.elevatedBlocked"));
    });
    // Generic in-app toast: any view can `emit("carimbo:toast", message)`.
    const unToast = listen<string>("carimbo:toast", (e) => {
      if (e.payload) push(e.payload);
    });
    return () => {
      un.then((f) => f());
      unToast.then((f) => f());
    };
  });
</script>

<div class="toasts" aria-live="polite" aria-atomic="false">
  {#each toasts as toast (toast.id)}
    <div class="toast" role="status">
      <span class="toast-icon"><Icon name="clipboard" size={16} /></span>
      <span class="toast-message">{toast.message}</span>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    bottom: var(--space-4);
    right: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 1000;
    pointer-events: none;
  }
  /* Floating-window depth (elev-3) plus a leading accent bar — the toast
     reads as a deliberate, self-contained surface rather than a bare bordered
     box, and the accent edge gives it a clear "this is Carimbo talking to
     you" identity at a glance. */
  .toast {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    padding-left: calc(var(--space-4) + 3px);
    max-width: 360px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-3);
    font-size: var(--fs-sm);
    pointer-events: auto;
    overflow: hidden;
    animation: toast-in var(--transition-slow);
  }
  .toast::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent);
  }
  .toast-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    background: var(--accent-weak);
    color: var(--accent);
    border-radius: var(--radius-sm);
  }
  .toast-message {
    min-width: 0;
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
