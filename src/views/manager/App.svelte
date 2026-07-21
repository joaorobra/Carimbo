<script lang="ts">
  import { snippetStore } from "../../lib/stores/snippets.svelte";
  import { settingsStore } from "../../lib/stores/settings.svelte";
  import { i18n, t } from "../../lib/i18n";
  import FolderTree from "./FolderTree.svelte";
  import SnippetList from "./SnippetList.svelte";
  import SnippetEditor from "./SnippetEditor.svelte";
  import ClipboardHistory from "./ClipboardHistory.svelte";
  import ColorTools from "./ColorTools.svelte";
  import Settings from "./Settings.svelte";
  import Icon from "../../components/Icon.svelte";
  import Toasts from "../../components/Toasts.svelte";
  import Onboarding from "../../components/Onboarding.svelte";
  import RegionPicker from "../../components/RegionPicker.svelte";
  import TitleBar from "./TitleBar.svelte";
  import { api } from "../../lib/api";
  import { listen } from "@tauri-apps/api/event";

  type View = "snippets" | "clipboard" | "colors" | "settings";
  let view = $state<View>("snippets");
  let creating = $state(false);
  let list: SnippetList | undefined = $state();

  // A color picked via the global color hotkey. When it arrives the manager is
  // being shown; we switch to the Colors page and hand the color to ColorTools
  // (which loads it). Bumped each time so repeat picks of the same color still
  // re-trigger the load.
  let incomingColor = $state<{ r: number; g: number; b: number; seq: number } | null>(
    null,
  );

  $effect(() => {
    const un = listen<{ r: number; g: number; b: number } | null>(
      "colors:open",
      (e) => {
        view = "colors";
        if (e.payload) {
          incomingColor = { ...e.payload, seq: (incomingColor?.seq ?? 0) + 1 };
        }
      },
    );
    return () => {
      un.then((f) => f());
    };
  });

  // First run flows through two gates, checked once after the stores load:
  //   1. Region picker — until the user picks a region (`region.chosen`), so we
  //      seed the right examples and date format before anything else.
  //   2. Onboarding — only if, after that, the library is still empty (e.g. the
  //      user cleared the seeded examples), as a create-your-first fallback.
  let showRegionPicker = $state(false);
  let showOnboarding = $state(false);
  let firstRunChecked = $state(false);

  // Initialize stores once on mount, then decide about the first-run gates.
  $effect(() => {
    settingsStore.init();
    i18n.init();
    snippetStore.init().then(async () => {
      if (firstRunChecked) return;
      firstRunChecked = true;
      const all = await api.getAllSettings();
      if (all["region.chosen"] !== true) {
        showRegionPicker = true;
        return;
      }
      const dismissed = all["onboarding.dismissed"] === true;
      if (!dismissed && snippetStore.snippets.length === 0) {
        showOnboarding = true;
      }
    });
  });

  // The region picker seeds examples, so after it the library is normally
  // non-empty — only fall back to onboarding when it isn't.
  async function finishRegionPicker() {
    showRegionPicker = false;
    await snippetStore.refresh();
    const dismissed =
      (await api.getAllSettings())["onboarding.dismissed"] === true;
    if (!dismissed && snippetStore.snippets.length === 0) {
      showOnboarding = true;
    }
  }

  async function dismissOnboarding() {
    showOnboarding = false;
    await api.setSetting("onboarding.dismissed", true);
  }

  function newSnippet() {
    snippetStore.select(null);
    creating = true;
    view = "snippets";
  }

  function editorDone() {
    creating = false;
  }

  // When a snippet is picked from the list, leave create mode.
  $effect(() => {
    if (snippetStore.selectedId) creating = false;
  });

  const showEditor = $derived(creating || snippetStore.selected !== null);

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      e.preventDefault();
      newSnippet();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      view = "snippets";
      list?.focusSearch();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<Toasts />

