//! Palette-hotkey commands: read the current accelerators and change them live.
//!
//! The palette can be opened by up to two global shortcuts:
//!   * `palette.hotkey`  — the primary; opens the palette on the *main* tab.
//!   * `palette.hotkey2` — an optional secondary; opens the *other* tab.
//! `palette.mainTab` ("snippets" | "clipboard") decides which tab is "main".
//!
//! Accelerators are Tauri global-shortcut strings like "Control+Shift+Space".
//! Changing any of these re-registers with the OS immediately (no restart) and
//! persists to settings so the choice survives a relaunch.

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};

use crate::core::error::CoreError;
use crate::core::repo::settings_repo;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyInfo {
    /// The primary accelerator currently in effect.
    pub hotkey: String,
    /// The secondary accelerator, or empty string when none is set.
    pub hotkey2: String,
    /// Which tab the primary opens: "snippets" | "clipboard".
    pub main_tab: String,
    /// The screen-color-picker accelerator, or empty string when none is set.
    pub color_hotkey: String,
    /// The built-in default primary, so the UI can offer "restore default".
    pub default: String,
    /// The built-in default color-picker accelerator.
    pub color_default: String,
}

/// Read a stored JSON-string setting, if present and parseable.
fn read_string(state: &AppState, key: &str) -> Option<String> {
    let conn = state.db.lock();
    settings_repo::get(&conn, key)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<String>(&raw).ok())
}

fn persist_string(state: &AppState, key: &str, value: &str) -> Result<(), CoreError> {
    let conn = state.db.lock();
    let encoded = serde_json::to_string(value)
        .map_err(|e| CoreError::Invalid(format!("serialização: {e}")))?;
    settings_repo::set(&conn, key, &encoded)?;
    Ok(())
}

fn info(state: &AppState) -> HotkeyInfo {
    let hotkey = read_string(state, "palette.hotkey")
        .unwrap_or_else(|| crate::DEFAULT_HOTKEY.to_string());
    let hotkey2 = read_string(state, "palette.hotkey2").unwrap_or_default();
    let main_tab = read_string(state, "palette.mainTab")
        .map(|s| crate::main_tab_for(&s).to_string())
        .unwrap_or_else(|| crate::TAB_SNIPPETS.to_string());
    let color_hotkey = read_string(state, "color.hotkey")
        .unwrap_or_else(|| crate::DEFAULT_COLOR_HOTKEY.to_string());
    HotkeyInfo {
        hotkey,
        hotkey2,
        main_tab,
        color_hotkey,
        default: crate::DEFAULT_HOTKEY.to_string(),
        color_default: crate::DEFAULT_COLOR_HOTKEY.to_string(),
    }
}

/// Return the palette hotkeys + main tab currently in effect (persisted or
/// default).
#[tauri::command]
pub fn hotkey_get(state: State<'_, AppState>) -> HotkeyInfo {
    info(&state)
}

/// Change the palette hotkeys and/or main tab. The three fields are optional;
/// omitting one leaves it unchanged. Values are registered with the OS first and
/// only persisted if registration of the *primary* succeeds (a bad secondary is
/// tolerated — it just won't bind). On failure the previous bindings stay active
/// and a friendly error is returned so the UI can show it. Returns the state now
/// in effect.
#[tauri::command]
pub fn hotkey_set<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    hotkey: Option<String>,
    hotkey2: Option<String>,
    main_tab: Option<String>,
    color_hotkey: Option<String>,
) -> Result<HotkeyInfo, CoreError> {
    // Start from what's currently stored, then apply the provided overrides.
    let cur = info(&state);

    let primary = hotkey.map(|h| h.trim().to_string()).unwrap_or(cur.hotkey);
    if primary.is_empty() {
        return Err(CoreError::Invalid("atalho vazio".into()));
    }
    // A blank secondary clears it; otherwise trim.
    let secondary = hotkey2
        .map(|h| h.trim().to_string())
        .unwrap_or(cur.hotkey2);
    let main = main_tab
        .map(|t| crate::main_tab_for(&t).to_string())
        .unwrap_or(cur.main_tab);
    // A blank color hotkey clears it; otherwise trim.
    let color = color_hotkey
        .map(|h| h.trim().to_string())
        .unwrap_or(cur.color_hotkey);

    // Register first; if the primary fails, don't persist (keep the working set).
    let sec_opt = if secondary.is_empty() {
        None
    } else {
        Some(secondary.as_str())
    };
    crate::apply_palette_hotkeys(&app, &primary, sec_opt, &main)
        .map_err(CoreError::Invalid)?;

    // Apply the color hotkey too. A failure here shouldn't roll back the palette
    // registration that already succeeded, so surface it after persisting the
    // palette bits.
    let color_result = crate::apply_color_hotkey(&app, &color);

    persist_string(&state, "palette.hotkey", &primary)?;
    persist_string(&state, "palette.hotkey2", &secondary)?;
    persist_string(&state, "palette.mainTab", &main)?;
    // Only persist the color hotkey if it registered (or was cleared) cleanly, so
    // a bad combo doesn't get saved and fail again on next launch.
    if color_result.is_ok() {
        persist_string(&state, "color.hotkey", &color)?;
    }
    color_result.map_err(CoreError::Invalid)?;

    Ok(info(&state))
}
