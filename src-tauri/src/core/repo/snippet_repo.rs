//! Snippet CRUD + search. Soft deletes for sync-readiness. Trigger uniqueness
//! among live rows is enforced by a partial UNIQUE index; we translate the
//! resulting constraint error into a friendly `DuplicateTrigger`.

use rusqlite::{params, Connection, Row};

use crate::core::clock::{new_id, now_ms};
use crate::core::error::{CoreError, CoreResult};
use crate::core::models::{NewSnippet, Snippet, UpdateSnippet};

const COLUMNS: &str = "id, folder_id, name, trigger, body, body_html, is_favorite, use_count, \
     last_used_at, sort_order, created_at, updated_at";

// Same columns qualified with the `s` alias, for the FTS JOIN where bare `name`,
// `body`, `trigger` would be ambiguous against the FTS table's columns.
const COLUMNS_S: &str = "s.id, s.folder_id, s.name, s.trigger, s.body, s.body_html, s.is_favorite, \
     s.use_count, s.last_used_at, s.sort_order, s.created_at, s.updated_at";

/// A "frecency" score expression (frequency × recency) for ORDER BY. Higher is
/// more relevant. It blends how often a snippet is used (`use_count`) with how
/// recently (`last_used_at`), using Firefox-style bucketed recency weights so it
/// needs only portable CASE/arithmetic — no ln()/exp() SQL math functions that
/// a given SQLite build may lack.
///
/// `{col}` is the qualified column prefix ("" for the plain table, "s." for the
/// FTS join). `?_now` is a bound parameter carrying `now_ms()`; callers must
/// supply it. A snippet never used (`last_used_at IS NULL`) scores 0 and sorts
/// below any used snippet, which is the intent — untouched snippets fall to the
/// bottom of the frecency ordering (favorites and text relevance still rank
/// above frecency in the ORDER BY, so this only breaks ties among the rest).
fn frecency_expr(col: &str, now_param: &str) -> String {
    // Recency multiplier by age bucket (ms). Weights roughly halve per bucket.
    // < 1h: 100, < 1d: 70, < 1w: 40, < 1mo: 20, older: 10.
    format!(
        "(COALESCE({col}use_count, 0) * (CASE \
            WHEN {col}last_used_at IS NULL THEN 0 \
            WHEN {now} - {col}last_used_at < 3600000 THEN 100 \
            WHEN {now} - {col}last_used_at < 86400000 THEN 70 \
            WHEN {now} - {col}last_used_at < 604800000 THEN 40 \
            WHEN {now} - {col}last_used_at < 2592000000 THEN 20 \
            ELSE 10 END))",
        col = col,
        now = now_param,
    )
}

fn row_to_snippet(row: &Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get("id")?,
        folder_id: row.get("folder_id")?,
        name: row.get("name")?,
        trigger: row.get("trigger")?,
        body: row.get("body")?,
        body_html: row.get("body_html")?,
        is_favorite: row.get::<_, i64>("is_favorite")? != 0,
        use_count: row.get("use_count")?,
        last_used_at: row.get("last_used_at")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Normalize a trigger: trim, and treat empty as None so blank strings don't
/// collide in the unique index.
fn clean_trigger(t: Option<String>) -> Option<String> {
    t.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Normalize an optional HTML body: treat an empty/whitespace string as None so
/// a "plain" snippet is never stored with a phantom empty rich form.
fn clean_html(h: Option<String>) -> Option<String> {
    h.filter(|s| !s.trim().is_empty())
}

fn is_trigger_conflict(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

/// List all live snippets. Favorites first, then by frecency (most-used and
/// most-recently-used float up), then newest-updated as a stable tie-breaker for
/// never-used snippets. This is the palette's empty-query ordering, so the
/// snippets a user reaches for most sit at the top of the popup.
pub fn list(conn: &Connection) -> CoreResult<Vec<Snippet>> {
    let frecency = frecency_expr("", "?1");
    let sql = format!(
        "SELECT {COLUMNS} FROM snippets WHERE deleted_at IS NULL \
         ORDER BY is_favorite DESC, {frecency} DESC, updated_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![now_ms()], row_to_snippet)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> CoreResult<Snippet> {
    let sql = format!("SELECT {COLUMNS} FROM snippets WHERE id = ?1 AND deleted_at IS NULL");
    conn.query_row(&sql, params![id], row_to_snippet)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound,
            other => CoreError::Sqlite(other),
        })
}

pub fn create(conn: &Connection, input: NewSnippet) -> CoreResult<Snippet> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("snippet name is empty".into()));
    }
    let trigger = clean_trigger(input.trigger);
    let body_html = clean_html(input.body_html);
    let id = new_id();
    let now = now_ms();
    let res = conn.execute(
        "INSERT INTO snippets
           (id, folder_id, name, trigger, body, body_html, is_favorite, use_count, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, ?8, ?8)",
        params![
            id,
            input.folder_id,
            name,
            trigger,
            input.body,
            body_html,
            input.is_favorite as i64,
            now
        ],
    );
    match res {
        Ok(_) => get(conn, &id),
        Err(e) if is_trigger_conflict(&e) => {
            Err(CoreError::DuplicateTrigger(trigger.unwrap_or_default()))
        }
        Err(e) => Err(CoreError::Sqlite(e)),
    }
}

