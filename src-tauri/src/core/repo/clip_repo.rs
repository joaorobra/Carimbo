//! Clipboard-history CRUD, dedupe, and retention. Soft deletes for sync-ready.

use rusqlite::{params, Connection, Row};

use crate::core::classify::ContentType;
use crate::core::clock::{new_id, now_ms};
use crate::core::error::CoreResult;
use crate::core::models::{ClipEntry, ClipKind, NewClip};

const COLUMNS: &str = "id, kind, content_type, content, image_path, preview, is_pinned, \
     folder_id, source_app, created_at, updated_at";

fn row_to_clip(row: &Row) -> rusqlite::Result<ClipEntry> {
    Ok(ClipEntry {
        id: row.get("id")?,
        kind: ClipKind::from_str(&row.get::<_, String>("kind")?),
        content_type: ContentType::from_str(&row.get::<_, String>("content_type")?),
        content: row.get("content")?,
        image_path: row.get("image_path")?,
        preview: row.get("preview")?,
        is_pinned: row.get::<_, i64>("is_pinned")? != 0,
        folder_id: row.get("folder_id")?,
        source_app: row.get("source_app")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Insert a captured clip, deduplicating by content hash. If an identical live
/// entry exists we just bump its `updated_at` (moving it to the top) instead of
/// creating a duplicate. Returns true if a NEW row was created.
pub fn insert_dedup(conn: &Connection, clip: &NewClip) -> CoreResult<bool> {
    let now = now_ms();
    // Look for an existing live entry with the same hash.
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM clip_entries
             WHERE content_hash = ?1 AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
            params![clip.content_hash],
            |r| r.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE clip_entries SET updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        return Ok(false);
    }

    let id = new_id();
    conn.execute(
        "INSERT INTO clip_entries
           (id, kind, content_type, content, image_path, preview, content_hash,
            source_app, is_pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?9)",
        params![
            id,
            clip.kind.as_str(),
            clip.content_type.as_str(),
            clip.content,
            clip.image_path,
            clip.preview,
            clip.content_hash,
            clip.source_app,
            now
        ],
    )?;
    Ok(true)
}

/// List live entries, pinned first then newest, up to `limit`.
pub fn list(conn: &Connection, limit: i64) -> CoreResult<Vec<ClipEntry>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM clip_entries WHERE deleted_at IS NULL
         ORDER BY is_pinned DESC, created_at DESC LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![limit], row_to_clip)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Full-text search over clipboard text. Empty query returns the recent list.
pub fn search(conn: &Connection, query: &str, limit: i64) -> CoreResult<Vec<ClipEntry>> {
    let q = query.trim();
    if q.is_empty() {
        return list(conn, limit);
    }
    let match_expr = q
        .split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ");
    let sql = format!(
        "SELECT c.id, c.kind, c.content_type, c.content, c.image_path, c.preview, \
                c.is_pinned, c.folder_id, c.source_app, c.created_at, c.updated_at
         FROM clip_entries c
         JOIN clips_fts f ON f.rowid = c.rowid
         WHERE clips_fts MATCH ?1 AND c.deleted_at IS NULL
         ORDER BY c.is_pinned DESC, c.created_at DESC LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![match_expr, limit], row_to_clip)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn set_pinned(conn: &Connection, id: &str, pinned: bool) -> CoreResult<()> {
    conn.execute(
        "UPDATE clip_entries SET is_pinned = ?2, updated_at = ?3
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, pinned as i64, now_ms()],
    )?;
    Ok(())
}

/// File a clip into a clipboard folder (or `None` to unfile it). Filed clips are
/// exempt from retention (see [`prune`]), so this doubles as "keep this".
pub fn set_folder(conn: &Connection, id: &str, folder_id: Option<&str>) -> CoreResult<()> {
    conn.execute(
        "UPDATE clip_entries SET folder_id = ?2, updated_at = ?3
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, folder_id, now_ms()],
    )?;
    Ok(())
}

pub fn soft_delete(conn: &Connection, id: &str) -> CoreResult<Option<String>> {
    // Return the image path (if any) so the caller can delete the file.
    let image_path: Option<String> = conn
        .query_row(
            "SELECT image_path FROM clip_entries WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    conn.execute(
        "UPDATE clip_entries SET deleted_at = ?2, updated_at = ?2
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now_ms()],
    )?;
    Ok(image_path)
}

/// Distinct non-null `source_app` values across live clips, alphabetically.
/// Feeds the expansion exclusion-list picker so users can choose from apps
/// they've actually copied from rather than typing exe names by hand.
pub fn distinct_source_apps(conn: &Connection) -> CoreResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT source_app FROM clip_entries
         WHERE deleted_at IS NULL AND source_app IS NOT NULL AND source_app <> ''
         ORDER BY source_app COLLATE NOCASE ASC",
    )?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> CoreResult<ClipEntry> {
    let sql = format!("SELECT {COLUMNS} FROM clip_entries WHERE id = ?1 AND deleted_at IS NULL");
    conn.query_row(&sql, params![id], row_to_clip)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => crate::core::error::CoreError::NotFound,
            other => crate::core::error::CoreError::Sqlite(other),
        })
}

