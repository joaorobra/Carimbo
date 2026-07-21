// Lightweight reactive i18n for both windows.
//
// English is the default and the fallback; Brazilian Portuguese is opt-in via the
// `ui.language` setting. The active language and the user's `format.region` live
// in one small reactive store so the whole UI re-renders on a switch and both
// windows stay in sync (a change in the manager broadcasts `settings:changed`,
// which the palette/radial pick up).
//
// Usage in a component:
//   import { i18n, t } from "../../lib/i18n";
//   $effect(() => { i18n.init(); });        // once, like settingsStore
//   <h1>{t("nav.settings")}</h1>
// `t()` reads the reactive `lang`, so it re-runs when the language changes.

import { api } from "../api";
import { Events, onEvent } from "../events";
import { retry } from "../retry";
import { en, type StringKey } from "./en";
import { ptBR } from "./pt-BR";

export type Lang = "en" | "pt-BR";
export type Region = "us" | "br";

export const DEFAULT_LANG: Lang = "en";
export const DEFAULT_REGION: Region = "us";

/** Settings keys shared with the backend. `format.region` matches Region::SETTING_KEY. */
export const LANG_KEY = "ui.language";
export const REGION_KEY = "format.region";

const CATALOGS: Record<Lang, Record<StringKey, string>> = {
  en,
  "pt-BR": ptBR,
};

/** Substitute {name} placeholders in a template from `params`. */
function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (whole, key) =>
    key in params ? String(params[key]) : whole,
  );
}

function normalizeLang(v: unknown): Lang {
  return v === "pt-BR" ? "pt-BR" : "en";
}
function normalizeRegion(v: unknown): Region {
  return v === "br" ? "br" : "us";
}

class I18nStore {
  lang = $state<Lang>(DEFAULT_LANG);
  region = $state<Region>(DEFAULT_REGION);
  #initialized = false;

  async init(): Promise<void> {
    if (this.#initialized) return;
    // Retry the first load so a cold-start race doesn't strand the UI on
    // defaults (same guard settingsStore uses).
    await retry(() => this.load());
    this.#initialized = true;
    document.documentElement.setAttribute("lang", this.lang);
    // Live-switch when another window changes the language/region.
    await onEvent(Events.SettingsChanged, () => this.reload());
  }

  async load(): Promise<void> {
    const all = await api.getAllSettings();
    this.lang = normalizeLang(all[LANG_KEY]);
    this.region = normalizeRegion(all[REGION_KEY]);
  }

  async reload(): Promise<void> {
    await this.load();
    document.documentElement.setAttribute("lang", this.lang);
  }

  async setLang(lang: Lang): Promise<void> {
    this.lang = lang;
    document.documentElement.setAttribute("lang", lang);
    await api.setSetting(LANG_KEY, lang);
  }

  async setRegion(region: Region): Promise<void> {
    this.region = region;
    await api.setSetting(REGION_KEY, region);
  }

  /** Translate `key`, filling {placeholders} from `params`. Falls back to the
      English string, then to the raw key, so a UI element is never blank. */
  t(key: StringKey, params?: Record<string, string | number>): string {
    const template = CATALOGS[this.lang][key] ?? en[key] ?? key;
    return interpolate(template, params);
  }
}

export const i18n = new I18nStore();

/** Convenience wrapper so components can `import { t }` and call `t(key)`
    directly. Reads the reactive `i18n.lang`, so callers re-render on a switch. */
export function t(key: StringKey, params?: Record<string, string | number>): string {
  return i18n.t(key, params);
}
