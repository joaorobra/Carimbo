//! Settings are a simple key/value store. Values are JSON-encoded strings so any
//! serializable shape round-trips. The frontend reads/writes the whole map or
//! individual keys.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::core::clock::now_ms;
use crate::core::error::CoreResult;

/// Read every setting as a key -> raw-JSON-string map.
pub fn get_all(conn: &Connection) -> CoreResult<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(rows)
}

/// Read a single setting's raw value, if present.
pub fn get(conn: &Connection, key: &str) -> CoreResult<Option<String>> {
    let value = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .ok();
    Ok(value)
}

/// The user's expansion-exclusion list: executable base names (e.g.
/// `KeePass.exe`) in which automatic shortcut expansion should never fire.
/// Stored under `expansion.excludedApps` as a JSON array of strings. Returns an
/// empty vec when unset or unparseable (fail-open — expansion keeps working).
pub fn excluded_apps(conn: &Connection) -> Vec<String> {
    get(conn, "expansion.excludedApps")
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

/// Upsert a single setting.
pub fn set(conn: &Connection, key: &str, value: &str) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now_ms()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Db;

    #[test]
    fn set_then_get_roundtrip_and_upsert() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        set(&conn, "theme", "\"dark\"").unwrap();
        assert_eq!(get(&conn, "theme").unwrap().as_deref(), Some("\"dark\""));
        set(&conn, "theme", "\"hc-light\"").unwrap();
        assert_eq!(get(&conn, "theme").unwrap().as_deref(), Some("\"hc-light\""));
        assert_eq!(get_all(&conn).unwrap().len(), 1);
    }
}
