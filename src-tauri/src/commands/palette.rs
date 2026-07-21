//! Palette commands: positioning/showing the palette window and inserting a
//! chosen snippet into the previously-focused app.
//!
//! Flow:
//!   1. Hotkey fires -> `open_palette` captures the foreground window into state,
//!      positions the pre-created palette at the active monitor, shows + focuses.
//!   2. User searches (snippets_search) and picks a snippet.
//!   3. `palette_insert` hides the palette, restores the captured window, expands
//!      tokens, and pastes.

use std::collections::HashMap;

use tauri::{AppHandle, Manager, Runtime};

use crate::core::error::CoreError;
use crate::core::region::Region;
use crate::core::repo::snippet_repo;
use crate::core::tokens::{self, TokenContext, Variable};
use crate::core::transform::TransformKind;
use crate::state::AppState;
use crate::tray::PALETTE_WINDOW;

/// Show the palette. On Windows this first records the foreground window so we
/// can paste back into it. Called from the global-shortcut handler.
///
/// `tab` selects which tab the palette opens on ("snippets" | "clipboard").
/// `None` lets the UI keep its default (snippets). The primary hotkey passes the
/// user's chosen main tab; the secondary hotkey passes the other one.
pub fn open_palette<R: Runtime>(app: &AppHandle<R>, tab: Option<&str>) {
    #[cfg(windows)]
    {
        // Capture BEFORE showing the palette (showing it steals foreground).
        let target = crate::platform::os::focus::capture_foreground();
        if let Some(state) = app.try_state::<AppState>() {
            state.set_palette_target(target);
        }
    }

    if let Some(win) = app.get_webview_window(PALETTE_WINDOW) {
        position_on_active_monitor(&win);
        let _ = win.show();
        let _ = win.set_focus();
        // Tell the palette UI to reset its query, reload, and open `tab`.
        use tauri::Emitter;
        let _ = win.emit("palette:open", tab);
    }
}

/// Open the palette straight into the fill-in form for `snippet_id`, driven by a
/// *typed trigger* whose snippet has `[[variables]]`. The caller (expansion
/// worker) has already captured the target window and how many trigger chars to
/// delete; we stash both so `palette_insert` can backspace the trigger and paste
/// into the right app once the user fills the form.
#[cfg(windows)]
pub fn open_palette_form<R: Runtime>(
    app: &AppHandle<R>,
    snippet_id: String,
    target: crate::platform::os::focus::CapturedWindow,
    delete_chars: usize,
) {
    if let Some(state) = app.try_state::<AppState>() {
        state.set_palette_target(Some(target));
        state.set_pending_trigger_delete(delete_chars);
    }

    if let Some(win) = app.get_webview_window(PALETTE_WINDOW) {
        position_on_active_monitor(&win);
        // Re-assert topmost: the palette must open over the app being typed in.
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        use tauri::Emitter;
        // Payload is the source of truth; the UI jumps to the form for this id.
        let _ = win.emit("palette:open-form", snippet_id);
    }
}

/// Hide the palette (Esc / blur / after insert).
#[tauri::command]
pub fn palette_hide<R: Runtime>(app: AppHandle<R>) {
    if let Some(win) = app.get_webview_window(PALETTE_WINDOW) {
        let _ = win.hide();
    }
    // A dismissed form-open leaves a stale trigger-delete count; clear it so a
    // later hotkey-driven insert never backspaces characters it shouldn't.
    #[cfg(windows)]
    if let Some(state) = app.try_state::<AppState>() {
        let _ = state.take_pending_trigger_delete();
    }
}

