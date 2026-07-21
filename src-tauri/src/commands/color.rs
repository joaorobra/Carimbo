//! Screen color picker: pick a pixel color anywhere on screen.
//!
//! Flow (mirrors the radial's window choreography):
//!   1. `color_pick_start` hides the manager (so colors under it are reachable),
//!      shows the pre-created frameless "colorpicker" overlay, and starts the
//!      platform picking session (mouse hook + sampler thread).
//!   2. A consumer thread forwards live samples to the overlay as `color:move`
//!      and repositions it near the cursor — offset far enough that it never
//!      lands inside its own sample grid.
//!   3. Left click picks -> overlay hides, manager returns, `color:picked`.
//!      Esc / right click cancels -> same, but `color:cancelled`.

use serde::Serialize;
use tauri::{AppHandle, Runtime};

use crate::core::error::CoreError;
#[cfg(windows)]
use crate::state::AppState;

/// Label of the frameless live-preview window (declared in tauri.conf.json).
pub const PICKER_WINDOW: &str = "colorpicker";

/// Live payload for the overlay: cursor position plus the pixel grid around it
/// (`size` is odd; `rgb` is row-major R,G,B triples; center = cursor pixel).
#[derive(Clone, Serialize)]
pub struct MovePayload {
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub rgb: Vec<u8>,
}

#[derive(Clone, Serialize)]
pub struct PickedPayload {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Start a screen pick from the global color hotkey: pick a color, then open the
/// manager on the Colors page with it loaded. Best-effort — errors are logged,
/// not surfaced (there's no window in focus to show them in).
#[cfg(windows)]
pub fn start_pick_to_colors<R: Runtime>(app: &AppHandle<R>) {
    use std::sync::atomic::Ordering;
    use tauri::Manager;

    if let Some(state) = app.try_state::<AppState>() {
        // Flag the in-flight pick so `consume` routes the result to the Colors
        // page instead of leaving it in place.
        state.color_pick_to_colors.store(true, Ordering::SeqCst);
    }
    if let Err(e) = color_pick_start(app.clone()) {
        // Session already running (or hook failed): clear the flag so a later
        // in-view pick isn't wrongly routed.
        if let Some(state) = app.try_state::<AppState>() {
            state.color_pick_to_colors.store(false, Ordering::SeqCst);
        }
        tracing::warn!("color hotkey pick failed to start: {e:?}");
    }
}

/// Start a screen pick. No-op error if a session is already running.
#[tauri::command]
pub fn color_pick_start<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    #[cfg(windows)]
    {
        use tauri::Manager;

        // 11x11 grid, ~30 samples/s.
        let rx = crate::platform::os::color_picker::start(5, 33)
            .map_err(CoreError::Other)?;

        if let Some(main) = app.get_webview_window(crate::tray::MAIN_WINDOW) {
            let _ = main.hide();
        }
        if let Some(overlay) = app.get_webview_window(PICKER_WINDOW) {
            // The overlay is a passive readout: clicks pass through it (the
            // hook decides what they mean) and it must never steal focus from
            // the app whose colors are being sampled.
            let _ = overlay.set_ignore_cursor_events(true);
            prevent_activation(&overlay);
            let _ = overlay.set_always_on_top(true);
            if let Ok(pos) = overlay.cursor_position() {
                position_overlay(&overlay, pos.x as i32, pos.y as i32);
            }
            let _ = overlay.show();
        }

        std::thread::Builder::new()
            .name("carimbo-color-events".into())
            .spawn(move || consume(app, rx))
            .map_err(|e| CoreError::Other(format!("color picker thread: {e}")))?;
    }
    #[cfg(not(windows))]
    let _ = &app;
    Ok(())
}

/// Copy a formatted color value (hex/rgb/hsl/…) to the clipboard. Goes through
/// the same Win32 path as clip copies so history/classification behave alike.
#[tauri::command]
pub fn color_copy(text: String) -> Result<(), CoreError> {
    #[cfg(windows)]
    {
        if !crate::platform::os::clipboard::set_text(&text) {
            return Err(CoreError::Other("could not write to the clipboard".into()));
        }
    }
    #[cfg(not(windows))]
    let _ = &text;
    Ok(())
}

