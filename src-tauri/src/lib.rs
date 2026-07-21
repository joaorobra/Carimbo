//! Carimbo application library. `main.rs` is a thin shim over `run()`.
//!
//! Module map (grows across milestones — see the plan):
//! * `platform` — OS-specific integrations (Windows in v1)
//! * `core`     — OS-agnostic logic (expansion matcher today; db/repo/etc. later)
//! * `tray`     — system tray icon + menu

pub mod app_services;
pub mod clipboard_service;
pub mod commands;
pub mod core;
#[cfg(windows)]
pub mod expansion_service;
pub mod paths;
pub mod platform;
pub mod state;
pub mod tray;

use std::sync::Arc;

use tauri::{Listener, Manager, WindowEvent};

use crate::core::db::Db;
use crate::state::AppState;

/// Entry point invoked by the binary. Sets up plugins (single-instance MUST be
/// first), windows, and the tray.
pub fn run() {
    // Keep the file-appender guard alive for the process lifetime.
    let _log_guard = init_logging();

    tauri::Builder::default()
        // Single-instance MUST be registered before any other plugin. When a
        // second launch happens, focus the existing manager window.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            crate::tray::show_main(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::snippets::snippets_list,
            commands::snippets::snippets_search,
            commands::snippets::snippets_get,
            commands::snippets::snippets_create,
            commands::snippets::snippets_update,
            commands::snippets::snippets_delete,
            commands::snippets::snippets_set_favorite,
            commands::snippets::snippets_seed_defaults,
            commands::folders::folders_list,
            commands::folders::folders_create,
            commands::folders::folders_rename,
            commands::folders::folders_delete,
            commands::settings::settings_get_all,
            commands::settings::settings_set,
            commands::hotkey::hotkey_get,
            commands::hotkey::hotkey_set,
            commands::palette::palette_hide,
            commands::palette::palette_insert,
            commands::palette::palette_variables,
            commands::radial::radial_pick,
            commands::radial::radial_dismiss,
            commands::clipboard::clips_list,
            commands::clipboard::clips_search,
            commands::clipboard::clips_set_pinned,
            commands::clipboard::clips_set_folder,
            commands::clipboard::clips_delete,
            commands::clipboard::clips_copy,
            commands::clipboard::clips_paste,
            commands::clipboard::clips_paste_text,
            commands::clipboard::clips_transform,
            commands::clipboard::clips_promote_to_snippet,
            commands::clipboard::clips_open_url,
            commands::clipboard::clips_reveal_path,
            commands::clipboard::clips_reveal_image,
            commands::expansion::expansion_status,
            commands::expansion::expansion_set_enabled,
            commands::expansion::expansion_set_paused,
            commands::expansion::expansion_get_excluded,
            commands::expansion::expansion_set_excluded,
            commands::expansion::expansion_known_apps,
            commands::color::color_pick_start,
            commands::color::color_copy,
            commands::backup::backup_export,
            commands::backup::backup_import,
            commands::backup::snippets_import_foreign,
        ])
        .setup(|app| {
            // Resolve the data directory (honours portable mode) and open the DB.
            let app_paths = crate::paths::resolve(app.handle())?;
            let db = Arc::new(Db::open(app_paths.db_file())?);
            tracing::info!("data dir: {}", app_paths.data_dir.display());
            app.manage(AppState::new(db.clone()));

            crate::tray::build(app.handle())?;
            register_palette_hotkey(app.handle());

            // Start clipboard history monitoring + retention.
            #[cfg(windows)]
            {
                match crate::clipboard_service::start(
                    app.handle().clone(),
                    db.clone(),
                    app_paths.clone(),
                ) {
                    Ok(monitor) => {
                        if let Some(state) = app.try_state::<AppState>() {
                            state.set_clipboard_monitor(monitor);
                        }
                        tracing::info!("clipboard monitor started");
                    }
                    Err(e) => tracing::warn!("clipboard monitor failed to start: {e}"),
                }
            }

            // Start the expansion service if the user enabled it (opt-in; the
            // keyboard hook is never installed silently on first run).
            #[cfg(windows)]
            {
                if crate::app_services::expansion_enabled(app.handle()) {
                    if let Some(state) = app.try_state::<AppState>() {
                        crate::app_services::start_expansion(app.handle(), &state);
                    }
                }

                // Hot-reload expansion triggers whenever snippets change.
                let app_handle = app.handle().clone();
                app.listen(crate::commands::snippets::SNIPPETS_CHANGED, move |_| {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Ok(guard) = state.expansion.lock() {
                            if let Some(svc) = guard.as_ref() {
                                svc.reload_triggers(&state.db);
                            }
                        }
                    }
                });
            }

            // Intercept the manager window close: hide to tray instead of
            // quitting. The tray "Sair" item is the real quit path.
            if let Some(main) = app.get_webview_window(crate::tray::MAIN_WINDOW) {
                let main_clone = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_clone.hide();
                    }
                });

                // The main window is frameless (decorations:false) so we can draw
                // our own title bar. Windows 11 still paints a 1px border around
                // it in the system accent color, which on some setups reads as a
                // stray red/pink frame hugging the window. Ask DWM to drop the
                // border color entirely; rounded corners, resize, and snap all
                // stay intact (unlike making the window transparent). Best-effort:
                // failure (e.g. pre-Win11) just leaves the default border.
                #[cfg(windows)]
                if let Ok(handle) = main.hwnd() {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute, DWMWA_BORDER_COLOR,
                    };
                    // DWMWA_COLOR_NONE — suppress the border rather than tint it.
                    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
                    // Rebuild the HWND from its raw pointer so this doesn't depend
                    // on tauri's `windows` version matching ours.
                    let hwnd = HWND(handle.0 as *mut std::ffi::c_void);
                    unsafe {
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_BORDER_COLOR,
                            (&DWMWA_COLOR_NONE as *const u32).cast(),
                            std::mem::size_of::<u32>() as u32,
                        );
                    }
                }
            }

            // The palette hides itself on blur so it never lingers over other
            // apps. The palette UI also hides on Esc/selection.
            if let Some(palette) = app.get_webview_window(crate::tray::PALETTE_WINDOW) {
                let palette_clone = palette.clone();
                palette.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = palette_clone.hide();
                    }
                });
            }

            // The radial dismisses on blur too. We route through the frontend's
            // dismiss so the captured foreground window is restored (best-effort).
            if let Some(radial) = app.get_webview_window(crate::commands::radial::RADIAL_WINDOW) {
                let radial_clone = radial.clone();
                radial.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = radial_clone.hide();
                        #[cfg(windows)]
                        if let Some(state) =
                            radial_clone.app_handle().try_state::<AppState>()
                        {
                            if let Some(pending) = state.take_pending_radial() {
                                let _ = crate::platform::os::focus::restore_foreground(
                                    pending.target,
                                );
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Carimbo");
}

/// Initialize tracing to both the console and a daily-rolling file under the
/// user's app-data logs dir, so users can attach logs to bug reports. Returns a
/// guard that must be kept alive for the non-blocking writer to flush.
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    // Resolve a logs dir from APPDATA without needing the Tauri app handle (which
    // isn't available this early). Best-effort — fall back to console-only.
    let logs_dir = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Carimbo").join("logs"));

    let file_layer = logs_dir.as_ref().and_then(|dir| {
        std::fs::create_dir_all(dir).ok()?;
        let appender = tracing_appender::rolling::daily(dir, "carimbo.log");
        let (nb, guard) = tracing_appender::non_blocking(appender);
        let layer = fmt::layer().with_ansi(false).with_writer(nb);
        Some((layer, guard))
    });

    match file_layer {
        Some((layer, guard)) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer())
                .with(layer)
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer())
                .init();
            None
        }
    }
}

