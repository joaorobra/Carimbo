//! Backup commands: write the library to a JSON file and restore it from one.
//! The frontend picks the file path (native save/open dialog); the backend owns
//! the JSON shape and all disk I/O so serialization stays in one place.

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::backup::{self, ImportReport};
use crate::core::clock::now_ms;
use crate::core::error::CoreError;
use crate::core::import::{self, ForeignImportReport, ImportFormat};
use crate::state::AppState;

/// Serialize the whole library and write it to `path` (chosen by a save dialog).
#[tauri::command]
pub fn backup_export(state: State<'_, AppState>, path: String) -> Result<(), CoreError> {
    let json = {
        let conn = state.db.lock();
        backup::export_json(&conn, now_ms())?
    };
    std::fs::write(&path, json)
        .map_err(|e| CoreError::Invalid(format!("could not write backup file: {e}")))?;
    Ok(())
}

/// Read a backup file at `path` (chosen by an open dialog) and merge it into the
/// library. Emits `snippets:changed` so all views (and the expansion engine)
/// pick up the new snippets. Returns a per-category count for the UI.
#[tauri::command]
pub fn backup_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportReport, CoreError> {
    let json = std::fs::read_to_string(&path)
        .map_err(|e| CoreError::Invalid(format!("could not read backup file: {e}")))?;
    let report = {
        let conn = state.db.lock();
        backup::import_json(&conn, &json)?
    };
    if report.folders_added > 0 || report.snippets_added > 0 {
        let _ = app.emit(crate::commands::snippets::SNIPPETS_CHANGED, ());
    }
    Ok(report)
}

/// Import a snippet library exported from *another* expander (espanso YAML,
/// TextExpander/aText-style CSV, or a plain JSON array) at `path`. The format is
/// taken from the file extension, falling back to a content sniff, unless the
/// caller pins one via `format`. Additive like restore — nothing is overwritten
/// — and a trigger that clashes with an existing one is dropped so the snippet
/// still imports. Emits `snippets:changed` on success.
#[tauri::command]
pub fn snippets_import_foreign<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: String,
    format: Option<ImportFormat>,
) -> Result<ForeignImportReport, CoreError> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| CoreError::Invalid(format!("could not read import file: {e}")))?;
    let format = format
        .or_else(|| ImportFormat::from_extension(&path))
        .unwrap_or_else(|| ImportFormat::sniff(&content));
    let report = {
        let conn = state.db.lock();
        import::import(&conn, &content, format)?
    };
    if report.snippets_added > 0 {
        let _ = app.emit(crate::commands::snippets::SNIPPETS_CHANGED, ());
    }
    Ok(report)
}
