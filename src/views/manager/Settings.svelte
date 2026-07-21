<script lang="ts">
  import { settingsStore } from "../../lib/stores/settings.svelte";
  import { api } from "../../lib/api";
  import { Events, onEvent } from "../../lib/events";
  import { i18n, t, type Lang, type Region } from "../../lib/i18n";
  import {
    GLASS_OPACITY_MAX,
    GLASS_OPACITY_MIN,
    type Theme,
  } from "../../lib/theme";
  import type { PaletteTab } from "../../lib/types";
  import { isCommandError } from "../../lib/types";
  import HotkeyRecorder from "../../components/HotkeyRecorder.svelte";
  import Icon from "../../components/Icon.svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";

  // --- palette hotkeys (primary + optional secondary + which tab is "main") ---
  let hotkey = $state("");
  let hotkey2 = $state("");
  let mainTab = $state<PaletteTab>("snippets");
  let defaultHotkey = $state("Control+Shift+Space");
  let colorHotkey = $state("");
  let defaultColorHotkey = $state("Control+Shift+C");
  let hotkeyError = $state<string | null>(null);
  let hotkeySaving = $state(false);

  $effect(() => {
    api.getHotkey().then((h) => {
      hotkey = h.hotkey;
      hotkey2 = h.hotkey2;
      mainTab = h.mainTab;
      defaultHotkey = h.default;
      colorHotkey = h.colorHotkey;
      defaultColorHotkey = h.colorDefault;
    });
  });

  // Apply a partial change to the hotkey config. Registers with the OS + persists
  // in one round-trip; on failure the backend keeps the previous binding and we
  // surface the reason without clobbering the local fields.
  async function saveHotkey(patch: {
    hotkey?: string;
    hotkey2?: string;
    mainTab?: PaletteTab;
    colorHotkey?: string;
  }) {
    hotkeyError = null;
    hotkeySaving = true;
    try {
      const h = await api.setHotkey(patch);
      hotkey = h.hotkey;
      hotkey2 = h.hotkey2;
      mainTab = h.mainTab;
      colorHotkey = h.colorHotkey;
    } catch (err) {
      hotkeyError = isCommandError(err) ? err.message : t("hotkey.errorSet");
    } finally {
      hotkeySaving = false;
    }
  }

  // The label under each recorder — names the tab that shortcut opens.
  const secondaryTab = $derived<PaletteTab>(
    mainTab === "snippets" ? "clipboard" : "snippets",
  );
  function tabLabel(tab: PaletteTab): string {
    return tab === "snippets" ? t("nav.snippets") : t("nav.clipboard");
  }

  // Clipboard retention settings (loaded lazily; defaults mirror the backend).
  let retentionDays = $state(30);
  let retentionMax = $state(500);

  // Expansion state.
  let expansionEnabled = $state(false);
  let expansionSupported = $state(true);
  let injectMethod = $state<"paste" | "type">("paste");

  // Per-app exclusion list (apps where expansion never fires).
  let excludedApps = $state<string[]>([]);
  let knownApps = $state<string[]>([]);
  let newExcludedApp = $state("");

  // Pull the settings/expansion state the backend owns into local fields. Kept
  // as a named function (not an inline effect) so backend-driven changes can
  // re-run it — this window is hidden to the tray, not destroyed, so without a
  // live refresh these fields would stay stale until an F5.
  async function loadLocal() {
    const all = await api.getAllSettings();
    if (typeof all["clipboard.retentionDays"] === "number")
      retentionDays = all["clipboard.retentionDays"] as number;
    if (typeof all["clipboard.retentionMax"] === "number")
      retentionMax = all["clipboard.retentionMax"] as number;
    injectMethod = all["expansion.injectMethod"] === "type" ? "type" : "paste";
    const s = await api.expansionStatus();
    expansionEnabled = s.enabled;
    expansionSupported = s.supported;
    excludedApps = await api.getExcludedApps();
    knownApps = await api.knownApps();
  }

  async function addExcludedApp(name: string) {
    const trimmed = name.trim();
    if (!trimmed) return;
    excludedApps = await api.setExcludedApps([...excludedApps, trimmed]);
    newExcludedApp = "";
  }

  async function removeExcludedApp(name: string) {
    excludedApps = await api.setExcludedApps(
      excludedApps.filter((a) => a !== name),
    );
  }

  // Known apps not already excluded — offered as quick-add chips.
  let quickAddApps = $derived(
    knownApps.filter(
      (a) => !excludedApps.some((e) => e.toLowerCase() === a.toLowerCase()),
    ),
  );

  // Backup / restore feedback line, shown briefly after an export/import.
  let backupMessage = $state<string | null>(null);
  let backupBusy = $state(false);

  function defaultBackupName(): string {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    const stamp = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    return `carimbo-backup-${stamp}.json`;
  }

  async function exportBackup() {
    if (backupBusy) return;
    backupBusy = true;
    backupMessage = null;
    try {
      const path = await save({
        defaultPath: defaultBackupName(),
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return; // user cancelled
      await api.backupExport(path);
      backupMessage = t("settings.backupExported");
    } catch {
      backupMessage = t("settings.backupError");
    } finally {
      backupBusy = false;
    }
  }

  async function importBackup() {
    if (backupBusy) return;
    backupBusy = true;
    backupMessage = null;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return; // user cancelled
      const report = await api.backupImport(path);
      backupMessage = t("settings.backupImported", {
        snippets: report.snippetsAdded,
        folders: report.foldersAdded,
        dropped: report.triggersDropped,
      });
    } catch (e) {
      backupMessage =
        typeof e === "object" && e && "message" in e
          ? String((e as { message: unknown }).message)
          : t("settings.backupError");
    } finally {
      backupBusy = false;
    }
  }

  // Import a library exported from another expander. Format is auto-detected
  // from the file extension / content by the backend, so the picker just accepts
  // the common export types.
  async function importForeign() {
    if (backupBusy) return;
    backupBusy = true;
    backupMessage = null;
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Snippet exports",
            extensions: ["yml", "yaml", "csv", "tsv", "json"],
          },
        ],
      });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return; // user cancelled
      const report = await api.importForeign(path);
      backupMessage =
        report.snippetsAdded === 0
          ? t("settings.importEmpty")
          : t("settings.importDone", {
              snippets: report.snippetsAdded,
              dropped: report.triggersDropped,
              skipped: report.skipped,
            });
    } catch (e) {
      backupMessage =
        typeof e === "object" && e && "message" in e
          ? String((e as { message: unknown }).message)
          : t("settings.backupError");
    } finally {
      backupBusy = false;
    }
  }

  $effect(() => {
    loadLocal();
    // Re-read when a setting changes elsewhere, or when the backend reports the
    // hook was blocked (expansion is off now, so the toggle must reflect that).
    const uns = [
      onEvent(Events.SettingsChanged, () => loadLocal()),
      onEvent(Events.ExpansionBlocked, () => loadLocal()),
    ];
    return () => {
      for (const un of uns) un.then((f) => f());
    };
  });

  async function toggleExpansion(enabled: boolean) {
    const status = await api.setExpansionEnabled(enabled);
    expansionEnabled = status.enabled;
  }

  async function setInjectMethod(method: "paste" | "type") {
    injectMethod = method;
    await api.setSetting("expansion.injectMethod", method);
    // Re-apply by toggling the service if it's running.
    if (expansionEnabled) {
      await api.setExpansionEnabled(false);
      await api.setExpansionEnabled(true);
    }
  }

  async function saveRetentionDays(v: number) {
    retentionDays = v;
    await api.setSetting("clipboard.retentionDays", v);
  }
  async function saveRetentionMax(v: number) {
    retentionMax = v;
    await api.setSetting("clipboard.retentionMax", v);
  }

  // Theme options — labels resolve through t() at render time so they follow
  // the UI language.
  const themes: { value: Theme; labelKey: Parameters<typeof t>[0] }[] = [
    { value: "system", labelKey: "settings.theme.system" },
    { value: "light", labelKey: "settings.theme.light" },
    { value: "dark", labelKey: "settings.theme.dark" },
    { value: "hc-light", labelKey: "settings.theme.hcLight" },
    { value: "hc-dark", labelKey: "settings.theme.hcDark" },
  ];

  const languages: { value: Lang; labelKey: Parameters<typeof t>[0] }[] = [
    { value: "en", labelKey: "settings.language.en" },
    { value: "pt-BR", labelKey: "settings.language.ptBR" },
  ];

  const regions: { value: Region; labelKey: Parameters<typeof t>[0] }[] = [
    { value: "us", labelKey: "settings.region.us" },
    { value: "br", labelKey: "settings.region.br" },
  ];

  let a = $derived(settingsStore.appearance);

  // --- section navigation ---------------------------------------------------
  // The panel used to be seven stacked cards in one long scroll; it's now a
  // left rail (segmented strip on narrow widths) + one visible section at a
  // time, so a user jumping in to flip one toggle isn't scanning past six
  // unrelated cards to find it. Every control below is unchanged — only its
  // container changed from "always visible" to "visible when its tab is
  // active" (kept mounted-and-hidden, not destroyed, so effects/state aren't
  // disturbed by switching tabs).
  type SectionId =
    | "appearance"
    | "language"
    | "quickSearch"
    | "expansion"
    | "clipboard"
    | "backup"
    | "cloud";

  const sections: { id: SectionId; labelKey: Parameters<typeof t>[0] }[] = [
    { id: "appearance", labelKey: "settings.appearance" },
    { id: "language", labelKey: "settings.language" },
    { id: "quickSearch", labelKey: "settings.quickSearch" },
    { id: "expansion", labelKey: "settings.expansion" },
    { id: "clipboard", labelKey: "settings.clipboard" },
    { id: "backup", labelKey: "settings.backup" },
    { id: "cloud", labelKey: "settings.cloud" },
  ];

  let activeSection = $state<SectionId>("appearance");
  const tabIds = sections.map((s) => s.id);

  // Roving tabindex + arrow-key navigation, matching the tablist pattern
  // (ARIA APG): only the active tab is in the tab order, Up/Down (and
  // Left/Right, since the rail can render as a horizontal strip on narrow
  // widths) move focus and selection together, Home/End jump to the ends.
  function onTabKeydown(e: KeyboardEvent) {
    const i = tabIds.indexOf(activeSection);
    let next = -1;
    if (e.key === "ArrowDown" || e.key === "ArrowRight") next = (i + 1) % tabIds.length;
    else if (e.key === "ArrowUp" || e.key === "ArrowLeft")
      next = (i - 1 + tabIds.length) % tabIds.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = tabIds.length - 1;
    if (next < 0) return;
    e.preventDefault();
    activeSection = tabIds[next];
    const btn = document.getElementById(`settings-tab-${tabIds[next]}`);
    btn?.focus();
  }
