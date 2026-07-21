<script lang="ts">
  import { clipboardStore } from "../../lib/stores/clipboard.svelte";
  import { api } from "../../lib/api";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { contentTypeMeta, hexColor } from "../../lib/clips";
  import { t } from "../../lib/i18n";
  import { timeAgo } from "../../lib/time";
  import Icon from "../../components/Icon.svelte";
  import ClipActions from "./ClipActions.svelte";
  import ImageLightbox from "../../components/ImageLightbox.svelte";
  import type { ClipEntry } from "../../lib/types";

  $effect(() => {
    clipboardStore.init();
  });

  // The image clip currently open in the lightbox (null = closed).
  let preview = $state<ClipEntry | null>(null);

  let creatingFolder = $state(false);
  let newFolderName = $state("");

  async function onSearch(e: Event) {
    await clipboardStore.setQuery((e.target as HTMLInputElement).value);
  }

  async function addFolder() {
    const name = newFolderName.trim();
    if (!name) {
      creatingFolder = false;
      return;
    }
    await clipboardStore.createFolder(name);
    newFolderName = "";
    creatingFolder = false;
  }
</script>

<div class="history">
  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        type="search"
        placeholder={t("clipboard.searchPlaceholder")}
        aria-label={t("clipboard.searchAria")}
        value={clipboardStore.query}
        oninput={onSearch}
        autocomplete="off"
      />
    </div>
  </div>

  <!-- Folder filter rail: All + each clipboard folder + create. Filed clips are
       exempt from retention, so folders double as "keep these". -->
  <div class="folders" role="tablist" aria-label={t("nav.folders")}>
    <button
      class="chip"
      class:active={clipboardStore.activeFolderId === null}
      role="tab"
      aria-selected={clipboardStore.activeFolderId === null}
      onclick={() => clipboardStore.setActiveFolder(null)}
    >
      {t("clipboard.allFolders")}
    </button>
    {#each clipboardStore.folders as f (f.id)}
      <button
        class="chip"
        class:active={clipboardStore.activeFolderId === f.id}
        role="tab"
        aria-selected={clipboardStore.activeFolderId === f.id}
        onclick={() => clipboardStore.setActiveFolder(f.id)}
      >
        <Icon name="folder" size={13} />
        <span>{f.name}</span>
      </button>
    {/each}
    {#if creatingFolder}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="folder-input"
        placeholder={t("clipboard.folderPlaceholder")}
        bind:value={newFolderName}
        onblur={addFolder}
        onkeydown={(e) => {
          if (e.key === "Enter") addFolder();
          else if (e.key === "Escape") {
            creatingFolder = false;
            newFolderName = "";
          }
        }}
        autofocus
      />
    {:else}
      <button
        class="chip add"
        title={t("clipboard.newFolder")}
        aria-label={t("clipboard.newFolder")}
        onclick={() => (creatingFolder = true)}
      >
        <Icon name="plus" size={14} />
      </button>
    {/if}
  </div>

  {#if clipboardStore.visible.length === 0}
    <div class="empty">
      <Icon name="clipboard" size={40} />
      <p>
        {clipboardStore.query
          ? t("clipboard.emptyNoResults")
          : t("clipboard.emptyNone")}
      </p>
    </div>
  {:else}
    <ul class="items">
      {#each clipboardStore.visible as entry (entry.id)}
        {@const meta = contentTypeMeta(entry.contentType)}
        {@const swatch = entry.contentType === "color" ? hexColor(entry.content) : null}
        <li class="item" class:pinned={entry.isPinned}>
          <div class="content">
            {#if entry.kind === "image" && entry.imagePath}
              <button
                class="thumb-btn"
                title={t("lightbox.open")}
                aria-label={t("lightbox.open")}
                onclick={() => (preview = entry)}
              >
                <img
                  class="thumb"
                  src={convertFileSrc(entry.imagePath)}
                  alt={entry.preview}
                  loading="lazy"
                />
                <span class="thumb-zoom" aria-hidden="true">
                  <Icon name="maximize" size={14} />
                </span>
              </button>
            {:else if swatch}
              <span class="swatch" style="background:{swatch}" aria-hidden="true"></span>
              <p class="text mono">{entry.preview}</p>
            {:else}
              <p class="text">{entry.preview}</p>
            {/if}

            <div class="tags">
              {#if meta.badgeKey}
                <span class="badge badge-{entry.contentType}">{t(meta.badgeKey)}</span>
              {/if}
              {#if entry.sourceApp}
                <span class="src" title={t("clipboard.sourceApp", { app: entry.sourceApp })}>
                  {entry.sourceApp}
                </span>
              {/if}
              <span class="meta">{timeAgo(entry.createdAt)}</span>
            </div>
          </div>

          <ClipActions {entry} />
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if preview && preview.imagePath}
  <ImageLightbox
    src={convertFileSrc(preview.imagePath)}
    alt={preview.preview}
    onCopy={() => clipboardStore.copy(preview!.id)}
    onReveal={() => api.revealClipImage(preview!.id)}
    onClose={() => (preview = null)}
  />
{/if}

<style>
  .history {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .toolbar {
    padding: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    transition:
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    min-height: var(--hit-target);
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
  }
  /* Horizontal, scrollable folder chips under the search box. */
  .folders {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    scrollbar-width: thin;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    padding: var(--space-1) var(--space-3);
    min-height: 28px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    font-weight: 500;
    transition:
      color var(--transition-fast),
      border-color var(--transition-fast),
      background var(--transition-fast);
  }
  .chip:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .chip.active {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-weak);
  }
  .chip.add {
    padding: var(--space-1);
    width: 28px;
    justify-content: center;
  }
  .folder-input {
    flex-shrink: 0;
    width: 140px;
    padding: var(--space-1) var(--space-3);
    min-height: 28px;
    background: var(--bg-inset);
    border: 1px solid var(--accent);
    border-radius: 999px;
    color: var(--text);
    font-size: var(--fs-sm);
    outline: none;
  }
  .items {
    list-style: none;
    margin: 0;
    padding: var(--space-2);
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    transition:
      border-color var(--transition-fast),
      background var(--transition-fast);
  }
  .item:hover {
    border-color: var(--border-strong);
  }
  .item.pinned {
    border-color: var(--accent);
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .text {
    margin: 0;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-sm);
  }
  .text.mono {
    font-family: var(--font-mono);
  }
  /* Clickable thumbnail opens the lightbox; a zoom glyph fades in on hover. */
  .thumb-btn {
    position: relative;
    display: inline-flex;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: zoom-in;
    line-height: 0;
  }
  .thumb {
    max-height: 48px;
    max-width: 120px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    transition: border-color var(--transition-fast);
  }
  .thumb-btn:hover .thumb,
  .thumb-btn:focus-visible .thumb {
    border-color: var(--accent);
  }
  .thumb-zoom {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--scrim) 55%, transparent);
    color: #fff;
    opacity: 0;
    transition: opacity var(--transition-fast);
  }
  .thumb-btn:hover .thumb-zoom,
  .thumb-btn:focus-visible .thumb-zoom {
    opacity: 1;
  }
  .swatch {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
  }
  /* Trailing cluster of metadata: type badge, source app, time. */
  .tags {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  .badge {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    background: var(--bg-inset);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .badge-url,
  .badge-email {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-weak);
  }
  .src {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }
  .meta {
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  /* Reveal the action cluster on hover/focus (mirrors the old behaviour). */
  .item :global(.actions) {
    opacity: 0;
    transition: opacity var(--transition-fast);
  }
  .item:hover :global(.actions),
  .item:focus-within :global(.actions),
  .item.pinned :global(.actions) {
    opacity: 1;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    flex: 1;
    padding: var(--space-6);
    color: var(--text-muted);
    text-align: center;
  }
</style>
