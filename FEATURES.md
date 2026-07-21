# Carimbo — Features

A fast, accessible **snippet manager + clipboard history** toolbelt for Windows,
built for people who type for a living. Insert names, phone numbers, IDs, URLs and
signatures instantly — either through a global search palette or by typing an
abbreviation that expands in place.

- **Platform:** Windows-first (Tauri 2). Mac/Linux/Android and cloud sync are planned for phase 2.
- **Stack:** Rust + Tauri 2 backend · Svelte 5 + Vite + TypeScript frontend · bundled SQLite (FTS5) · Win32 (`windows` crate) for clipboard, keyboard hook, injection, and focus control.
- **Windows:** four native windows — **manager**, **palette**, **radial** disambiguation picker, and **color-picker** overlay. Frameless with a custom title bar.
- **Status:** v1 feature-complete (M0–M6). Cloud sync and subscription are phase 2.

---

## Snippets

- **CRUD & organization** — snippets have a name, an optional trigger (e.g. `;cpf`), a plain-text body, and an optional rich-text (HTML) body. A live token/variable preview shows in the editor.
- **Folders** — hierarchical folders (nested via `parent_id`), with separate trees for snippets and clipboard items, and per-folder counts.
- **Favorites & frecency** — star favorites; usage count and last-used time rank results in the palette.
- **Rich-text snippets** — when an HTML body is present, insertion puts both formatted (CF_HTML) and plain text on the clipboard, so Outlook / Word / Gmail get formatting while plain targets degrade gracefully.
- **Accent-insensitive search** — full-text search (FTS5, `unicode61 remove_diacritics 2`), so "endereco" matches "endereço".

## Search palette

A global hotkey opens a fast, keyboard-driven popup anywhere.

- **Two tabs** — Snippets and Clipboard, switchable in the popup.
- **Keyboard-first** — arrow keys navigate, Enter inserts, Tab switches tabs, Esc closes. Results group into Favorites vs Others and rank by frecency.
- **Insert vs paste** — snippets are typed into the app you were in; clips are pasted. An "Insert as… / Paste as…" menu applies a text transform without leaving the palette.
- **Fill-in-form variables** — if a snippet contains `[[key:Label]]` form variables, the palette prompts you to fill them before insertion.

## Abbreviation expansion (auto-type)

Type a trigger like `;email` in any app and it expands in place. **Opt-in** — the low-level keyboard hook is never installed silently.

- **Matcher** — a rolling buffer does a suffix match on triggers; case-insensitive, longest-trigger-wins, Unicode-aware backspacing. Triggers hot-reload when snippets change.
- **Runtime control** — enable / disable / pause at any time.
- **Disambiguation radial** — when a typed trigger is confusably similar to others (edit distance ≤ 2), a radial picker shows up to 9 candidates with number-key (1–9) shortcuts.
- **Two injection methods** — paste (Ctrl+V with clipboard save/restore) or synthetic Unicode typing, user-selectable.
- **Per-app exclusion** — a JSON list of executables (e.g. `KeePass.exe`) where expansion never fires; running apps are suggested in Settings. Expansion into elevated windows is blocked safely with a toast.
- **ABNT2 / dead-key safe** — accented composition (`á`, `ç`, `ã`, `õ`) is never corrupted by the hook.

## Clipboard history

- **Automatic capture, zero polling** — a message-only window listens for clipboard updates via `AddClipboardFormatListener`.
- **Privacy-aware** — honors password-manager "don't record" clipboard formats (KeePass / 1Password) and skips Carimbo's own paste writes.
- **Text & images** — text is SHA-256 hashed for dedupe; images are saved as PNG thumbnails (up to 64 MB), with a lightbox preview.
- **Retention** — default 30 days or 500 items, whichever comes first; both configurable. Cleanup runs at startup then hourly.
- **Type classification** — each clip is classified once at capture (`text`, `url`, `email`, `color`, `path`, `files`, `image`), driving a badge and type-specific actions: open a URL, reveal a file path or image in Explorer.
- **Actions** — search, pin, move to folder, delete, copy, paste, transform, and **promote a clip into a snippet**.

