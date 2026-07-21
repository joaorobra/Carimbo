//! Snippet commands. Thin layer over `snippet_repo` — no SQL here. Each mutation
//! emits `snippets:changed` so all windows (and the future expansion engine)
//! refresh.

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::error::CoreError;
use crate::core::models::{NewSnippet, Snippet, UpdateSnippet};
use crate::core::region::Region;
use crate::core::repo::snippet_repo;
use crate::core::seed;
use crate::state::AppState;

pub const SNIPPETS_CHANGED: &str = "snippets:changed";

fn notify_changed<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(SNIPPETS_CHANGED, ());
}

#[tauri::command]
pub fn snippets_list(state: State<'_, AppState>) -> Result<Vec<Snippet>, CoreError> {
    let conn = state.db.lock();
    snippet_repo::list(&conn)
}

#[tauri::command]
pub fn snippets_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Snippet>, CoreError> {
    let conn = state.db.lock();
    snippet_repo::search(&conn, &query)
}

/// Fetch a single snippet by id. Used by the palette when a typed trigger opens
/// the fill-in form and the UI needs the snippet's name/body.
#[tauri::command]
pub fn snippets_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<Snippet, CoreError> {
    let conn = state.db.lock();
    snippet_repo::get(&conn, &id)
}

#[tauri::command]
pub fn snippets_create<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    input: NewSnippet,
) -> Result<Snippet, CoreError> {
    let snippet = {
        let conn = state.db.lock();
        snippet_repo::create(&conn, input)?
    };
    notify_changed(&app);
    Ok(snippet)
}

#[tauri::command]
pub fn snippets_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    input: UpdateSnippet,
) -> Result<Snippet, CoreError> {
    let snippet = {
        let conn = state.db.lock();
        snippet_repo::update(&conn, input)?
    };
    notify_changed(&app);
    Ok(snippet)
}

#[tauri::command]
pub fn snippets_delete<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), CoreError> {
    {
        let conn = state.db.lock();
        snippet_repo::soft_delete(&conn, &id)?;
    }
    notify_changed(&app);
    Ok(())
}

/// Seed the region-specific example snippets on first run. Idempotent and safe
/// to call repeatedly: it only seeds when the library is empty, so a user who
/// deletes the examples never gets them back, and a double-invoke (e.g. the
/// first-run picker racing with something) can't duplicate them. A seed row
/// whose trigger happens to collide with an existing one is skipped rather than
/// failing the whole batch. Returns the number of snippets actually inserted.
#[tauri::command]
pub fn snippets_seed_defaults<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    region: Region,
) -> Result<usize, CoreError> {
    let inserted = {
        let conn = state.db.lock();
        // Only seed an empty library — never add to a library the user has
        // started building (or intentionally emptied).
        if !snippet_repo::list(&conn)?.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        for example in seed::examples_for(region) {
            match snippet_repo::create(&conn, example) {
                Ok(_) => count += 1,
                // A duplicate trigger shouldn't abort the whole seed; skip it.
                Err(CoreError::DuplicateTrigger(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        count
    };
    if inserted > 0 {
        notify_changed(&app);
    }
    Ok(inserted)
}

/// Toggle favorite without a full update payload (used by the star button).
#[tauri::command]
pub fn snippets_set_favorite<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    is_favorite: bool,
) -> Result<Snippet, CoreError> {
    let snippet = {
        let conn = state.db.lock();
        let current = snippet_repo::get(&conn, &id)?;
        snippet_repo::update(
            &conn,
            UpdateSnippet {
                id: current.id,
                name: current.name,
                trigger: current.trigger,
                body: current.body,
                body_html: current.body_html,
                folder_id: current.folder_id,
                is_favorite,
            },
        )?
    };
    notify_changed(&app);
    Ok(snippet)
}