</script>

<div class="settings">
  <nav class="rail" aria-label={t("nav.sections")}>
    <div
      class="rail-tabs"
      role="tablist"
      aria-orientation="vertical"
      tabindex="-1"
      onkeydown={onTabKeydown}
    >
      {#each sections as sec (sec.id)}
        {@const isCloud = sec.id === "cloud"}
        <button
          id={`settings-tab-${sec.id}`}
          class="rail-tab"
          class:active={activeSection === sec.id}
          class:muted={isCloud}
          role="tab"
          type="button"
          aria-selected={activeSection === sec.id}
          aria-controls={`settings-panel-${sec.id}`}
          tabindex={activeSection === sec.id ? 0 : -1}
          onclick={() => (activeSection = sec.id)}
        >
          <span class="rail-icon">
            {#if sec.id === "appearance"}
              <Icon name="settings" size={16} />
            {:else if sec.id === "language"}
              <Icon name="mail" size={16} />
            {:else if sec.id === "quickSearch"}
              <Icon name="search" size={16} />
            {:else if sec.id === "expansion"}
              <Icon name="sparkles" size={16} />
            {:else if sec.id === "clipboard"}
              <Icon name="clipboard" size={16} />
            {:else if sec.id === "backup"}
              <Icon name="download" size={16} />
            {:else}
              <!-- Cloud has no dedicated icon in the shared set; a small inline
                   glyph local to this component keeps it visually consistent
                   with the Lucide stroke icons without touching Icon.svelte. -->
              <svg
                class="cloud-icon"
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path
                  d="M17.5 19H8a5 5 0 0 1-1-9.9 5.5 5.5 0 0 1 10.7 1.7A4 4 0 0 1 17.5 19Z"
                />
              </svg>
            {/if}
          </span>
          <span class="rail-label">{t(sec.labelKey)}</span>
          {#if isCloud}
            <span class="rail-badge">{t("settings.comingSoon")}</span>
          {/if}
        </button>
      {/each}
    </div>
  </nav>

  <div class="panes">
  <div
    class="card"
    id="settings-panel-appearance"
    role="tabpanel"
    aria-labelledby="settings-tab-appearance"
    hidden={activeSection !== "appearance"}
  >
    <h2>{t("settings.appearance")}</h2>

  <div class="setting">
    <label class="label" for="theme-select">{t("settings.theme")}</label>
    <select
      id="theme-select"
      value={a.theme}
      onchange={(e) =>
        settingsStore.setTheme((e.target as HTMLSelectElement).value as Theme)}
    >
      {#each themes as opt (opt.value)}
        <option value={opt.value}>{t(opt.labelKey)}</option>
      {/each}
    </select>
  </div>

  <div class="setting">
    <label class="label" for="font-range">
      {t("settings.fontSize", { pct: Math.round(a.fontScale * 100) })}
    </label>
    <input
      id="font-range"
      type="range"
      min="0.875"
      max="1.5"
      step="0.125"
      value={a.fontScale}
      oninput={(e) =>
        settingsStore.setFontScale(
          parseFloat((e.target as HTMLInputElement).value),
        )}
    />
  </div>

  <div class="setting">
    <label class="label" for="glass-range">
      {t("settings.glassOpacity", { pct: Math.round(a.glassOpacity * 100) })}
    </label>
    <input
      id="glass-range"
      type="range"
      min={GLASS_OPACITY_MIN}
      max={GLASS_OPACITY_MAX}
      step="0.02"
      value={a.glassOpacity}
      oninput={(e) =>
        settingsStore.setGlassOpacity(
          parseFloat((e.target as HTMLInputElement).value),
        )}
    />
    <p class="note">{t("settings.glassOpacityNote")}</p>
  </div>

  <div class="setting">
    <span class="label">{t("settings.density")}</span>
    <div class="toggle-group" role="group" aria-label={t("settings.density")}>
      <button
        class:active={a.density === "compact"}
        onclick={() => settingsStore.setDensity("compact")}
      >
        {t("settings.density.compact")}
      </button>
      <button
        class:active={a.density === "comfortable"}
        onclick={() => settingsStore.setDensity("comfortable")}
      >
        {t("settings.density.comfortable")}
      </button>
    </div>
  </div>

  <div class="setting">
    <label class="check">
      <input
        type="checkbox"
        checked={a.motion === "reduce"}
        onchange={(e) =>
          settingsStore.setMotion(
            (e.target as HTMLInputElement).checked ? "reduce" : "system",
          )}
      />
      <span>{t("settings.reduceMotion")}</span>
    </label>
  </div>
  </div>

  <div
    class="card"
    id="settings-panel-language"
    role="tabpanel"
    aria-labelledby="settings-tab-language"
    hidden={activeSection !== "language"}
  >
    <h2>{t("settings.language")}</h2>

  <div class="setting">
    <label class="label" for="lang-select">{t("settings.uiLanguage")}</label>
    <select
      id="lang-select"
      value={i18n.lang}
      onchange={(e) =>
        i18n.setLang((e.target as HTMLSelectElement).value as Lang)}
    >
      {#each languages as opt (opt.value)}
        <option value={opt.value}>{t(opt.labelKey)}</option>
      {/each}
    </select>
  </div>

  <div class="setting">
    <label class="label" for="region-select">{t("settings.region")}</label>
    <select
      id="region-select"
      value={i18n.region}
      onchange={(e) =>
        i18n.setRegion((e.target as HTMLSelectElement).value as Region)}
    >
      {#each regions as opt (opt.value)}
        <option value={opt.value}>{t(opt.labelKey)}</option>
      {/each}
    </select>
    <p class="note">{t("settings.region.note")}</p>
  </div>
  </div>

  <div
    class="card"
    id="settings-panel-quickSearch"
    role="tabpanel"
    aria-labelledby="settings-tab-quickSearch"
    hidden={activeSection !== "quickSearch"}
  >
    <h2>{t("settings.quickSearch")}</h2>

    <div class="setting">
      <span class="label">{t("settings.mainTab")}</span>
      <div class="toggle-group" role="group" aria-label={t("settings.mainTab")}>
        <button
          class:active={mainTab === "snippets"}
          disabled={hotkeySaving}
          onclick={() => saveHotkey({ mainTab: "snippets" })}
        >
          <Icon name="snippet" size={14} />
          {t("nav.snippets")}
        </button>
        <button
          class:active={mainTab === "clipboard"}
          disabled={hotkeySaving}
          onclick={() => saveHotkey({ mainTab: "clipboard" })}
        >
          <Icon name="clipboard" size={14} />
          {t("nav.clipboard")}
        </button>
      </div>
      <p class="note">{t("settings.mainTabNote")}</p>
    </div>

    <div class="setting">
      <span class="label">{t("settings.globalHotkey")}</span>
      <div class="hotkey-row">
        <HotkeyRecorder
          value={hotkey}
          disabled={hotkeySaving}
          ariaLabel={t("hotkey.recordAria")}
          onRecord={(accel) => saveHotkey({ hotkey: accel })}
        />
        <span class="hotkey-target">
          {t("settings.opensTab", { tab: tabLabel(mainTab) })}
        </span>
      </div>
      <button
        class="link"
        disabled={hotkeySaving}
        onclick={() => saveHotkey({ hotkey: defaultHotkey })}
      >
        {t("hotkey.restoreDefault")}
      </button>
      <p class="note">{t("settings.hotkeyNote", { combo: "Ctrl+Shift+Space" })}</p>
    </div>

    <div class="setting">
      <span class="label">{t("settings.secondHotkey")}</span>
      <div class="hotkey-row">
        <HotkeyRecorder
          value={hotkey2}
          disabled={hotkeySaving}
          ariaLabel={t("settings.secondHotkey")}
          emptyLabel={t("hotkey.none")}
          onRecord={(accel) => saveHotkey({ hotkey2: accel })}
          onClear={() => saveHotkey({ hotkey2: "" })}
        />
        {#if hotkey2}
          <span class="hotkey-target">
            {t("settings.opensTab", { tab: tabLabel(secondaryTab) })}
          </span>
        {/if}
      </div>
      <p class="note">{t("settings.secondHotkeyNote", { tab: tabLabel(secondaryTab) })}</p>
    </div>

    <div class="setting">
      <span class="label">{t("settings.colorHotkey")}</span>
      <div class="hotkey-row">
        <HotkeyRecorder
          value={colorHotkey}
          disabled={hotkeySaving}
          ariaLabel={t("settings.colorHotkey")}
          emptyLabel={t("hotkey.none")}
          onRecord={(accel) => saveHotkey({ colorHotkey: accel })}
          onClear={() => saveHotkey({ colorHotkey: "" })}
        />
        <span class="hotkey-target">
          {t("settings.opensTab", { tab: t("nav.colors") })}
        </span>
      </div>
      <button
        class="link"
        disabled={hotkeySaving}
        onclick={() => saveHotkey({ colorHotkey: defaultColorHotkey })}
      >
        {t("hotkey.restoreDefault")}
      </button>
      <p class="note">{t("settings.colorHotkeyNote")}</p>
    </div>

    {#if hotkeyError}
      <p class="hotkey-err" role="alert">{hotkeyError}</p>
    {/if}
  </div>

  <div
    class="card"
    id="settings-panel-expansion"
    role="tabpanel"
    aria-labelledby="settings-tab-expansion"
    hidden={activeSection !== "expansion"}
  >
    <h2>{t("settings.expansion")}</h2>

  <div class="setting">
    <label class="check">
      <input
        type="checkbox"
        checked={expansionEnabled}
        disabled={!expansionSupported}
        onchange={(e) =>
          toggleExpansion((e.target as HTMLInputElement).checked)}
      />
      <span>{t("settings.expansionEnable")}</span>
    </label>
    <p class="note">{t("settings.expansionNote", { trigger: ";email" })}</p>
  </div>

  {#if expansionEnabled}
    <div class="setting">
      <span class="label">{t("settings.injectMethod")}</span>
      <div class="toggle-group" role="group" aria-label={t("settings.injectMethod")}>
        <button
          class:active={injectMethod === "paste"}
          onclick={() => setInjectMethod("paste")}
        >
          {t("settings.injectPaste")}
        </button>
        <button
          class:active={injectMethod === "type"}
          onclick={() => setInjectMethod("type")}
        >
          {t("settings.injectType")}
        </button>
      </div>
      <p class="note">{t("settings.injectNote")}</p>
    </div>

    <div class="setting">
      <span class="label">{t("settings.excludedApps")}</span>
      <p class="note">{t("settings.excludedAppsNote")}</p>

      {#if excludedApps.length > 0}
        <ul class="chip-list" aria-label={t("settings.excludedApps")}>
          {#each excludedApps as app (app)}
            <li class="chip">
              <span class="chip-name">{app}</span>
              <button
                class="chip-remove"
                title={t("settings.excludedRemove", { app })}
                aria-label={t("settings.excludedRemove", { app })}
                onclick={() => removeExcludedApp(app)}
              >
                ×
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      <form
        class="add-row"
        onsubmit={(e) => {
          e.preventDefault();
          addExcludedApp(newExcludedApp);
        }}
      >
        <input
          class="mono"
          bind:value={newExcludedApp}
          placeholder={t("settings.excludedPlaceholder")}
          spellcheck="false"
          autocomplete="off"
          aria-label={t("settings.excludedAdd")}
        />
        <button
          type="submit"
          class="add-btn"
          disabled={!newExcludedApp.trim()}
        >
          {t("settings.excludedAdd")}
        </button>
      </form>

      {#if quickAddApps.length > 0}
        <div class="quick-add">
          <span class="quick-label">{t("settings.excludedSuggestions")}</span>
          <div class="chip-list">
            {#each quickAddApps.slice(0, 8) as app (app)}
              <button class="chip suggest" onclick={() => addExcludedApp(app)}>
                + {app}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
  </div>

  <div
    class="card"
    id="settings-panel-clipboard"
    role="tabpanel"
    aria-labelledby="settings-tab-clipboard"
    hidden={activeSection !== "clipboard"}
  >
    <h2>{t("settings.clipboard")}</h2>

  <div class="setting">
    <label class="label" for="retention-days">
      {t("settings.retentionDays", {
        n: retentionDays,
        plural: retentionDays === 1 ? "" : "s",
      })}
    </label>
    <input
      id="retention-days"
      type="range"
      min="1"
      max="90"
      step="1"
      value={retentionDays}
      oninput={(e) =>
        saveRetentionDays(parseInt((e.target as HTMLInputElement).value))}
    />
  </div>

  <div class="setting">
    <label class="label" for="retention-max">
      {t("settings.retentionMax", { n: retentionMax })}
    </label>
    <input
      id="retention-max"
      type="range"
      min="50"
      max="2000"
      step="50"
      value={retentionMax}
      oninput={(e) =>
        saveRetentionMax(parseInt((e.target as HTMLInputElement).value))}
    />
  </div>
  <p class="note">{t("settings.retentionNote")}</p>
  </div>

  <div
    class="card"
    id="settings-panel-backup"
    role="tabpanel"
    aria-labelledby="settings-tab-backup"
    hidden={activeSection !== "backup"}
  >
    <h2>{t("settings.backup")}</h2>
    <p class="note">{t("settings.backupNote")}</p>
    <div class="backup-actions">
      <button class="backup-btn" disabled={backupBusy} onclick={exportBackup}>
        {t("settings.backupExport")}
      </button>
      <button class="backup-btn" disabled={backupBusy} onclick={importBackup}>
        {t("settings.backupImport")}
      </button>
    </div>

    <div class="import-foreign">
      <h3 class="subhead">{t("settings.import")}</h3>
      <p class="note">{t("settings.importNote")}</p>
      <button class="backup-btn" disabled={backupBusy} onclick={importForeign}>
        {t("settings.importButton")}
      </button>
    </div>

    {#if backupMessage}
      <p class="backup-msg" role="status">{backupMessage}</p>
    {/if}
  </div>

  <!-- Cloud is a placeholder, not a shipped feature — demoted to a muted,
       visually secondary teaser rather than a card with equal weight to the
       real settings sections. No sub-head divider, dimmer text, and the
       "coming soon" badge does the labeling instead of an h2. -->
  <div
    class="card cloud-card"
    id="settings-panel-cloud"
    role="tabpanel"
    aria-labelledby="settings-tab-cloud"
    hidden={activeSection !== "cloud"}
  >
    <div class="coming-soon">
      <span class="coming-soon-icon">
        <svg
          viewBox="0 0 24 24"
          width="20"
          height="20"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path
            d="M17.5 19H8a5 5 0 0 1-1-9.9 5.5 5.5 0 0 1 10.7 1.7A4 4 0 0 1 17.5 19Z"
          />
        </svg>
      </span>
      <div class="coming-soon-text">
        <h2 class="coming-soon-head">{t("settings.cloud")}</h2>
        <p>{t("settings.cloudDesc")}</p>
      </div>
      <span class="badge">{t("settings.comingSoon")}</span>
    </div>
  </div>
  </div>
</div>

<style>
  /* Sectioned nav replaces the old single-scroll stack of seven cards: a
     left rail lists every section (icon + label), the pane on the right
     shows exactly one at a time. Below ~640px the rail collapses into a
     horizontal segmented strip (see the media query) so the manager's
     ~600-760px content pane never has to squeeze a sidebar and a card side
     by side. */
  .settings {
    height: 100%;
    /* Cap the content width and center it, so a maximized window on a wide
       monitor shows a comfortable centered column rather than the rail + card
       stranded in the top-left corner beside a sea of empty space. The auto
       side margins collapse to nothing once the pane is narrower than the cap,
       so nothing changes at the manager's default ~1000px width. */
    max-width: 960px;
    margin-inline: auto;
    padding: var(--space-6) var(--space-5);
    display: grid;
    grid-template-columns: 176px 1fr;
    align-items: start;
    gap: var(--space-5);
    overflow-y: auto;
  }
  .rail {
    position: sticky;
    top: 0;
    align-self: start;
  }
  .rail-tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rail-tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
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
  .rail-tab:hover:not(.active) {
    background: var(--bg-hover);
    color: var(--text);
  }
  .rail-tab.active {
    background: var(--accent-weak);
    color: var(--text);
    font-weight: 600;
  }
  /* Accent bar echoes the main sidebar's active-item idiom, so the settings
     rail reads as the same nav pattern rather than a one-off control. */
  .rail-tab.active::before {
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
  .rail-icon {
    display: flex;
    flex-shrink: 0;
    color: currentColor;
  }
  .rail-icon :global(svg),
  .cloud-icon {
    flex-shrink: 0;
  }
  .rail-label {
    flex: 1;
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Cloud tab is demoted in the rail too: dimmer at rest, and a tiny pill
     instead of prose so it never competes with the real sections. */
  .rail-tab.muted {
    color: var(--text-muted);
    opacity: 0.75;
  }
  .rail-tab.muted:hover:not(.active) {
    opacity: 1;
  }
  .rail-tab.muted.active {
    opacity: 1;
  }
  .rail-badge {
    flex-shrink: 0;
    padding: 1px 6px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
    background: var(--bg-inset);
    color: var(--text-muted);
    border-radius: 999px;
  }
  .panes {
    min-width: 0;
  }
  /* Each settings group is a card, so the panel reads as an organized section
     rather than one long undifferentiated form. Only the active section's
     card is shown; the rest stay mounted (via [hidden]) so their local state
     and effects aren't disturbed by switching tabs. */
  .card {
    width: 100%;
    max-width: 620px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    /* A short, eased fade-in on the section that just became visible — long
       enough to register as a transition, short enough to never feel like a
       load. Reduced-motion users get this zeroed by the global guards. */
    animation: pane-in var(--transition-slow);
  }
  .card[hidden] {
    display: none;
  }
  @keyframes pane-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  h2 {
    font-size: var(--fs-lg);
    margin: 0;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  /* Narrow manager widths: fold the rail into a horizontal segmented strip
     above the content, matching the toggle-group idiom already used
     elsewhere on this page. */
  @media (max-width: 640px) {
    .settings {
      grid-template-columns: 1fr;
    }
    .rail {
      position: static;
    }
    .rail-tabs {
      flex-direction: row;
      flex-wrap: wrap;
      gap: var(--space-1);
      padding: var(--space-1);
      background: var(--bg-inset);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
    }
    .rail-tab {
      flex: 1 1 auto;
      justify-content: center;
      border-radius: var(--radius-sm);
    }
    .rail-tab.active::before {
      display: none;
    }
    .rail-label {
      flex: none;
    }
    .rail-badge {
      display: none;
    }
  }
  .setting {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .label {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-muted);
  }
  select,
  input[type="range"] {
    max-width: 320px;
  }
  input[type="range"] {
    accent-color: var(--accent);
  }
  select {
    padding: var(--space-3);
    min-height: var(--hit-target);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    transition: border-color var(--transition-fast);
  }
  select:focus {
    border-color: var(--accent);
  }
  .toggle-group {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    width: fit-content;
  }
  .toggle-group button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: calc(var(--hit-target) - 6px);
    padding: 0 var(--space-4);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .toggle-group button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .toggle-group button:hover:not(.active) {
    color: var(--text);
  }
  .toggle-group button.active {
    background: var(--bg-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
  }
  .check input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .note {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--text-muted);
    max-width: 480px;
  }
  /* A recorder plus the "opens: <tab>" caption that tells the user what firing
     this shortcut does. */
  .hotkey-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .hotkey-target {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .link {
    align-self: flex-start;
    background: none;
    border: none;
    color: var(--accent);
    font-size: var(--fs-sm);
    padding: var(--space-1) 0;
  }
  .link:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .hotkey-err {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--danger);
  }
  .chip-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 4px 6px 4px 10px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: var(--fs-sm);
  }
  .chip-name {
    font-family: var(--font-mono);
  }
  .chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 999px;
    color: var(--text-muted);
    font-size: var(--fs-md);
    line-height: 1;
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }
  .chip-remove:hover {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .chip.suggest {
    cursor: pointer;
    color: var(--text-muted);
    font-family: var(--font-mono);
    transition:
      color var(--transition-fast),
      border-color var(--transition-fast);
  }
  .chip.suggest:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .add-row {
    display: flex;
    gap: var(--space-2);
    max-width: 420px;
  }
  .add-row input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    min-height: var(--hit-target);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
  }
  .add-row input:focus {
    border-color: var(--accent);
  }
  .add-btn {
    flex-shrink: 0;
    padding: 0 var(--space-4);
    min-height: var(--hit-target);
    background: var(--bg-inset);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    transition:
      background var(--transition-fast),
      border-color var(--transition-fast);
  }
  .add-btn:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .quick-add {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .quick-label {
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .backup-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .backup-btn {
    padding: 0 var(--space-4);
    min-height: var(--hit-target);
    background: var(--bg-inset);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    transition:
      background var(--transition-fast),
      border-color var(--transition-fast);
  }
  .backup-btn:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .backup-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .backup-msg {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--accent);
  }
  .import-foreign {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border);
  }
  .import-foreign .backup-btn {
    align-self: flex-start;
  }
  .subhead {
    margin: 0;
    font-size: var(--fs-base);
    font-weight: 600;
    color: var(--text);
  }
  /* Cloud's whole card sits visually quieter than the other sections — no
     elevated-surface contrast beyond the shared .card border, muted icon
     well, and a subdued badge instead of the accent-colored one used
     elsewhere — so it reads as "not here yet" rather than an equal feature. */
  .cloud-card {
    background: var(--bg-inset);
  }
  .coming-soon {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    color: var(--text-muted);
  }
  .coming-soon-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-muted);
  }
  .coming-soon-text {
    flex: 1;
    min-width: 0;
  }
  .coming-soon-head {
    margin: 0 0 var(--space-1);
    padding: 0;
    border: none;
    font-size: var(--fs-base);
    font-weight: 600;
    color: var(--text);
  }
  .coming-soon p {
    margin: 0;
    font-size: var(--fs-sm);
  }
  .badge {
    flex-shrink: 0;
    padding: 2px 10px;
    font-size: var(--fs-xs);
    background: var(--bg-elevated);
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    font-weight: 600;
  }
</style>
