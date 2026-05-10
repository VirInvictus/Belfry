//! Parser: token stream → [`crate::ast::Expr`].
//!
//! Forgiving: unknown field names and unknown `is:NAME` predicates
//! degrade to non-fatal warnings, falling back to freeform text. New
//! operators can land in minor releases without breaking saved
//! perspectives.
