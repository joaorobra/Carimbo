//! Clipboard-history commands: list/search/pin/delete/file history entries,
//! transform-and-paste them into the previously focused app, and promote a clip
//! into a reusable snippet.

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::error::CoreError;
use crate::core::models::{ClipEntry, NewSnippet, Snippet};
use crate::core::repo::{clip_repo, snippet_repo};
use crate::core::transform::TransformKind;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 200;

/// Emitted after a mutation that changes which clips exist or how they're filed,
/// so any open history view refreshes. Reuses the capture event name the store
/// already listens on.
const CLIPBOARD_CHANGED: &str = "clipboard:new-entry";

fn notify_changed<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(CLIPBOARD_CHANGED, ());
}

/// Resolve the text a clip should contribute, applying an optional transform.
/// Returns `None` for image clips (no text to act on).
fn transformed_text(entry: &ClipEntry, transform: Option<TransformKind>) -> Option<String> {
    let text = entry.content.as_deref()?;
    Some(match transform {
        Some(kind) => kind.apply(text),
        None => text.to_string(),
    })
}

#[tauri::command]
pub fn clips_list(state: State<'_, AppState>) -> Result<Vec<ClipEntry>, CoreError> {
    let conn = state.db.lock();
    clip_repo::list(&conn, DEFAULT_LIMIT)
}

#[tauri::command]
pub fn clips_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ClipEntry>, CoreError> {
    let conn = state.db.lock();
    clip_repo::search(&conn, &query, DEFAULT_LIMIT)
}

#[tauri::command]
pub fn clips_set_pinned(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<(), CoreError> {
    let conn = state.db.lock();
    clip_repo::set_pinned(&conn, &id, pinned)
}

/// File a clip into a clipboard folder (or unfile it with `folder_id: None`).
/// Filed clips are exempt from retention.
#[tauri::command]
pub fn clips_set_folder<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    folder_id: Option<String>,
) -> Result<(), CoreError> {
    {
        let conn = state.db.lock();
        clip_repo::set_folder(&conn, &id, folder_id.as_deref())?;
    }
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn clips_delete<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), CoreError> {
    let image_path = {
        let conn = state.db.lock();
        clip_repo::soft_delete(&conn, &id)?
    };
    // Best-effort remove the backing image file for an image entry.
    if let Some(p) = image_path {
        let _ = std::fs::remove_file(p);
    }
    notify_changed(&app);
    Ok(())
}

/// Preview a transform without copying/pasting — the UI uses this to show the
/// result in a "paste as…" menu. Returns the transformed text (empty for images).
#[tauri::command]
pub fn clips_transform(
    state: State<'_, AppState>,
    id: String,
    transform: TransformKind,
) -> Result<String, CoreError> {
    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };
    Ok(transformed_text(&entry, Some(transform)).unwrap_or_default())
}

/// Create a reusable snippet from a text clip. The clip's text becomes the body;
/// `name` (or a preview-derived default) names it. Returns the new snippet.
#[tauri::command]
pub fn clips_promote_to_snippet<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    folder_id: Option<String>,
) -> Result<Snippet, CoreError> {
    let snippet = {
        let conn = state.db.lock();
        let entry = clip_repo::get(&conn, &id)?;
        let Some(body) = entry.content else {
            return Err(CoreError::Invalid("cannot promote an image clip".into()));
        };
        // Name: caller-supplied, else the first line trimmed to a sane length.
        let name = name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| {
                let first = body.lines().next().unwrap_or("").trim();
                first.chars().take(40).collect::<String>()
            });
        let name = if name.is_empty() { "Sem título".into() } else { name };
        snippet_repo::create(
            &conn,
            NewSnippet {
                name,
                trigger: None,
                body,
                body_html: None,
                folder_id,
                is_favorite: false,
            },
        )?
    };
    // A new snippet exists — tell snippet views (and the expansion engine) to
    // refresh via the same event the snippet commands emit.
    let _ = app.emit(crate::commands::snippets::SNIPPETS_CHANGED, ());
    Ok(snippet)
}

/// Open a `url`-classified clip in the default browser. Returns whether the
/// shell accepted it. No-op error for non-text clips.
#[tauri::command]
pub fn clips_open_url(state: State<'_, AppState>, id: String) -> Result<bool, CoreError> {
    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };
    let Some(text) = entry.content else {
        return Ok(false);
    };
    let target = text.trim();
    // A mail-style clip (email address, no scheme) opens as a mailto:.
    let target = if entry.content_type == crate::core::classify::ContentType::Email
        && !target.starts_with("mailto:")
    {
        format!("mailto:{target}")
    } else {
        target.to_string()
    };
    #[cfg(windows)]
    {
        return Ok(crate::platform::os::shell::open(&target));
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        Ok(false)
    }
}

