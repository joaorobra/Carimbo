// Frontend token preview — mirrors core::tokens for the editor's live preview.
// The authoritative expansion happens in Rust at insertion time; this is only
// to show the user roughly what they'll get.

import { t, i18n, type Region } from "./i18n";
import type { StringKey } from "./i18n/en";

export interface TokenInfo {
  token: string;
  /** i18n keys, resolved at render time so the token palette follows the UI
      language. The form-field token uses the region-appropriate example syntax
      via its own `token` value below. */
  labelKey: StringKey;
  descKey: StringKey;
}

export const TOKENS: TokenInfo[] = [
  { token: "{date}", labelKey: "token.date.label", descKey: "token.date.desc" },
  { token: "{time}", labelKey: "token.time.label", descKey: "token.time.desc" },
  {
    token: "{datetime}",
    labelKey: "token.datetime.label",
    descKey: "token.datetime.desc",
  },
  {
    token: "{clipboard}",
    labelKey: "token.clipboard.label",
    descKey: "token.clipboard.desc",
  },
  {
    token: "{cursor}",
    labelKey: "token.cursor.label",
    descKey: "token.cursor.desc",
  },
  { token: "{uuid}", labelKey: "token.uuid.label", descKey: "token.uuid.desc" },
  {
    token: "[[field:Label]]",
    labelKey: "token.field.label",
    descKey: "token.field.desc",
  },
];

/** Resolve a token's label/description in the active language. */
export function tokenLabel(info: TokenInfo): string {
  return t(info.labelKey);
}
export function tokenDesc(info: TokenInfo): string {
  return t(info.descKey);
}

/** A `[[key]]` / `[[key:Label]]` form variable. Mirrors core::tokens::Variable. */
export interface Variable {
  key: string;
  label: string;
}

/** Extract the ordered, de-duplicated form variables in `body`. Mirrors
    core::tokens::extract_variables so the editor and the popup agree on which
    fields a snippet will prompt for. */
export function extractVariables(body: string): Variable[] {
  const vars: Variable[] = [];
  const seen = new Set<string>();
  let i = 0;
  while (i < body.length) {
    if (body.startsWith("[[", i)) {
      const end = body.indexOf("]]", i);
      if (end !== -1) {
        const inner = body.slice(i + 2, end);
        const colon = inner.indexOf(":");
        const key = (colon === -1 ? inner : inner.slice(0, colon)).trim();
        if (key) {
          if (!seen.has(key)) {
            seen.add(key);
            const rawLabel = colon === -1 ? "" : inner.slice(colon + 1).trim();
            vars.push({ key, label: rawLabel || key });
          }
          i = end + 2;
          continue;
        }
      }
    }
    i += 1;
  }
  return vars;
}

/** Format a date shifted by `days`, in the region's order. Mirrors the backend's
    {date±Nunit} formatting so the editor preview matches what gets inserted. */
function formatShiftedDate(days: number, reg: Region): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  const dd = String(d.getDate()).padStart(2, "0");
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const yyyy = d.getFullYear();
  return reg === "br" ? `${dd}/${mm}/${yyyy}` : `${mm}/${dd}/${yyyy}`;
}

/** Parse a `date±N<unit>` token inner (e.g. "date+7d") and return the formatted
    shifted date, or null if it isn't a valid offset. Mirrors core::tokens. */
function shiftedDatePreview(inner: string, reg: Region): string | null {
  const m = /^date([+-])(\d+)([dwmy])$/.exec(inner);
  if (!m) return null;
  const sign = m[1] === "+" ? 1 : -1;
  const n = parseInt(m[2], 10);
  const perUnit = { d: 1, w: 7, m: 30, y: 365 }[m[3] as "d" | "w" | "m" | "y"];
  return formatShiftedDate(sign * n * perUnit, reg);
}

export function previewBody(body: string, region?: Region): string {
  // Default to the user's active region so the editor preview matches what the
  // backend will actually insert (US month-first, Brazil day-first).
  const reg = region ?? i18n.region;
  const now = new Date();
  const dd = String(now.getDate()).padStart(2, "0");
  const mm = String(now.getMonth() + 1).padStart(2, "0");
  const yyyy = now.getFullYear();
  const hh = String(now.getHours()).padStart(2, "0");
  const min = String(now.getMinutes()).padStart(2, "0");
  const date = reg === "br" ? `${dd}/${mm}/${yyyy}` : `${mm}/${dd}/${yyyy}`;
  const time = `${hh}:${min}`;

  // Single left-to-right scan mirroring core::tokens::expand: handles escaped
  // braces and leaves unknown tokens verbatim without fragile replace chains.
  let out = "";
  let i = 0;
  while (i < body.length) {
    if (body.startsWith("{{", i)) {
      out += "{";
      i += 2;
    } else if (body.startsWith("}}", i)) {
      out += "}";
      i += 2;
    } else if (body.startsWith("[[", i)) {
      // Form variable: show its label in angle brackets as a placeholder,
      // e.g. [[nome:Nome do cliente]] -> «Nome do cliente».
      const end = body.indexOf("]]", i);
      if (end === -1) {
        out += body.slice(i);
        break;
      }
      const inner = body.slice(i + 2, end);
      const colon = inner.indexOf(":");
      const key = (colon === -1 ? inner : inner.slice(0, colon)).trim();
      const label = colon === -1 ? "" : inner.slice(colon + 1).trim();
      out += key ? `«${label || key}»` : body.slice(i, end + 2);
      i = end + 2;
    } else if (body[i] === "{") {
      const close = body.indexOf("}", i);
      if (close === -1) {
        out += body.slice(i);
        break;
      }
      const token = body.slice(i, close + 1);
      const inner = body.slice(i + 1, close);
      switch (token) {
        case "{date}":
          out += date;
          break;
        case "{time}":
          out += time;
          break;
        case "{datetime}":
          out += `${date} ${time}`;
          break;
        case "{clipboard}":
          out += "…"; // ellipsis placeholder
          break;
        case "{cursor}":
          out += "▮"; // caret placeholder (emits nothing at insert time)
          break;
        case "{uuid}":
          out += "00000000-0000-4000-8000-000000000000"; // shape placeholder
          break;
        default: {
          const shifted = shiftedDatePreview(inner, reg);
          out += shifted ?? token; // unknown token: verbatim
        }
      }
      i = close + 1;
    } else {
      out += body[i];
      i += 1;
    }
  }
  return out;
}
