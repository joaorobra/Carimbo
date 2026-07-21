//! Startup/orchestration helpers for the background services (expansion), shared
//! between the app `setup` and the runtime enable/disable command.

#[cfg(windows)]
use tauri::{AppHandle, Manager, Runtime, State};

#[cfg(windows)]
use crate::platform::os::paste::InjectMethod;
#[cfg(windows)]
use crate::state::AppState;

/// Read the configured injection method ("paste" default, or "type").
#[cfg(windows)]
pub fn inject_method(state: &AppState) -> InjectMethod {
    let conn = state.db.lock();
    let raw = crate::core::repo::settings_repo::get(&conn, "expansion.injectMethod")
        .ok()
        .flatten();
    match raw.as_deref() {
        Some("\"type\"") => InjectMethod::Type,
        _ => InjectMethod::Paste,
    }
}

/// Start the expansion service and store it in state. Replaces any existing one.
#[cfg(windows)]
pub fn start_expansion<R: Runtime>(app: &AppHandle<R>, state: &State<'_, AppState>) {
    let method = inject_method(state);
    match crate::expansion_service::ExpansionService::start(
        app.clone(),
        state.db.clone(),
        method,
    ) {
        Ok(svc) => {
            if let Ok(mut guard) = state.expansion.lock() {
                *guard = Some(svc);
            }
            tracing::info!("expansion service started");
        }
        Err(e) => tracing::warn!("expansion service failed to start: {e}"),
    }
}

/// Whether expansion is enabled per settings (default: false — opt-in on first
/// run so the keyboard hook is never silently installed).
#[cfg(windows)]
pub fn expansion_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    let conn = state.db.lock();
    crate::core::repo::settings_repo::get(&conn, "expansion.enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false)
}