/// Forward picker events to the frontend until the session ends, then restore
/// the windows and report the outcome.
#[cfg(windows)]
fn consume<R: Runtime>(
    app: AppHandle<R>,
    rx: std::sync::mpsc::Receiver<crate::platform::os::color_picker::PickerEvent>,
) {
    use crate::platform::os::color_picker::PickerEvent;
    use tauri::{Emitter, Manager};

    let overlay = app.get_webview_window(PICKER_WINDOW);
    let mut picked: Option<(u8, u8, u8)> = None;

    for ev in rx {
        match ev {
            PickerEvent::Move(s) => {
                if let Some(o) = &overlay {
                    position_overlay(o, s.x, s.y);
                }
                let _ = app.emit_to(
                    PICKER_WINDOW,
                    "color:move",
                    MovePayload {
                        x: s.x,
                        y: s.y,
                        size: s.size,
                        rgb: s.rgb,
                    },
                );
            }
            PickerEvent::Picked(s) => {
                picked = Some(s.center());
                break;
            }
            PickerEvent::Cancelled => break,
        }
    }

    if let Some(o) = &overlay {
        let _ = o.hide();
    }

    // Was this pick started by the color hotkey? If so, route the result to the
    // Colors page (and never emit the in-view `color:picked`/`color:cancelled`,
    // which the ColorTools view would otherwise also act on).
    let to_colors = app
        .try_state::<AppState>()
        .map(|s| {
            s.color_pick_to_colors
                .swap(false, std::sync::atomic::Ordering::SeqCst)
        })
        .unwrap_or(false);

    crate::tray::show_main(&app);

    match (picked, to_colors) {
        // Hotkey pick, got a color: open Colors with it. The manager is now
        // shown; `colors:open` tells App.svelte to switch to the Colors page and
        // ColorTools to load the color.
        (Some((r, g, b)), true) => {
            let _ = app.emit("colors:open", Some(PickedPayload { r, g, b }));
        }
        // Hotkey pick, cancelled: still open Colors (the user asked for it), but
        // with no color to load.
        (None, true) => {
            let _ = app.emit("colors:open", Option::<PickedPayload>::None);
        }
        // In-view pick: the existing behavior — ColorTools listens for these.
        (Some((r, g, b)), false) => {
            let _ = app.emit("color:picked", PickedPayload { r, g, b });
        }
        (None, false) => {
            let _ = app.emit("color:cancelled", ());
        }
    }
}

/// Anchor the overlay below-right of the cursor, flipping/clamping at monitor
/// edges — same idiom as the radial, but with a bigger gap so the overlay stays
/// clear of the sample grid it would otherwise appear in.
#[cfg(windows)]
fn position_overlay<R: Runtime>(win: &tauri::WebviewWindow<R>, cx: i32, cy: i32) {
    const OFFSET: i32 = 28;

    let Ok(size) = win.outer_size() else {
        return;
    };
    let (w, h) = (size.width as i32, size.height as i32);
    let (mut x, mut y) = (cx + OFFSET, cy + OFFSET);

    if let Some(mon) = win.monitor_from_point(cx as f64, cy as f64).ok().flatten() {
        let mpos = mon.position();
        let msize = mon.size();
        let (left, top) = (mpos.x, mpos.y);
        let right = mpos.x + msize.width as i32;
        let bottom = mpos.y + msize.height as i32;

        if x + w > right {
            x = cx - OFFSET - w;
        }
        if y + h > bottom {
            y = cy - OFFSET - h;
        }
        x = x.clamp(left, (right - w).max(left));
        y = y.clamp(top, (bottom - h).max(top));
    }

    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Add WS_EX_NOACTIVATE so `show()` (and stray programmatic focus) never
/// activates the overlay — activation would close menus/dropdowns the user is
/// trying to sample. Best-effort.
#[cfg(windows)]
fn prevent_activation<R: Runtime>(win: &tauri::WebviewWindow<R>) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };

    let Ok(hwnd) = win.hwnd() else {
        return;
    };
    // Tauri links its own `windows` crate version; go through the raw pointer
    // value so the two HWND types never meet.
    let hwnd = windows::Win32::Foundation::HWND(hwnd.0);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
    }
}