/// The default palette hotkey. Configurable in settings under `palette.hotkey`.
pub const DEFAULT_HOTKEY: &str = "Control+Shift+Space";

/// The default screen-color-picker hotkey. Configurable in settings under
/// `color.hotkey`. Firing it starts a screen pick immediately; the picked color
/// then opens the manager on the Colors page.
pub const DEFAULT_COLOR_HOTKEY: &str = "Control+Shift+C";

/// The two palette tabs, as the UI names them. The primary hotkey opens the
/// user's chosen main tab; the secondary hotkey opens the other one.
pub const TAB_SNIPPETS: &str = "snippets";
pub const TAB_CLIPBOARD: &str = "clipboard";

/// Read the configured palette hotkeys (or defaults) and register them at
/// startup. Delegates to [`apply_palette_hotkeys`], which records what got
/// registered so the bindings can be changed later without a restart.
fn register_palette_hotkey<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let primary = read_hotkey_setting(app, "palette.hotkey")
        .unwrap_or_else(|| DEFAULT_HOTKEY.to_string());
    let secondary = read_hotkey_setting(app, "palette.hotkey2");
    let main_tab = read_main_tab(app);
    let _ = apply_palette_hotkeys(app, &primary, secondary.as_deref(), &main_tab);

    let color = read_hotkey_setting(app, "color.hotkey")
        .unwrap_or_else(|| DEFAULT_COLOR_HOTKEY.to_string());
    let _ = apply_color_hotkey(app, &color);
}

/// (Re)register the screen-color-picker hotkey. Unregisters whatever we had
/// before (tracked in `AppState.current_color_hotkey`) so this is safe to call
/// repeatedly as the user edits it. On a parse/registration failure the previous
/// binding is left untouched and an `Err` is returned. An empty string clears
/// the hotkey (no binding).
pub fn apply_color_hotkey<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    accel: &str,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let accel = accel.trim();

    // Unregister the previous color hotkey first, so re-applying is clean.
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cur) = state.current_color_hotkey.lock() {
            if let Some(prev) = cur.as_deref() {
                if let Ok(prev_sc) = prev.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    let _ = app.global_shortcut().unregister(prev_sc);
                }
            }
            *cur = None;
        }
    }

    // Empty accelerator means "no color hotkey".
    if accel.is_empty() {
        return Ok(());
    }

    let sc: tauri_plugin_global_shortcut::Shortcut = accel
        .parse()
        .map_err(|e| format!("atalho inválido: {e}"))?;

    app.global_shortcut()
        .on_shortcut(sc, move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::commands::color::start_pick_to_colors(app);
            }
        })
        .map_err(|e| format!("não foi possível registrar {accel:?}: {e}"))?;

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cur) = state.current_color_hotkey.lock() {
            *cur = Some(accel.to_string());
        }
    }
    tracing::info!("color picker hotkey registered: {accel}");
    Ok(())
}

