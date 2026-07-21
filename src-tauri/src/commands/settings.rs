//! Settings commands. Values are opaque JSON strings from the frontend's point
//! of view; the frontend owns the shape (theme, fontScale, density, motion, …).

use std::collections::HashMap;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::error::CoreError;
use crate::core::repo::settings_repo;
use crate::state::AppState;

pub const SETTINGS_CHANGED: &str = "settings:changed";

/// Return all settings as a key -> raw-JSON-value map.
#[tauri::command]
pub fn settings_get_all(
    state: State<'_, AppState>,
) -> Result<HashMap<String, serde_json::Value>, CoreError> {
    let conn = state.db.lock();
    let raw = settings_repo::get_all(&conn)?;
    // Parse each stored JSON string into a Value so the frontend gets real types.
    let mut out = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let value = serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v));
        out.insert(k, value);
    }
    Ok(out)
}

/// Upsert a single setting. `value` is any JSON value; it's stored serialized.
#[tauri::command]
pub fn settings_set<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), CoreError> {
    let encoded = serde_json::to_string(&value)
        .map_err(|e| CoreError::Invalid(format!("unserializable setting: {e}")))?;
    {
        let conn = state.db.lock();
        settings_repo::set(&conn, &key, &encoded)?;
    }
    // Broadcast so other windows (palette) pick up e.g. a theme change live.
    let _ = app.emit(SETTINGS_CHANGED, &key);
    Ok(())
}
