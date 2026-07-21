// Barrel so callers can `import { t, i18n } from "../lib/i18n"` — TypeScript
// resolves a directory import to `index.ts`, not `index.svelte.ts`. The runes
// live in `./store.svelte`; re-exporting the singleton keeps its reactivity
// (the reactive state is on the instance, not this module).

export {
  i18n,
  t,
  DEFAULT_LANG,
  DEFAULT_REGION,
  LANG_KEY,
  REGION_KEY,
  type Lang,
  type Region,
} from "./store.svelte";
export type { StringKey } from "./en";
