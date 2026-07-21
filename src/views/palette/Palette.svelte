<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { api } from "../../lib/api";
  import { settingsStore } from "../../lib/stores/settings.svelte";
  import { i18n, t } from "../../lib/i18n";
  import type { ClipEntry, Snippet, TransformKind, Variable } from "../../lib/types";
  import type { ClipAction } from "../../lib/clips";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import Icon from "../../components/Icon.svelte";
  import ResultRow from "./ResultRow.svelte";
  import ClipRow from "./ClipRow.svelte";
  import VariableForm from "./VariableForm.svelte";
  import ImageLightbox from "../../components/ImageLightbox.svelte";

  type Tab = "snippets" | "clipboard";
  type Mode = "list" | "form";

  let mode = $state<Mode>("list");
  let tab = $state<Tab>("snippets");
  let query = $state("");
  let snippets = $state<Snippet[]>([]);
  let clips = $state<ClipEntry[]>([]);
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let listId = "palette-list";

  // The snippet + its variables being filled in the form step. `formFromTrigger`
  // records that we arrived via a typed trigger (no list behind us), so the
  // form's back/Esc closes the palette instead of returning to the list.
  let formSnippet = $state<Snippet | null>(null);
  let formVariables = $state<Variable[]>([]);
  let formFromTrigger = $state(false);

  // The image clip open in the lightbox (null = closed).
  let preview = $state<ClipEntry | null>(null);

  // A transient inline error (e.g. an insert that failed). Cleared on the next
  // action so it never lingers past its cause.
  let error = $state<string | null>(null);
  // Index of the row currently flashing "chosen" (accent flash before the
  // window hides), or null. Gives instant feedback that a pick registered.
  let firing = $state<number | null>(null);
  // Index of the row whose "more options" (paste-as…) menu is open, or null.
  // Owned here so only one menu opens at a time and it closes on nav/tab change.
  let menuAt = $state<number | null>(null);
  // Bumped on every `palette:open` so `{#key openSeq}` remounts `.palette-card`,
  // re-triggering its scale+opacity entrance animation each time the window is
  // shown (a plain CSS animation only plays once per mount otherwise).
  let openSeq = $state(0);
  // Bumped on every tab switch so a keyed wrapper around the results list
  // remounts, replaying a quick crossfade instead of a hard cut.
  let tabSeq = $state(0);

  // Light grouping: with no active search on the snippets tab, float favorites
  // to the top under a divider so they're always a glance away. This re-sorts
  // the same already-loaded `snippets` array (no API/store change). `results`
  // (below) is the SINGLE source of render order — `active`/`palette-opt-i`/
  // aria-activedescendant/choose() all index into it, so grouping can never
  // desync the list from the keyboard cursor the way two independent `{#each}`
  // counters could.
  const groupSnippets = $derived(tab === "snippets" && !query.trim());
  const orderedSnippets = $derived(
    groupSnippets
      ? [
          ...snippets.filter((s) => s.isFavorite),
          ...snippets.filter((s) => !s.isFavorite),
        ]
      : snippets,
  );
  // Section headers are shown only for a genuine mix of favorite + non-favorite
  // snippets: "Favorites" above index 0, "Others" above the first non-favorite
  // (index favCount). An all-one-group list gets no headers — a lone header over
  // the whole list is just noise.
  const favCount = $derived(
    groupSnippets ? snippets.filter((s) => s.isFavorite).length : 0,
  );
  const showGroups = $derived(favCount > 0 && favCount < snippets.length);

  const results = $derived<(Snippet | ClipEntry)[]>(
    tab === "snippets" ? orderedSnippets : clips,
  );

  // Load appearance + language so the palette matches the manager.
  $effect(() => {
    settingsStore.init();
    i18n.init();
  });

  async function refresh() {
    const q = query.trim();
    if (tab === "snippets") {
      snippets = q ? await api.searchSnippets(q) : await api.listSnippets();
    } else {
      clips = q ? await api.searchClips(q) : await api.listClips();
    }
    active = 0;
  }

  // Reset + reload whenever the palette is opened via the hotkey/tray. The
  // payload names which tab to open ("snippets" | "clipboard"); the primary
  // hotkey sends the user's main tab, the secondary sends the other one. A
  // missing/unknown payload keeps the default (snippets).
  $effect(() => {
    const un = listen<Tab | null>("palette:open", async (e) => {
      mode = "list";
      tab = e.payload === "clipboard" ? "clipboard" : "snippets";
      query = "";
      preview = null;
      error = null;
      firing = null;
      menuAt = null;
      openSeq++;
      await refresh();
      requestAnimationFrame(() => input?.focus());
    });
    return () => {
      un.then((f) => f());
    };
  });

  // Opened straight into the fill-in form by a typed trigger whose snippet has
  // `[[variables]]`. The event carries the snippet id; we load it and its
  // variables and jump to the form step (backend already captured the target
  // window + how many trigger chars to delete on insert).
  $effect(() => {
    const un = listen<string>("palette:open-form", async (e) => {
      const id = e.payload;
      try {
        const [snippet, vars] = await Promise.all([
          api.getSnippet(id),
          api.paletteVariables(id),
        ]);
        if (vars.length === 0) {
          // No variables after all (e.g. edited between fire and open): just
          // insert directly rather than showing an empty form.
          await api.paletteInsert(id);
          return;
        }
        formSnippet = snippet;
        formVariables = vars;
        formFromTrigger = true;
        mode = "form";
      } catch {
        // Snippet vanished; close rather than stranding a broken form.
        await hide();
      }
    });
    return () => {
      un.then((f) => f());
    };
  });

  // Initial load exactly once (in case the window is shown before the first
  // palette:open event). Guarded so it never re-runs on query/state changes.
  let didInit = false;
  $effect(() => {
    if (didInit) return;
    didInit = true;
    refresh();
    input?.focus();
  });

  async function onInput(e: Event) {
    query = (e.target as HTMLInputElement).value;
    error = null;
    await refresh();
  }

  async function switchTab(next: Tab) {
    if (next === tab) return;
    tab = next;
    active = 0;
    menuAt = null;
    error = null;
    tabSeq++;
    await refresh();
    input?.focus();
  }

  async function hide() {
    await api.paletteHide();
  }

  // --- selection -----------------------------------------------------------

  async function chooseSnippet(s: Snippet) {
    // If the snippet has form variables, collect them before inserting.
    const vars = await api.paletteVariables(s.id);
    if (vars.length > 0) {
      formSnippet = s;
      formVariables = vars;
      formFromTrigger = false; // came from the list; back returns to it
      mode = "form";
      return;
    }
    await api.paletteInsert(s.id);
  }

  async function chooseClip(c: ClipEntry) {
    // Text clips paste their content; image clips paste the backing PNG as an
    // image. Both flow through the same backend command.
    await api.pasteClip(c.id);
  }

  async function choose(index: number) {
    const item = results[index];
    if (!item) {
      await hide();
      return;
    }
    error = null;
    // Flash the chosen row so the pick registers visually before the window
    // hides (insert/paste hides it backend-side on success).
    firing = index;
    try {
      if (tab === "snippets") await chooseSnippet(item as Snippet);
      else await chooseClip(item as ClipEntry);
    } catch {
      // Insert/paste failed and the window is still up: explain, don't strand.
      firing = null;
      error = t("palette.insertError");
    }
  }

  // Insert/paste the row at `index` with a "paste as…" transform chosen from its
  // more-options menu. Snippets route through paletteInsert (skipping the
  // fill-in form: a transform on the whole result makes per-variable prompts
  // moot); clips route through pasteClip. Same firing-flash + error handling as
  // `choose`.
  async function chooseTransformed(index: number, kind: TransformKind) {
    const item = results[index];
    if (!item) return;
    menuAt = null;
    error = null;
    firing = index;
    try {
      if (tab === "snippets") {
        await api.paletteInsert((item as Snippet).id, undefined, kind);
      } else {
        await api.pasteClip((item as ClipEntry).id, kind);
      }
    } catch {
      firing = null;
      error = t("palette.insertError");
    }
  }

  // Run a content-aware action chosen from a clip row's more-options menu:
  // open a link in the browser, reveal a path in Explorer, or paste a color
  // reformatted as HEX/RGB/HSL. Paste-style actions flash the row and hide the
  // palette (backend-side) exactly like a normal pick; open/reveal actions keep
  // the palette up so the user can keep working.
  async function runClipAction(index: number, action: ClipAction) {
    const c = results[index] as ClipEntry | undefined;
    if (!c) return;
    menuAt = null;
    error = null;
    try {
      switch (action.run.t) {
        case "openUrl":
          await api.openClipUrl(c.id);
          await hide();
          break;
        case "revealPath":
          await api.revealClipPath(c.id);
          await hide();
          break;
        case "pasteText":
          firing = index;
          await api.pasteClipText(action.run.text);
          break;
        case "soon":
          // Deferred (e.g. image editing) — the menu already renders it inert.
          break;
      }
    } catch {
      firing = null;
      error = t("palette.insertError");
    }
  }

  // Mouse hover activates a row. If a different row's more-options menu is open,
  // close it — its trigger hides on non-active rows, so leaving it open reads as
  // an orphaned popover.
  function hoverRow(index: number) {
    active = index;
    if (menuAt !== null && menuAt !== index) menuAt = null;
  }

  async function togglePin(c: ClipEntry) {
    await api.setClipPinned(c.id, !c.isPinned);
    await refresh();
  }

  // Promote a text clip into a reusable snippet without leaving the palette.
  async function saveClipAsSnippet(c: ClipEntry) {
    try {
      await api.promoteClipToSnippet(c.id);
    } catch {
      // Non-fatal: the row's inline check already fired; a failure here just
      // means no snippet was created. Nothing to surface in the compact palette.
    }
  }

  // --- form step -----------------------------------------------------------

  async function submitForm(values: Record<string, string>) {
    if (!formSnippet) return;
    await api.paletteInsert(formSnippet.id, values);
  }

  function backToList() {
    // A trigger-opened form has no list behind it — cancelling closes the
    // palette (which also clears the pending trigger-delete in the backend).
    if (formFromTrigger) {
      hide();
      return;
    }
    mode = "list";
    formSnippet = null;
    formVariables = [];
    requestAnimationFrame(() => input?.focus());
  }

  // --- keyboard ------------------------------------------------------------

  function onKeydown(e: KeyboardEvent) {
    // While the lightbox is open it owns the keyboard (its own Esc closes it).
    if (preview) return;
    // An open more-options menu intercepts Escape (close the menu, keep the
    // palette) and closes on any navigation so it never lingers over a new row.
    if (menuAt !== null) {
      if (e.key === "Escape") {
        e.preventDefault();
        menuAt = null;
        return;
      }
      if (e.key !== "Tab") menuAt = null;
    }
    // Tab / Ctrl+Tab cycles the two tabs from the search box.
    if (e.key === "Tab") {
      e.preventDefault();
      switchTab(tab === "snippets" ? "clipboard" : "snippets");
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (results.length) active = (active + 1) % results.length;
        break;
      case "ArrowUp":
        e.preventDefault();
        if (results.length)
          active = (active - 1 + results.length) % results.length;
        break;
      case "PageDown":
        e.preventDefault();
        active = Math.min(active + 5, results.length - 1);
        break;
      case "PageUp":
        e.preventDefault();
        active = Math.max(active - 5, 0);
        break;
      case "Home":
        e.preventDefault();
        active = 0;
        break;
      case "End":
        e.preventDefault();
        active = results.length - 1;
        break;
      case "Enter":
        e.preventDefault();
        choose(active);
        break;
      case "Escape":
        e.preventDefault();
        hide();
        break;
    }
  }

  // Keep the active row scrolled into view.
  $effect(() => {
    if (mode !== "list") return;
    const el = document.getElementById(`palette-opt-${active}`);
    el?.scrollIntoView({ block: "nearest" });
  });

  const emptyText = $derived(
    tab === "snippets"
      ? query
        ? t("palette.emptyNoSnippets")
        : t("palette.emptyNoSnippetsYet")
      : query
        ? t("palette.emptyNoClips")
        : t("palette.emptyNoClipsYet"),
  );

  // Second line under the empty title: a concrete next step, not a dead end.
  const emptyHint = $derived(
    query
      ? t("palette.emptyNoResultsHint")
      : tab === "snippets"
        ? t("palette.emptyNoSnippetsHint")
        : t("palette.emptyNoClipsHint"),
  );

  const emptyIcon = $derived(tab === "snippets" ? "snippet" : "clipboard");
