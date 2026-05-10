//! Belfry's headless data layer.
//!
//! Single-writer SQLite worker, feed fetch + parse pipeline, libmpv host
//! and filter chains, OPML round-trip. GUI-free; the foundation every
//! other crate builds on.
//!
//! See `spec.md` §2.1 (single-writer worker), §5 (playback engine),
//! §6 (library + filesystem), §7 (feed handling).

pub mod db;
pub mod errors;
pub mod fetch;
pub mod opml;
pub mod paths;
pub mod playback;