/// Import a snippet from a backup, preserving its original id so re-importing
/// the same backup is idempotent (a live row with that id already present is
/// skipped, returning `Skipped`). Otherwise it's inserted preserving
/// name/body/folder/favorite/use-stats. If its `trigger` clashes with an
/// existing live trigger, the trigger is dropped so the snippet still imports —
/// the outcome tells the caller so it can report how many lost their shortcut.
pub enum ImportOutcome {
    Inserted,
    /// Inserted, but the trigger was dropped due to a conflict.
    InsertedTriggerDropped,
    /// A live row with the same id already existed; nothing changed.
    Skipped,
}

pub fn import_snippet(conn: &Connection, s: &Snippet) -> CoreResult<ImportOutcome> {
    // Same-id row already present -> treat as already-imported, skip.
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM snippets WHERE id = ?1 AND deleted_at IS NULL",
            params![s.id],
            |_| Ok(()),
        )
        .is_ok();
    if exists {
        return Ok(ImportOutcome::Skipped);
    }

    let name = s.name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("imported snippet name is empty".into()));
    }
    let trigger = clean_trigger(s.trigger.clone());
    let body_html = clean_html(s.body_html.clone());

    // Try with the trigger first; on a UNIQUE conflict, retry without it.
    let now = now_ms();
    let try_insert = |trig: Option<&String>| {
        conn.execute(
            "INSERT INTO snippets
               (id, folder_id, name, trigger, body, body_html, is_favorite, use_count,
                last_used_at, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                s.id,
                s.folder_id,
                name,
                trig,
                s.body,
                body_html,
                s.is_favorite as i64,
                s.use_count,
                s.last_used_at,
                s.sort_order,
                s.created_at,
                now,
            ],
        )
    };

    match try_insert(trigger.as_ref()) {
        Ok(_) => Ok(ImportOutcome::Inserted),
        Err(e) if is_trigger_conflict(&e) && trigger.is_some() => {
            // Retry without the conflicting trigger so the snippet still imports.
            try_insert(None)?;
            Ok(ImportOutcome::InsertedTriggerDropped)
        }
        Err(e) => Err(CoreError::Sqlite(e)),
    }
}

pub fn update(conn: &Connection, input: UpdateSnippet) -> CoreResult<Snippet> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("snippet name is empty".into()));
    }
    let trigger = clean_trigger(input.trigger);
    let body_html = clean_html(input.body_html);
    let res = conn.execute(
        "UPDATE snippets
         SET folder_id = ?2, name = ?3, trigger = ?4, body = ?5, body_html = ?6, is_favorite = ?7, updated_at = ?8
         WHERE id = ?1 AND deleted_at IS NULL",
        params![
            input.id,
            input.folder_id,
            name,
            trigger,
            input.body,
            body_html,
            input.is_favorite as i64,
            now_ms()
        ],
    );
    match res {
        Ok(0) => Err(CoreError::NotFound),
        Ok(_) => get(conn, &input.id),
        Err(e) if is_trigger_conflict(&e) => {
            Err(CoreError::DuplicateTrigger(trigger.unwrap_or_default()))
        }
        Err(e) => Err(CoreError::Sqlite(e)),
    }
}

pub fn soft_delete(conn: &Connection, id: &str) -> CoreResult<()> {
    // Clearing the trigger on delete frees it for reuse immediately (the partial
    // unique index only covers live rows, but a tombstone keeping the trigger
    // would still be found by the trigger-lookup used by the expansion engine).
    let changed = conn.execute(
        "UPDATE snippets SET deleted_at = ?2, trigger = NULL, updated_at = ?2
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now_ms()],
    )?;
    if changed == 0 {
        return Err(CoreError::NotFound);
    }
    Ok(())
}

/// Record that a snippet was inserted (palette/expansion ranking).
pub fn bump_use(conn: &Connection, id: &str) -> CoreResult<()> {
    conn.execute(
        "UPDATE snippets SET use_count = use_count + 1, last_used_at = ?2, updated_at = ?2
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now_ms()],
    )?;
    Ok(())
}

/// All live triggers as (trigger, body) pairs — loaded by the expansion engine.
pub fn all_triggers(conn: &Connection) -> CoreResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT trigger, body FROM snippets
         WHERE deleted_at IS NULL AND trigger IS NOT NULL",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Live snippets whose (lowercased) trigger is in `triggers`. Used by the
