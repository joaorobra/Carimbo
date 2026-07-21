<script lang="ts">
  import { snippetStore } from "../../lib/stores/snippets.svelte";
  import { api } from "../../lib/api";
  import { isCommandError } from "../../lib/types";
  import { t } from "../../lib/i18n";
  import Icon from "../../components/Icon.svelte";

  let creating = $state(false);
  let newName = $state("");
  // Guards against the double-invoke when Enter sets `creating = false`, which
  // unmounts the input and fires its `onblur` handler mid-flight.
  let saving = $state(false);

  function countIn(folderId: string | null): number {
    if (folderId === null) return snippetStore.snippets.length;
    return snippetStore.snippets.filter((s) => s.folderId === folderId).length;
  }

  async function addFolder() {
    if (saving) return;
    const name = newName.trim();
    if (!name) {
      creating = false;
      return;
    }
    saving = true;
    try {
      await api.createFolder(name, "snippet", null);
      // Refresh explicitly rather than trusting the `folders:changed` event
      // round-trip — the same pattern the snippet editor uses. Without this the
      // new folder wouldn't appear until (and unless) the event arrived, which
      // is exactly the "new folders aren't saved" symptom users hit.
      await snippetStore.refreshFolders();
      newName = "";
      creating = false;
    } catch (e) {
      alert(isCommandError(e) ? e.message : t("folders.errorCreate"));
    } finally {
      saving = false;
    }
  }

  async function removeFolder(id: string, name: string) {
    if (!confirm(t("folders.confirmDelete", { name }))) return;
    await api.deleteFolder(id);
    // Refresh snippets too: they keep their folder_id when the folder is
    // soft-deleted, so the list/counts would otherwise show a stale folder
    // filter until the next reload.
    await Promise.all([
      snippetStore.refreshFolders(),
      snippetStore.refreshSnippets(),
    ]);
    if (snippetStore.activeFolderId === id) snippetStore.activeFolderId = null;
  }
</script>

<nav class="tree" aria-label={t("nav.folders")}>
  <button
    class="row"
    class:active={snippetStore.activeFolderId === null}
    onclick={() => (snippetStore.activeFolderId = null)}
  >
    <Icon name="snippet" size={16} />
    <span class="name">{t("folders.all")}</span>
    <span class="count">{countIn(null)}</span>
  </button>

  {#each snippetStore.folders as folder (folder.id)}
    <div class="row-wrap">
      <button
        class="row"
        class:active={snippetStore.activeFolderId === folder.id}
        onclick={() => (snippetStore.activeFolderId = folder.id)}
      >
        <Icon name="folder" size={16} />
        <span class="name">{folder.name}</span>
        <span class="count">{countIn(folder.id)}</span>
      </button>
      <button
        class="del"
        title={t("folders.deleteTitle")}
        aria-label={t("folders.deleteAria", { name: folder.name })}
        onclick={() => removeFolder(folder.id, folder.name)}
      >
        <Icon name="trash" size={14} />
      </button>
    </div>
  {/each}

  {#if creating}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="new-input"
      bind:value={newName}
      placeholder={t("folders.newPlaceholder")}
      autofocus
      onblur={addFolder}
      onkeydown={(e) => {
        if (e.key === "Enter") addFolder();
        if (e.key === "Escape") {
          creating = false;
          newName = "";
        }
      }}
    />
  {:else}
    <button class="add" onclick={() => (creating = true)}>
      <Icon name="plus" size={16} />
      <span>{t("folders.new")}</span>
    </button>
  {/if}
</nav>

<style>
  .tree {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
  }
  .row-wrap {
    display: flex;
    align-items: center;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    min-width: 0;
    min-height: var(--hit-target);
    padding: 0 var(--space-2);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    text-align: left;
    cursor: pointer;
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .row :global(svg) {
    flex-shrink: 0;
  }
  .row:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .row.active {
    background: var(--accent-weak);
    color: var(--text);
    font-weight: 600;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    flex-shrink: 0;
    min-width: 20px;
    padding: 1px 6px;
    text-align: center;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    background: var(--bg-inset);
    border-radius: 999px;
  }
  .row.active .count {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--text);
  }
  .row:hover .count {
    background: var(--bg-elevated);
  }
  .del {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--hit-target);
    height: var(--hit-target);
    background: transparent;
    border: none;
    color: var(--text-muted);
    opacity: 0;
    border-radius: var(--radius-sm);
    transition:
      opacity var(--transition-fast),
      color var(--transition-fast);
  }
  .row-wrap:hover .del,
  .del:focus-visible {
    opacity: 1;
  }
  .del:hover {
    color: var(--danger);
  }
  .add,
  .new-input {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    margin-top: var(--space-1);
    padding: 0 var(--space-2);
    border-radius: var(--radius-md);
    border: 1px dashed var(--border-strong);
    background: transparent;
    color: var(--text-muted);
  }
  .add {
    cursor: pointer;
    transition:
      color var(--transition-fast),
      border-color var(--transition-fast);
  }
  .add:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .new-input {
    border-style: solid;
    color: var(--text);
    background: var(--bg-elevated);
  }
</style>
