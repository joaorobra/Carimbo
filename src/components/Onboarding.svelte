<script lang="ts">
  import { api } from "../lib/api";
  import { snippetStore } from "../lib/stores/snippets.svelte";
  import { t } from "../lib/i18n";
  import Icon from "./Icon.svelte";

  // Shown on first run (no snippets yet). Creates the user's first snippet in
  // under a minute, then dismisses itself.
  interface Props {
    onDismiss: () => void;
  }
  let { onDismiss }: Props = $props();

  let name = $state("");
  let body = $state("");
  let trigger = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);

  const canSave = $derived(name.trim().length > 0 && body.length > 0);

  // Render the shortcut in the tip as styled <kbd> chips: split the translated
  // template on its literal {combo} placeholder (left unfilled) and drop the
  // keys in between. Keys of the default palette shortcut.
  const comboKeys = ["Ctrl", "Shift", "Space"];
  const tipParts = $derived(t("onboarding.tip").split("{combo}"));

  async function createFirst() {
    if (!canSave) return;
    saving = true;
    error = null;
    try {
      await api.createSnippet({
        name: name.trim(),
        body,
        trigger: trigger.trim() || null,
        folderId: null,
      });
      await snippetStore.refresh();
      onDismiss();
    } catch (e) {
      error = t("onboarding.error");
      saving = false;
    }
  }
</script>

<div class="overlay">
  <div class="card" role="dialog" aria-label={t("onboarding.welcomeAria")} aria-modal="true">
    <button class="skip" onclick={onDismiss} aria-label={t("onboarding.skipAria")}>
      <Icon name="close" size={18} />
    </button>

    <div class="welcome">
      <h1>{t("onboarding.title")}</h1>
      <p class="lead">{t("onboarding.lead")}</p>
    </div>

    <div class="form">
      <label class="field">
        <span class="label">{t("onboarding.name")}</span>
        <input bind:value={name} placeholder={t("onboarding.namePlaceholder")} />
      </label>
      <label class="field">
        <span class="label">{t("onboarding.text")}</span>
        <textarea bind:value={body} rows="3" placeholder={t("onboarding.textPlaceholder")}></textarea>
      </label>
      <label class="field">
        <span class="label">{t("onboarding.trigger")} <span class="opt">{t("editor.optional")}</span></span>
        <input bind:value={trigger} class="mono" placeholder={t("onboarding.triggerPlaceholder")} />
      </label>

      {#if error}<div class="error" role="alert">{error}</div>{/if}

      <div class="tips">
        <p>
          <Icon name="search" size={14} />
          <span>{tipParts[0]}</span>
          {#each comboKeys as k, i (k)}<kbd>{k}</kbd>{#if i < comboKeys.length - 1}<span class="plus">+</span>{/if}{/each}
          <span>{tipParts[1] ?? ""}</span>
        </p>
      </div>

      <div class="actions">
        <button class="secondary" onclick={onDismiss}>{t("onboarding.skip")}</button>
        <button class="primary" disabled={!canSave || saving} onclick={createFirst}>
          <Icon name="check" size={16} />
          <span>{t("onboarding.create")}</span>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--scrim);
    z-index: 2000;
    padding: var(--space-4);
  }
  .card {
    position: relative;
    width: 100%;
    max-width: 480px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
    padding: var(--space-6);
    max-height: 90vh;
    overflow-y: auto;
  }
  .skip {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    display: flex;
    padding: var(--space-1);
    background: transparent;
    border: none;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
  }
  .skip {
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast);
  }
  .skip:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  h1 {
    margin: 0 0 var(--space-2);
    font-size: var(--fs-xl);
  }
  .lead {
    margin: 0 0 var(--space-5);
    color: var(--text-muted);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
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
  .opt {
    font-weight: 400;
  }
  input,
  textarea {
    padding: var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    min-height: var(--hit-target);
    transition: border-color var(--transition-fast);
  }
  input:focus,
  textarea:focus {
    border-color: var(--accent);
  }
  .mono {
    font-family: var(--font-mono);
  }
  textarea {
    resize: vertical;
  }
  .tips {
    padding: var(--space-3);
    background: var(--bg-inset);
    border-radius: var(--radius-md);
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .tips p {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .plus {
    margin: 0 -2px;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 1px 5px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .error {
    padding: var(--space-2) var(--space-3);
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-2);
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
      background var(--transition-fast);
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
  .secondary {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--text);
  }
  .secondary:hover {
    background: var(--bg-hover);
  }
</style>