/// disambiguation picker to resolve a set of confusably-similar triggers to
/// full snippets for display + ranking. Matching is case-insensitive to align
/// with the NOCASE trigger index and the runtime matcher.
pub fn by_triggers(conn: &Connection, triggers: &[String]) -> CoreResult<Vec<Snippet>> {
    if triggers.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {COLUMNS} FROM snippets
         WHERE deleted_at IS NULL AND trigger IS NOT NULL
           AND lower(trigger) IN ({})",
        // One placeholder per trigger.
        std::iter::repeat("?")
            .take(triggers.len())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let lowered: Vec<String> = triggers.iter().map(|t| t.to_lowercase()).collect();
    let params = rusqlite::params_from_iter(lowered.iter());
    let rows = stmt
        .query_map(params, row_to_snippet)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Full-text search over name/body/trigger. Empty query returns the normal list.
/// Ranking blends FTS relevance with frecency (frequency × recency) and favorite
/// status so the snippets a user reaches for most float up among equally-good
/// text matches.
pub fn search(conn: &Connection, query: &str) -> CoreResult<Vec<Snippet>> {
    let q = query.trim();
    if q.is_empty() {
        return list(conn);
    }
    // Build a prefix MATCH query: each term becomes term* so partial words hit.
    // Quote terms to neutralize FTS operators the user might type.
    let match_expr = q
        .split_whitespace()
        .map(|term| {
            let escaped = term.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" ");

    // ?1 = MATCH expr, ?2 = now (for frecency). Favorites first, then frecency,
    // then bm25 text relevance as the final ordering key.
    let frecency = frecency_expr("s.", "?2");
    let sql = format!(
        "SELECT {COLUMNS_S} FROM snippets s
         JOIN snippets_fts f ON f.rowid = s.rowid
         WHERE snippets_fts MATCH ?1 AND s.deleted_at IS NULL
         ORDER BY s.is_favorite DESC, {frecency} DESC, bm25(snippets_fts)"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![match_expr, now_ms()], row_to_snippet)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
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
    fn create_get_list_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let s = create(&conn, new("Nome", Some(";nm"), "Rubem")).unwrap();
        assert_eq!(s.name, "Nome");
        assert_eq!(s.trigger.as_deref(), Some(";nm"));
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn duplicate_trigger_is_rejected() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("A", Some(";x"), "a")).unwrap();
        let err = create(&conn, new("B", Some(";x"), "b")).unwrap_err();
        assert!(matches!(err, CoreError::DuplicateTrigger(_)));
    }

    #[test]
    fn duplicate_trigger_is_case_insensitive() {
        // A user typing ";cpf" doesn't distinguish it from ";CPF"; the NOCASE
        // unique index must reject the case-variant as a duplicate.
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("A", Some(";cpf"), "a")).unwrap();
        let err = create(&conn, new("B", Some(";CPF"), "b")).unwrap_err();
        assert!(matches!(err, CoreError::DuplicateTrigger(_)));
    }

    #[test]
    fn update_to_case_variant_trigger_is_rejected() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("A", Some(";cpf"), "a")).unwrap();
        let b = create(&conn, new("B", Some(";end"), "b")).unwrap();
        let err = update(
            &conn,
            UpdateSnippet {
                id: b.id,
                name: "B".into(),
                trigger: Some(";CPF".into()),
                body: "b".into(),
                body_html: None,
                folder_id: None,
                is_favorite: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::DuplicateTrigger(_)));
    }

    #[test]
    fn soft_delete_frees_trigger_for_reuse() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let s = create(&conn, new("A", Some(";x"), "a")).unwrap();
        soft_delete(&conn, &s.id).unwrap();
        // Same trigger can now be used again.
        create(&conn, new("B", Some(";x"), "b")).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn fts_is_accent_insensitive_ptbr() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("Endereço", None, "Rua das Flores, 123")).unwrap();
        // Searching without the cedilla/accent should still find it.
        let hits = search(&conn, "endereco").unwrap();
        assert_eq!(hits.len(), 1, "accent-insensitive search should match");
    }

    #[test]
    fn search_prefix_matches_partial_words() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("Telefone", None, "+55 11 99999-0000")).unwrap();
        assert_eq!(search(&conn, "tel").unwrap().len(), 1);
    }

    #[test]
    fn update_changes_persist_and_reindex_fts() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let s = create(&conn, new("Old", None, "old body")).unwrap();
        update(
            &conn,
            UpdateSnippet {
                id: s.id.clone(),
                name: "Novo".into(),
                trigger: None,
                body: "corpo novo".into(),
                body_html: None,
                folder_id: None,
                is_favorite: true,
            },
        )
        .unwrap();
        assert_eq!(search(&conn, "corpo").unwrap().len(), 1);
        assert_eq!(search(&conn, "old").unwrap().len(), 0, "old text de-indexed");
    }

    #[test]
    fn by_triggers_is_case_insensitive_and_live_only() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("CPF", Some(";CPF"), "111")).unwrap();
        create(&conn, new("CPF longo", Some(";cpfs"), "222")).unwrap();
        let gone = create(&conn, new("Velho", Some(";cpx"), "x")).unwrap();
        soft_delete(&conn, &gone.id).unwrap();

        // Query with lowercase; the stored ";CPF" must still match.
        let got = by_triggers(&conn, &[";cpf".into(), ";cpfs".into(), ";cpx".into()]).unwrap();
        let mut names: Vec<String> = got.into_iter().map(|s| s.name).collect();
        names.sort();
        assert_eq!(names, vec!["CPF", "CPF longo"]); // deleted one excluded
    }

    #[test]
    fn list_orders_by_frecency() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let stale = create(&conn, new("Stale", None, "a")).unwrap();
        let fresh = create(&conn, new("Fresh", None, "b")).unwrap();
        let never = create(&conn, new("Never", None, "c")).unwrap();

        // Both used the same number of times, but at very different recencies.
        let now = now_ms();
        // "Stale": used 5x, long ago (older than a month).
        conn.execute(
            "UPDATE snippets SET use_count = 5, last_used_at = ?2 WHERE id = ?1",
            params![stale.id, now - 40 * 86_400_000],
        )
        .unwrap();
        // "Fresh": used 5x, minutes ago.
        conn.execute(
            "UPDATE snippets SET use_count = 5, last_used_at = ?2 WHERE id = ?1",
            params![fresh.id, now - 60_000],
        )
        .unwrap();
        // "Never" keeps use_count 0 / last_used_at NULL.

        let ordered: Vec<String> = list(&conn).unwrap().into_iter().map(|s| s.name).collect();
        // Fresh outranks Stale (recency), and never-used sinks to the bottom.
        assert_eq!(ordered, vec!["Fresh", "Stale", "Never"]);
        let _ = never;
    }

    #[test]
    fn favorite_outranks_frecency_in_list() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let heavy = create(&conn, new("Heavy", None, "a")).unwrap();
        let fav = create(&conn, new("Fav", None, "b")).unwrap();
        let now = now_ms();
        conn.execute(
            "UPDATE snippets SET use_count = 99, last_used_at = ?2 WHERE id = ?1",
            params![heavy.id, now - 60_000],
        )
        .unwrap();
        conn.execute(
            "UPDATE snippets SET is_favorite = 1 WHERE id = ?1",
            params![fav.id],
        )
        .unwrap();
        let ordered: Vec<String> = list(&conn).unwrap().into_iter().map(|s| s.name).collect();
        // Favorite pinned above even a heavily-used snippet.
        assert_eq!(ordered.first().map(String::as_str), Some("Fav"));
    }

    #[test]
    fn search_ranks_frecent_match_first() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        // Two snippets that both match "mail"; the frequently+recently used one
        // should come first.
        let rare = create(&conn, new("Mailing address", None, "x")).unwrap();
        let common = create(&conn, new("Mail signature", None, "y")).unwrap();
        let now = now_ms();
        conn.execute(
            "UPDATE snippets SET use_count = 20, last_used_at = ?2 WHERE id = ?1",
            params![common.id, now - 60_000],
        )
        .unwrap();
        let _ = rare;
        let hits: Vec<String> = search(&conn, "mail").unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(hits.first().map(String::as_str), Some("Mail signature"));
    }

    #[test]
    fn body_html_roundtrips_and_empty_normalizes_to_none() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        let mut input = new("Rich", None, "Hello");
        input.body_html = Some("<b>Hello</b>".into());
        let s = create(&conn, input).unwrap();
        assert_eq!(get(&conn, &s.id).unwrap().body_html.as_deref(), Some("<b>Hello</b>"));

        // Updating with an all-whitespace html clears it back to a plain snippet.
        update(
            &conn,
            UpdateSnippet {
                id: s.id.clone(),
                name: "Rich".into(),
                trigger: None,
                body: "Hello".into(),
                body_html: Some("   ".into()),
                folder_id: None,
                is_favorite: false,
            },
        )
        .unwrap();
        assert_eq!(get(&conn, &s.id).unwrap().body_html, None);
    }

    #[test]
    fn all_triggers_returns_live_only() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.lock();
        create(&conn, new("A", Some(";a"), "aa")).unwrap();
        let b = create(&conn, new("B", Some(";b"), "bb")).unwrap();
        soft_delete(&conn, &b.id).unwrap();
        let triggers = all_triggers(&conn).unwrap();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].0, ";a");
    }
}
