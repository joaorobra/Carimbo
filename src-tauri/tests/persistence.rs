//! Integration test: data survives closing and reopening the database.
//! This mirrors the M1 "survives restart" acceptance criterion without a GUI.

use carimbo_lib::core::db::Db;
use carimbo_lib::core::models::{NewSnippet, UpdateSnippet};
use carimbo_lib::core::repo::snippet_repo;

fn temp_db_path() -> std::path::PathBuf {
    // Unique per run without needing rand: use the process id + a nanosecond
    // component from the OS temp dir uniqueness is not guaranteed, so include pid.
    let mut p = std::env::temp_dir();
    p.push(format!("carimbo-test-{}.db", std::process::id()));
    // Clean any leftover from a previous aborted run.
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn snippets_persist_across_reopen() {
    let path = temp_db_path();

    // First "session": create some pt-BR snippets.
    {
        let db = Db::open(&path).expect("open db");
        let conn = db.lock();
        snippet_repo::create(
            &conn,
            NewSnippet {
                name: "Endereço".into(),
                trigger: Some(";end".into()),
                body: "Rua das Flores, 123 — São Paulo".into(),
                body_html: None,
                folder_id: None,
                is_favorite: false,
            },
        )
        .expect("create endereco");
        snippet_repo::create(
            &conn,
            NewSnippet {
                name: "Saudação".into(),
                trigger: Some(";oi".into()),
                body: "Olá, tudo bem?".into(),
                body_html: None,
                folder_id: None,
                is_favorite: true,
            },
        )
        .expect("create saudacao");
    } // db dropped here — connection closed, WAL checkpointed on close.

    // Second "session": reopen the same file and confirm the data is there.
    {
        let db = Db::open(&path).expect("reopen db");
        let conn = db.lock();
        let all = snippet_repo::list(&conn).expect("list");
        assert_eq!(all.len(), 2, "both snippets should persist");

        // Accent-insensitive FTS still works after reopen.
        let hits = snippet_repo::search(&conn, "endereco").expect("search");
        assert_eq!(hits.len(), 1, "accent-insensitive search after reopen");
        assert_eq!(hits[0].name, "Endereço");

        // Favorite flag persisted.
        let fav = all.iter().find(|s| s.name == "Saudação").unwrap();
        assert!(fav.is_favorite);
    }

    let _ = std::fs::remove_file(&path);
    // Also clean WAL/SHM sidecar files.
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

/// Migration 0002 must resolve a pre-existing case-collision that the original
/// BINARY-collation index allowed (e.g. ";cpf" and ";CPF" both live), keeping the
/// most-recently-updated snippet and freeing the older one's trigger. This drives
/// the raw migration SQL through rusqlite (which, unlike the sqlite3 CLI on this
/// machine, has FTS5) so the FTS-sync triggers fire for real.
#[test]
fn migration_0002_collapses_case_collision() {
    use rusqlite::Connection;

    let mut path = std::env::temp_dir();
    path.push(format!("carimbo-mig-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);

    // 1. Build a v1 schema and seed a case-collision that only the OLD binary
    //    index permitted. We insert directly so we don't depend on repo-layer
    //    normalization.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../src/core/migrations/0001_init.sql"))
            .unwrap();
        // Older ";cpf" (updated_at 100) and newer ";CPF" (updated_at 200): the
        // newer survives, the older is tombstoned.
        conn.execute(
            "INSERT INTO snippets (id, name, trigger, body, is_favorite, use_count, sort_order, created_at, updated_at)
             VALUES ('old', 'CPF antigo', ';cpf', '111', 0, 0, 0, 100, 100),
                    ('new', 'CPF novo',   ';CPF', '222', 0, 0, 0, 200, 200),
                    ('solo','ASD',        'ASD',  '333', 0, 0, 0, 300, 300)",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1_i64).unwrap();
    }

    // 2. Reopen through the real Db, which runs migration 0002 (1 -> 2).
    {
        let db = Db::open(&path).expect("reopen runs migration 0002");
        let conn = db.lock();
        let all = snippet_repo::list(&conn).expect("list after migration");

        // The older ";cpf" is tombstoned; ";CPF" and "ASD" remain live.
        let live_triggers: Vec<Option<String>> =
            all.iter().map(|s| s.trigger.clone()).collect();
        assert_eq!(all.len(), 2, "one duplicate tombstoned, two survive");
        assert!(
            live_triggers.contains(&Some(";CPF".into())),
            "newer ;CPF survives, got {live_triggers:?}"
        );
        assert!(
            live_triggers.contains(&Some("ASD".into())),
            "non-colliding ASD untouched"
        );
        assert!(
            !live_triggers.contains(&Some(";cpf".into())),
            "older ;cpf tombstoned"
        );

        // 3. The NOCASE index now rejects a case-variant of a live trigger.
        let err = snippet_repo::update(
            &conn,
            UpdateSnippet {
                id: "solo".into(),
                name: "ASD".into(),
                trigger: Some(";Cpf".into()), // case-variant of the surviving ;CPF
                body: "333".into(),
                body_html: None,
                folder_id: None,
                is_favorite: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, carimbo_lib::core::error::CoreError::DuplicateTrigger(_)),
            "NOCASE index should reject case-variant, got {err:?}"
        );
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}
