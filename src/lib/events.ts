// Typed subscriptions to backend events. Returns an unlisten function.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const Events = {
  SnippetsChanged: "snippets:changed",
  FoldersChanged: "folders:changed",
  SettingsChanged: "settings:changed",
  ClipboardNew: "clipboard:new-entry",
  // Emitted by the backend when the keyboard hook can't run (e.g. the focused
  // app is elevated). Expansion is effectively off, so any UI showing its
  // enabled state must re-read status instead of staying stale.
  ExpansionBlocked: "expansion:blocked-elevated",
} as const;

export function onEvent(
  name: string,
  handler: () => void,
): Promise<UnlistenFn> {
  return listen(name, () => handler());
}
