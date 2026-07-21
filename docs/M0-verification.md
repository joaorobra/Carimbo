# M0 verification — manual test checklist

The two risk spikes exercise the highest-risk Windows integrations. They compile,
the OS-agnostic matcher passes its unit tests, and the full app + installer build.
The remaining checks below need a human at the keyboard because they involve
pasting/expanding into real applications.

Run everything from `src-tauri/`.

## Automated (already green)

- [x] `cargo test --lib` → 7/7 expansion matcher tests pass
- [x] `cargo check --bins` → lib + both spikes compile, zero warnings
- [x] `cargo tauri build` → produces `carimbo.exe` and `Carimbo_0.1.0_x64-setup.exe`
- [x] `npm run build` / `npm run check` → frontend builds, 0 type/a11y errors

## Spike A — focus capture/restore + paste (`spike_focus`)

```powershell
cargo run --release --bin spike_focus
```

For each target app below: run the command, then within 4 seconds click into the
app and place the caret. Confirm the marker text appears AND your clipboard is
intact afterward.

- [ ] Notepad
- [ ] WordPad / Word
- [ ] Chrome address bar or a text field
- [ ] VS Code editor
- [ ] Windows Terminal (paste-heavy edge case)

Pass criteria: marker lands in ≥3 apps; original clipboard restored every time;
never pastes into the wrong window (if focus can't be restored it aborts safely).

## Spike B — keyboard hook + abbreviation expansion (`spike_hook`)

```powershell
cargo run --release --bin spike_hook
```

Leave the console running. In each app, type `;oi` and confirm it becomes
`Olá, tudo bem?` in place.

- [ ] Notepad
- [ ] Word
- [ ] Chrome text field
- [ ] VS Code
- [ ] Slack / Teams (if available)

Dead-key / ABNT2 checks (critical — must NOT be corrupted by the hook):

- [ ] Type `á` (acute accent then `a`) — composes normally
- [ ] Type `ç`, `ã`, `õ` — all compose normally
- [ ] Type fast for a few seconds — no phantom expansions, no dropped keys

Stability:

- [ ] CPU stays ~0% while idle (Task Manager)
- [ ] Leave it running several minutes — hook keeps working (no silent unhook)

Press Ctrl+C in the console to exit.

## App smoke test

```powershell
cargo tauri dev
```

- [ ] Manager window opens and is themed (try OS light/dark)
- [ ] Tray icon appears; right-click menu has Buscar / Abrir / Sair
- [ ] Closing the manager window hides it to tray (does not quit)
- [ ] Left-clicking the tray shows the palette window (frameless, on top)
- [ ] Palette hides when it loses focus

## Installer test (optional, M6 preview)

- [ ] Run `Carimbo_0.1.0_x64-setup.exe` on this machine or a clean Win10/11 VM
- [ ] App installs per-user without a UAC prompt
- [ ] Uninstalls cleanly
