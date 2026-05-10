//! Worker command enum. Each variant carries a `oneshot::Sender` reply channel.
//!
//! Consumers don't construct `Command` directly — they call typed methods on
//! [`crate::db::WorkerHandle`]. The enum is internal to the `db` module.

use tokio::sync::oneshot;

use crate::db::domain::{Chapter, Episode, ListeningSession, Playback, Show, ShowSettings, Tag};
use crate::errors::Result;

pub(crate) enum Command {
    // --- Shows -----------------------------------------------------------
    InsertShow {
        show: Show,
        reply: oneshot::Sender<Result<i64>>,
    },
    UpdateShow {
        show: Show,
        reply: oneshot::Sender<Result<()>>,
    },
    DeleteShow {
        id: i64,
        reply: oneshot::Sender<Result<()>>,
    },
    GetShow {
        id: i64,
        reply: oneshot::Sender<Result<Option<Show>>>,
    },
    ListShows {
        reply: oneshot::Sender<Result<Vec<Show>>>,
    },

    // --- Episodes --------------------------------------------------------
    InsertEpisode {
        episode: Episode,
        reply: oneshot::Sender<Result<i64>>,
    },
    UpdateEpisode {
        episode: Episode,
        reply: oneshot::Sender<Result<()>>,
    },
    DeleteEpisode {
        id: i64,
        reply: oneshot::Sender<Result<()>>,
    },
    GetEpisode {
        id: i64,
        reply: oneshot::Sender<Result<Option<Episode>>>,
    },
    GetEpisodeByGuid {
        show_id: i64,
        guid: String,
        reply: oneshot::Sender<Result<Option<Episode>>>,
    },
    ListEpisodesForShow {
        show_id: i64,
        limit: usize,
        reply: oneshot::Sender<Result<Vec<Episode>>>,
    },

    // --- Playback --------------------------------------------------------
    UpsertPlayback {
        playback: Playback,
        reply: oneshot::Sender<Result<()>>,
    },
    GetPlayback {
        episode_id: i64,
        reply: oneshot::Sender<Result<Option<Playback>>>,
    },

    // --- Show settings ---------------------------------------------------
    UpsertShowSettings {
        settings: ShowSettings,
        reply: oneshot::Sender<Result<()>>,
    },
    GetShowSettings {
        show_id: i64,
        reply: oneshot::Sender<Result<Option<ShowSettings>>>,
    },

    // --- Chapters --------------------------------------------------------
    ReplaceChapters {
        episode_id: i64,
        chapters: Vec<Chapter>,
        reply: oneshot::Sender<Result<()>>,
    },
    ListChapters {
        episode_id: i64,
        reply: oneshot::Sender<Result<Vec<Chapter>>>,
    },

    // --- Tags ------------------------------------------------------------
    GetOrCreateTag {
        name: String,
        reply: oneshot::Sender<Result<i64>>,
    },
    ListTags {
        reply: oneshot::Sender<Result<Vec<Tag>>>,
    },
    SetShowTags {
        show_id: i64,
        tag_ids: Vec<i64>,
        reply: oneshot::Sender<Result<()>>,
    },
    ListTagsForShow {
        show_id: i64,
        reply: oneshot::Sender<Result<Vec<Tag>>>,
    },

    // --- Listening sessions ---------------------------------------------
    InsertSession {
        session: ListeningSession,
        reply: oneshot::Sender<Result<i64>>,
    },
    ListSessionsForEpisode {
        episode_id: i64,
        reply: oneshot::Sender<Result<Vec<ListeningSession>>>,
    },
    TotalRealSeconds {
        from: i64,
        to: i64,
        reply: oneshot::Sender<Result<f64>>,
    },
    TotalSmartSpeedSaved {
        from: i64,
        to: i64,
        reply: oneshot::Sender<Result<f64>>,
    },

    // --- Worker control --------------------------------------------------
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

impl Command {
    /// Stable string name for tracing instrumentation.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::InsertShow { .. } => "insert_show",
            Self::UpdateShow { .. } => "update_show",
            Self::DeleteShow { .. } => "delete_show",
            Self::GetShow { .. } => "get_show",
            Self::ListShows { .. } => "list_shows",
            Self::InsertEpisode { .. } => "insert_episode",
            Self::UpdateEpisode { .. } => "update_episode",
            Self::DeleteEpisode { .. } => "delete_episode",
            Self::GetEpisode { .. } => "get_episode",
            Self::GetEpisodeByGuid { .. } => "get_episode_by_guid",
            Self::ListEpisodesForShow { .. } => "list_episodes_for_show",
            Self::UpsertPlayback { .. } => "upsert_playback",
            Self::GetPlayback { .. } => "get_playback",
            Self::UpsertShowSettings { .. } => "upsert_show_settings",
            Self::GetShowSettings { .. } => "get_show_settings",
            Self::ReplaceChapters { .. } => "replace_chapters",
            Self::ListChapters { .. } => "list_chapters",
            Self::GetOrCreateTag { .. } => "get_or_create_tag",
            Self::ListTags { .. } => "list_tags",
            Self::SetShowTags { .. } => "set_show_tags",
            Self::ListTagsForShow { .. } => "list_tags_for_show",
            Self::InsertSession { .. } => "insert_session",
            Self::ListSessionsForEpisode { .. } => "list_sessions_for_episode",
            Self::TotalRealSeconds { .. } => "total_real_seconds",
            Self::TotalSmartSpeedSaved { .. } => "total_smart_speed_saved",
            Self::Shutdown { .. } => "shutdown",
        }
    }
}
