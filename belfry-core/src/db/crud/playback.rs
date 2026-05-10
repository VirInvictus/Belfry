//! `playback` table CRUD.
//!
//! Keyed by `episode_id` (one row per episode). `upsert` is the only
//! write operation — playback rows are insert-or-replace, not selectively
//! mutated. The `played` column maps to `PlayedState` (sentinels 0..=3).

use chrono::TimeZone;
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::domain::{Playback, PlayedState};
use crate::errors::Result;

pub(crate) fn upsert(conn: &Connection, p: &Playback) -> Result<()> {
    conn.execute(
        "INSERT INTO playback (
            episode_id, position, played, last_played, play_count, starred,
            in_queue, queue_position
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(episode_id) DO UPDATE SET
            position = excluded.position,
            played = excluded.played,
            last_played = excluded.last_played,
            play_count = excluded.play_count,
            starred = excluded.starred,
            in_queue = excluded.in_queue,
            queue_position = excluded.queue_position",
        params![
            p.episode_id,
            p.position,
            p.played as u8 as i64,
            p.last_played.map(|t| t.timestamp()),
            p.play_count as i64,
            p.starred,
            p.in_queue,
            p.queue_position,
        ],
    )?;
    Ok(())
}

pub(crate) fn get(conn: &Connection, episode_id: i64) -> Result<Option<Playback>> {
    conn.query_row(
        "SELECT * FROM playback WHERE episode_id = ?1",
        params![episode_id],
        row_to_playback,
    )
    .optional()
    .map_err(Into::into)
}

fn row_to_playback(row: &rusqlite::Row<'_>) -> rusqlite::Result<Playback> {
    let played: i64 = row.get("played")?;
    let last_played: Option<i64> = row.get("last_played")?;
    Ok(Playback {
        episode_id: row.get("episode_id")?,
        position: row.get("position")?,
        played: PlayedState::from_i64(played).unwrap_or(PlayedState::Unplayed),
        last_played: last_played.and_then(|t| chrono::Utc.timestamp_opt(t, 0).single()),
        play_count: row.get::<_, i64>("play_count")? as u32,
        starred: row.get("starred")?,
        in_queue: row.get("in_queue")?,
        queue_position: row.get("queue_position")?,
    })
}
