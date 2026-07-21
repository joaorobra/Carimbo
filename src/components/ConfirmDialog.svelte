<script lang="ts">
  import Modal from "./Modal.svelte";

  // Thin wrapper over Modal for a yes/no confirmation (e.g. delete). The
  // message is plain text handed in by the caller (already i18n-resolved),
  // rendered as the dialog body.
  interface Props {
    title: string;
    message: string;
    confirmLabel: string;
    cancelLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
    /** Danger-styled confirm button — on by default since this is mainly used for destructive confirms. */
    danger?: boolean;
  }
  let {
    title,
    message,
    confirmLabel,
    cancelLabel,
    onConfirm,
    onCancel,
    danger = true,
  }: Props = $props();
</script>

<Modal {title} {confirmLabel} {cancelLabel} {onConfirm} {onCancel} {danger}>
  <p class="message">{message}</p>
</Modal>

<style>
  .message {
    margin: 0;
  }
</style>