/// Return the form variables (`[[key]]`) a snippet needs filled before it can be
/// inserted, in body order. Empty vec -> insert directly with no prompt.
#[tauri::command]
pub fn palette_variables(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<Variable>, CoreError> {
    let conn = state.db.lock();
    let snippet = snippet_repo::get(&conn, &id)?;
    Ok(tokens::extract_variables(&snippet.body))
}

/// Insert the snippet with `id` into the app that was focused when the palette
/// opened. `variables` carries the values the user filled in for `[[key]]` form
/// variables (empty/omitted when the snippet has none). `transform` is an
/// optional "paste as…" text transform (plain/UPPERCASE/slug/…) chosen from the
/// row's more-options menu; when set, the fully-expanded text is transformed and
/// pasted as plain text (rich HTML is dropped, since the transform operates on
/// the plain string). Returns a short status string the UI can surface.
#[tauri::command]
pub fn palette_insert<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: String,
    variables: Option<HashMap<String, String>>,
    transform: Option<TransformKind>,
) -> Result<String, CoreError> {
    // Fetch the snippet body, optional rich form, and region (drives {date}).
    let (body, body_html, region) = {
        let conn = state.db.lock();
        let snippet = snippet_repo::get(&conn, &id)?;
        (snippet.body, snippet.body_html, Region::load(&conn))
    };

    // Hide the palette first so focus can return to the target.
    if let Some(win) = app.get_webview_window(PALETTE_WINDOW) {
        let _ = win.hide();
    }

    #[cfg(windows)]
    {
        use crate::platform::os::clipboard;
        use crate::platform::os::paste::{
            insert_html_into, insert_into_with_cursor, InjectMethod, InsertResult,
        };

        let Some(target) = state.take_palette_target() else {
            // No target -> also drop any pending trigger-delete so it can't leak
            // into a later insert.
            let _ = state.take_pending_trigger_delete();
            return Ok("no-target".into());
        };

        // When a typed trigger opened this form, the trigger text is still in the
        // target app; backspace it before pasting. Hotkey/tray path -> 0.
        let delete_chars = state.take_pending_trigger_delete().unwrap_or(0);

        // Resolve {clipboard} from the CURRENT clipboard (what the user had
        // before we touch it), only if the body needs it.
        let needs_clip =
            body.contains("{clipboard}") || body_html.as_deref().is_some_and(|h| h.contains("{clipboard}"));
        let ctx = TokenContext {
            clipboard: if needs_clip {
                clipboard::get_text()
            } else {
                None
            },
            variables: variables.unwrap_or_default(),
        };
        let expanded = tokens::expand_full(&body, &ctx, region);

        let result = match (transform, body_html) {
            // "Paste as…" transform chosen: apply it to the expanded plain text and
            // paste as plain text. This bypasses rich HTML (the transform operates
            // on the plain string) and {cursor} (a whole-string transform
            // invalidates caret math).
            (Some(kind), _) => {
                let text = kind.apply(&expanded.text);
                insert_into_with_cursor(
                    target,
                    &text,
                    InjectMethod::Paste,
                    delete_chars,
                    None,
                )
            }
            // Rich snippet: paste HTML with the plain body as the fallback. Tokens
            // are expanded in the HTML too; {cursor} isn't honoured in rich mode
            // (caret math across markup isn't reliable).
            (None, Some(html)) => {
                let html_expanded = tokens::expand(&html, &ctx, region);
                insert_html_into(target, &html_expanded, &expanded.text, delete_chars)
            }
            (None, None) => insert_into_with_cursor(
                target,
                &expanded.text,
                InjectMethod::Paste,
                delete_chars,
                expanded.cursor_from_end,
            ),
        };

        // Record usage on success for palette ranking.
        if matches!(result, InsertResult::Ok) {
            let conn = state.db.lock();
            let _ = snippet_repo::bump_use(&conn, &id);
        }

        return Ok(match result {
            InsertResult::Ok => "ok",
            InsertResult::FocusLost => "focus-lost",
            InsertResult::Elevated => "elevated",
            InsertResult::ClipboardFailed => "clipboard-failed",
        }
        .into());
    }

    #[cfg(not(windows))]
    {
        let _ = (&app, &body, &variables, &transform, region);
        Ok("unsupported".into())
    }
}

/// Center the palette on the monitor that currently contains the cursor, so it
/// appears where the user is working on multi-monitor setups.
fn position_on_active_monitor<R: Runtime>(win: &tauri::WebviewWindow<R>) {
    // Best-effort: fall back to the primary monitor / existing position.
    let cursor = win.cursor_position().ok();
    let monitor = match &cursor {
        Some(pos) => win.monitor_from_point(pos.x, pos.y).ok().flatten(),
        None => None,
    }
    .or_else(|| win.primary_monitor().ok().flatten());

    if let Some(mon) = monitor {
        if let Ok(size) = win.outer_size() {
            let mpos = mon.position();
            let msize = mon.size();
            let x = mpos.x + ((msize.width as i32 - size.width as i32) / 2);
            // Bias slightly above vertical center — palettes read better high.
            let y = mpos.y + ((msize.height as i32 - size.height as i32) / 3);
            let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
}
