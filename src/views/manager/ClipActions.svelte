<script lang="ts">
  import { api } from "../../lib/api";
  import { clipboardStore } from "../../lib/stores/clipboard.svelte";
  import { contentTypeMeta, TRANSFORMS } from "../../lib/clips";
  import { t } from "../../lib/i18n";
  import { emit } from "@tauri-apps/api/event";
  import Icon from "../../components/Icon.svelte";
  import type { ClipEntry } from "../../lib/types";

  interface Props {
    entry: ClipEntry;
  }
  let { entry }: Props = $props();

  const meta = $derived(contentTypeMeta(entry.contentType));
  const isText = $derived(entry.kind === "text");
  const isImage = $derived(entry.kind === "image");

  // Which popover is open ("copy" transforms, "folder", or none). Only one at a
  // time; clicking elsewhere closes it.
  let open = $state<"copy" | "folder" | null>(null);
  let copiedKind = $state<string | null>(null);

  function toggle(which: "copy" | "folder") {
    open = open === which ? null : which;
  }
  function close() {
    open = null;
  }

  async function primaryAction() {
    if (meta.action === "open") await api.openClipUrl(entry.id);
    else if (meta.action === "reveal") await api.revealClipPath(entry.id);
  }

  async function copyAs(kind: (typeof TRANSFORMS)[number]["kind"]) {
    await clipboardStore.copy(entry.id, kind);
    copiedKind = kind;
    close();
    setTimeout(() => {
      if (copiedKind === kind) copiedKind = null;
    }, 1200);
  }

  // Image copy: place the PNG back on the clipboard (no transform).
  let imageCopied = $state(false);
  async function copyImage() {
    await clipboardStore.copy(entry.id);
    imageCopied = true;
    setTimeout(() => (imageCopied = false), 1200);
  }

  async function revealImage() {
    await api.revealClipImage(entry.id);
  }

  async function promote() {
    const name = await clipboardStore.promote(entry.id);
    await emit("carimbo:toast", t("clipboard.promoted", { name }));
  }

  async function moveTo(folderId: string | null) {
    await clipboardStore.setFolder(entry.id, folderId);
    close();
  }
</script>

<svelte:window onclick={close} />

<!-- Stop propagation so clicks inside the cluster don't trigger the window
     close-on-click (which would dismiss a just-opened popover). -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="actions" onclick={(e) => e.stopPropagation()}>
  {#if meta.action}
    <button
      class="act"
      title={meta.action === "open"
        ? entry.contentType === "email"
          ? t("clipboard.email")
          : t("clipboard.openUrl")
        : t("clipboard.reveal")}
      aria-label={meta.action === "open" ? t("clipboard.openUrlAria") : t("clipboard.revealAria")}
      onclick={primaryAction}
    >
      <Icon name={meta.icon ?? "external-link"} size={16} />
    </button>
  {/if}

  {#if isImage}
    <!-- Copy image back to the clipboard -->
    <button
      class="act"
      title={t("clipboard.copy")}
      aria-label={t("clipboard.copyAria")}
      onclick={copyImage}
    >
      <Icon name={imageCopied ? "check" : "copy"} size={16} />
    </button>

    <!-- Reveal the backing PNG in Explorer (save/reuse the file) -->
    <button
      class="act"
      title={t("lightbox.save")}
      aria-label={t("lightbox.save")}
      onclick={revealImage}
    >
      <Icon name="download" size={16} />
    </button>
  {/if}

  {#if isText}
    <!-- Copy-as (transform) menu -->
    <div class="menu-wrap">
      <button
        class="act"
        class:on={open === "copy"}
        title={t("clipboard.copyAs")}
        aria-label={t("clipboard.transformAria")}
        aria-haspopup="menu"
        aria-expanded={open === "copy"}
        onclick={() => toggle("copy")}
      >
        <Icon name={copiedKind ? "check" : "wand"} size={16} />
      </button>
      {#if open === "copy"}
        <div class="menu" role="menu">
          <p class="menu-head">{t("clipboard.copyAs")}</p>
          {#each TRANSFORMS as tr (tr.kind)}
            <button class="menu-item" role="menuitem" onclick={() => copyAs(tr.kind)}>
              {t(tr.labelKey)}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Save as snippet -->
    <button
      class="act"
      title={t("clipboard.promote")}
      aria-label={t("clipboard.promoteAria")}
      onclick={promote}
    >
      <Icon name="file-plus" size={16} />
    </button>
  {/if}

  <!-- Move to folder (both text and image clips; filing exempts from retention) -->
  <div class="menu-wrap">
    <button
      class="act"
      class:on={open === "folder"}
      title={t("clipboard.moveToFolder")}
      aria-label={t("clipboard.moveToFolder")}
      aria-haspopup="menu"
      aria-expanded={open === "folder"}
      onclick={() => toggle("folder")}
    >
      <Icon name="folder-input" size={16} />
    </button>
    {#if open === "folder"}
      <div class="menu" role="menu">
        <p class="menu-head">{t("clipboard.moveToFolder")}</p>
        <button
          class="menu-item"
          class:sel={entry.folderId === null}
          role="menuitemradio"
          aria-checked={entry.folderId === null}
          onclick={() => moveTo(null)}
        >
          {t("clipboard.removeFromFolder")}
        </button>
        {#each clipboardStore.folders as f (f.id)}
          <button
            class="menu-item"
            class:sel={entry.folderId === f.id}
            role="menuitemradio"
            aria-checked={entry.folderId === f.id}
            onclick={() => moveTo(f.id)}
          >
            {f.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <button
    class="act"
    class:on={entry.isPinned}
    title={entry.isPinned ? t("clipboard.unpin") : t("clipboard.pin")}
    aria-label={entry.isPinned ? t("clipboard.unpin") : t("clipboard.pin")}
    onclick={() => clipboardStore.togglePin(entry)}
  >
    <Icon name={entry.isPinned ? "star-filled" : "star"} size={16} />
  </button>

  <button
    class="act danger"
    title={t("clipboard.delete")}
    aria-label={t("clipboard.deleteAria")}
    onclick={() => clipboardStore.remove(entry.id)}
  >
    <Icon name="trash" size={16} />
  </button>
</div>

<style>
  .actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .menu-wrap {
    position: relative;
  }
  .act {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--hit-target);
    height: var(--hit-target);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .act:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .act.on {
    color: var(--star);
  }
  .act.danger:hover {
    color: var(--danger);
  }
  /* Popover anchored to its trigger; sits above the row on the trailing edge. */
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 20;
    min-width: 168px;
    padding: var(--space-1);
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .menu-head {
    margin: 0;
    padding: var(--space-1) var(--space-2);
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .menu-item {
    text-align: left;
    padding: var(--space-2);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background var(--transition-fast);
  }
  .menu-item:hover {
    background: var(--accent-weak);
  }
  .menu-item.sel {
    color: var(--accent);
    font-weight: 600;
  }
</style>
