//! Stress fixture generator for `belfry --fixture <scale>`.
//!
//! Synthesizes a library at one of four scales — Small (100 episodes),
//! Medium (1K), Large (10K), Stress (50K). Uses simple uniform data;
//! good for exercising the schema and worker, not realistic-looking
//! for UI screenshots.

use std::str::FromStr;

use chrono::Utc;

use crate::db::WorkerHandle;
use crate::db::domain::{Episode, EpisodeType, Show};
use crate::errors::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureScale {
    /// 100 episodes across 10 shows.
    Small,
    /// 1,000 episodes across 50 shows.
    Medium,
    /// 10,000 episodes across 100 shows.
    Large,
    /// 50,000 episodes across 100 shows.
    Stress,
}

impl FixtureScale {
    pub fn shows(&self) -> usize {
        match self {
            Self::Small => 10,
            Self::Medium => 50,
            Self::Large | Self::Stress => 100,
        }
    }

    pub fn episodes_per_show(&self) -> usize {
        match self {
            Self::Small => 10,
            Self::Medium => 20,
            Self::Large => 100,
            Self::Stress => 500,
        }
    }
}

impl FromStr for FixtureScale {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "stress" => Ok(Self::Stress),
            other => Err(Error::InvalidEnum {
                field: "fixture_scale",
                value: other.to_string(),
            }),
        }
    }
}

/// Synthesize a library at the given scale.
pub async fn generate(handle: &WorkerHandle, scale: FixtureScale) -> Result<()> {
    let now = Utc::now();

    for show_idx in 0..scale.shows() {
        let show = Show {
            id: 0,
            slug: format!("show-{show_idx:04}"),
            feed_url: format!("https://example.com/show-{show_idx:04}.rss"),
            title: format!("Show {show_idx}"),
            author: Some(format!("Author {}", show_idx % 20)),
            description: Some(format!("Synthesized show {show_idx} for stress testing.")),
            homepage_url: None,
            cover_path: None,
            accent_rgb: None,
            apple_podcasts_id: None,
            last_fetched: Some(now),
            last_modified: None,
            etag: None,
            fetch_interval: 3600,
            auth_user: None,
            auth_pass_ref: None,
            auto_download: true,
            keep_count: 0,
            priority: (show_idx % 10) as i32,
            folder_path: format!("shows/show-{show_idx:04}"),
        };
        let show_id = handle.insert_show(show).await?;

        for ep_idx in 0..scale.episodes_per_show() {
            let pub_date = now - chrono::Duration::days(ep_idx as i64);
            let episode = Episode {
                id: 0,
                show_id,
                guid: format!("show-{show_idx:04}-ep-{ep_idx:04}"),
                title: format!("Show {show_idx} Episode {ep_idx}"),
                description: Some(format!("Synthesized episode {ep_idx} of show {show_idx}.")),
                pub_date: Some(pub_date),
                duration: Some(1800 + (ep_idx as u32 % 3600)),
                file_size: Some(50_000_000 + (ep_idx as u64 * 100_000)),
                audio_url: Some(format!(
                    "https://example.com/show-{show_idx:04}/ep-{ep_idx:04}.mp3"
                )),
                audio_path: None,
                folder_path: format!(
                    "shows/show-{show_idx:04}/{}-ep-{ep_idx:04}",
                    pub_date.format("%Y-%m-%d")
                ),
                mime_type: Some("audio/mpeg".to_string()),
                season: None,
                episode_number: Some(ep_idx as u32),
                episode_type: Some(EpisodeType::Full),
            };
            handle.insert_episode(episode).await?;
        }
    }

    Ok(())
}
