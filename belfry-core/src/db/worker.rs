//! Single-writer SQLite worker.
//!
//! The worker owns the writable `rusqlite::Connection` on a
//! `tokio::task::spawn_blocking` thread. Every consumer (GTK / CLI / fetch
//! loop / playback / session-recorder) holds an `mpsc::Sender<Command>` and
//! never touches the connection directly.
//!
//! Reads use the separate `pool::ReadPool` (one read-only connection per
//! handle, distinct from the writer). The split is the v0.1 invariant.
//!
//! See `spec.md` §2.1.

use std::path::PathBuf;
use std::time::Instant;

use rusqlite::Connection;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::db::command::Command;
use crate::db::connection;
use crate::db::crud;
use crate::db::domain::{Chapter, Episode, ListeningSession, Playback, Show, ShowSettings, Tag};
use crate::db::migrations;
use crate::errors::{Error, Result};

const CHANNEL_CAPACITY: usize = 64;

/// Handle to the running worker. Cloneable; backed by a tokio mpsc sender.
#[derive(Clone)]
pub struct WorkerHandle {
    tx: mpsc::Sender<Command>,
}

impl WorkerHandle {
    async fn dispatch<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<Result<T>>) -> Command,
    ) -> Result<T> {
        let (reply, recv) = oneshot::channel();
        self.tx.send(build(reply)).await?;
        recv.await?
    }

    // --- Shows ----------------------------------------------------------

    pub async fn insert_show(&self, show: Show) -> Result<i64> {
        self.dispatch(|reply| Command::InsertShow { show, reply })
            .await
    }

    pub async fn update_show(&self, show: Show) -> Result<()> {
        self.dispatch(|reply| Command::UpdateShow { show, reply })
            .await
    }

    pub async fn delete_show(&self, id: i64) -> Result<()> {
        self.dispatch(|reply| Command::DeleteShow { id, reply })
            .await
    }

    pub async fn get_show(&self, id: i64) -> Result<Option<Show>> {
        self.dispatch(|reply| Command::GetShow { id, reply }).await
    }

    pub async fn list_shows(&self) -> Result<Vec<Show>> {
        self.dispatch(|reply| Command::ListShows { reply }).await
    }

    // --- Episodes -------------------------------------------------------

    pub async fn insert_episode(&self, episode: Episode) -> Result<i64> {
        self.dispatch(|reply| Command::InsertEpisode { episode, reply })
            .await
    }

    pub async fn update_episode(&self, episode: Episode) -> Result<()> {
        self.dispatch(|reply| Command::UpdateEpisode { episode, reply })
            .await
    }

    pub async fn delete_episode(&self, id: i64) -> Result<()> {
        self.dispatch(|reply| Command::DeleteEpisode { id, reply })
            .await
    }

    pub async fn get_episode(&self, id: i64) -> Result<Option<Episode>> {
        self.dispatch(|reply| Command::GetEpisode { id, reply })
            .await
    }

    pub async fn get_episode_by_guid(&self, show_id: i64, guid: String) -> Result<Option<Episode>> {
        self.dispatch(|reply| Command::GetEpisodeByGuid {
            show_id,
            guid,
            reply,
        })
        .await
    }

    pub async fn list_episodes_for_show(&self, show_id: i64, limit: usize) -> Result<Vec<Episode>> {
        self.dispatch(|reply| Command::ListEpisodesForShow {
            show_id,
            limit,
            reply,
        })
        .await
    }

    // --- Playback -------------------------------------------------------

    pub async fn upsert_playback(&self, playback: Playback) -> Result<()> {
        self.dispatch(|reply| Command::UpsertPlayback { playback, reply })
            .await
    }

    pub async fn get_playback(&self, episode_id: i64) -> Result<Option<Playback>> {
        self.dispatch(|reply| Command::GetPlayback { episode_id, reply })
            .await
    }

    // --- Show settings --------------------------------------------------

    pub async fn upsert_show_settings(&self, settings: ShowSettings) -> Result<()> {
        self.dispatch(|reply| Command::UpsertShowSettings { settings, reply })
            .await
    }

    pub async fn get_show_settings(&self, show_id: i64) -> Result<Option<ShowSettings>> {
        self.dispatch(|reply| Command::GetShowSettings { show_id, reply })
            .await
    }

    // --- Chapters -------------------------------------------------------

    pub async fn replace_chapters(&self, episode_id: i64, chapters: Vec<Chapter>) -> Result<()> {
        self.dispatch(|reply| Command::ReplaceChapters {
            episode_id,
            chapters,
            reply,
        })
        .await
    }

    pub async fn list_chapters(&self, episode_id: i64) -> Result<Vec<Chapter>> {
        self.dispatch(|reply| Command::ListChapters { episode_id, reply })
            .await
    }

    // --- Tags -----------------------------------------------------------

    pub async fn get_or_create_tag(&self, name: String) -> Result<i64> {
        self.dispatch(|reply| Command::GetOrCreateTag { name, reply })
            .await
    }

    pub async fn list_tags(&self) -> Result<Vec<Tag>> {
        self.dispatch(|reply| Command::ListTags { reply }).await
    }

    pub async fn set_show_tags(&self, show_id: i64, tag_ids: Vec<i64>) -> Result<()> {
        self.dispatch(|reply| Command::SetShowTags {
            show_id,
            tag_ids,
            reply,
        })
        .await
    }

    pub async fn list_tags_for_show(&self, show_id: i64) -> Result<Vec<Tag>> {
        self.dispatch(|reply| Command::ListTagsForShow { show_id, reply })
            .await
    }

    // --- Sessions -------------------------------------------------------

    pub async fn insert_session(&self, session: ListeningSession) -> Result<i64> {
        self.dispatch(|reply| Command::InsertSession { session, reply })
            .await
    }

    pub async fn list_sessions_for_episode(
        &self,
        episode_id: i64,
    ) -> Result<Vec<ListeningSession>> {
        self.dispatch(|reply| Command::ListSessionsForEpisode { episode_id, reply })
            .await
    }

    pub async fn total_real_seconds(&self, from: i64, to: i64) -> Result<f64> {
        self.dispatch(|reply| Command::TotalRealSeconds { from, to, reply })
            .await
    }

    pub async fn total_smart_speed_saved(&self, from: i64, to: i64) -> Result<f64> {
        self.dispatch(|reply| Command::TotalSmartSpeedSaved { from, to, reply })
            .await
    }

    // --- Control --------------------------------------------------------

    /// Send a shutdown ack; the worker loop exits when all `WorkerHandle`
    /// clones are dropped (the channel closes).
    pub async fn shutdown_ack(&self) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        self.tx.send(Command::Shutdown { reply }).await?;
        recv.await.map_err(|_| Error::WorkerChannelClosed)
    }
}

