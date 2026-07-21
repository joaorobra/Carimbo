<script lang="ts">
  import Modal from "./Modal.svelte";

  // Thin wrapper over Modal for a single-line text prompt (e.g. the rich-text
  // editor's "Link URL" entry). Enter submits, Esc/backdrop/Cancel abort with
  // nothing — the caller decides what an empty value means (addLink treats it
  // as "no-op", matching the native prompt() it replaces).
  interface Props {
    title: string;
    label: string;
    confirmLabel: string;
    cancelLabel: string;
    placeholder?: string;
    /** Called with the trimmed, non-empty input. Never called for empty input. */
    onSubmit: (value: string) => void;
    onCancel: () => void;
  }
  let {
    title,
    label,
    confirmLabel,
    cancelLabel,
    placeholder,
    onSubmit,
    onCancel,
  }: Props = $props();

  let value = $state("");
  const canSubmit = $derived(value.trim().length > 0);

  function submit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
  }
</script>

<Modal
  {title}
  {confirmLabel}
  {cancelLabel}
  onConfirm={submit}
  {onCancel}
  confirmDisabled={!canSubmit}
>
  <label class="field">
    <span class="label">{label}</span>
    <input
      data-autofocus
      type="text"
      bind:value
      {placeholder}
      autocomplete="off"
      spellcheck="false"
      onkeydown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          submit();
        }
      }}
    />
  </label>
</Modal>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .label {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-muted);
  }
  input {
    padding: var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    min-height: var(--hit-target);
    transition:
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  input:focus {
    border-color: var(--accent);
    outline: none;
  }
</style>
