# Carimbo

A fast, intuitive, accessible **snippet manager + clipboard history** toolbelt for people who type for a living. Insert names, phone numbers, IDs, long URLs and signatures instantly — either through a global search palette or by typing an abbreviation that expands in place.

English-first, with a built-in Brazilian Portuguese interface. On first run you pick your region (United States by default, or Brazil): this sets the date format (`mm/dd/yyyy` vs. `dd/mm/yyyy`), the interface language, and a set of ready-to-use example snippets tailored to that region. Both can be changed anytime under **Settings → Language & region**.

Windows-first (Tauri 2). Mac/Linux/Android and optional cloud sync are planned for later phases.

## Status

v1 feature-complete (M0–M6). Snippets with a search palette and abbreviation
expansion, clipboard history, themes/accessibility, onboarding, and NSIS +
portable packaging are all in place. Cloud sync and subscription are phase 2.

## Features

- **Snippets** — create/organize text snippets with optional folders, favorites,
  and dynamic tokens (`{date}`, `{time}`, `{datetime}`, `{clipboard}`).
- **Search palette** — a global hotkey (`Ctrl+Shift+Space`) opens a fast,
  keyboard-driven search anywhere; pick a snippet and it pastes into the app you
  were in.
- **Abbreviation expansion** — type a trigger like `;email` in any app and it
  expands in place (opt-in; installs a keyboard hook).
- **Clipboard history** — automatic capture with search, pin, and image
  thumbnails; honors password-manager privacy formats; configurable retention.
- **Accessible** — light/dark/high-contrast themes, adjustable font size, density
  toggle, reduced-motion, full keyboard navigation, ARIA-driven palette.

## Stack

- **Backend:** Rust + [Tauri 2](https://tauri.app)
- **Frontend:** Svelte 5 + Vite + TypeScript (multi-page: `index.html` = manager window, `palette.html` = palette window)
- **Data (M1+):** SQLite via `rusqlite` with FTS5 search
- **Windows integration:** `windows` crate (clipboard listener, low-level keyboard hook, text injection, foreground-window control)

## Prerequisites

- [Rust](https://rustup.rs/) (stable, MSVC toolchain) with the `x86_64-pc-windows-msvc` target
- Visual Studio Build Tools (MSVC C++ + Windows SDK)
- [Node.js](https://nodejs.org/) 20+
- Tauri CLI: `cargo install tauri-cli --version "^2"`
- WebView2 Runtime (preinstalled on Windows 11)

## Develop

```bash
npm install
cargo tauri dev        # runs Vite + the Tauri app with hot reload
```

## Build

```bash
cargo tauri build      # produces the NSIS installer under src-tauri/target/release/bundle
```

## M0 risk spikes

Two standalone binaries validate the highest-risk Windows integrations before the
architecture is committed. Run them from `src-tauri/`:

```bash
cargo run --bin spike_focus   # foreground capture/restore + paste-into-previous-app
cargo run --bin spike_hook    # low-level keyboard hook + ";oi" -> "Olá, tudo bem?" expansion
```

Each file's top comment documents the manual test procedure. These binaries are
excluded from release bundles.

## Project layout

```text
src/                     Svelte frontend
  main.ts / palette.ts   window bootstraps
  styles/                design tokens + themes (light/dark/high-contrast)
  views/manager/         manager window UI
  views/palette/         palette window UI
src-tauri/
  src/
    lib.rs               app builder: plugins, windows, tray
    core/                OS-agnostic logic (expansion matcher; db/repo later)
    platform/windows/    Win32: clipboard, keyboard hook, injection, focus
    tray.rs              system tray
  spikes/                M0 risk-validation binaries
```
