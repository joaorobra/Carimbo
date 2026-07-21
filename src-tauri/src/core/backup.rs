//! Backup export/import: a versioned JSON envelope carrying the user's folders
//! and snippets, so a library can be saved to a file and restored (same machine
//! after a reinstall, or moved to another machine).
//!
//! Import is additive and idempotent: rows are keyed by their stable UUID id, so
//! re-importing the same file changes nothing. A snippet whose trigger collides
//! with one already present is imported without its trigger (never dropped
//! entirely) so no content is ever lost. Clipboard history is intentionally NOT
//! part of a backup — it's transient by design and can hold sensitive pastes.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::core::error::{CoreError, CoreResult};
use crate::core::models::{Folder, Snippet};
use crate::core::repo::{folder_repo, snippet_repo};

/// Bump when the envelope shape changes incompatibly. Import refuses versions it
/// doesn't understand rather than silently mis-reading them.
pub const BACKUP_VERSION: u32 = 1;

/// The on-disk backup document.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// Envelope schema version (see [`BACKUP_VERSION`]).
    pub version: u32,
    /// App identifier, so a stray JSON file isn't mistaken for a backup.
    pub app: String,
    /// When the backup was taken (unix ms). Informational only.
    pub exported_at: i64,
    pub folders: Vec<Folder>,
    pub snippets: Vec<Snippet>,
}

/// What an import did, so the UI can report a clear result.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub folders_added: usize,
    pub snippets_added: usize,
    /// Snippets imported but whose shortcut was dropped due to a conflict.
    pub triggers_dropped: usize,
    /// Rows skipped because they already existed (same id).
    pub skipped: usize,
}

/// Build the backup document from the current library.
pub fn export(conn: &Connection, now_ms: i64) -> CoreResult<Backup> {
    Ok(Backup {
        version: BACKUP_VERSION,
        app: "carimbo".into(),
        exported_at: now_ms,
        folders: folder_repo::list_all(conn)?,
        snippets: snippet_repo::list(conn)?,
    })
}

/// Serialize the backup to pretty JSON (human-inspectable).
pub fn export_json(conn: &Connection, now_ms: i64) -> CoreResult<String> {
    let backup = export(conn, now_ms)?;
    serde_json::to_string_pretty(&backup)
        .map_err(|e| CoreError::Invalid(format!("could not serialize backup: {e}")))
}

/// Parse and apply a backup JSON string. Folders are imported before snippets so
/// snippet `folder_id` references resolve. Returns a per-category count.
pub fn import_json(conn: &Connection, json: &str) -> CoreResult<ImportReport> {
    let backup: Backup = serde_json::from_str(json)
        .map_err(|e| CoreError::Invalid(format!("not a valid backup file: {e}")))?;

    if backup.app != "carimbo" {
        return Err(CoreError::Invalid(
            "this file isn't a Carimbo backup".into(),
        ));
    }
    if backup.version > BACKUP_VERSION {
        return Err(CoreError::Invalid(format!(
            "backup version {} is newer than this app supports ({BACKUP_VERSION})",
            backup.version
        )));
    }

    let mut report = ImportReport::default();

    // Folders first — snippets may reference them.
    for f in &backup.folders {
        if folder_repo::insert_preserving_id(conn, f)? {
            report.folders_added += 1;
        } else {
            report.skipped += 1;
        }
    }

    for s in &backup.snippets {
        match snippet_repo::import_snippet(conn, s)? {
            snippet_repo::ImportOutcome::Inserted => report.snippets_added += 1,
            snippet_repo::ImportOutcome::InsertedTriggerDropped => {
                report.snippets_added += 1;
                report.triggers_dropped += 1;
            }
            snippet_repo::ImportOutcome::Skipped => report.skipped += 1,
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Db;
    use crate::core::models::NewSnippet;

    fn new(name: &str, trigger: Option<&str>, body: &str) -> NewSnippet {
        NewSnippet {
            name: name.into(),
            trigger: trigger.map(|s| s.into()),
            body: body.into(),
            body_html: None,
            folder_id: None,
            is_favorite: false,
        }
    }

    #[test]
    fn export_then_import_into_fresh_db_roundtrips() {
        let src = Db::open_in_memory().unwrap();
        {
            let conn = src.lock();
            snippet_repo::create(&conn, new("A", Some(";a"), "aaa")).unwrap();
            snippet_repo::create(&conn, new("B", Some(";b"), "bbb")).unwrap();
        }
        let json = {
            let conn = src.lock();
            export_json(&conn, 123).unwrap()
        };

        // Import into a brand-new database.
        let dst = Db::open_in_memory().unwrap();
        let report = {
            let conn = dst.lock();
            import_json(&conn, &json).unwrap()
        };
        assert_eq!(report.snippets_added, 2);
        let conn = dst.lock();
        assert_eq!(snippet_repo::list(&conn).unwrap().len(), 2);
    }

    #[test]
    fn reimport_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        {
            let conn = db.lock();
            snippet_repo::create(&conn, new("A", Some(";a"), "aaa")).unwrap();
        }
        let json = {
            let conn = db.lock();
            export_json(&conn, 1).unwrap()
        };
        // Importing back into the SAME db must add nothing (same ids).
        let conn = db.lock();
        let report = import_json(&conn, &json).unwrap();
        assert_eq!(report.snippets_added, 0);
        assert!(report.skipped >= 1);
        assert_eq!(snippet_repo::list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn trigger_conflict_drops_trigger_not_snippet() {
        // Destination already uses ";a"; importing a different snippet that also
        // wants ";a" should still import, minus the trigger.
        let src = Db::open_in_memory().unwrap();
        {
            let conn = src.lock();
            snippet_repo::create(&conn, new("Imported", Some(";a"), "from-backup")).unwrap();
        }
        let json = {
            let conn = src.lock();
            export_json(&conn, 1).unwrap()
        };

        let dst = Db::open_in_memory().unwrap();
        {
            let conn = dst.lock();
            snippet_repo::create(&conn, new("Existing", Some(";a"), "already-here")).unwrap();
        }
        let conn = dst.lock();
        let report = import_json(&conn, &json).unwrap();
        assert_eq!(report.snippets_added, 1);
        assert_eq!(report.triggers_dropped, 1);
        // Both snippets now exist; only one carries ";a".
        let all = snippet_repo::list(&conn).unwrap();
        assert_eq!(all.len(), 2);
        let with_trigger = all.iter().filter(|s| s.trigger.as_deref() == Some(";a")).count();
        assert_eq!(with_trigger, 1);
    }

    #[test]
    fn rejects_non_carimbo_json() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let err = import_json(&conn, r#"{"version":1,"app":"other","exportedAt":0,"folders":[],"snippets":[]}"#).unwrap_err();
        assert!(matches!(err, CoreError::Invalid(_)));
    }

    #[test]
    fn rejects_newer_version() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let err = import_json(&conn, r#"{"version":999,"app":"carimbo","exportedAt":0,"folders":[],"snippets":[]}"#).unwrap_err();
        assert!(matches!(err, CoreError::Invalid(_)));
    }
}
