//! `episodes` table CRUD.
//!
//! FTS5 sync on `episode_fts` is handled by triggers in `0001_initial.sql`;
//! this module never touches `episode_fts` directly.

use std::str::FromStr;

use chrono::TimeZone;
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::domain::{Episode, EpisodeType};
use crate::errors::Result;

pub(crate) fn insert(conn: &Connection, episode: &Episode) -> Result<i64> {
    let episode_type_str: Option<&str> = episode.episode_type.as_ref().map(EpisodeType::as_str);
    conn.execute(
        "INSERT INTO episodes (
            show_id, guid, title, description, pub_date, duration, file_size,
            audio_url, audio_path, folder_path, mime_type, season,
            episode_number, episode_type
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
        )",
        params![
            episode.show_id,
            episode.guid,
            episode.title,
            episode.description,
            episode.pub_date.map(|t| t.timestamp()),
            episode.duration.map(|d| d as i64),
            episode.file_size.map(|d| d as i64),
            episode.audio_url,
            episode.audio_path,
            episode.folder_path,
            episode.mime_type,
            episode.season.map(|s| s as i64),
            episode.episode_number.map(|n| n as i64),
            episode_type_str,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn update(conn: &Connection, episode: &Episode) -> Result<()> {
    let episode_type_str: Option<&str> = episode.episode_type.as_ref().map(EpisodeType::as_str);
    conn.execute(
        "UPDATE episodes SET
            show_id = ?1, guid = ?2, title = ?3, description = ?4, pub_date = ?5,
            duration = ?6, file_size = ?7, audio_url = ?8, audio_path = ?9,
            folder_path = ?10, mime_type = ?11, season = ?12,
            episode_number = ?13, episode_type = ?14
         WHERE id = ?15",
        params![
            episode.show_id,
            episode.guid,
            episode.title,
            episode.description,
            episode.pub_date.map(|t| t.timestamp()),
            episode.duration.map(|d| d as i64),
            episode.file_size.map(|d| d as i64),
            episode.audio_url,
            episode.audio_path,
            episode.folder_path,
            episode.mime_type,
            episode.season.map(|s| s as i64),
            episode.episode_number.map(|n| n as i64),
            episode_type_str,
            episode.id,
        ],
    )?;
    Ok(())
}

pub(crate) fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM episodes WHERE id = ?1", params![id])?;
    Ok(())
}

pub(crate) fn get(conn: &Connection, id: i64) -> Result<Option<Episode>> {
    conn.query_row(
        "SELECT * FROM episodes WHERE id = ?1",
        params![id],
        row_to_episode,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn get_by_guid(conn: &Connection, show_id: i64, guid: &str) -> Result<Option<Episode>> {
    conn.query_row(
        "SELECT * FROM episodes WHERE show_id = ?1 AND guid = ?2",
        params![show_id, guid],
        row_to_episode,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_for_show(conn: &Connection, show_id: i64, limit: usize) -> Result<Vec<Episode>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM episodes WHERE show_id = ?1
         ORDER BY pub_date DESC NULLS LAST, id DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![show_id, limit as i64], row_to_episode)?;
    let mut episodes = Vec::new();
    for row in rows {
        episodes.push(row?);
    }
    Ok(episodes)
}

fn row_to_episode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
    let pub_date: Option<i64> = row.get("pub_date")?;
    let episode_type_str: Option<String> = row.get("episode_type")?;
    let episode_type = episode_type_str.and_then(|s| EpisodeType::from_str(&s).ok());
    let duration: Option<i64> = row.get("duration")?;
    let file_size: Option<i64> = row.get("file_size")?;
    let season: Option<i64> = row.get("season")?;
    let episode_number: Option<i64> = row.get("episode_number")?;
    Ok(Episode {
        id: row.get("id")?,
        show_id: row.get("show_id")?,
        guid: row.get("guid")?,
        title: row.get("title")?,
        description: row.get("description")?,
        pub_date: pub_date.and_then(|t| chrono::Utc.timestamp_opt(t, 0).single()),
        duration: duration.map(|d| d as u32),
        file_size: file_size.map(|d| d as u64),
        audio_url: row.get("audio_url")?,
        audio_path: row.get("audio_path")?,
        folder_path: row.get("folder_path")?,
        mime_type: row.get("mime_type")?,
        season: season.map(|s| s as u32),
        episode_number: episode_number.map(|n| n as u32),
        episode_type,
    })
}
