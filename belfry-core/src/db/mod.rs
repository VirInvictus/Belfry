//! SQLite worker, read pool, and schema migrations.
//!
//! See `spec.md` §2.1 (single-writer worker), §4 (data model), §4.3 (migrations).

mod command;
mod connection;
mod crud;
pub mod domain;
pub mod fixtures;
pub(crate) mod migrations;
pub mod pool;
mod worker;

pub use domain::*;
pub use pool::ReadPool;
pub use worker::{WorkerHandle, spawn_worker};

/// Schema version of the bundled `0001_initial.sql`.
pub use migrations::CURRENT_VERSION;
