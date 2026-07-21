<script lang="ts">
  // Custom title bar for the frameless main window (decorations:false in
  // tauri.conf.json). Replaces the native Windows chrome so the wordmark can
  // live where the OS icon/title used to be, and the window controls can
  // match the design system instead of the OS theme.
  //
  // The bar itself (minus the control buttons) is the drag region — Tauri
  // wires `data-tauri-drag-region` to window-move + double-click-to-maximize
  // automatically, provided `core:window:allow-start-dragging` is granted
  // (see src-tauri/capabilities/default.json).
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t } from "../../lib/i18n";
  import Wordmark from "../../components/Wordmark.svelte";
  import Icon from "../../components/Icon.svelte";

  const win = getCurrentWindow();

  let maximized = $state(false);

  // Track maximize state for the maximize/restore glyph swap. Resize covers
  // both the toggle button and OS-level moves (snap layouts, drag-to-edge,
  // double-click-drag-region), so the glyph never goes stale.
  $effect(() => {
    win.isMaximized().then((v) => (maximized = v));
    const un = win.onResized(() => {
      win.isMaximized().then((v) => (maximized = v));
    });
    return () => {
      un.then((f) => f());
    };
  });

  function minimize() {
    win.minimize();
  }
  function toggleMaximize() {
    win.toggleMaximize();
  }
  function close() {
    win.close();
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand-slot" data-tauri-drag-region>
    <Wordmark size={16} />
  </div>

  <!-- Empty spacer: still part of the drag region so any gap in the bar is draggable. -->
  <div class="spacer" data-tauri-drag-region></div>

  <div class="controls" data-tauri-drag-region="false">
    <button
      type="button"
      class="control"
      aria-label={t("titlebar.minimize")}
      onclick={minimize}
    >
      <Icon name="minimize" size={16} />
    </button>
    <button
      type="button"
      class="control"
      aria-label={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
      onclick={toggleMaximize}
    >
      <Icon name={maximized ? "restore" : "window-maximize"} size={16} />
    </button>
    <button
      type="button"
      class="control control-close"
      aria-label={t("titlebar.close")}
      onclick={close}
    >
      <Icon name="window-close" size={16} />
    </button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    align-items: stretch;
    height: 40px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
  }
  .brand-slot {
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
  }
  .spacer {
    flex: 1;
  }
  .controls {
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
  }
  .control {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .control:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  /* Focus ring must stay visible even though the button sits flush against
     the window edge — inset so it doesn't clip against the titlebar top. */
  .control:focus-visible {
    outline-offset: -2px;
  }
  .control-close:hover {
    background: var(--danger);
    color: var(--accent-text);
  }
</style>
