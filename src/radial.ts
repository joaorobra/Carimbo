import "./styles/base.css";
import { mount } from "svelte";
import Radial from "./views/radial/Radial.svelte";
import { applyAppearance, DEFAULT_APPEARANCE } from "./lib/theme";

// Apply defaults immediately; the settings store (initialized in Radial.svelte)
// loads and re-applies persisted appearance so the radial matches the manager.
applyAppearance(DEFAULT_APPEARANCE);

const target = document.getElementById("radial");
if (!target) throw new Error("#radial mount point missing");

const radial = mount(Radial, { target });

export default radial;