## Text transforms

Selectable in palette and clipboard menus: **Plain, UpperCase, LowerCase, TitleCase, Trim, SingleLine, Slug, Base64 Encode, Base64 Decode**.

## Color picker & tools

- **Screen color picker** — a global hotkey (or in-app button) launches a live magnifier overlay that streams an 11×11 pixel grid (~30 samples/s) near the cursor without stealing focus. Left-click picks a color; Esc / right-click cancels.
- **Color tools page** — convert and copy between **hex, rgb, hsl, hsv**, and generate a lighter/darker tint strip around a base color.

## Tokens & variables

Resolved at insertion time:

- `{date}` — local date in region order (US `mm/dd/yyyy`, BR `dd/mm/yyyy`)
- `{date+7d}` / `{date-3d}` — date shifted by days (`d`), weeks (`w`), months (`m`), or years (`y`)
- `{time}` — `HH:MM` · `{datetime}` — date + time
- `{clipboard}` — current clipboard text
- `{uuid}` — a fresh random UUID v4
- `{cursor}` — where the caret lands after insertion (first one honored)
- `[[key]]` / `[[key:Label]]` — form variables prompted before insertion (`Label` is the field caption)
- Escaping — `{{` / `}}` produce literal braces; unknown tokens are left verbatim.

## Settings

A key/value settings store surfaces:

- **Appearance** — theme, font size, density, reduce-motion
- **Language & region** — UI language and region (independent axes)
- **Quick search** — primary palette hotkey, which tab it opens, an optional secondary hotkey, and the color-picker hotkey; hotkeys re-register live with no restart
- **Expansion** — enable toggle, injection method, excluded-apps list
- **Clipboard** — retention days and max item count
- **Backup** — export / import a Carimbo envelope, or import a library from another expander
- **Cloud** — "coming soon" placeholder

### Global hotkey defaults

- **Palette:** `Ctrl+Shift+Space`
- **Color picker:** `Ctrl+Shift+C`
- Optional secondary palette hotkey opens the other tab (none by default).

## Accessibility & themes

- **Four themes** — light, dark, high-contrast light, high-contrast dark, plus "Automatic (OS)".
- **Adjustable** — font-size scaling, density toggle, reduced-motion — applied consistently across manager and palette windows.
- **Keyboard & ARIA** — full keyboard navigation and an ARIA-driven palette.

## Internationalization & region

- **Two UI languages** — English (default) and Brazilian Portuguese, at full key parity, with plural/count interpolation.
- **Region is a separate axis** from UI language — US (default) vs Brazil — driving the `{date}` order and first-run seed content. You can run an English UI with Brazilian date order, or vice versa.

## Onboarding & packaging

- **First-run region gate** — pick US (pre-selected) or Brazil before anything else; Brazil switches the UI to Portuguese and dates to day-first.
- **Seed snippets** — region-appropriate examples (US phone/date/address vs Brazilian CPF/DDD/CEP) that demonstrate triggers, tokens, and form variables.
- **Onboarding guide** — shown when no snippets exist; walks you through creating the first one, then self-dismisses.
- **Backup / import** — round-trips a versioned JSON envelope of folders + snippets (additive, idempotent by UUID; clipboard history excluded). Also imports libraries from other expanders.
- **NSIS installer** — per-user install (no UAC prompt), English + Portuguese (BR); WebView2 installed on demand.
- **Portable mode** — a marker file next to the exe keeps all data in `<exe>\data\`, so Carimbo runs from a USB drive without touching `%APPDATA%`.

## System integration

- **System tray** — left-click opens the palette; the menu opens the manager, toggles features, and quits. Closing the manager hides to tray instead of quitting.
- **Single instance** — a second launch focuses the existing window.
- **Autostart** — launches at boot minimized (via the autostart plugin).
- **Sync-ready data model** — UUID primary keys, unix-ms timestamps, and soft-delete tombstones anticipate phase-2 cloud sync (not yet implemented).