/// Spawn the worker. Runs migrations on the open writer connection, then
/// hands off to the dispatch loop on a blocking thread.
pub fn spawn_worker(db_path: PathBuf) -> Result<WorkerHandle> {
    let mut conn = connection::open_writer(&db_path)?;
    migrations::run(&mut conn)?;

    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);

    tokio::task::spawn_blocking(move || worker_loop(conn, rx));

    Ok(WorkerHandle { tx })
}

fn worker_loop(mut conn: Connection, mut rx: mpsc::Receiver<Command>) {
    while let Some(command) = rx.blocking_recv() {
        let kind = command.kind();
        let start = Instant::now();
        handle(&mut conn, command);
        tracing::trace!(
            kind,
            elapsed_us = start.elapsed().as_micros() as u64,
            "worker cmd"
        );
    }
    tracing::info!("worker shutdown");
}

fn handle(conn: &mut Connection, command: Command) {
    match command {
        // Shows
        Command::InsertShow { show, reply } => {
            let _ = reply.send(crud::shows::insert(conn, &show));
        }
        Command::UpdateShow { show, reply } => {
            let _ = reply.send(crud::shows::update(conn, &show));
        }
        Command::DeleteShow { id, reply } => {
            let _ = reply.send(crud::shows::delete(conn, id));
        }
        Command::GetShow { id, reply } => {
            let _ = reply.send(crud::shows::get(conn, id));
        }
        Command::ListShows { reply } => {
            let _ = reply.send(crud::shows::list(conn));
        }

        // Episodes
        Command::InsertEpisode { episode, reply } => {
            let _ = reply.send(crud::episodes::insert(conn, &episode));
        }
        Command::UpdateEpisode { episode, reply } => {
            let _ = reply.send(crud::episodes::update(conn, &episode));
        }
        Command::DeleteEpisode { id, reply } => {
            let _ = reply.send(crud::episodes::delete(conn, id));
        }
        Command::GetEpisode { id, reply } => {
            let _ = reply.send(crud::episodes::get(conn, id));
        }
        Command::GetEpisodeByGuid {
            show_id,
            guid,
            reply,
        } => {
            let _ = reply.send(crud::episodes::get_by_guid(conn, show_id, &guid));
        }
        Command::ListEpisodesForShow {
            show_id,
            limit,
            reply,
        } => {
            let _ = reply.send(crud::episodes::list_for_show(conn, show_id, limit));
        }

        // Playback
        Command::UpsertPlayback { playback, reply } => {
            let _ = reply.send(crud::playback::upsert(conn, &playback));
        }
        Command::GetPlayback { episode_id, reply } => {
            let _ = reply.send(crud::playback::get(conn, episode_id));
        }

        // Show settings
        Command::UpsertShowSettings { settings, reply } => {
            let _ = reply.send(crud::show_settings::upsert(conn, &settings));
        }
        Command::GetShowSettings { show_id, reply } => {
            let _ = reply.send(crud::show_settings::get(conn, show_id));
        }

        // Chapters
        Command::ReplaceChapters {
            episode_id,
            chapters,
            reply,
        } => {
            let _ = reply.send(crud::chapters::replace(conn, episode_id, &chapters));
        }
        Command::ListChapters { episode_id, reply } => {
            let _ = reply.send(crud::chapters::list_for_episode(conn, episode_id));
        }

        // Tags
        Command::GetOrCreateTag { name, reply } => {
            let _ = reply.send(crud::tags::get_or_create(conn, &name));
        }
        Command::ListTags { reply } => {
            let _ = reply.send(crud::tags::list(conn));
        }
        Command::SetShowTags {
            show_id,
            tag_ids,
            reply,
        } => {
            let _ = reply.send(crud::tags::set_show_tags(conn, show_id, &tag_ids));
        }
        Command::ListTagsForShow { show_id, reply } => {
            let _ = reply.send(crud::tags::list_for_show(conn, show_id));
        }

        // Sessions
        Command::InsertSession { session, reply } => {
            let _ = reply.send(crud::sessions::insert(conn, &session));
        }
        Command::ListSessionsForEpisode { episode_id, reply } => {
            let _ = reply.send(crud::sessions::list_for_episode(conn, episode_id));
        }
        Command::TotalRealSeconds { from, to, reply } => {
            let _ = reply.send(crud::sessions::total_real_seconds(conn, from, to));
        }
        Command::TotalSmartSpeedSaved { from, to, reply } => {
            let _ = reply.send(crud::sessions::total_smart_speed_saved(conn, from, to));
        }

        // Control
        Command::Shutdown { reply } => {
            let _ = reply.send(());
            // Loop exits naturally when the last sender drops.
        }
    }
}