/// Which tab the *primary* hotkey opens ("snippets" | "clipboard"). The default
/// main tab is Snippets, matching the palette's own default.
pub fn main_tab_for(main_tab: &str) -> &'static str {
    if main_tab == TAB_CLIPBOARD {
        TAB_CLIPBOARD
    } else {
        TAB_SNIPPETS
    }
}

/// The tab the *secondary* hotkey opens: whichever one the primary doesn't.
pub fn other_tab(main_tab: &str) -> &'static str {
    if main_tab_for(main_tab) == TAB_SNIPPETS {
        TAB_CLIPBOARD
    } else {
        TAB_SNIPPETS
    }
}

/// (Re)register the palette hotkeys. `primary` opens `main_tab`; `secondary`
/// (when `Some` and non-empty) opens the other tab. Both previously-registered
/// accelerators (tracked in `AppState.current_hotkey` / `current_hotkey2`) are
/// unregistered first, so this is safe to call repeatedly as the user edits the
/// shortcuts. On a parse/registration failure of the *primary* the previous
/// bindings are left untouched and an `Err` is returned; a bad secondary is
/// reported but never blocks the primary.
pub fn apply_palette_hotkeys<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    primary: &str,
    secondary: Option<&str>,
    main_tab: &str,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    // Parse the primary first — if it's invalid, change nothing.
    let primary_sc: tauri_plugin_global_shortcut::Shortcut = primary
        .parse()
        .map_err(|e| format!("atalho inválido: {e}"))?;

    // A secondary equal to the primary is meaningless (one accelerator can't open
    // two tabs); treat that and the empty string as "no secondary".
    let secondary = secondary
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != primary);

    // Unregister whatever we had before (both slots), so re-applying is clean.
    if let Some(state) = app.try_state::<AppState>() {
        for slot in [&state.current_hotkey, &state.current_hotkey2] {
            if let Ok(mut cur) = slot.lock() {
                if let Some(prev) = cur.as_deref() {
                    if let Ok(prev_sc) =
                        prev.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    {
                        let _ = app.global_shortcut().unregister(prev_sc);
                    }
                }
                *cur = None;
            }
        }
    }

    let primary_tab = main_tab_for(main_tab);
    app.global_shortcut()
        .on_shortcut(primary_sc, move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::commands::palette::open_palette(app, Some(primary_tab));
            }
        })
        .map_err(|e| format!("não foi possível registrar {primary:?}: {e}"))?;

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cur) = state.current_hotkey.lock() {
            *cur = Some(primary.to_string());
        }
    }
    tracing::info!("palette hotkey registered: {primary} -> {primary_tab}");

    // Register the secondary if present. A failure here (e.g. combo taken by
    // another app) is logged but not fatal — the primary still works.
    if let Some(sec) = secondary {
        let sec_tab = other_tab(main_tab);
        match sec.parse::<tauri_plugin_global_shortcut::Shortcut>() {
            Ok(sec_sc) => {
                match app.global_shortcut().on_shortcut(
                    sec_sc,
                    move |app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            crate::commands::palette::open_palette(app, Some(sec_tab));
                        }
                    },
                ) {
                    Ok(()) => {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut cur) = state.current_hotkey2.lock() {
                                *cur = Some(sec.to_string());
                            }
                        }
                        tracing::info!("palette hotkey registered: {sec} -> {sec_tab}");
                    }
                    Err(e) => tracing::warn!("secondary hotkey {sec:?} failed: {e}"),
                }
            }
            Err(e) => tracing::warn!("secondary hotkey {sec:?} invalid: {e}"),
        }
    }

    Ok(())
}

/// Read a stored palette accelerator (JSON string) from settings, if present.
fn read_hotkey_setting<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    key: &str,
) -> Option<String> {
    let state = app.try_state::<AppState>()?;
    let conn = state.db.lock();
    let raw = crate::core::repo::settings_repo::get(&conn, key).ok()??;
    // Stored as a JSON string.
    serde_json::from_str::<String>(&raw).ok()
}

/// Read the configured main tab ("snippets" | "clipboard"), defaulting to
/// snippets when unset or unrecognized.
fn read_main_tab<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> String {
    read_hotkey_setting(app, "palette.mainTab")
        .map(|s| main_tab_for(&s).to_string())
        .unwrap_or_else(|| TAB_SNIPPETS.to_string())
}
