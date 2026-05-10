//! `tags` and `show_tags` (many-to-many) CRUD.
//!
//! Tags attach to *shows*, not episodes (per spec §4.1). `get_or_create`
//! is idempotent — repeated calls with the same name return the same id.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::domain::Tag;
use crate::errors::Result;

pub(crate) fn get_or_create(conn: &Connection, name: &str) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row("SELECT id FROM tags WHERE name = ?1", params![name], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute("INSERT INTO tags (name) VALUES (?1)", params![name])?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn list(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

/// Replace the set of tags attached to a show. Atomic.
pub(crate) fn set_show_tags(conn: &mut Connection, show_id: i64, tag_ids: &[i64]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM show_tags WHERE show_id = ?1", params![show_id])?;
    {
        let mut stmt = tx.prepare("INSERT INTO show_tags (show_id, tag_id) VALUES (?1, ?2)")?;
        for tag_id in tag_ids {
            stmt.execute(params![show_id, tag_id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn list_for_show(conn: &Connection, show_id: i64) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name
         FROM tags t
         INNER JOIN show_tags st ON st.tag_id = t.id
         WHERE st.show_id = ?1
         ORDER BY t.name ASC",
    )?;
    let rows = stmt.query_map(params![show_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}