/// Retention: hard-delete unpinned, folderless entries older than `max_age_ms`
/// or beyond `max_items` most-recent. Pinned/foldered entries are exempt.
/// Returns the image paths of removed rows so the caller can delete the files.
pub fn prune(
    conn: &Connection,
    max_age_ms: i64,
    max_items: i64,
) -> CoreResult<Vec<String>> {
    let now = now_ms();
    let cutoff = now - max_age_ms;
    let mut removed_images: Vec<String> = Vec::new();

    // Candidates removable by policy: not pinned, not in a folder, not tombstoned.
    // 1) Too old.
    {
        let mut stmt = conn.prepare(
            "SELECT image_path FROM clip_entries
             WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
               AND created_at < ?1 AND image_path IS NOT NULL",
        )?;
        let imgs = stmt
            .query_map(params![cutoff], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        removed_images.extend(imgs);
    }
    conn.execute(
        "DELETE FROM clip_entries
         WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
           AND created_at < ?1",
        params![cutoff],
    )?;

    // 2) Beyond the item cap (keep the newest `max_items` removable entries).
    {
        let mut stmt = conn.prepare(
            "SELECT image_path FROM clip_entries
             WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
               AND image_path IS NOT NULL
               AND id NOT IN (
                 SELECT id FROM clip_entries
                 WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
                 ORDER BY created_at DESC LIMIT ?1
               )",
        )?;
        let imgs = stmt
            .query_map(params![max_items], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        removed_images.extend(imgs);
    }
    conn.execute(
        "DELETE FROM clip_entries
         WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
           AND id NOT IN (
             SELECT id FROM clip_entries
             WHERE is_pinned = 0 AND folder_id IS NULL AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT ?1
           )",
        params![max_items],
    )?;

    Ok(removed_images)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Db;

    fn text_clip(s: &str, hash: &str) -> NewClip {
        NewClip {
            kind: ClipKind::Text,
            content_type: ContentType::Text,
            content: Some(s.into()),
            image_path: None,
            preview: s.chars().take(50).collect(),
            content_hash: hash.into(),
            source_app: None,
        }
    }

    #[test]
    fn insert_and_list() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        assert!(insert_dedup(&conn, &text_clip("hello", "h1")).unwrap());
        assert!(insert_dedup(&conn, &text_clip("world", "h2")).unwrap());
        assert_eq!(list(&conn, 100).unwrap().len(), 2);
    }

    #[test]
    fn dedupe_bumps_instead_of_duplicating() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        assert!(insert_dedup(&conn, &text_clip("same", "h1")).unwrap());
        // Same hash again: no new row.
        assert!(!insert_dedup(&conn, &text_clip("same", "h1")).unwrap());
        assert_eq!(list(&conn, 100).unwrap().len(), 1);
    }

    #[test]
    fn pin_survives_age_retention() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        insert_dedup(&conn, &text_clip("keep", "h1")).unwrap();
        let entry = &list(&conn, 1).unwrap()[0];
        set_pinned(&conn, &entry.id, true).unwrap();
        // Force it "old" by rewriting created_at far in the past.
        conn.execute(
            "UPDATE clip_entries SET created_at = 0 WHERE id = ?1",
            params![entry.id],
        )
        .unwrap();
        // Prune everything older than "now" — pinned must survive.
        prune(&conn, 1, 500).unwrap();
        assert_eq!(list(&conn, 100).unwrap().len(), 1);
    }

    #[test]
    fn unpinned_old_entries_pruned() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        insert_dedup(&conn, &text_clip("drop", "h1")).unwrap();
        let entry = &list(&conn, 1).unwrap()[0];
        conn.execute(
            "UPDATE clip_entries SET created_at = 0 WHERE id = ?1",
            params![entry.id],
        )
        .unwrap();
        prune(&conn, 1, 500).unwrap();
        assert_eq!(list(&conn, 100).unwrap().len(), 0);
    }

    #[test]
    fn item_cap_prunes_oldest_unpinned() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        for i in 0..5 {
            insert_dedup(&conn, &text_clip(&format!("c{i}"), &format!("h{i}"))).unwrap();
        }
        // Keep only 2 most recent; huge age so only the cap applies.
        prune(&conn, i64::MAX, 2).unwrap();
        assert_eq!(list(&conn, 100).unwrap().len(), 2);
    }

    #[test]
    fn foldered_clip_survives_retention() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        // A folder to file the clip into (kind is irrelevant to the FK).
        conn.execute(
            "INSERT INTO folders (id, name, kind, sort_order, created_at, updated_at)
             VALUES ('f1', 'Kept', 'clipboard', 0, 0, 0)",
            [],
        )
        .unwrap();
        insert_dedup(&conn, &text_clip("keep", "h1")).unwrap();
        let entry = &list(&conn, 1).unwrap()[0];
        set_folder(&conn, &entry.id, Some("f1")).unwrap();
        conn.execute(
            "UPDATE clip_entries SET created_at = 0 WHERE id = ?1",
            params![entry.id],
        )
        .unwrap();
        // Prune everything old — a foldered clip must survive.
        prune(&conn, 1, 500).unwrap();
        assert_eq!(list(&conn, 100).unwrap().len(), 1);
    }

    #[test]
    fn clip_fts_search() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        insert_dedup(&conn, &text_clip("Reunião importante", "h1")).unwrap();
        insert_dedup(&conn, &text_clip("outra coisa", "h2")).unwrap();
        // Accent-insensitive prefix search.
        assert_eq!(search(&conn, "reuniao", 100).unwrap().len(), 1);
    }
}