</script>

{#key openSeq}
<div class="palette-card">
  {#if mode === "form" && formSnippet}
    <!-- Key by snippet id so the form re-mounts per pick: its value map is
         seeded at construction and the autofocus effect re-runs. -->
    {#key formSnippet.id}
      <VariableForm
        snippet={formSnippet}
        variables={formVariables}
        onSubmit={submitForm}
        onBack={backToList}
      />
    {/key}
  {:else}
    <div class="search-bar">
      <!-- Inset "well" gives the command bar a defined boundary instead of a
           borderless input; the stamp glyph reads as the app's own mark
           (distinct from the plain search icon used elsewhere, e.g. empty
           state) so this reads as a command bar, not a generic text field. -->
      <div class="search-well">
        <span class="search-glyph" aria-hidden="true"><Icon name="stamp" size={18} /></span>
        <!-- ARIA combobox pattern: input owns focus, activedescendant tracks row -->
        <input
          bind:this={input}
          bind:value={query}
          class="search"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-activedescendant={results.length ? `palette-opt-${active}` : undefined}
          aria-label={tab === "snippets" ? t("palette.searchSnippet") : t("palette.searchClipboard")}
          placeholder={tab === "snippets" ? t("palette.searchSnippetPlaceholder") : t("palette.searchClipboardPlaceholder")}
          autocomplete="off"
          spellcheck="false"
          oninput={onInput}
          onkeydown={onKeydown}
        />
      </div>
    </div>

    <div class="tabs" role="tablist" aria-label={t("nav.sections")}>
      <button
        class="tab"
        class:active={tab === "snippets"}
        role="tab"
        aria-selected={tab === "snippets"}
        tabindex="-1"
        onclick={() => switchTab("snippets")}
      >
        <Icon name="snippet" size={16} />
        <span>{t("nav.snippets")}</span>
        {#if tab === "snippets" && snippets.length}
          <span class="tab-count">{snippets.length}</span>
        {/if}
      </button>
      <button
        class="tab"
        class:active={tab === "clipboard"}
        role="tab"
        aria-selected={tab === "clipboard"}
        tabindex="-1"
        onclick={() => switchTab("clipboard")}
      >
        <Icon name="clipboard" size={16} />
        <span>{t("nav.clipboard")}</span>
        {#if tab === "clipboard" && clips.length}
          <span class="tab-count">{clips.length}</span>
        {/if}
      </button>
    </div>

    <!-- Keyed on tabSeq (not query/results) so a tab switch replays a quick
         crossfade while typing in the same tab never re-triggers it. -->
    {#key tabSeq}
    <ul id={listId} class="results crossfade" role="listbox" aria-label={t("palette.results", { n: results.length, plural: results.length === 1 ? "" : "s" })}>
      {#if results.length === 0}
        <li class="empty" role="presentation">
          <span class="empty-icon"><Icon name={emptyIcon} size={28} /></span>
          <span class="empty-title">{emptyText}</span>
          <span class="empty-hint">{emptyHint}</span>
        </li>
      {:else if tab === "snippets"}
        {#each orderedSnippets as s, i (s.id)}
          {#if showGroups && i === 0}
            <li class="divider" role="presentation">{t("palette.favorites")}</li>
          {:else if showGroups && i === favCount}
            <li class="divider" role="presentation">{t("palette.otherSnippets")}</li>
          {/if}
          <ResultRow
            snippet={s}
            active={i === active}
            firing={i === firing}
            menuOpen={i === menuAt}
            {query}
            id={`palette-opt-${i}`}
            index={i}
            onSelect={() => choose(i)}
            onHover={() => hoverRow(i)}
            onMenuToggle={() => {
              active = i;
              menuAt = menuAt === i ? null : i;
            }}
            onMenuClose={() => (menuAt = null)}
            onTransform={(kind) => chooseTransformed(i, kind)}
          />
        {/each}
      {:else}
        {#each clips as c, i (c.id)}
          <ClipRow
            entry={c}
            active={i === active}
            firing={i === firing}
            menuOpen={i === menuAt}
            index={i}
            {query}
            id={`palette-opt-${i}`}
            onSelect={() => choose(i)}
            onHover={() => hoverRow(i)}
            onTogglePin={() => togglePin(c)}
            onSaveSnippet={() => saveClipAsSnippet(c)}
            onPreview={() => (preview = c)}
            onMenuToggle={() => {
              active = i;
              menuAt = menuAt === i ? null : i;
            }}
            onMenuClose={() => (menuAt = null)}
            onTransform={(kind) => chooseTransformed(i, kind)}
            onAction={(a) => runClipAction(i, a)}
          />
        {/each}
      {/if}
    </ul>
    {/key}

    {#if error}
      <div class="error-strip" role="alert">
        <Icon name="close" size={14} />
        <span>{error}</span>
      </div>
    {/if}

    <!-- Decluttered footer: the tab strip above already shows a live count pill,
         so this keeps only the two hints that matter on every open (move +
         act) rather than repeating Tab/Esc/count as well. Quieter type than
         before — it's a reference, not something to read every time. -->
    <div class="footer" aria-live="polite">
      <span class="hint"><kbd>↑</kbd><kbd>↓</kbd> {t("palette.navigate")}</span>
      <span class="hint"
        ><kbd>Enter</kbd> {tab === "snippets" ? t("palette.insert") : t("palette.paste")}</span
      >
    </div>
  {/if}

  {#if preview && preview.imagePath}
    <ImageLightbox
      src={convertFileSrc(preview.imagePath)}
      alt={preview.preview}
      onCopy={() => api.copyClip(preview!.id)}
      onPaste={() => {
        // Pasting hides the palette window backend-side; drop the overlay too.
        const c = preview!;
        preview = null;
        api.pasteClip(c.id);
      }}
      onReveal={() => api.revealClipImage(preview!.id)}
      onClose={() => (preview = null)}
    />
  {/if}
</div>
{/key}

<style>
  /* Premium glass material: this card IS the window (transparent + shadow:false
     in tauri.conf), so it owns all visible surface + depth. Blur/saturate reads
     as a real glass pane; --elev-3 is the "floating window" tier of the shared
     elevation scale. Falls back to a solid surface where backdrop-filter isn't
     supported (see @supports below); --surface-glass itself already collapses
     to solid Canvas under forced-colors, and --elev-3 to a 1px border ring, so
     no separate forced-colors override is needed here. */
  .palette-card {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface-glass);
    backdrop-filter: blur(20px) saturate(1.4);
    -webkit-backdrop-filter: blur(20px) saturate(1.4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--elev-3);
    overflow: hidden;
    /* Entrance: subtle scale+opacity pop on every {#key openSeq} remount (i.e.
       every palette:open). --ease-spring gives it a touch of overshoot so it
       reads as a deliberate "arrival" rather than a fade. The keyframe's own
       start state (not just the transition) carries the reduced-motion floor:
       under the global guards `animation-duration` collapses to ~0ms, so the
       animation lands on its 100% (fully visible) frame instantly. */
    animation: palette-in var(--transition-slow) var(--ease-spring) both;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .palette-card {
      background: var(--bg-elevated);
    }
  }
  @keyframes palette-in {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  .search-bar {
    padding: var(--space-3) var(--space-4);
  }
  /* Confident command-bar well: a real inset boundary (not a borderless input)
     so the search field reads with intent, like the rest of the card. */
  .search-well {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color var(--transition-fast);
  }
  .search-well:focus-within {
    border-color: var(--border-strong);
  }
  .search-glyph {
    display: flex;
    flex-shrink: 0;
    color: var(--accent);
  }
  .search {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-lg);
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
  }
  /* Explicit placeholder color so it's consistent across WebView2 themes. */
  .search::placeholder {
    color: var(--text-muted);
    opacity: 1;
  }
  /* Segmented tabs sit directly under the search box, above the results.
     Underline marks the active tab; the whole strip has one bottom border. */
  .tabs {
    display: flex;
    gap: var(--space-1);
    padding: 0 var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    font-weight: 600;
    transition:
      color var(--transition-fast),
      border-color var(--transition-fast);
  }
  .tab :global(svg) {
    flex-shrink: 0;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  /* Count pill on the active tab — tells the user the size of each list. */
  .tab-count {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--accent-weak);
    color: var(--accent);
    font-size: var(--fs-xs);
    font-weight: 700;
    line-height: 18px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .results {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    overflow-y: auto;
    flex: 1;
  }
  /* Quick crossfade on tab switch: {#key tabSeq} remounts this list, so the
     animation plays once per switch and lands fully opaque at rest. */
  .results.crossfade {
    animation: results-crossfade var(--transition) var(--ease-out) both;
  }
  @keyframes results-crossfade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  /* "Favorites" divider above the grouped favorite snippets (no active query).
     role="presentation" like .empty — it's a label, not a listbox option, so it
     never consumes an aria-activedescendant index. */
  .divider {
    padding: var(--space-2) var(--space-3) var(--space-1);
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-5);
    text-align: center;
    color: var(--text-muted);
  }
  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-lg);
    background: var(--bg-inset);
    color: var(--accent);
    box-shadow: var(--elev-1);
    margin-bottom: var(--space-1);
  }
  .empty-title {
    font-size: var(--fs-lg);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    color: var(--text);
  }
  .empty-hint {
    font-size: var(--fs-sm);
    max-width: 34ch;
  }
  /* Inline failure notice above the footer — transient, cleared on next action. */
  .error-strip {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .error-strip :global(svg) {
    flex-shrink: 0;
  }
  /* Decluttered + quieted: down from four hint groups + a duplicate count to
     the two that matter every time (move, act). Lower opacity than before so
     it reads as a passive reference rather than competing with the results. */
  .footer {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-muted);
    opacity: 0.7;
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 1px 5px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
</style>
