//! AST for the search expression language.
//!
//! `Expr` is the root node type. Field operators (`show:`, `tag:`,
//! `duration:`, `pub:`, etc.) and state predicates (`is:played`,
//! `is:starred`, etc.) live on enum variants here.
