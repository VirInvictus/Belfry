//! `show_settings` table CRUD.
//!
//! Per-show overrides — speed, smart_speed, voice_boost, skip intro/outro,
//! per-show skip-forward/back, inbox_policy. Keyed by `show_id`. `upsert`
//! is the only write path.

use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::domain::{InboxPolicy, ShowSettings};
use crate::errors::Result;

pub(crate) fn upsert(conn: &Connection, s: &ShowSettings) -> Result<()> {
    conn.execute(
        "INSERT INTO show_settings (
            show_id, playback_speed, smart_speed, voice_boost,
            skip_intro, skip_outro, skip_forward, skip_back, inbox_policy
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(show_id) DO UPDATE SET
            playback_speed = excluded.playback_speed,
            smart_speed = excluded.smart_speed,
            voice_boost = excluded.voice_boost,
            skip_intro = excluded.skip_intro,
            skip_outro = excluded.skip_outro,
            skip_forward = excluded.skip_forward,
            skip_back = excluded.skip_back,
            inbox_policy = excluded.inbox_policy",
        params![
            s.show_id,
            s.playback_speed,
            s.smart_speed,
            s.voice_boost,
            s.skip_intro as i64,
            s.skip_outro as i64,
            s.skip_forward.map(|n| n as i64),
            s.skip_back.map(|n| n as i64),
            s.inbox_policy.as_str(),
        ],
    )?;
    Ok(())
}

pub(crate) fn get(conn: &Connection, show_id: i64) -> Result<Option<ShowSettings>> {
    conn.query_row(
        "SELECT * FROM show_settings WHERE show_id = ?1",
        params![show_id],
        row_to_show_settings,
    )
    .optional()
    .map_err(Into::into)
}

fn row_to_show_settings(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShowSettings> {
    let inbox_policy_str: String = row.get("inbox_policy")?;
    let skip_forward: Option<i64> = row.get("skip_forward")?;
    let skip_back: Option<i64> = row.get("skip_back")?;
    Ok(ShowSettings {
        show_id: row.get("show_id")?,
        playback_speed: row.get("playback_speed")?,
        smart_speed: row.get("smart_speed")?,
        voice_boost: row.get("voice_boost")?,
        skip_intro: row.get::<_, i64>("skip_intro")? as u32,
        skip_outro: row.get::<_, i64>("skip_outro")? as u32,
        skip_forward: skip_forward.map(|n| n as u32),
        skip_back: skip_back.map(|n| n as u32),
        inbox_policy: InboxPolicy::from_str(&inbox_policy_str).unwrap_or_default(),
    })
}
