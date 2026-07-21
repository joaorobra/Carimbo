// Color model conversions and formatting for the Colors view. Pure functions,
// no Tauri dependencies, so the math is trivially testable.
//
// Conventions: r/g/b are 0–255 integers; h is 0–360; s/l/v are 0–100. All
// converters round to integers at the boundary — the UI edits whole values.

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface Hsl {
  h: number;
  s: number;
  l: number;
}

const clamp255 = (n: number) => Math.min(255, Math.max(0, Math.round(n)));

/** Parse "#rgb" or "#rrggbb" (leading "#" optional, any case). */
export function hexToRgb(hex: string): Rgb | null {
  const m = hex.trim().replace(/^#/, "");
  if (/^[0-9a-f]{3}$/i.test(m)) {
    return {
      r: parseInt(m[0] + m[0], 16),
      g: parseInt(m[1] + m[1], 16),
      b: parseInt(m[2] + m[2], 16),
    };
  }
  if (/^[0-9a-f]{6}$/i.test(m)) {
    return {
      r: parseInt(m.slice(0, 2), 16),
      g: parseInt(m.slice(2, 4), 16),
      b: parseInt(m.slice(4, 6), 16),
    };
  }
  return null;
}

export function rgbToHex({ r, g, b }: Rgb): string {
  const to2 = (n: number) => clamp255(n).toString(16).padStart(2, "0");
  return `#${to2(r)}${to2(g)}${to2(b)}`;
}

export function rgbToHsl({ r, g, b }: Rgb): Hsl {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  const l = (max + min) / 2;

  let h = 0;
  if (d !== 0) {
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  return { h: Math.round(h), s: Math.round(s * 100), l: Math.round(l * 100) };
}

export function hslToRgb({ h, s, l }: Hsl): Rgb {
  const sn = Math.min(100, Math.max(0, s)) / 100;
  const ln = Math.min(100, Math.max(0, l)) / 100;
  const hn = ((h % 360) + 360) % 360;

  const c = (1 - Math.abs(2 * ln - 1)) * sn;
  const x = c * (1 - Math.abs(((hn / 60) % 2) - 1));
  const m = ln - c / 2;
  let rn = 0;
  let gn = 0;
  let bn = 0;
  if (hn < 60) [rn, gn, bn] = [c, x, 0];
  else if (hn < 120) [rn, gn, bn] = [x, c, 0];
  else if (hn < 180) [rn, gn, bn] = [0, c, x];
  else if (hn < 240) [rn, gn, bn] = [0, x, c];
  else if (hn < 300) [rn, gn, bn] = [x, 0, c];
  else [rn, gn, bn] = [c, 0, x];

  return {
    r: clamp255((rn + m) * 255),
    g: clamp255((gn + m) * 255),
    b: clamp255((bn + m) * 255),
  };
}

export function rgbToHsv({ r, g, b }: Rgb): { h: number; s: number; v: number } {
  const { h } = rgbToHsl({ r, g, b });
  const max = Math.max(r, g, b) / 255;
  const min = Math.min(r, g, b) / 255;
  const s = max === 0 ? 0 : (max - min) / max;
  return { h, s: Math.round(s * 100), v: Math.round(max * 100) };
}

// --- CSS-ready format strings (what the copy buttons place on the clipboard) --

export function formatRgb(c: Rgb): string {
  return `rgb(${clamp255(c.r)}, ${clamp255(c.g)}, ${clamp255(c.b)})`;
}

export function formatHsl(c: Rgb): string {
  const { h, s, l } = rgbToHsl(c);
  return `hsl(${h}, ${s}%, ${l}%)`;
}

export function formatHsv(c: Rgb): string {
  const { h, s, v } = rgbToHsv(c);
  return `hsv(${h}, ${s}%, ${v}%)`;
}

/**
 * The tone strip: `steps` darker tones, the base, `steps` lighter tones
 * (2*steps+1 entries; base sits at index `steps`). Hue/saturation stay fixed;
 * lightness walks evenly toward — but never reaches — black and white, so the
 * extreme tones remain recognizably the same color.
 */
export function toneScale(base: Rgb, steps = 5): Rgb[] {
  const { h, s, l } = rgbToHsl(base);
  const out: Rgb[] = [];
  for (let i = -steps; i <= steps; i++) {
    if (i === 0) {
      out.push(base);
      continue;
    }
    const t = i / (steps + 1);
    const nl = i < 0 ? l + t * l : l + t * (100 - l);
    out.push(hslToRgb({ h, s, l: nl }));
  }
  return out;
}

/** Black or white, whichever reads better on top of `c` (for swatch labels). */
export function contrastText(c: Rgb): string {
  // Relative luminance, sRGB.
  const lin = (n: number) => {
    const x = n / 255;
    return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
  };
  const lum = 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  return lum > 0.4 ? "#000000" : "#ffffff";
}
