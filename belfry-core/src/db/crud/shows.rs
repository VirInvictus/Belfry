//! `shows` table CRUD.

use chrono::TimeZone;
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::domain::Show;
use crate::errors::Result;

pub(crate) fn insert(conn: &Connection, show: &Show) -> Result<i64> {
    conn.execute(
        "INSERT INTO shows (
            slug, feed_url, title, author, description, homepage_url,
            cover_path, accent_rgb, apple_podcasts_id, last_fetched,
            last_modified, etag, fetch_interval, auth_user, auth_pass_ref,
            auto_download, keep_count, priority, folder_path
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
        )",
        params![
            show.slug,
            show.feed_url,
            show.title,
            show.author,
            show.description,
            show.homepage_url,
            show.cover_path,
            show.accent_rgb,
            show.apple_podcasts_id,
            show.last_fetched.map(|t| t.timestamp()),
            show.last_modified,
            show.etag,
            show.fetch_interval as i64,
            show.auth_user,
            show.auth_pass_ref,
            show.auto_download,
            show.keep_count as i64,
            show.priority,
            show.folder_path,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn update(conn: &Connection, show: &Show) -> Result<()> {
    conn.execute(
        "UPDATE shows SET
            slug = ?1, feed_url = ?2, title = ?3, author = ?4, description = ?5,
            homepage_url = ?6, cover_path = ?7, accent_rgb = ?8, apple_podcasts_id = ?9,
            last_fetched = ?10, last_modified = ?11, etag = ?12, fetch_interval = ?13,
            auth_user = ?14, auth_pass_ref = ?15, auto_download = ?16, keep_count = ?17,
            priority = ?18, folder_path = ?19
         WHERE id = ?20",
        params![
            show.slug,
            show.feed_url,
            show.title,
            show.author,
            show.description,
            show.homepage_url,
            show.cover_path,
            show.accent_rgb,
            show.apple_podcasts_id,
            show.last_fetched.map(|t| t.timestamp()),
            show.last_modified,
            show.etag,
            show.fetch_interval as i64,
            show.auth_user,
            show.auth_pass_ref,
            show.auto_download,
            show.keep_count as i64,
            show.priority,
            show.folder_path,
            show.id,
        ],
    )?;
    Ok(())
}

pub(crate) fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM shows WHERE id = ?1", params![id])?;
    Ok(())
}

pub(crate) fn get(conn: &Connection, id: i64) -> Result<Option<Show>> {
    conn.query_row(
        "SELECT * FROM shows WHERE id = ?1",
        params![id],
        row_to_show,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list(conn: &Connection) -> Result<Vec<Show>> {
    let mut stmt = conn.prepare("SELECT * FROM shows ORDER BY priority DESC, title ASC")?;
    let rows = stmt.query_map([], row_to_show)?;
    let mut shows = Vec::new();
    for row in rows {
        shows.push(row?);
    }
    Ok(shows)
}

fn row_to_show(row: &rusqlite::Row<'_>) -> rusqlite::Result<Show> {
    let last_fetched: Option<i64> = row.get("last_fetched")?;
    let accent: Option<i64> = row.get("accent_rgb")?;
    Ok(Show {
        id: row.get("id")?,
        slug: row.get("slug")?,
        feed_url: row.get("feed_url")?,
        title: row.get("title")?,
        author: row.get("author")?,
        description: row.get("description")?,
        homepage_url: row.get("homepage_url")?,
        cover_path: row.get("cover_path")?,
        accent_rgb: accent.map(|v| v as u32),
        apple_podcasts_id: row.get("apple_podcasts_id")?,
        last_fetched: last_fetched.and_then(|t| chrono::Utc.timestamp_opt(t, 0).single()),
        last_modified: row.get("last_modified")?,
        etag: row.get("etag")?,
        fetch_interval: row.get::<_, i64>("fetch_interval")? as u32,
        auth_user: row.get("auth_user")?,
        auth_pass_ref: row.get("auth_pass_ref")?,
        auto_download: row.get("auto_download")?,
        keep_count: row.get::<_, i64>("keep_count")? as u32,
        priority: row.get("priority")?,
        folder_path: row.get("folder_path")?,
    })
}
