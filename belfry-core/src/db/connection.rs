//! Open SQLite connections with the right PRAGMAs.
//!
//! Writer connections enable WAL, NORMAL synchronous, foreign keys, and
//! in-memory temp store. Reader connections open `SQLITE_OPEN_READ_ONLY`
//! so a buggy query attempting INSERT errors at the engine level — no
//! caller can corrupt the database through a read path.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::errors::Result;

pub(crate) fn open_writer(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    apply_writer_pragmas(&conn)?;
    Ok(conn)
}

pub(crate) fn open_reader(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(conn)
}

fn apply_writer_pragmas(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA temp_store = MEMORY;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writer_applies_pragmas() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = open_writer(&path).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        assert_eq!(synchronous, 1); // 1 == NORMAL
    }

    #[test]
    fn writer_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/sub/dir/test.db");
        let _conn = open_writer(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn reader_opens_after_writer_creates_db() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let _writer = open_writer(&path).unwrap();
        let _reader = open_reader(&path).unwrap();
    }

    #[test]
    fn reader_rejects_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let _writer = open_writer(&path).unwrap();
        let reader = open_reader(&path).unwrap();
        // Attempting any write should fail — no table, no schema, just engine-level rejection.
        let result = reader.execute("CREATE TABLE foo (id INTEGER)", []);
        assert!(result.is_err());
    }
}
