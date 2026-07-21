// Typed wrappers around the Tauri command layer. Every backend call goes through
// here so views never touch `invoke` directly.

import { invoke } from "@tauri-apps/api/core";
import type {
  ClipEntry,
  ExpansionStatus,
  Folder,
  FolderKind,
  ForeignImportReport,
  HotkeyInfo,
  ImportFormat,
  ImportReport,
  NewSnippet,
  PaletteTab,
  Snippet,
  TransformKind,
  UpdateSnippet,
  Variable,
} from "./types";

export const api = {
  // --- snippets ---
  listSnippets: () => invoke<Snippet[]>("snippets_list"),
  searchSnippets: (query: string) =>
    invoke<Snippet[]>("snippets_search", { query }),
  getSnippet: (id: string) => invoke<Snippet>("snippets_get", { id }),
  createSnippet: (input: NewSnippet) =>
    invoke<Snippet>("snippets_create", { input }),
  updateSnippet: (input: UpdateSnippet) =>
    invoke<Snippet>("snippets_update", { input }),
  deleteSnippet: (id: string) => invoke<void>("snippets_delete", { id }),
  setFavorite: (id: string, isFavorite: boolean) =>
    invoke<Snippet>("snippets_set_favorite", { id, isFavorite }),
  /** Seed the region's example snippets on first run. Idempotent: the backend
      only seeds when the library is empty. Returns how many were inserted. */
  seedDefaultSnippets: (region: "us" | "br") =>
    invoke<number>("snippets_seed_defaults", { region }),

  // --- folders ---
  listFolders: (kind: FolderKind) =>
    invoke<Folder[]>("folders_list", { kind }),
  createFolder: (name: string, kind: FolderKind, parentId: string | null) =>
    invoke<Folder>("folders_create", { name, kind, parentId }),
  renameFolder: (id: string, name: string) =>
    invoke<Folder>("folders_rename", { id, name }),
  deleteFolder: (id: string) => invoke<void>("folders_delete", { id }),

  // --- settings ---
  getAllSettings: () =>
    invoke<Record<string, unknown>>("settings_get_all"),
  setSetting: (key: string, value: unknown) =>
    invoke<void>("settings_set", { key, value }),

  // --- palette (quick popup) ---
  getHotkey: () => invoke<HotkeyInfo>("hotkey_get"),
  /** Change any of the palette hotkeys / main tab. Omitted fields are left as
      they are. Pass `hotkey2: ""` to clear the secondary shortcut. */
  setHotkey: (patch: {
    hotkey?: string;
    hotkey2?: string;
    mainTab?: PaletteTab;
    colorHotkey?: string;
  }) =>
    invoke<HotkeyInfo>("hotkey_set", {
      hotkey: patch.hotkey ?? null,
      hotkey2: patch.hotkey2 ?? null,
      mainTab: patch.mainTab ?? null,
      colorHotkey: patch.colorHotkey ?? null,
    }),
  paletteHide: () => invoke<void>("palette_hide"),
  /** Form variables a snippet needs filled before insertion (body order). */
  paletteVariables: (id: string) =>
    invoke<Variable[]>("palette_variables", { id }),
  /** Insert a snippet, passing filled `[[key]]` values (omit when none) and an
      optional "paste as…" transform (plain/UPPERCASE/slug/…) from the row's
      more-options menu. A transform pastes the expanded text as plain text. */
  paletteInsert: (
    id: string,
    variables?: Record<string, string>,
    transform?: TransformKind,
  ) =>
    invoke<string>("palette_insert", {
      id,
      variables: variables ?? null,
      transform: transform ?? null,
    }),

  // --- expansion ---
  expansionStatus: () => invoke<ExpansionStatus>("expansion_status"),
  setExpansionEnabled: (enabled: boolean) =>
    invoke<ExpansionStatus>("expansion_set_enabled", { enabled }),
  setExpansionPaused: (paused: boolean) =>
    invoke<ExpansionStatus>("expansion_set_paused", { paused }),
  /** Apps (exe names) where automatic expansion is suppressed. */
  getExcludedApps: () => invoke<string[]>("expansion_get_excluded"),
  /** Replace the exclusion list; returns the normalized, de-duplicated result. */
  setExcludedApps: (apps: string[]) =>
    invoke<string[]>("expansion_set_excluded", { apps }),
  /** Exe names seen in clipboard history, offered as quick picks. */
  knownApps: () => invoke<string[]>("expansion_known_apps"),

  // --- backup / restore ---
  /** Write the whole library (folders + snippets) to `path` as JSON. */
  backupExport: (path: string) => invoke<void>("backup_export", { path }),
  /** Merge a backup file at `path` into the library; returns a count report. */
  backupImport: (path: string) =>
    invoke<ImportReport>("backup_import", { path }),
  /**
   * Import a library exported from another expander (espanso YAML, CSV, or a
   * JSON array) at `path`. Omit `format` to auto-detect from the extension /
   * content. Additive; a clashing trigger is dropped, never the snippet.
   */
  importForeign: (path: string, format?: ImportFormat) =>
    invoke<ForeignImportReport>("snippets_import_foreign", { path, format }),

  // --- color picker ---
  /** Start a pick-from-screen session. The result arrives as a `color:picked`
      (or `color:cancelled`) event once the user clicks / cancels. */
  colorPickStart: () => invoke<void>("color_pick_start"),
  /** Copy a formatted color value (hex/rgb/hsl/…) to the clipboard. */
  colorCopy: (text: string) => invoke<void>("color_copy", { text }),

  // --- clipboard history ---
  listClips: () => invoke<ClipEntry[]>("clips_list"),
  searchClips: (query: string) => invoke<ClipEntry[]>("clips_search", { query }),
  setClipPinned: (id: string, pinned: boolean) =>
    invoke<void>("clips_set_pinned", { id, pinned }),
  /** File a clip into a clipboard folder (null unfiles it). Filed clips are
      exempt from retention. */
  setClipFolder: (id: string, folderId: string | null) =>
    invoke<void>("clips_set_folder", { id, folderId }),
  deleteClip: (id: string) => invoke<void>("clips_delete", { id }),
  /** Copy a text clip back to the clipboard, optionally transformed. */
  copyClip: (id: string, transform?: TransformKind) =>
    invoke<void>("clips_copy", { id, transform: transform ?? null }),
  /** Paste a text clip into the app focused when the palette opened,
      optionally transformed ("paste as UPPERCASE", plain text, slug, …). */
  pasteClip: (id: string, transform?: TransformKind) =>
    invoke<string>("clips_paste", { id, transform: transform ?? null }),
  /** Paste an arbitrary string (derived from a clip, e.g. a color reformatted
      as rgb()/hsl()/hex) into the app focused when the palette opened. */
  pasteClipText: (text: string) => invoke<string>("clips_paste_text", { text }),
  /** Preview a transform's result without copying/pasting. */
  transformClip: (id: string, transform: TransformKind) =>
    invoke<string>("clips_transform", { id, transform }),
  /** Turn a text clip into a reusable snippet. Returns the new snippet. */
  promoteClipToSnippet: (
    id: string,
    name?: string | null,
    folderId?: string | null,
  ) =>
    invoke<Snippet>("clips_promote_to_snippet", {
      id,
      name: name ?? null,
      folderId: folderId ?? null,
    }),
  /** Open a url/email clip in the browser (or mail client). */
  openClipUrl: (id: string) => invoke<boolean>("clips_open_url", { id }),
  /** Reveal a path/files clip in Explorer. */
  revealClipPath: (id: string) => invoke<boolean>("clips_reveal_path", { id }),
  /** Reveal an image clip's backing PNG in Explorer (to save/use the file). */
  revealClipImage: (id: string) => invoke<boolean>("clips_reveal_image", { id }),
};
