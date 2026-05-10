//! `chapters` table CRUD.
//!
//! Chapters arrive in batches from the source (`<podcast:chapters>` JSON,
//! ID3 CHAP frames, show-notes regex inference) and are replaced atomically
//! per episode. There's no notion of "update one chapter."

use rusqlite::{Connection, params};

use crate::db::domain::Chapter;
use crate::errors::Result;

pub(crate) fn replace(conn: &mut Connection, episode_id: i64, chapters: &[Chapter]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM chapters WHERE episode_id = ?1",
        params![episode_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO chapters (
                episode_id, start_time, end_time, title, url, image_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for c in chapters {
            stmt.execute(params![
                episode_id,
                c.start_time,
                c.end_time,
                c.title,
                c.url,
                c.image_path,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn list_for_episode(conn: &Connection, episode_id: i64) -> Result<Vec<Chapter>> {
    let mut stmt =
        conn.prepare("SELECT * FROM chapters WHERE episode_id = ?1 ORDER BY start_time ASC")?;
    let rows = stmt.query_map(params![episode_id], row_to_chapter)?;
    let mut chapters = Vec::new();
    for row in rows {
        chapters.push(row?);
    }
    Ok(chapters)
}

fn row_to_chapter(row: &rusqlite::Row<'_>) -> rusqlite::Result<Chapter> {
    Ok(Chapter {
        id: row.get("id")?,
        episode_id: row.get("episode_id")?,
        start_time: row.get("start_time")?,
        end_time: row.get("end_time")?,
        title: row.get("title")?,
        url: row.get("url")?,
        image_path: row.get("image_path")?,
    })
}
