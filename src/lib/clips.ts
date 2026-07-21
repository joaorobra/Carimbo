// Frontend metadata for clipboard content types and transforms. Mirrors the
// Rust core::classify::ContentType and core::transform::TransformKind sets so
// the UI (badges, "paste as…" menu, type actions) stays in lockstep with the
// backend. Labels resolve at render time via i18n so they follow the language.

import type { ClipEntry, ContentType, TransformKind } from "./types";
import type { StringKey } from "./i18n/en";
import { formatHsl, formatRgb, hexToRgb, rgbToHex } from "./color";

/** The ordered transforms offered in a "paste as… / copy as…" menu. `plain`
    leads because "paste as plain text" is the most common ask. */
export const TRANSFORMS: { kind: TransformKind; labelKey: StringKey }[] = [
  { kind: "plain", labelKey: "transform.plain" },
  { kind: "upperCase", labelKey: "transform.upperCase" },
  { kind: "lowerCase", labelKey: "transform.lowerCase" },
  { kind: "titleCase", labelKey: "transform.titleCase" },
  { kind: "trim", labelKey: "transform.trim" },
  { kind: "singleLine", labelKey: "transform.singleLine" },
  { kind: "slug", labelKey: "transform.slug" },
  { kind: "base64Encode", labelKey: "transform.base64Encode" },
  { kind: "base64Decode", labelKey: "transform.base64Decode" },
];

export type ClipActionKind = "open" | "reveal" | null;

interface ContentTypeMeta {
  /** i18n key for the badge label. `null` = no badge (plain text / image). */
  badgeKey: StringKey | null;
  /** Icon name for the type-specific primary action, if any. */
  icon: "external-link" | "mail" | "folder-input" | null;
  /** Which primary action the row affords beyond copy/paste. */
  action: ClipActionKind;
}

const META: Record<ContentType, ContentTypeMeta> = {
  text: { badgeKey: null, icon: null, action: null },
  image: { badgeKey: null, icon: null, action: null },
  url: { badgeKey: "clipboard.type.url", icon: "external-link", action: "open" },
  email: { badgeKey: "clipboard.type.email", icon: "mail", action: "open" },
  color: { badgeKey: "clipboard.type.color", icon: null, action: null },
  path: {
    badgeKey: "clipboard.type.path",
    icon: "folder-input",
    action: "reveal",
  },
  files: {
    badgeKey: "clipboard.type.files",
    icon: "folder-input",
    action: "reveal",
  },
};

export function contentTypeMeta(type: ContentType): ContentTypeMeta {
  return META[type] ?? META.text;
}

/** A hex color a `color` clip represents, for a swatch. Returns the clip's
    content trimmed if it's a valid CSS hex color, else null. */
export function hexColor(content: string | null): string | null {
  if (!content) return null;
  const s = content.trim();
  return /^#([0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(s)
    ? s
    : null;
}

/**
 * Dynamic, content-aware actions a clip offers beyond plain paste — surfaced in
 * the palette row's more-options menu. Each names an i18n label + icon and a
 * discriminated `run` the palette dispatches on. Mirrors the manager's action
 * cluster but as data, so the palette and the manager can't drift apart.
 *
 * - url / email → open in the browser (or mail client)
 * - path / files → reveal in Explorer
 * - color → paste reformatted as HEX / RGB / HSL (for developers)
 * - image → edit (deferred — shown disabled with a "soon" hint)
 */
export type ClipAction =
  | { id: string; icon: IconName; labelKey: StringKey; run: { t: "openUrl" } }
  | { id: string; icon: IconName; labelKey: StringKey; run: { t: "revealPath" } }
  | {
      id: string;
      icon: IconName;
      labelKey: StringKey;
      /** Paste this already-formatted string into the focused app. */
      run: { t: "pasteText"; text: string };
    }
  | {
      id: string;
      icon: IconName;
      labelKey: StringKey;
      /** Not yet available; rendered disabled with a "soon" note. */
      run: { t: "soon" };
    };

/** Icon names an action may use. Subset of Icon.svelte's `name` union. */
type IconName =
  | "external-link"
  | "mail"
  | "folder-input"
  | "hash"
  | "edit";

export function clipActions(entry: ClipEntry): ClipAction[] {
  switch (entry.contentType) {
    case "url":
      return [
        {
          id: "open",
          icon: "external-link",
          labelKey: "clipboard.openUrl",
          run: { t: "openUrl" },
        },
      ];
    case "email":
      return [
        {
          id: "open",
          icon: "mail",
          labelKey: "clipboard.email",
          run: { t: "openUrl" },
        },
      ];
    case "path":
    case "files":
      return [
        {
          id: "reveal",
          icon: "folder-input",
          labelKey: "clipboard.reveal",
          run: { t: "revealPath" },
        },
      ];
    case "color": {
      const rgb = hexToRgb(entry.content ?? "");
      if (!rgb) return [];
      // Normalize hex (short → long, lowercase) so "paste as HEX" is canonical.
      return [
        {
          id: "hex",
          icon: "hash",
          labelKey: "clipboard.pasteHex",
          run: { t: "pasteText", text: rgbToHex(rgb) },
        },
        {
          id: "rgb",
          icon: "hash",
          labelKey: "clipboard.pasteRgb",
          run: { t: "pasteText", text: formatRgb(rgb) },
        },
        {
          id: "hsl",
          icon: "hash",
          labelKey: "clipboard.pasteHsl",
          run: { t: "pasteText", text: formatHsl(rgb) },
        },
      ];
    }
    case "image":
      return [
        {
          id: "edit",
          icon: "edit",
          labelKey: "clipboard.editImage",
          run: { t: "soon" },
        },
      ];
    default:
      return [];
  }
}
