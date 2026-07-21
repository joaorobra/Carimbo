//! User region. Drives locale-sensitive formatting (today: the `{date}` token's
//! order). The UI *language* is a separate axis owned by the frontend — a user
//! can run the app in English while keeping Brazilian date order, or vice versa.
//!
//! Default is the United States (`US`): Carimbo ships US-first and only switches
//! to Brazilian conventions when the user (or the first-run picker) selects it.

use serde::{Deserialize, Serialize};

/// Where the user is. Determines conventions like date ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    /// United States — month/day/year, the default.
    #[default]
    US,
    /// Brazil — day/month/year.
    BR,
}

impl Region {
    /// Parse the JSON-string value stored in settings under `format.region`
    /// (e.g. `"us"` / `"br"`). Unknown/missing values fall back to the default
    /// (US), so a corrupt setting never breaks insertion.
    pub fn from_setting(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim_matches('"').to_ascii_lowercase()).as_deref() {
            Some("br") => Region::BR,
            _ => Region::US,
        }
    }

    /// The settings key the region is persisted under (shared with the frontend).
    pub const SETTING_KEY: &'static str = "format.region";

    /// Read the user's region from the settings table. Any read error or missing
    /// value yields the default (US) — insertion must never fail over a setting.
    pub fn load(conn: &rusqlite::Connection) -> Self {
        let raw = crate::core::repo::settings_repo::get(conn, Self::SETTING_KEY).ok().flatten();
        Region::from_setting(raw.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_us() {
        assert_eq!(Region::default(), Region::US);
    }

    #[test]
    fn parses_setting_values() {
        assert_eq!(Region::from_setting(Some("\"br\"")), Region::BR);
        assert_eq!(Region::from_setting(Some("br")), Region::BR);
        assert_eq!(Region::from_setting(Some("\"BR\"")), Region::BR);
        assert_eq!(Region::from_setting(Some("\"us\"")), Region::US);
        assert_eq!(Region::from_setting(None), Region::US);
        assert_eq!(Region::from_setting(Some("garbage")), Region::US);
    }
}
