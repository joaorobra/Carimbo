//! Folder CRUD. All deletes are soft (set deleted_at) for sync-readiness.

use rusqlite::{params, Connection, Row};

use crate::core::clock::{new_id, now_ms};
use crate::core::error::{CoreError, CoreResult};
use crate::core::models::{Folder, FolderKind};

fn row_to_folder(row: &Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: FolderKind::from_str(&row.get::<_, String>("kind")?),
        parent_id: row.get("parent_id")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// List live folders of a kind, ordered for display.
pub fn list(conn: &Connection, kind: FolderKind) -> CoreResult<Vec<Folder>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, parent_id, sort_order, created_at, updated_at
         FROM folders
         WHERE deleted_at IS NULL AND kind = ?1
         ORDER BY sort_order, name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map(params![kind.as_str()], row_to_folder)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every live folder of both kinds, for a full backup export.
pub fn list_all(conn: &Connection) -> CoreResult<Vec<Folder>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, parent_id, sort_order, created_at, updated_at
         FROM folders WHERE deleted_at IS NULL
         ORDER BY sort_order, name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([], row_to_folder)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Insert (or, if a live row with `id` already exists, leave untouched) a folder,
/// preserving its original id so snippet `folder_id` references from the same
/// backup still resolve. Used by import. Returns true if a row was inserted.
pub fn insert_preserving_id(conn: &Connection, f: &Folder) -> CoreResult<bool> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM folders WHERE id = ?1 AND deleted_at IS NULL",
            params![f.id],
            |_| Ok(()),
        )
        .is_ok();
    if exists {
        return Ok(false);
    }
    conn.execute(
        "INSERT OR REPLACE INTO folders
           (id, name, kind, parent_id, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            f.id,
            f.name,
            f.kind.as_str(),
            f.parent_id,
            f.sort_order,
            f.created_at,
            f.updated_at,
        ],
    )?;
    Ok(true)
}

pub fn create(
    conn: &Connection,
    name: &str,
    kind: FolderKind,
    parent_id: Option<&str>,
) -> CoreResult<Folder> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("folder name is empty".into()));
    }
    let id = new_id();
    let now = now_ms();
    conn.execute(
        "INSERT INTO folders (id, name, kind, parent_id, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
        params![id, name, kind.as_str(), parent_id, now],
    )?;
    get(conn, &id)
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> CoreResult<Folder> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("folder name is empty".into()));
    }
    let changed = conn.execute(
        "UPDATE folders SET name = ?2, updated_at = ?3
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, name, now_ms()],
    )?;
    if changed == 0 {
        return Err(CoreError::NotFound);
    }
    get(conn, id)
}

/// Soft-delete a folder. Snippets referencing it keep their folder_id but the
/// folder no longer appears; callers can reassign snippets first if desired.
pub fn soft_delete(conn: &Connection, id: &str) -> CoreResult<()> {
    let changed = conn.execute(
        "UPDATE folders SET deleted_at = ?2, updated_at = ?2
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now_ms()],
    )?;
    if changed == 0 {
        return Err(CoreError::NotFound);
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> CoreResult<Folder> {
    conn.query_row(
        "SELECT id, name, kind, parent_id, sort_order, created_at, updated_at
         FROM folders WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_folder,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound,
        other => CoreError::Sqlite(other),
    })
}
