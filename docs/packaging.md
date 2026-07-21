# Packaging & release

## Build the installer

```powershell
cargo tauri build
```

Produces, under `src-tauri/target/release/`:

- `carimbo.exe` — the standalone executable (also the base for the portable build)
- `bundle/nsis/Carimbo_<version>_x64-setup.exe` — the NSIS installer

The installer is **per-user** (`installMode: currentUser`) so it needs no UAC
elevation — this matters for the target audience on locked-down office machines,
and keeps us away from the elevated-injection pitfalls (see the expansion engine).
It's localized to Portuguese (BR) and English.

WebView2 is installed on demand via `downloadBootstrapper` (a tiny stub that
fetches the runtime if missing). Windows 11 and updated Windows 10 already have it.

## Build the portable distribution

```powershell
cargo tauri build
pwsh scripts/package-portable.ps1
```

Produces `src-tauri/target/release/Carimbo_<version>_portable_x64.zip` containing
`carimbo.exe`, a `portable` marker file, and a readme. The marker switches Carimbo
into portable mode (data stored in `data\` next to the exe — see
[src-tauri/src/paths.rs](../src-tauri/src/paths.rs)), so it runs from a USB drive
without touching `%APPDATA%`.

The portable exe still requires the WebView2 runtime to be present (it can't
bootstrap it the way the installer does); the readme links Microsoft's installer.

## Code signing (before public release)

Unsigned builds — especially ones that install a low-level keyboard hook — hit
SmartScreen "unknown publisher" warnings and can trigger antivirus heuristics.
Signing is **not required to build**, but strongly recommended before distributing.

Wire it by adding to `src-tauri/tauri.conf.json` under `bundle.windows`:

```json
"certificateThumbprint": "<THUMBPRINT>",
"digestAlgorithm": "sha256",
"timestampUrl": "http://timestamp.digicert.com"
```

Recommended cheapest viable path: **Azure Trusted Signing** (~US$10/month,
supports individual accounts) — integrates with Tauri's `signCommand`. Classic OV
certificates (US$100–400/yr) work but no longer bypass SmartScreen instantly;
reputation builds over downloads. EV certs reach reputation faster but cost more.

Decision can be deferred to the first public release; the config knob above is all
that's needed to turn it on.

## Verification matrix (do before shipping)

- Install on a clean Windows 11 machine → app runs, tray appears, autostart works
- Install on a clean Windows 10 22H2 VM without WebView2 → bootstrapper installs it
- Uninstall → Run-key removed, app gone (data optionally retained)
- Portable zip on a non-install path (e.g. USB) → runs, writes only to its `data\`
