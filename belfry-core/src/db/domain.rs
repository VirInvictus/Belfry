//! Domain types that map onto the `0001_initial.sql` schema.
//!
//! Field comments mirror spec §4.1. Types use Rust idiom (`bool` for
//! INTEGER 0/1, `chrono::DateTime<Utc>` for unix-epoch INTEGER columns,
//! enums for sentinel TEXT/INTEGER values).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub id: i64,
    pub slug: String,
    pub feed_url: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub cover_path: Option<String>,
    pub accent_rgb: Option<u32>,
    pub apple_podcasts_id: Option<String>,
    pub last_fetched: Option<DateTime<Utc>>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub fetch_interval: u32,
    pub auth_user: Option<String>,
    pub auth_pass_ref: Option<String>,
    pub auto_download: bool,
    pub keep_count: u32,
    pub priority: i32,
    pub folder_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeType {
    Full,
    Trailer,
    Bonus,
}

impl EpisodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Trailer => "trailer",
            Self::Bonus => "bonus",
        }
    }
}

impl std::str::FromStr for EpisodeType {
    type Err = crate::errors::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "full" => Ok(Self::Full),
            "trailer" => Ok(Self::Trailer),
            "bonus" => Ok(Self::Bonus),
            other => Err(crate::errors::Error::InvalidEnum {
                field: "episode_type",
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: i64,
    pub show_id: i64,
    pub guid: String,
    pub title: String,
    pub description: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub duration: Option<u32>,
    pub file_size: Option<u64>,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub folder_path: String,
    pub mime_type: Option<String>,
    pub season: Option<u32>,
    pub episode_number: Option<u32>,
    pub episode_type: Option<EpisodeType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlayedState {
    Unplayed = 0,
    InProgress = 1,
    PlayedFully = 2,
    ArchivedUnlistened = 3,
}

impl PlayedState {
    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::Unplayed),
            1 => Some(Self::InProgress),
            2 => Some(Self::PlayedFully),
            3 => Some(Self::ArchivedUnlistened),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playback {
    pub episode_id: i64,
    pub position: f64,
    pub played: PlayedState,
    pub last_played: Option<DateTime<Utc>>,
    pub play_count: u32,
    pub starred: bool,
    pub in_queue: bool,
    pub queue_position: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InboxPolicy {
    #[default]
    Inbox,
    AlwaysQueue,
    AlwaysArchive,
}

impl InboxPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::AlwaysQueue => "always_queue",
            Self::AlwaysArchive => "always_archive",
        }
    }
}

impl std::str::FromStr for InboxPolicy {
    type Err = crate::errors::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(Self::Inbox),
            "always_queue" => Ok(Self::AlwaysQueue),
            "always_archive" => Ok(Self::AlwaysArchive),
            other => Err(crate::errors::Error::InvalidEnum {
                field: "inbox_policy",
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowSettings {
    pub show_id: i64,
    pub playback_speed: f64,
    pub smart_speed: bool,
    pub voice_boost: bool,
    pub skip_intro: u32,
    pub skip_outro: u32,
    pub skip_forward: Option<u32>,
    pub skip_back: Option<u32>,
    pub inbox_policy: InboxPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListeningSession {
    pub id: i64,
    pub episode_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub real_seconds: f64,
    pub audio_seconds: f64,
    pub smart_speed_saved: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: i64,
    pub episode_id: i64,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_show() -> Show {
        Show {
            id: 1,
            slug: "test".to_string(),
            feed_url: "https://example.com/feed".to_string(),
            title: "Title".to_string(),
            author: Some("Author".to_string()),
            description: None,
            homepage_url: None,
            cover_path: None,
            accent_rgb: Some(0x00ff8800),
            apple_podcasts_id: Some("12345".to_string()),
            last_fetched: None,
            last_modified: None,
            etag: None,
            fetch_interval: 3600,
            auth_user: None,
            auth_pass_ref: None,
            auto_download: true,
            keep_count: 0,
            priority: 0,
            folder_path: "shows/test".to_string(),
        }
    }

    #[test]
    fn show_serde_round_trip() {
        let show = sample_show();
        let json = serde_json::to_string(&show).unwrap();
        let back: Show = serde_json::from_str(&json).unwrap();
        assert_eq!(show, back);
    }

    #[test]
    fn played_state_round_trip() {
        for state in [
            PlayedState::Unplayed,
            PlayedState::InProgress,
            PlayedState::PlayedFully,
            PlayedState::ArchivedUnlistened,
        ] {
            assert_eq!(PlayedState::from_i64(state as i64), Some(state));
        }
        assert_eq!(PlayedState::from_i64(99), None);
    }

    #[test]
    fn inbox_policy_str_round_trip() {
        for policy in [
            InboxPolicy::Inbox,
            InboxPolicy::AlwaysQueue,
            InboxPolicy::AlwaysArchive,
        ] {
            let parsed: InboxPolicy = policy.as_str().parse().unwrap();
            assert_eq!(parsed, policy);
        }
        assert!(matches!(
            "nonsense".parse::<InboxPolicy>(),
            Err(crate::errors::Error::InvalidEnum { .. })
        ));
    }

    #[test]
    fn inbox_policy_default_is_inbox() {
        assert_eq!(InboxPolicy::default(), InboxPolicy::Inbox);
    }

    #[test]
    fn episode_type_str_round_trip() {
        for ep_type in [EpisodeType::Full, EpisodeType::Trailer, EpisodeType::Bonus] {
            let parsed: EpisodeType = ep_type.as_str().parse().unwrap();
            assert_eq!(parsed, ep_type);
        }
        assert!(matches!(
            "invalid".parse::<EpisodeType>(),
            Err(crate::errors::Error::InvalidEnum { .. })
        ));
    }
}
