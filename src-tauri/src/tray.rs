//! System tray icon and menu. Left-click opens the palette; the menu exposes the
//! manager, feature toggles (wired to real state in M3/M4), and Quit.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

pub const PALETTE_WINDOW: &str = "palette";
pub const MAIN_WINDOW: &str = "main";

/// Tray menu labels. The tray is native and built once at startup, so it can't
/// re-render on a live language switch; we read the persisted `ui.language` and
/// pick a label set, defaulting to English (the app is English-first). A change
/// takes effect on the next launch.
struct TrayLabels {
    open_manager: &'static str,
    open_palette: &'static str,
    toggle_expansion: &'static str,
    quit: &'static str,
}

const TRAY_EN: TrayLabels = TrayLabels {
    open_manager: "Open Carimbo",
    open_palette: "Search snippet…",
    toggle_expansion: "Pause/resume expansion",
    quit: "Quit",
};

const TRAY_PT_BR: TrayLabels = TrayLabels {
    open_manager: "Abrir Carimbo",
    open_palette: "Buscar snippet…",
    toggle_expansion: "Pausar/retomar expansão",
    quit: "Sair",
};

/// Read the persisted UI language (best-effort) and return the matching tray
/// labels. English is the default when the setting is missing/unreadable.
fn tray_labels<R: Runtime>(app: &AppHandle<R>) -> &'static TrayLabels {
    use crate::state::AppState;
    let is_pt = app
        .try_state::<AppState>()
        .and_then(|state| {
            let conn = state.db.lock();
            crate::core::repo::settings_repo::get(&conn, "ui.language").ok()?
        })
        .map(|raw| raw.trim_matches('"') == "pt-BR")
        .unwrap_or(false);
    if is_pt {
        &TRAY_PT_BR
    } else {
        &TRAY_EN
    }
}

/// Build the tray icon. Called once during setup.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let labels = tray_labels(app);
    let open_manager =
        MenuItem::with_id(app, "open_manager", labels.open_manager, true, None::<&str>)?;
    let open_palette =
        MenuItem::with_id(app, "open_palette", labels.open_palette, true, None::<&str>)?;
    let toggle_expansion = MenuItem::with_id(
        app,
        "toggle_expansion",
        labels.toggle_expansion,
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_palette,
            &open_manager,
            &sep,
            &toggle_expansion,
            &sep2,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("carimbo-tray")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("default window icon".into())
        })?)
        .tooltip("Carimbo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_manager" => show_main(app),
            "open_palette" => show_palette(app),
            "toggle_expansion" => toggle_expansion_pause(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left click (button up) opens the palette — the fast path.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_palette(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn show_palette<R: Runtime>(app: &AppHandle<R>) {
    // Route through `open_palette` so the tray path also captures the foreground
    // window (to paste back into) and resets the palette UI, matching the primary
    // hotkey. `None` opens the default (snippets) tab.
    crate::commands::palette::open_palette(app, None);
}

/// Toggle the expansion service's paused state from the tray. No-op if expansion
/// isn't currently enabled (nothing to pause).
#[cfg(windows)]
fn toggle_expansion_pause<R: Runtime>(app: &AppHandle<R>) {
    use crate::state::AppState;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(guard) = state.expansion.lock() {
            if let Some(svc) = guard.as_ref() {
                let now = !svc.is_paused();
                svc.set_paused(now);
                tracing::info!("expansion {}", if now { "paused" } else { "resumed" });
            }
        }
    }
}

#[cfg(not(windows))]
fn toggle_expansion_pause<R: Runtime>(_app: &AppHandle<R>) {}
