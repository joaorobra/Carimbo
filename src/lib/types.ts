// DTOs mirroring the Rust core::models (serialized camelCase). Keep in sync with
// src-tauri/src/core/models.rs.

export type FolderKind = "snippet" | "clipboard";

export interface Folder {
  id: string;
  name: string;
  kind: FolderKind;
  parentId: string | null;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export interface Snippet {
  id: string;
  folderId: string | null;
  name: string;
  trigger: string | null;
  body: string;
  /** Optional rich-text (HTML) body; null for a plain snippet. */
  bodyHtml: string | null;
  isFavorite: boolean;
  useCount: number;
  lastUsedAt: number | null;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export interface NewSnippet {
  name: string;
  trigger: string | null;
  body: string;
  bodyHtml?: string | null;
  folderId: string | null;
  isFavorite?: boolean;
}

export interface UpdateSnippet {
  id: string;
  name: string;
  trigger: string | null;
  body: string;
  bodyHtml?: string | null;
  folderId: string | null;
  isFavorite: boolean;
}

/** A `[[key]]` / `[[key:Label]]` form variable the user fills before insertion.
    Mirrors core::tokens::Variable (serialized camelCase). */
export interface Variable {
  key: string;
  label: string;
}

export interface ExpansionStatus {
  enabled: boolean;
  paused: boolean;
  supported: boolean;
}

/** Result of importing a backup file. Mirrors core::backup::ImportReport. */
export interface ImportReport {
  foldersAdded: number;
  snippetsAdded: number;
  triggersDropped: number;
  skipped: number;
}

/** Source format for importing a library exported from another expander. */
export type ImportFormat = "espanso" | "csv" | "json";

/**
 * Result of importing snippets from another expander. Mirrors
 * core::import::ForeignImportReport (no folders — a foreign import carries none).
 */
export interface ForeignImportReport {
  snippetsAdded: number;
  triggersDropped: number;
  skipped: number;
}

/** Which palette tab a hotkey opens. The primary hotkey opens `mainTab`; the
    optional secondary hotkey opens the other one. */
export type PaletteTab = "snippets" | "clipboard";

export interface HotkeyInfo {
  /** Primary accelerator (opens the main tab). */
  hotkey: string;
  /** Secondary accelerator (opens the other tab), or "" when unset. */
  hotkey2: string;
  /** Which tab the primary hotkey opens. */
  mainTab: PaletteTab;
  /** Screen-color-picker accelerator, or "" when unset. */
  colorHotkey: string;
  /** Built-in default primary, for a "restore default" affordance. */
  default: string;
  /** Built-in default color-picker accelerator. */
  colorDefault: string;
}

export type ClipKind = "text" | "image";

/** Refined classification of a clip's content — drives the row badge and the
    type-specific action. Mirrors core::classify::ContentType. */
export type ContentType =
  | "text"
  | "url"
  | "email"
  | "color"
  | "path"
  | "files"
  | "image";

/** A "paste as…" text transform. Mirrors core::transform::TransformKind. */
export type TransformKind =
  | "plain"
  | "upperCase"
  | "lowerCase"
  | "titleCase"
  | "trim"
  | "singleLine"
  | "slug"
  | "base64Encode"
  | "base64Decode";

export interface ClipEntry {
  id: string;
  kind: ClipKind;
  contentType: ContentType;
  content: string | null;
  imagePath: string | null;
  preview: string;
  isPinned: boolean;
  folderId: string | null;
  sourceApp: string | null;
  createdAt: number;
  updatedAt: number;
}

// Shape of the error a command rejects with (see core::error::CoreError).
export interface CommandError {
  kind:
    | "not_found"
    | "duplicate_trigger"
    | "invalid"
    | "migration"
    | "sqlite"
    | "other";
  message: string;
}

export function isCommandError(e: unknown): e is CommandError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e
  );
}
