//! SQL translator: [`crate::ast::Expr`] → `(WHERE clause, params)` when
//! every node maps cleanly. `None` for any subtree the translator
//! can't safely express; the call site falls back to the in-memory
//! path. All-or-nothing — no partial translation that could disagree
//! with the in-memory evaluator silently.
