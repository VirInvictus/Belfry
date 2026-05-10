//! Belfry CLI — headless surface for subscriptions, queue, downloads,
//! triage, listening stats, and rescan.
//!
//! See `spec.md` §8.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "belfry-cli", version, about = "Belfry headless surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Subscribe to a podcast feed.
    Add {
        /// Feed URL (RSS / Atom / JSON Feed).
        url: String,
    },
    /// Unsubscribe from a podcast.
    Remove {
        /// Show slug (or unique prefix).
        slug: String,
    },
    /// List items by kind.
    List {
        #[arg(value_enum)]
        kind: ListKind,
    },
    /// Force-fetch a show (or all shows when no slug given).
    Refresh { slug: Option<String> },
    /// Manually download an episode.
    Download {
        /// `<show-slug>/<episode-slug>` or unique prefix.
        spec: String,
    },
    /// Add to / remove from / reorder the queue.
    Queue {
        #[command(subcommand)]
        action: QueueAction,
    },
    /// Triage an episode: queue / archive / star.
    Triage {
        spec: String,
        #[arg(long, conflicts_with_all = ["archive", "star"])]
        queue: bool,
        #[arg(long, conflicts_with_all = ["queue", "star"])]
        archive: bool,
        #[arg(long, conflicts_with_all = ["queue", "archive"])]
        star: bool,
    },
    /// Hand off an episode to standalone mpv.
    Play { spec: String },
    /// Export the library as OPML to stdout.
    ExportOpml,
    /// Import shows from an OPML file on stdin.
    ImportOpml,
    /// Rebuild the database from the filesystem.
    Rescan,
    /// Listening time, time saved, top shows.
    Stats,
    /// Manage HTTP Basic auth credentials.
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum ListKind {
    Inbox,
    Queue,
    Played,
    Saved,
    Downloads,
    Shows,
}

#[derive(Subcommand, Debug)]
enum QueueAction {
    /// Append an episode to the queue.
    Add { spec: String },
    /// Remove an episode from the queue.
    Remove { spec: String },
    /// Move an episode to a specific queue position.
    Reorder { spec: String, position: usize },
}

#[derive(Subcommand, Debug)]
enum AuthAction {
    /// Set HTTP Basic credentials for a show (prompts for password).
    Set { slug: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();

    // Phase 0 scaffold — every subcommand stubs to a tracing line.
    // Implementations land per the milestones in `roadmap.md`.
    tracing::info!(?cli.command, "not yet implemented");
    Ok(())
}
