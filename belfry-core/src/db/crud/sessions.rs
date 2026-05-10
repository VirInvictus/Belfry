//! `listening_sessions` table — append-only.
//!
//! One row per playback span; never updated, never re-attributed. Drives
//! the time-saved counter (Phase 12) and listening-history view (Phase 17).

use chrono::TimeZone;
use rusqlite::{Connection, params};

use crate::db::domain::ListeningSession;
use crate::errors::Result;

pub(crate) fn insert(conn: &Connection, s: &ListeningSession) -> Result<i64> {
    conn.execute(
        "INSERT INTO listening_sessions (
            episode_id, started_at, ended_at, real_seconds, audio_seconds, smart_speed_saved
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            s.episode_id,
            s.started_at.timestamp(),
            s.ended_at.timestamp(),
            s.real_seconds,
            s.audio_seconds,
            s.smart_speed_saved,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn list_for_episode(
    conn: &Connection,
    episode_id: i64,
) -> Result<Vec<ListeningSession>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM listening_sessions WHERE episode_id = ?1 ORDER BY started_at ASC",
    )?;
    let rows = stmt.query_map(params![episode_id], row_to_session)?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

/// Sum of `real_seconds` across sessions in `[from, to)`.
pub(crate) fn total_real_seconds(conn: &Connection, from: i64, to: i64) -> Result<f64> {
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(real_seconds), 0.0)
         FROM listening_sessions
         WHERE started_at >= ?1 AND started_at < ?2",
        params![from, to],
        |r| r.get(0),
    )?;
    Ok(total)
}

/// Sum of `smart_speed_saved` across sessions in `[from, to)`.
pub(crate) fn total_smart_speed_saved(conn: &Connection, from: i64, to: i64) -> Result<f64> {
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(smart_speed_saved), 0.0)
         FROM listening_sessions
         WHERE started_at >= ?1 AND started_at < ?2",
        params![from, to],
        |r| r.get(0),
    )?;
    Ok(total)
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<ListeningSession> {
    let started_at: i64 = row.get("started_at")?;
    let ended_at: i64 = row.get("ended_at")?;
    Ok(ListeningSession {
        id: row.get("id")?,
        episode_id: row.get("episode_id")?,
        started_at: chrono::Utc
            .timestamp_opt(started_at, 0)
            .single()
            .unwrap_or_else(|| chrono::Utc.timestamp_opt(0, 0).unwrap()),
        ended_at: chrono::Utc
            .timestamp_opt(ended_at, 0)
            .single()
            .unwrap_or_else(|| chrono::Utc.timestamp_opt(0, 0).unwrap()),
        real_seconds: row.get("real_seconds")?,
        audio_seconds: row.get("audio_seconds")?,
        smart_speed_saved: row.get("smart_speed_saved")?,
    })
}
