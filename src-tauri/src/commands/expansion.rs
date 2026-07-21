//! Commands controlling the abbreviation-expansion service.

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};

use crate::core::error::CoreError;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionStatus {
    pub enabled: bool,
    pub paused: bool,
    /// Whether the running platform supports expansion at all.
    pub supported: bool,
}

#[tauri::command]
pub fn expansion_status(state: State<'_, AppState>) -> ExpansionStatus {
    #[cfg(windows)]
    {
        let guard = state.expansion.lock().ok();
        let (enabled, paused) = match guard.as_ref().and_then(|g| g.as_ref()) {
            Some(svc) => (true, svc.is_paused()),
            None => (false, false),
        };
        ExpansionStatus {
            enabled,
            paused,
            supported: true,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        ExpansionStatus {
            enabled: false,
            paused: false,
            supported: false,
        }
    }
}

/// Enable or disable expansion. Persists the choice and installs/uninstalls the
/// keyboard hook at runtime.
#[tauri::command]
pub fn expansion_set_enabled<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ExpansionStatus, CoreError> {
    // Persist first so a restart honours the choice.
    {
        let conn = state.db.lock();
        crate::core::repo::settings_repo::set(
            &conn,
            "expansion.enabled",
            if enabled { "true" } else { "false" },
        )?;
    }

    #[cfg(windows)]
    {
        if enabled {
            crate::app_services::start_expansion(&app, &state);
        } else if let Ok(mut guard) = state.expansion.lock() {
            *guard = None; // drop -> uninstall hook
        }
    }
    #[cfg(not(windows))]
    let _ = &app;

    Ok(expansion_status(state))
}

/// Get the user's per-app exclusion list (executable base names in which
/// expansion never fires).
#[tauri::command]
pub fn expansion_get_excluded(state: State<'_, AppState>) -> Vec<String> {
    let conn = state.db.lock();
    crate::core::repo::settings_repo::excluded_apps(&conn)
}

/// Replace the exclusion list. Names are trimmed, de-duplicated case-insensitively,
/// and empty entries dropped. Persisted under `expansion.excludedApps`; the
/// running service reads it live on each keystroke, so no restart is needed.
#[tauri::command]
pub fn expansion_set_excluded(
    state: State<'_, AppState>,
    apps: Vec<String>,
) -> Result<Vec<String>, CoreError> {
    // Normalize: trim, drop blanks, dedupe case-insensitively (keep first form).
    let mut seen = std::collections::HashSet::new();
    let cleaned: Vec<String> = apps
        .into_iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .filter(|a| seen.insert(a.to_lowercase()))
        .collect();
    let encoded = serde_json::to_string(&cleaned)
        .map_err(|e| CoreError::Invalid(format!("unserializable list: {e}")))?;
    {
        let conn = state.db.lock();
        crate::core::repo::settings_repo::set(&conn, "expansion.excludedApps", &encoded)?;
    }
    Ok(cleaned)
}

/// Executable names seen in recent clipboard history (`source_app`), offered as
/// quick picks for the exclusion list so users don't have to type exe names.
/// Deduplicated, sorted, most-common-looking first isn't needed — alphabetical
/// is predictable for a settings picker.
#[tauri::command]
pub fn expansion_known_apps(state: State<'_, AppState>) -> Result<Vec<String>, CoreError> {
    let conn = state.db.lock();
    crate::core::repo::clip_repo::distinct_source_apps(&conn)
}

/// Pause/resume expansion without uninstalling the hook (fast tray toggle).
#[tauri::command]
pub fn expansion_set_paused(
    state: State<'_, AppState>,
    paused: bool,
) -> ExpansionStatus {
    #[cfg(windows)]
    {
        if let Ok(guard) = state.expansion.lock() {
            if let Some(svc) = guard.as_ref() {
                svc.set_paused(paused);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = paused;
    expansion_status(state)
}
