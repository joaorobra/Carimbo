// Relative-time formatting, localized. Shared by the clipboard history list and
// the palette's clip rows so the "3 min / 2 h / 1 d" labels stay consistent and
// translate with the UI language.

import { t } from "./i18n";

/** A short "time since `ms`" label in the active language. */
export function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  const min = Math.floor(diff / 60000);
  if (min < 1) return t("time.now");
  if (min < 60) return t("time.minutes", { n: min });
  const h = Math.floor(min / 60);
  if (h < 24) return t("time.hours", { n: h });
  const d = Math.floor(h / 24);
  return t("time.days", { n: d });
}
