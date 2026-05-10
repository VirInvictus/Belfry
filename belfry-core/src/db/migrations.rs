//! Schema migrations.
//!
//! Versioned via SQLite `user_version` PRAGMA. v0.1 ships `0001_initial.sql`.
//! Post-1.0 the discipline is append-only and backwards-compatible
//! (see `spec.md` §4.3).

use rusqlite::Connection;

use crate::errors::Result;

pub const SCHEMA_V1: &str = include_str!("migrations/0001_initial.sql");

pub const CURRENT_VERSION: i32 = 1;

/// Apply any unapplied migrations. Idempotent: running this on a
/// fully-migrated database is a no-op.
pub(crate) fn run(conn: &mut Connection) -> Result<()> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if current < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(SCHEMA_V1)?;
        tx.commit()?;
        // 0001_initial.sql sets `PRAGMA user_version = 1` itself.
    }

    // Future migrations land here as additional `if current < N` blocks.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use tempfile::tempdir;

    #[test]
    fn applies_initial_schema() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut conn = connection::open_writer(&path).unwrap();

        run(&mut conn).unwrap();

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);

        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.filter_map(|r| r.ok()).collect()
        };

        // Spot-check the schema landed.
        for required in &[
            "shows",
            "episodes",
            "playback",
            "show_settings",
            "listening_sessions",
            "chapters",
            "tags",
            "show_tags",
        ] {
            assert!(
                tables.contains(&required.to_string()),
                "missing table: {required}"
            );
        }
    }

    #[test]
    fn idempotent_re_run() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut conn = connection::open_writer(&path).unwrap();

        run(&mut conn).unwrap();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap();

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn fts_triggers_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut conn = connection::open_writer(&path).unwrap();
        run(&mut conn).unwrap();

        let triggers: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' ORDER BY name")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.filter_map(|r| r.ok()).collect()
        };

        for required in &[
            "episodes_ai",
            "episodes_ad",
            "episodes_au",
            "shows_ai",
            "shows_ad",
            "shows_au",
        ] {
            assert!(
                triggers.contains(&required.to_string()),
                "missing trigger: {required}"
            );
        }
    }
}
