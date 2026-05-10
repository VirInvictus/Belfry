//! Read-only connection pool.
//!
//! Distinct from the writer task — multiple read-only handles, no write
//! contention. v0.1 ships a "pool" that opens a fresh handle on every
//! `open()` call; the structure is in place for Phase 1.8.5 / Phase 9
//! tuning if profiling shows the per-call open overhead matters.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db::connection;
use crate::errors::Result;

/// Cheap to clone — backed by a `PathBuf`.
#[derive(Clone)]
pub struct ReadPool {
    path: PathBuf,
}

impl ReadPool {
    /// Construct a pool. `_capacity` is reserved for a future bounded
    /// handle ring; v0.1 ignores it and opens fresh connections per call.
    pub fn new(path: PathBuf, _capacity: usize) -> Result<Self> {
        // Smoke-test: can we open a reader?
        let _conn = connection::open_reader(&path)?;
        Ok(Self { path })
    }

    /// The database file this pool reads from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open a read-only connection.
    ///
    /// In v0.1 every call opens a fresh handle. Post-1.0 this returns a
    /// pooled handle from a bounded ring (sized via `capacity`).
    pub fn open(&self) -> Result<Connection> {
        connection::open_reader(&self.path)
    }
}
