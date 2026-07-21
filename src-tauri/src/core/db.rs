//! SQLite connection, pragmas, and the migration runner.
//!
//! Concurrency model: a single `Connection` behind a `Mutex`. Carimbo's writes
//! are small and infrequent (a user editing snippets, the clipboard monitor
//! inserting one row per copy), so a single serialized connection is simpler and
//! avoids the footguns of a pool while WAL keeps reads from blocking. If profiling
//! ever shows contention we can move to a reader pool + one writer without
//! touching the repo API.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use super::error::{CoreError, CoreResult};

/// Embedded, ordered migrations. Index + 1 is the schema version they bring the
/// database to (i.e. migrations[0] takes user_version 0 -> 1).
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_trigger_nocase.sql"),
    include_str!("migrations/0003_clip_content_type.sql"),
    include_str!("migrations/0004_snippet_html.sql"),
];

/// Owns the database connection. Clone-free; wrap in `Arc` for shared state.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Open (creating if needed) the database at `path`, apply pragmas, and run
    /// any pending migrations.
    pub fn open(path: impl AsRef<Path>) -> CoreResult<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// Open an in-memory database — used by tests.
    pub fn open_in_memory() -> CoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> CoreResult<Self> {
        // WAL: readers don't block the single writer. NORMAL sync is the
        // recommended durability/speed trade-off under WAL. foreign_keys ON so
        // our REFERENCES are enforced.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Busy timeout so a momentarily-locked db retries instead of erroring.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        let db = Db {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Run all migrations newer than the stored `user_version`.
    fn migrate(&self) -> CoreResult<()> {
        let mut guard = self.lock();
        let current: i64 =
            guard.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let current = current as usize;

        if current > MIGRATIONS.len() {
            return Err(CoreError::Migration(format!(
                "database schema version {current} is newer than this build supports ({})",
                MIGRATIONS.len()
            )));
        }

        for (idx, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let version = idx + 1;
            let tx = guard.transaction()?;
            tx.execute_batch(sql)?;
            // user_version can't be parameterized; the value is a trusted usize.
            tx.pragma_update(None, "user_version", version as i64)?;
            tx.commit()?;
            tracing::info!("applied migration to schema version {version}");
        }
        Ok(())
    }

    /// Access the connection under the mutex. Panics only if the lock is
    /// poisoned, which means another thread panicked mid-write — unrecoverable.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_migrates_in_memory() {
        let db = Db::open_in_memory().unwrap();
        let guard = db.lock();
        let v: i64 = guard
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
    }

    #[test]
    fn migration_is_idempotent_across_reopen() {
        // Re-running open() on the same in-memory schema state shouldn't error;
        // here we just confirm a fresh open lands on the latest version and the
        // core tables exist.
        let db = Db::open_in_memory().unwrap();
        let guard = db.lock();
        let count: i64 = guard
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('folders','snippets','clip_entries','settings')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }
}
