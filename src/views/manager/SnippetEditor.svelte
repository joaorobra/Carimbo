<script lang="ts">
  import { snippetStore } from "../../lib/stores/snippets.svelte";
  import { api } from "../../lib/api";
  import { isCommandError } from "../../lib/types";
  import { TOKENS, previewBody, tokenLabel, tokenDesc } from "../../lib/tokens";
  import { t } from "../../lib/i18n";
  import Icon from "../../components/Icon.svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import PromptDialog from "../../components/PromptDialog.svelte";

  interface Props {
    /** When true the editor is in "create" mode (no selected snippet). */
    creating: boolean;
    onDone: () => void;
  }
  let { creating, onDone }: Props = $props();

  // Local editable copy. Re-seeds when the selection or create-mode changes.
  let name = $state("");
  let trigger = $state("");
  let body = $state("");
  // Rich (HTML) form. `rich` toggles the editor mode; `bodyHtml` holds the
  // markup when rich. A plain snippet keeps bodyHtml null.
  let rich = $state(false);
  let bodyHtml = $state("");
  let folderId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let bodyEl: HTMLTextAreaElement | undefined = $state();
  let richEl: HTMLDivElement | undefined = $state();

  // Track which snippet id we've loaded so we don't clobber edits on every
  // reactive tick — only reseed when the identity actually changes.
  let loadedKey = $state<string | null>(null);

  $effect(() => {
    const key = creating ? "__new__" : snippetStore.selectedId;
    if (key === loadedKey) return;
    loadedKey = key ?? null;
    error = null;
    if (creating || !snippetStore.selected) {
      name = "";
      trigger = "";
      body = "";
      rich = false;
      bodyHtml = "";
      folderId = snippetStore.activeFolderId;
    } else {
      const s = snippetStore.selected;
      name = s.name;
      trigger = s.trigger ?? "";
      body = s.body;
      rich = s.bodyHtml != null;
      bodyHtml = s.bodyHtml ?? "";
      folderId = s.folderId;
      // Seed the contenteditable's DOM when opening a rich snippet.
      if (rich) queueMicrotask(() => seedRich());
    }
  });

  function seedRich() {
    if (richEl && richEl.innerHTML !== bodyHtml) richEl.innerHTML = bodyHtml;
  }

  // Read the current markup + plain text out of the contenteditable.
  function syncFromRich() {
    if (!richEl) return;
    bodyHtml = richEl.innerHTML;
    body = richEl.innerText;
  }

  function toggleRich(enabled: boolean) {
    rich = enabled;
    if (enabled) {
      // Seed the rich editor from the current plain text (as paragraphs).
      bodyHtml = body
        ? body
            .split("\n")
            .map((line) => escapeHtml(line) || "<br>")
            .join("<br>")
        : "";
      queueMicrotask(() => {
        seedRich();
        richEl?.focus();
      });
    } else {
      // Leaving rich mode: keep the plain text, drop the markup.
      if (richEl) body = richEl.innerText;
      bodyHtml = "";
    }
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  // Rich toolbar commands. execCommand is deprecated but remains the pragmatic
  // way to format a contenteditable without pulling in an editor dependency.
  function format(command: string, value?: string) {
    richEl?.focus();
    document.execCommand(command, false, value);
    syncFromRich();
  }

  // Native confirm()/prompt() are replaced by in-app dialogs (Modal-backed).
  // These flags gate which dialog is mounted; only one is ever open at a time.
  let confirmingDelete = $state(false);
  let promptingLink = $state(false);

  // Opening the link prompt moves focus off the contenteditable, which clears
  // its DOM selection — so the range has to be captured before the dialog
  // mounts and reapplied just before createLink runs.
  let savedRange: Range | null = null;

  function captureSelection() {
    const sel = window.getSelection();
    savedRange =
      sel && sel.rangeCount > 0 && richEl?.contains(sel.anchorNode)
        ? sel.getRangeAt(0).cloneRange()
        : null;
  }

  function restoreSelection() {
    if (!savedRange) return;
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(savedRange);
  }

  function addLink() {
    captureSelection();
    promptingLink = true;
  }

  function confirmAddLink(url: string) {
    promptingLink = false;
    richEl?.focus();
    restoreSelection();
    format("createLink", url);
  }

  function cancelAddLink() {
    promptingLink = false;
  }

  const canSave = $derived(name.trim().length > 0);

  async function save() {
    error = null;
    // In rich mode, pull the latest markup/plain text straight from the DOM so a
    // final unsynced keystroke isn't lost.
    if (rich) syncFromRich();
    const payload = {
      name: name.trim(),
      trigger: trigger.trim() || null,
      body,
      bodyHtml: rich && bodyHtml.trim() ? bodyHtml : null,
      folderId,
    };
    try {
      if (creating || !snippetStore.selected) {
        const created = await api.createSnippet({ ...payload });
        // Refresh before selecting so the new row is in the local array — the
        // `snippets:changed` event round-trip is async and would otherwise leave
        // `selected` null for a tick, unmounting the editor to the placeholder.
        await snippetStore.refreshSnippets();
        snippetStore.select(created.id);
      } else {
        await api.updateSnippet({
          id: snippetStore.selected.id,
          ...payload,
          isFavorite: snippetStore.selected.isFavorite,
        });
        await snippetStore.refreshSnippets();
      }
      onDone();
    } catch (e) {
      error = isCommandError(e)
        ? e.kind === "duplicate_trigger"
          ? t("editor.errorDuplicateTrigger", { trigger })
          : e.message
        : t("editor.errorSave");
    }
  }

  function remove() {
    if (!snippetStore.selected) return;
    confirmingDelete = true;
  }

  async function confirmRemove() {
    confirmingDelete = false;
    if (!snippetStore.selected) return;
    await api.deleteSnippet(snippetStore.selected.id);
    snippetStore.select(null);
    onDone();
  }

  function cancelRemove() {
    confirmingDelete = false;
  }

  function insertToken(token: string) {
    const el = bodyEl;
    if (!el) {
      body += token;
      return;
    }
    const start = el.selectionStart;
    const end = el.selectionEnd;
    body = body.slice(0, start) + token + body.slice(end);
    // Restore caret after the inserted token on next tick.
    queueMicrotask(() => {
      el.focus();
      el.selectionStart = el.selectionEnd = start + token.length;
    });
  }
</script>

<div class="editor">
  <div class="fields">
    <label class="field">
      <span class="label">{t("editor.name")}</span>
      <input
        bind:value={name}
        placeholder={t("editor.namePlaceholder")}
        onkeydown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) save();
        }}
      />
    </label>

    <div class="row">
      <label class="field">
        <span class="label">{t("editor.trigger")} <span class="opt">{t("editor.optional")}</span></span>
        <input
          bind:value={trigger}
          class="mono"
          placeholder={t("editor.triggerPlaceholder")}
          spellcheck="false"
          autocomplete="off"
        />
      </label>
      <label class="field">
        <span class="label">{t("editor.folder")}</span>
        <select bind:value={folderId}>
          <option value={null}>{t("editor.noFolder")}</option>
          {#each snippetStore.folders as f (f.id)}
            <option value={f.id}>{f.name}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="field">
      <div class="body-label">
        <span class="label">{t("editor.content")}</span>
        <label class="rich-toggle">
          <input
            type="checkbox"
            checked={rich}
            onchange={(e) =>
              toggleRich((e.target as HTMLInputElement).checked)}
          />
          <span>{t("editor.richText")}</span>
        </label>
      </div>

      {#if !rich}
        <div class="tokens" role="group" aria-label={t("editor.insertTokenAria")}>
          {#each TOKENS as tok (tok.token)}
            <button
              type="button"
              class="token-btn"
              title={tokenDesc(tok)}
              onclick={() => insertToken(tok.token)}
            >
              {tokenLabel(tok)}
            </button>
          {/each}
        </div>
        <textarea
          bind:this={bodyEl}
          bind:value={body}
          class="mono"
          rows="8"
          placeholder={t("editor.bodyPlaceholder")}
        ></textarea>
      {:else}
        <!-- Cohesive segmented control: every command (including link/clear,
             previously Lucide icons sitting next to bold/italic/underline
             letter-glyphs) now renders as an inline SVG at the same stroke
             weight as the app's Lucide icon set, so the toolbar reads as one
             control instead of two mismatched styles. -->
        <div class="rich-toolbar" role="toolbar" aria-label={t("editor.richToolbar")}>
          <button type="button" title={t("editor.bold")} onclick={() => format("bold")}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 4h8a4 4 0 0 1 0 8H6z" />
              <path d="M6 12h9a4 4 0 0 1 0 8H6z" />
            </svg>
          </button>
          <button type="button" title={t("editor.italic")} onclick={() => format("italic")}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <line x1="19" y1="4" x2="10" y2="4" />
              <line x1="14" y1="20" x2="5" y2="20" />
              <line x1="15" y1="4" x2="9" y2="20" />
            </svg>
          </button>
          <button type="button" title={t("editor.underline")} onclick={() => format("underline")}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 4v6a6 6 0 0 0 12 0V4" />
              <line x1="4" y1="20" x2="20" y2="20" />
            </svg>
          </button>
          <span class="toolbar-divider" aria-hidden="true"></span>
          <button type="button" title={t("editor.link")} onclick={addLink}>
            <Icon name="external-link" size={14} />
          </button>
          <button
            type="button"
            title={t("editor.clearFormat")}
            onclick={() => format("removeFormat")}
          >
            <Icon name="wand" size={14} />
          </button>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          bind:this={richEl}
          class="rich-area"
          contenteditable="true"
          role="textbox"
          aria-multiline="true"
          aria-label={t("editor.content")}
          oninput={syncFromRich}
        ></div>
        <p class="note-inline">{t("editor.richNote")}</p>
      {/if}
    </div>

    {#if !rich && body.includes("{")}
      <div class="preview-box">
        <span class="label">{t("editor.preview")}</span>
        <div class="preview-text">{previewBody(body)}</div>
      </div>
    {/if}

    {#if error}
      <div class="error" role="alert">{error}</div>
    {/if}
  </div>

  <div class="actions">
    {#if !creating && snippetStore.selected}
      <button class="danger" onclick={remove}>
        <Icon name="trash" size={16} />
        <span>{t("editor.delete")}</span>
      </button>
    {/if}
    <div class="spacer"></div>
    <button class="secondary" onclick={onDone}>{t("editor.cancel")}</button>
    <button class="primary" disabled={!canSave} onclick={save}>
      <Icon name="check" size={16} />
      <span>{t("editor.save")}</span>
    </button>
  </div>
</div>

{#if confirmingDelete && snippetStore.selected}
  <ConfirmDialog
    title={t("editor.delete")}
    message={t("editor.confirmDelete", { name: snippetStore.selected.name })}
    confirmLabel={t("editor.delete")}
    cancelLabel={t("editor.cancel")}
    onConfirm={confirmRemove}
    onCancel={cancelRemove}
  />
{/if}

{#if promptingLink}
  <PromptDialog
    title={t("editor.link")}
    label={t("editor.linkPrompt")}
    confirmLabel={t("editor.link")}
    cancelLabel={t("editor.cancel")}
    placeholder="https://…"
    onSubmit={confirmAddLink}
    onCancel={cancelAddLink}
  />
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-5);
    /* Comfortable reading measure so short fields don't sprawl on a wide pane.
       Centered, so a maximized window keeps the form as a centered column
       rather than stranding it against the left edge beside empty space. */
    width: 100%;
    max-width: 720px;
    margin-inline: auto;
    overflow-y: auto;
    flex: 1;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
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
  select,
  textarea {
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
  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--accent);
    /* Mirrors the search field's focus-within ring in the list pane, so focus
       reads consistently across the two panes. */
    box-shadow: 0 0 0 1px var(--accent);
  }
  .mono {
    font-family: var(--font-mono);
  }
  textarea {
    resize: vertical;
    line-height: 1.6;
  }
  .body-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .tokens {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }
  .token-btn {
    padding: 4px 8px;
    font-size: var(--fs-xs);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      color var(--transition-fast),
      border-color var(--transition-fast);
  }
  .token-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .rich-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--fs-sm);
    color: var(--text-muted);
    cursor: pointer;
  }
  .rich-toggle input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
  .rich-toolbar {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    width: fit-content;
  }
  .rich-toolbar button {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 32px;
    padding: 0 var(--space-2);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .rich-toolbar button:hover {
    background: var(--bg-elevated);
    color: var(--text);
  }
  /* Separates the character-formatting group (B/I/U) from the
     insert/clear group (link, clear formatting) within the one control. */
  .toolbar-divider {
    align-self: stretch;
    width: 1px;
    margin: var(--space-1) 2px;
    background: var(--border);
  }
  .rich-area {
    min-height: 160px;
    padding: var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: 0 var(--radius-md) var(--radius-md) var(--radius-md);
    color: var(--text);
    line-height: 1.6;
    overflow-y: auto;
    transition: border-color var(--transition-fast);
  }
  .rich-area:focus {
    outline: none;
    border-color: var(--accent);
  }
  .rich-area :global(a) {
    color: var(--accent);
  }
  .note-inline {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .preview-box {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .preview-text {
    padding: var(--space-3);
    background: var(--bg);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-muted);
  }
  .error {
    padding: var(--space-3);
    background: var(--accent-weak);
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
  }
  .spacer {
    flex: 1;
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
  .danger {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--danger);
  }
  .danger:hover {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border-color: var(--danger);
  }
</style>