/// Reveal a `path`/`files` clip in Explorer, selecting the (first) file.
#[tauri::command]
pub fn clips_reveal_path(state: State<'_, AppState>, id: String) -> Result<bool, CoreError> {
    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };
    let Some(text) = entry.content else {
        return Ok(false);
    };
    // For a multi-path files clip, reveal the first path.
    let path = text.lines().next().unwrap_or("").trim();
    if path.is_empty() {
        return Ok(false);
    }
    #[cfg(windows)]
    {
        return Ok(crate::platform::os::shell::reveal(path));
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(false)
    }
}

/// Reveal an image clip's backing PNG in Explorer, selecting the file — the
/// "show on disk / save a copy" affordance for images (no file dialog needed).
#[tauri::command]
pub fn clips_reveal_image(state: State<'_, AppState>, id: String) -> Result<bool, CoreError> {
    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };
    let Some(path) = entry.image_path else {
        return Ok(false);
    };
    #[cfg(windows)]
    {
        return Ok(crate::platform::os::shell::reveal(&path));
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(false)
    }
}

/// Copy a clip back to the clipboard. Text clips place their content (optionally
/// transformed); image clips place the backing PNG as an image (CF_DIB).
#[tauri::command]
pub fn clips_copy(
    state: State<'_, AppState>,
    id: String,
    transform: Option<TransformKind>,
) -> Result<(), CoreError> {
    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };
    #[cfg(windows)]
    {
        use crate::core::models::ClipKind;
        match entry.kind {
            ClipKind::Image => {
                if let Some(path) = entry.image_path.as_deref() {
                    if !crate::platform::os::clipboard::set_image_from_png(path) {
                        return Err(CoreError::Invalid("failed to copy image".into()));
                    }
                }
            }
            ClipKind::Text => {
                if let Some(text) = transformed_text(&entry, transform) {
                    crate::platform::os::clipboard::set_text(&text);
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (&entry, &transform);
    }
    Ok(())
}

/// Paste a clip into the app that was focused when the palette opened. Text
/// clips paste their content (optionally transformed); image clips paste the
/// backing PNG as an image. Mirrors `palette_insert` but for a clipboard entry.
#[tauri::command]
pub fn clips_paste<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    transform: Option<TransformKind>,
) -> Result<String, CoreError> {
    use crate::core::models::ClipKind;

    let entry = {
        let conn = state.db.lock();
        clip_repo::get(&conn, &id)?
    };

    // Resolve what we'll paste up front so the non-windows branch stays simple.
    let image_path = match entry.kind {
        ClipKind::Image => entry.image_path.clone(),
        ClipKind::Text => None,
    };
    let text = transformed_text(&entry, transform);
    if image_path.is_none() && text.is_none() {
        return Ok("empty".into());
    }

    #[cfg(windows)]
    {
        use tauri::Manager;
        // Hide the palette so focus can return to the target.
        if let Some(win) = app.get_webview_window(crate::tray::PALETTE_WINDOW) {
            let _ = win.hide();
        }

        use crate::platform::os::paste::{
            insert_image_into, insert_into, InjectMethod, InsertResult,
        };
        let Some(target) = state.take_palette_target() else {
            return Ok("no-target".into());
        };
        let result = match image_path {
            Some(path) => insert_image_into(target, &path),
            // text is Some here: image_path is None only for text clips, and we
            // returned "empty" above when both were None.
            None => insert_into(target, text.as_deref().unwrap_or_default(), InjectMethod::Paste, 0),
        };
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
        let _ = (&app, &text, &image_path);
        Ok("unsupported".into())
    }
}

/// Paste a caller-supplied string into the app that was focused when the palette
/// opened. Unlike `clips_paste`, the text isn't tied to a clip's stored content —
/// it's a value the UI derived from a clip, e.g. a `color` clip reformatted as
/// `rgb(…)` / `hsl(…)` / a normalized hex. Shares the hide-palette + take-target
/// + paste flow with `clips_paste` so behavior stays identical.
#[tauri::command]
pub fn clips_paste_text<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    text: String,
) -> Result<String, CoreError> {
    if text.is_empty() {
        return Ok("empty".into());
    }

    #[cfg(windows)]
    {
        use tauri::Manager;
        // Hide the palette so focus can return to the target.
        if let Some(win) = app.get_webview_window(crate::tray::PALETTE_WINDOW) {
            let _ = win.hide();
        }

        use crate::platform::os::paste::{insert_into, InjectMethod, InsertResult};
        let Some(target) = state.take_palette_target() else {
            return Ok("no-target".into());
        };
        return Ok(match insert_into(target, &text, InjectMethod::Paste, 0) {
            InsertResult::Ok => "ok",
            InsertResult::FocusLost => "focus-lost",
            InsertResult::Elevated => "elevated",
            InsertResult::ClipboardFailed => "clipboard-failed",
        }
        .into());
    }

    #[cfg(not(windows))]
    {
        let _ = (&app, &text);
        Ok("unsupported".into())
    }
}
