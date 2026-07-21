//! Disambiguation picker: when the user types a trigger that is confusably
//! similar to others (off by 1–2 letters or case), the expansion engine shows a
//! small list picker instead of guessing. The user chooses; we then delete the
//! typed trigger and paste the chosen snippet into the app they were typing in.
//!
//! Flow (mirrors the palette):
//!   1. Expansion engine detects ambiguity, records `PendingRadial` (target
//!      window + how many trigger chars to delete), calls `show_radial`.
//!   2. `show_radial` positions the pre-created radial window at the cursor,
//!      shows/focuses it, and emits `radial:show` with the candidates.
//!   3. User picks -> `radial_pick` restores focus, deletes the trigger, pastes.
//!      User dismisses -> `radial_dismiss` just restores focus (trigger text is
//!      left as-typed, since we hadn't deleted it yet — nothing is lost).

use tauri::{AppHandle, Manager, Runtime};

use crate::core::error::CoreError;
use crate::state::AppState;

/// Label of the frameless radial window (declared in tauri.conf.json).
pub const RADIAL_WINDOW: &str = "radial";

/// Show the radial with the given candidates. Positions it centered on the
/// cursor so it appears right where the user is typing. Returns false if the
/// radial window doesn't exist. Safe to call from the expansion worker thread.
#[cfg(windows)]
pub fn show_radial<R: Runtime>(
    app: &AppHandle<R>,
    candidates: Vec<crate::expansion_service::RadialCandidate>,
) -> bool {
    use tauri::Emitter;

    let Some(win) = app.get_webview_window(RADIAL_WINDOW) else {
        return false;
    };
    position_on_cursor(&win);
    // Re-assert topmost on every show: Windows can demote an always-on-top
    // window (e.g. after another topmost window appears), and the radial must
    // never open underneath the app the user is typing in.
    let _ = win.set_always_on_top(true);
    let _ = win.show();
    let _ = win.set_focus();
    // Emit AFTER show so the window's listener is mounted; the UI also fetches
    // on mount as a fallback, but the payload is the source of truth.
    let _ = win.emit("radial:show", candidates);
    true
}

/// Insert the chosen snippet into the app that was focused when the radial
/// opened. Deletes the typed trigger first (its length was recorded in
/// `PendingRadial`).
#[tauri::command]
pub fn radial_pick<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<String, CoreError> {
    // Hide the radial first so focus can return to the target.
    if let Some(win) = app.get_webview_window(RADIAL_WINDOW) {
        let _ = win.hide();
    }

    #[cfg(windows)]
    {
        use crate::app_services::inject_method;
        use crate::core::repo::snippet_repo;

        let Some(pending) = state.take_pending_radial() else {
            return Ok("no-pending".into());
        };

        let body = {
            let conn = state.db.lock();
            snippet_repo::get(&conn, &id)?.body
        };

        // Restore the captured window, then delete the trigger + inject. We reuse
        // the same low-level path the direct expansion uses, including the
        // configured inject method (paste/type).
        let method = inject_method(&state);
        if !crate::platform::os::focus::restore_foreground(pending.target) {
            return Ok("focus-lost".into());
        }
        crate::expansion_service::expand_now(
            &app,
            &state.db,
            &body,
            Some(&id),
            pending.delete_chars,
            method,
        );
        return Ok("ok".into());
    }

    #[cfg(not(windows))]
    {
        let _ = (&app, &state, &id);
        Ok("unsupported".into())
    }
}

/// Dismiss the radial without choosing (Esc / blur / click-away). The typed
/// trigger is left in place (we never deleted it), so no input is lost.
#[tauri::command]
pub fn radial_dismiss<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, AppState>) {
    if let Some(win) = app.get_webview_window(RADIAL_WINDOW) {
        let _ = win.hide();
    }
    #[cfg(windows)]
    {
        // Best-effort: put focus back where it was so the user can keep typing.
        if let Some(pending) = state.take_pending_radial() {
            let _ = crate::platform::os::focus::restore_foreground(pending.target);
        }
    }
    #[cfg(not(windows))]
    let _ = &state;
}

/// Anchor the list card just below-right of the cursor, like a popup hanging off
/// the caret, and clamp it to the monitor so it never opens off-screen.
fn position_on_cursor<R: Runtime>(win: &tauri::WebviewWindow<R>) {
    // Small gap so the card doesn't sit directly under the cursor tip.
    const OFFSET: i32 = 12;

    let Ok(size) = win.outer_size() else {
        return;
    };
    let (w, h) = (size.width as i32, size.height as i32);

    if let Some(pos) = win.cursor_position().ok() {
        let (cx, cy) = (pos.x as i32, pos.y as i32);
        // Prefer below-right of the cursor; the monitor bounds decide whether we
        // flip to the other side so the whole card stays visible.
        let (mut x, mut y) = (cx + OFFSET, cy + OFFSET);

        if let Some(mon) = win.monitor_from_point(pos.x, pos.y).ok().flatten() {
            let mpos = mon.position();
            let msize = mon.size();
            let (left, top) = (mpos.x, mpos.y);
            let right = mpos.x + msize.width as i32;
            let bottom = mpos.y + msize.height as i32;

            // Flip above/left of the cursor if there isn't room below/right.
            if x + w > right {
                x = cx - OFFSET - w;
            }
            if y + h > bottom {
                y = cy - OFFSET - h;
            }
            // Final clamp for edge cases (cursor near a corner, tiny monitor).
            x = x.clamp(left, (right - w).max(left));
            y = y.clamp(top, (bottom - h).max(top));
        }

        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        return;
    }

    // Fallback: center on primary monitor.
    if let Some(mon) = win.primary_monitor().ok().flatten() {
        let mpos = mon.position();
        let msize = mon.size();
        let x = mpos.x + ((msize.width as i32 - w) / 2);
        let y = mpos.y + ((msize.height as i32 - h) / 2);
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
}
