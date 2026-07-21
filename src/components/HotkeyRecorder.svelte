<script lang="ts">
  import { t } from "../lib/i18n";
  import Icon from "./Icon.svelte";

  interface Props {
    /** Current accelerator (Tauri format, e.g. "Control+Shift+Space"), or "" for
        an unset shortcut. Owned by the parent so several recorders can be
        coordinated (primary vs. secondary). */
    value: string;
    /** Called with the newly-recorded accelerator once a full combo is pressed. */
    onRecord: (accel: string) => void;
    /** Optional clear affordance for an optional shortcut (e.g. the secondary).
        When provided, a "×" button removes the binding. */
    onClear?: () => void;
    /** ARIA label for the recorder button. */
    ariaLabel?: string;
    /** Text shown when `value` is empty (no shortcut set). */
    emptyLabel?: string;
    /** Disable interaction (e.g. while a save is in flight). */
    disabled?: boolean;
  }
  let {
    value,
    onRecord,
    onClear,
    ariaLabel,
    emptyLabel,
    disabled = false,
  }: Props = $props();

  let recording = $state(false);

  // Map a KeyboardEvent to a Tauri accelerator string, or null if the key is a
  // lone modifier (we wait for a non-modifier to complete the combo).
  function toAccelerator(e: KeyboardEvent): string | null {
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Control");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (e.metaKey) mods.push("Super");

    const key = e.key;
    // Lone modifier press: not a complete combo yet.
    if (["Control", "Alt", "Shift", "Meta", "OS"].includes(key)) return null;

    let main: string;
    if (key === " " || e.code === "Space") {
      main = "Space";
    } else if (/^[a-zA-Z]$/.test(key)) {
      main = key.toUpperCase();
    } else if (/^[0-9]$/.test(key)) {
      main = key;
    } else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) {
      main = key; // F1..F24
    } else if (key === "ArrowUp") main = "Up";
    else if (key === "ArrowDown") main = "Down";
    else if (key === "ArrowLeft") main = "Left";
    else if (key === "ArrowRight") main = "Right";
    else if (key === "Enter") main = "Enter";
    else if (key === "Tab") main = "Tab";
    else if (key === ",") main = "Comma";
    else if (key === ".") main = "Period";
    else if (key.length === 1) main = key.toUpperCase();
    else main = key; // best-effort fallback

    // A bare letter/number with no modifier would hijack normal typing; require
    // at least one modifier (Function keys are allowed alone).
    if (mods.length === 0 && !/^F([1-9]|1[0-9]|2[0-4])$/.test(main)) {
      return null;
    }
    return [...mods, main].join("+");
  }

  function onRecordKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = false;
      return;
    }
    const accel = toAccelerator(e);
    if (!accel) return; // wait for a full combo
    recording = false;
    onRecord(accel);
  }

  function startRecording() {
    if (disabled) return;
    recording = true;
  }

  // Pretty-print an accelerator for display (Control -> Ctrl, split into keys).
  function parts(accel: string): string[] {
    return accel.split("+").map((p) => (p === "Control" ? "Ctrl" : p));
  }
</script>

<div class="recorder-row">
  <div
    class="recorder"
    class:recording
    class:disabled
    role="button"
    tabindex={disabled ? -1 : 0}
    aria-label={ariaLabel ?? t("hotkey.recordAria")}
    onclick={startRecording}
    onkeydown={(e) => {
      if (recording) {
        onRecordKeydown(e);
      } else if (!disabled && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        startRecording();
      }
    }}
  >
    {#if recording}
      <span class="prompt">{t("hotkey.press")}</span>
    {:else if value}
      <span class="combo">
        {#each parts(value) as k (k)}
          <kbd>{k}</kbd>
        {/each}
      </span>
      <Icon name="settings" size={14} />
    {:else}
      <span class="prompt empty">{emptyLabel ?? t("hotkey.none")}</span>
      <Icon name="settings" size={14} />
    {/if}
  </div>

  {#if onClear && value && !recording}
    <button
      class="clear"
      title={t("hotkey.clear")}
      aria-label={t("hotkey.clear")}
      disabled={disabled}
      onclick={onClear}
    >
      <Icon name="close" size={14} />
    </button>
  {/if}
</div>

<style>
  .recorder-row {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .recorder {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    padding: 0 var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    cursor: pointer;
    transition:
      border-color var(--transition-fast),
      background var(--transition-fast);
  }
  .recorder:hover:not(.disabled) {
    border-color: var(--accent);
  }
  .recorder.recording {
    border-color: var(--accent);
    background: var(--accent-weak);
    color: var(--accent);
  }
  .recorder.disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .prompt {
    font-size: var(--fs-sm);
  }
  .prompt.empty {
    color: var(--text-muted);
  }
  .combo {
    display: inline-flex;
    gap: var(--space-1);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 2px 6px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-sm);
  }
  .clear {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .clear:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .clear:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
