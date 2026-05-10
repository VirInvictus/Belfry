//! Feed fetch coordinator.
//!
//! Single-shot fetch + parse + apply pipeline. Conditional GET, HTTP
//! Basic auth (via libsecret behind a `CredentialStore` trait), `podcast:`
//! namespace handling, per-show error cooldown.
//!
//! No continuous background loop — Phase 4 GUI ticker triggers refreshes.
//! See `spec.md` §7 (feed handling) and `roadmap.md` Phase 2.

pub mod namespace;