{#if showRegionPicker}
  <RegionPicker onDone={finishRegionPicker} />
{:else if showOnboarding}
  <Onboarding onDismiss={dismissOnboarding} />
{/if}

<div class="root">
  <TitleBar />
  <div class="shell">
    <aside class="sidebar">
      <header class="brand">
        <span class="brand-mark"><Icon name="stamp" size={16} /></span>
        <span class="brand-label">{t("app.name")}</span>
      </header>
      <nav class="nav-main" aria-label={t("nav.sections")}>
        <button
          class="nav-item"
          class:active={view === "snippets"}
          onclick={() => (view = "snippets")}
        >
          <Icon name="snippet" size={18} />
          <span>{t("nav.snippets")}</span>
        </button>
        <button
          class="nav-item"
          class:active={view === "clipboard"}
          onclick={() => (view = "clipboard")}
        >
          <Icon name="clipboard" size={18} />
          <span>{t("nav.clipboard")}</span>
        </button>
        <button
          class="nav-item"
          class:active={view === "colors"}
          onclick={() => (view = "colors")}
        >
          <Icon name="pipette" size={18} />
          <span>{t("nav.colors")}</span>
        </button>
      </nav>
      {#if view === "snippets"}
        <div class="folders">
          <div class="section-label">{t("nav.folders")}</div>
          <FolderTree />
        </div>
      {/if}
      <footer class="sidebar-footer">
        <button
          class="nav-item"
          class:active={view === "settings"}
          onclick={() => (view = "settings")}
        >
          <Icon name="settings" size={16} />
          <span>{t("nav.settings")}</span>
        </button>
      </footer>
    </aside>

    <main class="content">
      {#if view === "settings"}
        <Settings />
      {:else if view === "clipboard"}
        <ClipboardHistory />
      {:else if view === "colors"}
        <ColorTools {incomingColor} />
      {:else}
        <div class="two-col">
          <SnippetList bind:this={list} onNew={newSnippet} />
          {#if showEditor}
            <SnippetEditor {creating} onDone={editorDone} />
          {:else}
            <div class="placeholder">
              <span class="placeholder-icon"><Icon name="snippet" size={44} /></span>
              <p class="placeholder-title">{t("snippets.placeholderSelect")}</p>
              <button class="placeholder-action" onclick={newSnippet}>
                <Icon name="plus" size={16} />
                <span>{t("snippets.createFirst")}</span>
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </main>
  </div>
</div>

<style>
  /* Titlebar (auto height) stacked over the shell (fills the rest). Root
     must stay in the unbroken height:100% chain from #app (see base.css)
     down to here, or the whole UI collapses to the titlebar's height. */
  .root {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    flex: 1;
    min-height: 0;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    min-height: 0;
  }
  /* The titlebar already carries the full wordmark, so the sidebar header
     stays a quiet, compact echo of it rather than repeating the logotype. */
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-4);
    margin-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
  }
  .brand-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
  }
  .brand-label {
    font-size: var(--fs-sm);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
    color: var(--text-muted);
  }
  .nav-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 var(--space-2);
  }
  .folders {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    margin-top: var(--space-4);
  }
  /* Small caps section header separates the folder list from the main nav. */
  .section-label {
    padding: 0 var(--space-3) var(--space-1);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  /* margin-top:auto pins the footer to the bottom in every view. The folder
     list only renders on the Snippets tab; without this the footer would float
     up directly under the nav on Clipboard/Colors/Settings, so Settings would
     sit at the window's bottom in one view but mid-sidebar in the others. */
  .sidebar-footer {
    margin-top: auto;
    padding: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--hit-target);
    padding: 0 var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    text-align: left;
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .nav-item :global(svg) {
    flex-shrink: 0;
  }
  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--accent-weak);
    color: var(--text);
    font-weight: 600;
  }
  /* Accent bar marks the active section unambiguously. */
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 18px;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    background: var(--accent);
  }
  .content {
    min-width: 0;
    overflow: hidden;
  }
  /* List is a fixed rail; the editor (or the empty-state placeholder) fills
     the rest. Same columns whether or not an editor is open, so the list
     doesn't jump width when a snippet is selected. */
  .two-col {
    display: grid;
    grid-template-columns: minmax(280px, 360px) 1fr;
    height: 100%;
    min-height: 0;
  }
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    height: 100%;
    color: var(--text-muted);
  }
  /* Icon sits in a soft raised well (elev-1) instead of floating bare, so the
     empty state reads as a composed moment rather than a placeholder. */
  .placeholder-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 72px;
    height: 72px;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    box-shadow: var(--elev-1);
    color: var(--accent);
    opacity: 0.9;
  }
  .placeholder-title {
    margin: 0;
    /* A secondary empty-state prompt — sized to read as guidance, not a hero
       headline (the old --fs-2xl sprawled across the pane and broke mid-phrase).
       Constrained + balanced so it wraps on a natural break. */
    max-width: 24ch;
    font-size: var(--fs-lg);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    text-align: center;
    text-wrap: balance;
    color: var(--text);
  }
  .placeholder-action {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    padding: 0 var(--space-4);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    transition: filter var(--transition-fast);
  }
  .placeholder-action:hover {
    filter: brightness(1.08);
  }
</style>
