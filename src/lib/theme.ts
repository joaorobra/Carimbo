// Applies theme + accessibility preferences to <html>. Shared by both windows
// so the palette and manager always look consistent.

export type Theme = "system" | "light" | "dark" | "hc-light" | "hc-dark";
export type Density = "compact" | "comfortable";
export type Motion = "system" | "reduce";

export interface Appearance {
  theme: Theme;
  fontScale: number; // 0.875 .. 1.5
  density: Density;
  motion: Motion;
  /** Opacity of the frosted palette/radial window glass, 0.4 (very see-through)
      .. 1 (fully solid). Drives the `--glass-opacity` alpha; the default keeps
      the premium translucent look while letting users dial the background all
      the way solid. */
  glassOpacity: number;
}

/** Range of the palette-window transparency slider. `MAX` = fully opaque glass. */
export const GLASS_OPACITY_MIN = 0.4;
export const GLASS_OPACITY_MAX = 1;

export const DEFAULT_APPEARANCE: Appearance = {
  theme: "system",
  fontScale: 1,
  density: "compact",
  motion: "system",
  glassOpacity: 0.72,
};

function resolveSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function applyAppearance(a: Appearance): void {
  const root = document.documentElement;
  const theme = a.theme === "system" ? resolveSystemTheme() : a.theme;
  root.setAttribute("data-theme", theme);
  root.setAttribute("data-density", a.density);
  root.setAttribute("data-motion", a.motion === "reduce" ? "reduce" : "system");
  // Clamp defensively — a bad stored value must never make text unreadable.
  const scale = Math.min(1.5, Math.max(0.875, a.fontScale || 1));
  root.style.setProperty("--font-scale", String(scale));
  // Palette/radial glass alpha. Clamp so a stray stored value can't produce an
  // invisible (0) or out-of-range surface; the high-contrast themes ignore this
  // (they pin --surface-glass to a solid color regardless).
  const glass = Math.min(
    GLASS_OPACITY_MAX,
    Math.max(GLASS_OPACITY_MIN, a.glassOpacity || DEFAULT_APPEARANCE.glassOpacity),
  );
  root.style.setProperty("--glass-opacity", String(glass));
}

// Re-apply on OS theme change when following the system.
export function watchSystemTheme(getAppearance: () => Appearance): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    const a = getAppearance();
    if (a.theme === "system") applyAppearance(a);
  };
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}
