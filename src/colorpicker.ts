import "./styles/base.css";
import { mount } from "svelte";
import PickerOverlay from "./views/colorpicker/PickerOverlay.svelte";
import { applyAppearance, DEFAULT_APPEARANCE } from "./lib/theme";

// Apply defaults immediately; the settings store (initialized in the overlay)
// loads and re-applies persisted appearance so it matches the manager.
applyAppearance(DEFAULT_APPEARANCE);

const target = document.getElementById("colorpicker");
if (!target) throw new Error("#colorpicker mount point missing");

const overlay = mount(PickerOverlay, { target });

export default overlay;
