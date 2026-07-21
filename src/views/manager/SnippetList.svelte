<script lang="ts">
  import { snippetStore } from "../../lib/stores/snippets.svelte";
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n";
  import Icon from "../../components/Icon.svelte";
  import type { Snippet } from "../../lib/types";

  interface Props {
    onNew: () => void;
  }
  let { onNew }: Props = $props();

  let searchEl: HTMLInputElement | undefined = $state();

  async function onSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    await snippetStore.setQuery(value);
  }

  async function toggleFav(e: MouseEvent, s: Snippet) {
    e.stopPropagation();
    await api.setFavorite(s.id, !s.isFavorite);
    // Refresh explicitly rather than waiting on the snippets:changed round-trip,
    // so the star flips immediately (same pattern the editor/folder tree use).
    await snippetStore.refreshSnippets();
  }

  // Expose focus to the parent shell for Ctrl+F.
  export function focusSearch() {
    searchEl?.focus();
    searchEl?.select();
  }

  function preview(body: string): string {
    const firstLine = body.split("\n")[0];
    return firstLine.length > 80 ? firstLine.slice(0, 80) + "…" : firstLine;
  }
</script>

<div class="list-pane">
  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        bind:this={searchEl}
        type="search"
        placeholder={t("snippets.searchPlaceholder")}
        aria-label={t("snippets.searchAria")}
        value={snippetStore.query}
        oninput={onSearchInput}
        autocomplete="off"
        spellcheck="false"
      />
    </div>
    <button class="new-btn" onclick={onNew} title={t("snippets.newTitle")}>
      <Icon name="plus" size={18} />
    </button>
  </div>

  {#if snippetStore.visible.length === 0}
    <div class="empty">
      <Icon name="snippet" size={28} />
      {#if snippetStore.query}
        <p class="empty-title">{t("snippets.emptyNoResults", { query: snippetStore.query })}</p>
        <p class="empty-hint">{t("palette.emptyNoResultsHint")}</p>
      {:else}
        <p class="empty-title">{t("snippets.emptyNone")}</p>
        <p class="empty-hint">{t("snippets.placeholderSelect")}</p>
        <button class="empty-cta" onclick={onNew}>{t("snippets.createFirst")}</button>
      {/if}
    </div>
  {:else}
    <ul class="items" role="listbox" aria-label={t("nav.snippets")}>
      {#each snippetStore.visible as s, i (s.id)}
        <li class="row" style={`animation-delay: calc(var(--stagger) * ${Math.min(i, 12)})`}>
          <div
            class="item"
            class:selected={snippetStore.selectedId === s.id}
            role="option"
            aria-selected={snippetStore.selectedId === s.id}
            tabindex="0"
            onclick={() => snippetStore.select(s.id)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                snippetStore.select(s.id);
              }
            }}
          >
            <button
              class="fav"
              class:on={s.isFavorite}
              title={s.isFavorite ? t("snippets.favRemove") : t("snippets.favAdd")}
              aria-label={s.isFavorite ? t("snippets.favRemove") : t("snippets.favAdd")}
              onclick={(e) => toggleFav(e, s)}
            >
              <Icon name={s.isFavorite ? "star-filled" : "star"} size={16} />
            </button>
            <div class="body">
              <div class="name-row">
                <span class="name">{s.name}</span>
                {#if s.trigger}<span class="trigger">{s.trigger}</span>{/if}
              </div>
              <span class="preview">{preview(s.body)}</span>
            </div>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .list-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-right: 1px solid var(--border);
    min-width: 0;
  }
  .toolbar {
    display: flex;
    align-items: stretch;
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    min-width: 0;
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
  /* Input owns focus; the ring lives on the wrapper, so suppress the default. */
  .search input:focus-visible {
    outline: none;
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
  .new-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--hit-target);
    /* stretch to the search field's height (which the border makes ~2px
       taller than --hit-target) so their edges line up exactly. */
    align-self: stretch;
    min-height: var(--hit-target);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    transition: filter var(--transition-fast);
  }
  .new-btn:hover {
    filter: brightness(1.08);
  }
  .new-btn:active {
    filter: brightness(0.94);
  }
  .items {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    overflow-y: auto;
    flex: 1;
  }
  /* Subtle staggered entrance: each row fades/slides in slightly after the
     previous one (delay set inline, capped at 12 rows so a long list doesn't
     take forever to finish settling). Neutralized globally under
     prefers-reduced-motion / data-motion="reduce" (see base.css). */
  .row {
    animation: row-in var(--transition-slow) backwards;
  }
  @keyframes row-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .item {
    position: relative;
    display: flex;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    text-align: left;
    color: var(--text);
    cursor: pointer;
    transition: background var(--transition-fast);
  }
  .item:hover {
    background: var(--bg-hover);
  }
  .item.selected {
    background: var(--accent-weak);
  }
  /* Accent edge on the selected snippet — mirrors the sidebar's active marker. */
  .item.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: var(--space-2);
    bottom: var(--space-2);
    width: 3px;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    background: var(--accent);
  }
  .fav {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--hit-target);
    height: var(--hit-target);
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      color var(--transition-fast),
      background var(--transition-fast);
  }
  .fav:hover {
    background: var(--bg-elevated);
    color: var(--text);
  }
  .fav.on {
    color: var(--star);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .name-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trigger {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    padding: 1px 6px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }
  .preview {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-4);
    color: var(--text-muted);
    text-align: center;
  }
  /* The glyph sits above the copy as a quiet anchor rather than an
     illustration — muted so it doesn't compete with the title. */
  .empty :global(svg) {
    color: var(--text-muted);
    opacity: 0.6;
    margin-bottom: var(--space-1);
  }
  .empty-title {
    margin: 0;
    max-width: 30ch;
    font-size: var(--fs-2xl);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    color: var(--text);
  }
  .empty-hint {
    margin: 0;
    max-width: 32ch;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .empty-cta {
    margin-top: var(--space-2);
    min-height: var(--hit-target);
    padding: var(--space-2) var(--space-4);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    transition: filter var(--transition-fast);
  }
  .empty-cta:hover {
    filter: brightness(1.08);
  }
</style>
