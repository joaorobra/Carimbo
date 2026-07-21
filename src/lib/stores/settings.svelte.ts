// Appearance/settings store. Loads persisted values from the backend, applies
// them to <html>, and persists changes. Shared by both windows.

import { api } from "../api";
import { Events, onEvent } from "../events";
import { retry } from "../retry";
import {
  applyAppearance,
  DEFAULT_APPEARANCE,
  type Appearance,
  type Density,
  type Motion,
  type Theme,
  watchSystemTheme,
} from "../theme";

const KEYS = {
  theme: "appearance.theme",
  fontScale: "appearance.fontScale",
  density: "appearance.density",
  motion: "appearance.motion",
  glassOpacity: "appearance.glassOpacity",
} as const;

class SettingsStore {
  appearance = $state<Appearance>({ ...DEFAULT_APPEARANCE });
  #initialized = false;

  async init(): Promise<void> {
    if (this.#initialized) return;
    // Retry the first load so a cold-start race (backend state not yet managed)
    // doesn't leave appearance stuck on defaults until a manual F5. Latch
    // #initialized only after a successful load.
    await retry(() => this.load());
    this.#initialized = true;
    applyAppearance(this.appearance);
    watchSystemTheme(() => this.appearance);
    // If another window changes a setting, reload and re-apply.
    await onEvent(Events.SettingsChanged, () => this.reload());
  }

  async load(): Promise<void> {
    const all = await api.getAllSettings();
    this.appearance = {
      theme: (all[KEYS.theme] as Theme) ?? DEFAULT_APPEARANCE.theme,
      fontScale:
        typeof all[KEYS.fontScale] === "number"
          ? (all[KEYS.fontScale] as number)
          : DEFAULT_APPEARANCE.fontScale,
      density: (all[KEYS.density] as Density) ?? DEFAULT_APPEARANCE.density,
      motion: (all[KEYS.motion] as Motion) ?? DEFAULT_APPEARANCE.motion,
      glassOpacity:
        typeof all[KEYS.glassOpacity] === "number"
          ? (all[KEYS.glassOpacity] as number)
          : DEFAULT_APPEARANCE.glassOpacity,
    };
  }

  async reload(): Promise<void> {
    await this.load();
    applyAppearance(this.appearance);
  }

  async setTheme(theme: Theme): Promise<void> {
    this.appearance = { ...this.appearance, theme };
    applyAppearance(this.appearance);
    await api.setSetting(KEYS.theme, theme);
  }

  async setFontScale(fontScale: number): Promise<void> {
    this.appearance = { ...this.appearance, fontScale };
    applyAppearance(this.appearance);
    await api.setSetting(KEYS.fontScale, fontScale);
  }

  async setDensity(density: Density): Promise<void> {
    this.appearance = { ...this.appearance, density };
    applyAppearance(this.appearance);
    await api.setSetting(KEYS.density, density);
  }

  async setMotion(motion: Motion): Promise<void> {
    this.appearance = { ...this.appearance, motion };
    applyAppearance(this.appearance);
    await api.setSetting(KEYS.motion, motion);
  }

  async setGlassOpacity(glassOpacity: number): Promise<void> {
    this.appearance = { ...this.appearance, glassOpacity };
    applyAppearance(this.appearance);
    await api.setSetting(KEYS.glassOpacity, glassOpacity);
  }
}

export const settingsStore = new SettingsStore();
