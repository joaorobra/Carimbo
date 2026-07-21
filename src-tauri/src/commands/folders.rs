//! Folder commands. Thin layer over `folder_repo`.

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::error::CoreError;
use crate::core::models::{Folder, FolderKind};
use crate::core::repo::folder_repo;
use crate::state::AppState;

pub const FOLDERS_CHANGED: &str = "folders:changed";

fn notify_changed<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(FOLDERS_CHANGED, ());
}

#[tauri::command]
pub fn folders_list(
    state: State<'_, AppState>,
    kind: FolderKind,
) -> Result<Vec<Folder>, CoreError> {
    let conn = state.db.lock();
    folder_repo::list(&conn, kind)
}

#[tauri::command]
pub fn folders_create<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
    kind: FolderKind,
    parent_id: Option<String>,
) -> Result<Folder, CoreError> {
    let folder = {
        let conn = state.db.lock();
        folder_repo::create(&conn, &name, kind, parent_id.as_deref())?
    };
    notify_changed(&app);
    Ok(folder)
}

#[tauri::command]
pub fn folders_rename<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<Folder, CoreError> {
    let folder = {
        let conn = state.db.lock();
        folder_repo::rename(&conn, &id, &name)?
    };
    notify_changed(&app);
    Ok(folder)
}

#[tauri::command]
pub fn folders_delete<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), CoreError> {
    {
        let conn = state.db.lock();
        folder_repo::soft_delete(&conn, &id)?;
    }
    notify_changed(&app);
    Ok(())
}
