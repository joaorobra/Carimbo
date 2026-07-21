<script lang="ts">
  import type { Snippet, Variable } from "../../lib/types";
  import { previewBody } from "../../lib/tokens";
  import { t } from "../../lib/i18n";
  import Icon from "../../components/Icon.svelte";

  interface Props {
    snippet: Snippet;
    variables: Variable[];
    /** Insert with the collected values, keyed by variable key. */
    onSubmit: (values: Record<string, string>) => void;
    /** Return to the results list without inserting. */
    onBack: () => void;
  }
  let { snippet, variables, onSubmit, onBack }: Props = $props();

  // One editable value per variable key, seeded blank at construction. The
  // parent keys this component by snippet id, so it re-mounts (and re-seeds)
  // per pick — no reseeding effect, which would race the `bind:value` bindings
  // and leave the inputs unmounted on first render. Capturing the initial
  // `variables` here is intentional (hence the ignore) precisely because of
  // that re-mount contract.
  // svelte-ignore state_referenced_locally
  let values = $state<Record<string, string>>(
    Object.fromEntries(variables.map((v) => [v.key, ""])),
  );
  let fields: HTMLInputElement[] = $state([]);

  // Live preview of exactly what will be inserted, with filled values applied.
  const filledPreview = $derived.by(() => {
    let body = snippet.body;
    for (const v of variables) {
      const val = values[v.key] ?? "";
      // Replace both [[key]] and [[key:Label]] occurrences with the typed value.
      body = body.replaceAll(new RegExp(escapeVar(v.key), "g"), () => val);
    }
    return previewBody(body);
  });

  // Build a regex source matching [[key]] or [[key:anything]] for this key.
  function escapeVar(key: string): string {
    const k = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return `\\[\\[${k}(:[^\\]]*)?\\]\\]`;
  }

  function submit() {
    onSubmit({ ...values });
  }

  function onFieldKeydown(e: KeyboardEvent, index: number) {
    if (e.key === "Enter") {
      e.preventDefault();
      if ((e.ctrlKey || e.metaKey) || index === variables.length - 1) {
        submit();
      } else {
        fields[index + 1]?.focus();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      onBack();
    }
  }

  // Autofocus the first field when the form appears.
  $effect(() => {
    fields[0]?.focus();
    fields[0]?.select();
  });
</script>

<div class="form-card">
  <header class="form-head">
    <button class="back" title={t("form.back")} aria-label={t("form.backAria")} onclick={onBack}>
      <Icon name="back" size={18} />
    </button>
    <div class="titles">
      <span class="eyebrow">{t("form.fillFields")}</span>
      <h2 class="name">{snippet.name}</h2>
    </div>
  </header>

  <div class="fields">
    {#each variables as v, i (v.key)}
      <label class="field">
        <span class="label">{v.label}</span>
        <input
          bind:this={fields[i]}
          bind:value={values[v.key]}
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder={t("form.fieldPlaceholder", { label: v.label.toLowerCase() })}
          onkeydown={(e) => onFieldKeydown(e, i)}
        />
      </label>
    {/each}

    <div class="preview-box">
      <span class="label">{t("editor.preview")}</span>
      <div class="preview-text">{filledPreview}</div>
    </div>
  </div>

  <footer class="form-foot">
    <span class="hint"><kbd>Enter</kbd> {t("form.nextField")}</span>
    <span class="hint"><kbd>Ctrl</kbd><kbd>Enter</kbd> {t("form.insert")}</span>
    <span class="hint"><kbd>Esc</kbd> {t("form.backHint")}</span>
    <button class="insert" onclick={submit}>
      <Icon name="check" size={16} />
      <span>{t("form.insertBtn")}</span>
    </button>
  </footer>
</div>

<style>
  .form-card {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .form-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-2);
    border-bottom: 1px solid var(--border);
  }
  .back {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--hit-target);
    height: var(--hit-target);
    flex-shrink: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .back:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .titles {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .eyebrow {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .name {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
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
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .preview-box {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: auto;
  }
  .preview-text {
    padding: var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.5;
    max-height: 96px;
    overflow-y: auto;
  }
  .form-foot {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 1px 5px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .insert {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: auto;
    min-height: var(--hit-target);
    padding: 0 var(--space-4);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    transition: filter var(--transition-fast);
  }
  .insert:hover {
    filter: brightness(1.08);
  }
</style>
