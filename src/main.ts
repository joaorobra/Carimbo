import "./styles/base.css";
import { mount } from "svelte";
import App from "./views/manager/App.svelte";
import { applyAppearance, DEFAULT_APPEARANCE } from "./lib/theme";

// Apply defaults synchronously so the window never flashes unstyled; the
// settings store loads persisted appearance and re-applies during App init.
applyAppearance(DEFAULT_APPEARANCE);

const target = document.getElementById("app");
if (!target) throw new Error("#app mount point missing");

const app = mount(App, { target });

export default app;
