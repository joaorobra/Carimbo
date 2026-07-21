import "./styles/base.css";
import { mount } from "svelte";
import Palette from "./views/palette/Palette.svelte";
import { applyAppearance, DEFAULT_APPEARANCE } from "./lib/theme";

// Apply defaults immediately; the settings store (initialized in Palette.svelte)
// loads and re-applies persisted appearance so the palette matches the manager.
applyAppearance(DEFAULT_APPEARANCE);

const target = document.getElementById("palette");
if (!target) throw new Error("#palette mount point missing");

const palette = mount(Palette, { target });

export default palette;
