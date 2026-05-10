//! Calibre-shaped search expression language for Belfry's library.
//!
//! Grammar shape ported from Atrium's `atrium-search` (see
//! `ATTRIBUTIONS.md`); the implementation is independent so the two
//! projects can evolve without coupling. Atrium's evaluator is typed
//! against tasks; Belfry's is typed against episodes and shows.
//!
//! Two execution paths:
//!
//! - **In-memory** ([`eval`]) — walks an [`ast::Expr`] against an
//!   already-loaded `Vec<Episode>`. Handles every operator the grammar
//!   exposes, including the SQL-incompatible ones (regex, fuzzy).
//! - **SQL** ([`sql`]) — translates an [`ast::Expr`] into a `WHERE`
//!   fragment + bind parameters when *every* node maps cleanly. Returns
//!   `None` for any subtree the translator can't safely express; the
//!   call site falls back to the in-memory path.

pub mod ast;
pub mod eval;
pub mod lex;
pub mod parse;
pub mod sql;
