# Belfry — Patch Notes

Newest at top. Each release notes user-visible changes; implementation churn lives in commit messages.

---

## v0.0.2 (2026-05-09) — Phase 1: SQLite Spine

The data layer lands. Single-writer worker, read pool, schema migrations apply, every domain table has CRUD reachable through `WorkerHandle`. The engine is fully exercisable headless — no GUI yet, but `belfry-cli` and `belfry --fixture small` both drive real database round-trips. Phase 4 will wire this to the GTK shell.

### What's there

**Worker pattern** (`belfry-core::db::worker`). A dedicated `tokio::task::spawn_blocking` thread owns the writable `rusqlite::Connection`. Consumers hold `mpsc::Sender<Command>` (clonable; cheap). Each command carries a `oneshot::Sender<Result<T>>` reply. The worker loop uses `mpsc::blocking_recv()` — synchronous receive on the blocking thread, no `.await` between command dispatch and reply. WAL mode + `synchronous=NORMAL` + `foreign_keys=ON` + `temp_store=MEMORY` PRAGMAs applied at connection open. Channel capacity 64; back-pressure naturally rate-limits writers.

**Migration runner** (`belfry-core::db::migrations`). Versioned via SQLite `user_version` PRAGMA. v0.1 ships `0001_initial.sql` (full schema from spec §4.1). Idempotent: spawning the worker against a fully-migrated DB skips the schema apply. Future migrations append as `if current < N` blocks.

**Domain types** (`belfry-core::db::domain`). Type-mapped from the schema with Rust idiom — `bool` over INTEGER 0/1, `chrono::DateTime<Utc>` over unix-epoch INTEGER, enums for sentinel values (`PlayedState`, `EpisodeType`, `InboxPolicy`). All `serde::{Serialize, Deserialize}`. `FromStr` properly implemented (the inherent `from_str` shadows clippy caught early).

**CRUD modules** — one file per table under `belfry-core::db::crud`:
- `shows` — insert / update / delete / get / list (priority DESC, title ASC)
- `episodes` — insert / update / delete / get / get_by_guid (dedup) / list_for_show (pub_date DESC)
- `playback` — upsert (insert-or-replace, keyed by episode_id) / get
- `show_settings` — upsert (keyed by show_id) / get
- `chapters` — replace (DELETE+INSERT batch in transaction) / list_for_episode
- `tags` — get_or_create (idempotent) / list / set_show_tags (atomic) / list_for_show
- `sessions` — insert (append-only) / list_for_episode / total_real_seconds(from, to) / total_smart_speed_saved(from, to)

**FTS5 triggers** verified end-to-end on insert / update / delete. Episode and show titles + descriptions indexed; `episode_fts MATCH 'term'` returns hits across both columns. Trigger correctness pinned in `db_episodes::fts_trigger_*` tests.

**Read pool** (`belfry-core::db::pool::ReadPool`). v0.1 stub opens a fresh read-only connection per `open()` call. The structure is in place for Phase 9 / post-1.0 tuning if profiling shows the per-call open overhead matters; for v0.1's expected read rates it doesn't. Concurrent reads while the worker writes are validated against WAL semantics in `db_pool::pool_reads_concurrent_with_writer`.

**Fixture generator** (`belfry-core::db::fixtures`). `generate(handle, scale)` synthesizes 100 / 1K / 10K / 50K-episode databases. Wired through `belfry --fixture <scale>` so the GTK binary can seed a working dev DB without an interactive flow. The generator is the v0.1 prepayment for the debug-harness lane (spec §3.4); the interactive debug pane lands in Phase 9.

**Worker tracing** (Phase 2 lane prepayment). Every command logs at TRACE with `kind` + `elapsed_us`. Enable with `RUST_LOG=trace`; Phase 2 will reuse this directly without adding new instrumentation.

### Tests

| File | Tests | What it covers |
|---|---|---|
| `paths` (unit) | 4 | XDG resolution; preference; fallback; missing |
| `connection` (unit) | 4 | PRAGMAs applied; parent dirs created; reader rejects writes |
| `migrations` (unit) | 3 | Initial schema applies; idempotent; FTS triggers present |
| `domain` (unit) | 5 | serde round-trip; enum boundaries (PlayedState, InboxPolicy, EpisodeType); InboxPolicy default |
| `db_basic` | 7 | Shows CRUD; ordering; migrations on spawn; shutdown ack |
| `db_episodes` | 9 | Episodes CRUD + dedup + cascade delete + 3× FTS trigger verifications (insert / delete / update) |
| `db_state` | 7 | Playback upsert (create + replace); show_settings round-trip; inbox_policy enum; sessions append + range aggregations |
| `db_taxonomy` | 8 | Chapters replace (insert / overwrite / clear / cascade); tag get_or_create idempotence; set_show_tags atomic replace; show_tags cascade |
| `db_pool` | 4 | Path validation on construct; reads see committed writes; concurrent reads-during-writes (WAL); independent handles |
| `db_fixtures` | 2 + 2 ignored | Small (100 ep) / Medium (1K ep) counts; Large + cold-start gated behind `#[ignore]` |

**65 tests** pass (`cargo test --workspace`). `cargo clippy --workspace --all-targets -- -D warnings` clean. `cargo fmt --all --check` clean. `scripts/regression.sh` green.

### Performance gate (spec §12)

Cold-start time on 10K-episode fixture: **1.5 ms** (release build). Budget 250 ms. **165× under budget.**

This measures `belfry_core::db::spawn_worker(existing_db)` — schema-apply is a no-op on the 2nd spawn (user_version already 1), so the path measured is open-connection + check-version + spawn-task. The full app cold-start measurement (with GTK init) is a Phase 9 deliverable.

Captured in `docs/perf-notes.md`. Re-run via `cargo test --release -p belfry-core --test db_fixtures cold_start -- --ignored --nocapture`.

### Worker public API surface

23 typed methods on `WorkerHandle` covering every CRUD operation. Cloneable. The Phase 4 GTK integration (and the Phase 2 fetch coordinator + Phase 6 playback engine) all hold a `WorkerHandle` and route writes through it. The architectural commitment from spec §2.1 — "the GTK thread holds an `mpsc::Sender<Command>` and never touches the writable connection directly" — is the contract enforced by this API shape.

### Architectural decisions

- **`tempfile` (3.14)** added as a workspace dev-dependency for integration tests. Pre-approved per the standing dep rule. Used only in `[dev-dependencies]`; doesn't ship in the release binary.
- **`std::str::FromStr` over inherent `from_str`** for `EpisodeType` / `InboxPolicy` / `FixtureScale`. Clippy `should_implement_trait` caught the shadowing of `std::str::FromStr::from_str`; the trait impl is the cleanup. Callers use `"value".parse::<EpisodeType>()?`, return `Result<Self, Error::InvalidEnum>`.
- **Append-only `listening_sessions`**. One row per playback span, never updated, never re-attributed. Schema invariant; the spec §4.1 `listening_sessions` description is now backed by code that has no UPDATE path on the worker.
- **FTS5 sync via triggers**, not application-level. Atomic with the row mutation; no consistency window where the FTS index lags the content table. Trigger correctness tested for insert, delete, and update paths.
- **Read pool stub opens fresh handles per call.** The "smoke test on construct" pattern validates the path opens; the actual handle ring is post-1.0 work. v0.1 read rates don't justify the complexity.

### Known issues / scope cuts

- **Bulk-insert command not added to the worker.** `fixtures::generate` does 1K / 10K / 50K single inserts through the worker; Stress fixture takes ~30s. For development that's acceptable. If/when application code needs bulk ops (OPML import in Phase 3 will), a `BulkInsert*` command lands.
- **`belfry-cli` subcommands still stub to tracing lines.** The CLI's surface is defined but not wired to the worker. Phase 2 (fetch coordinator) is the first phase that wires the CLI through to real engine operations.
- **No `LibraryChanges` delta channel yet.** Worker replies via per-command oneshots only; the broadcast channel for UI deltas lands in Phase 4 alongside the GTK consumer.
- **No real read-pool handle ring.** Phase 9 maintenance pass revisits if profiling shows the per-call connection-open overhead matters.

### Release tasks

- [x] `VERSION` → `0.0.2`
- [x] `Cargo.toml` `[workspace.package]` `version = "0.0.2"`
- [x] `spec.md` frontmatter version
- [x] `patchnotes.md` — this entry
- [x] `data/io.github.virinvictus.Belfry.metainfo.xml` — `<release>` for v0.0.2
- [x] `docs/perf-notes.md` — Phase 1 row + method
- [x] `roadmap.md` — Phase 1 marked closed

## v0.0.1 (2026-05-09) — Phase 0: Scaffolding

The empty-repo-to-buildable-workspace bump. Cargo workspace stands up with four crates, doc set is complete, build infrastructure is wired, and `scripts/regression.sh` passes clean. No application logic yet — `cargo run -p belfry` opens an `AdwApplicationWindow` with a placeholder `AdwStatusPage`. Phase 1 (the SQLite worker) is next.

### What's there

**Cargo workspace.** Four members: `belfry-core` (headless data layer; will host the SQLite worker, fetch coordinator, libmpv host, OPML round-trip), `belfry-search` (Calibre-shaped search expression language; ports the grammar shape from Atrium's `atrium-search` without a Cargo dep), `belfry-cli` (clap-based CLI matching every subcommand in spec §8 — every one currently stubs to a tracing line), and `belfry` (the GTK4 binary). All four crates compile clean; `cargo clippy --workspace --all-targets -- -D warnings` passes; `cargo fmt --all --check` passes.

**Pinned v0.1 dependency set** in `[workspace.dependencies]`. The full set: tokio, reqwest (rustls-tls + gzip + brotli), rusqlite (bundled + chrono + trace + FTS5 implied), feed-rs, quick-xml, libmpv2, ammonia, id3, image, oo7 (libsecret credential storage), zbus (MPRIS2 + suspend inhibitor), serde + serde_json + toml, regex, chrono, url, anyhow, thiserror, tracing + tracing-subscriber, clap, gtk4-rs (with `v4_16`), libadwaita-rs (with `v1_7`). License analysis lives in `ATTRIBUTIONS.md` — the chain forces GPL-3-or-later via librubberband (Smart Speed's pitch-preserving stretch).

**Top-level docs.** `README.md` (Viaduct-shaped intro + status banner + reference apps + brief stack), per-project `CLAUDE.md` (Belfry-specific overrides for the global `~/.claude/CLAUDE.md`), `spec.md` (the contract, restructured to lead with "Overcast's audio engine and Castro's triage model on a filesystem you can `ls`"), `roadmap.md` (six milestones; six-version arc from v0.1 walking skeleton to v1.0 ship — superseded by v0.0.1 by the 20-phase plan that lands alongside v0.0.1), `patchnotes.md` (this file), `ATTRIBUTIONS.md` (design lineage + dependency licenses + the GPL-3-via-rubberband chain), `VERSION` (single source of truth, bumped to 0.0.1).

**`0001_initial.sql`** in `belfry-core/src/db/migrations/` with the full schema from spec §4.1 — `shows`, `episodes`, `playback`, `show_settings`, `listening_sessions`, `chapters`, `tags`, `show_tags`, `episode_fts`, `show_fts`. FTS5 sync triggers wired. `accent_rgb` for cover-extracted dynamic accent (Phase 14); `apple_podcasts_id` for Overcast-format OPML round-trip; `auth_user` + `auth_pass_ref` for HTTP Basic auth (credential lives in libsecret, never inline); `inbox_policy` for the Castro per-show always-queue/always-archive override.

**Meson wrapper** (`meson.build`, `meson_options.txt`) handles the data files and runtime dependency checks (gtk4 ≥ 4.16, libadwaita-1 ≥ 1.7, mpv ≥ 0.36, sqlite3 ≥ 3.38). Cargo handles the actual build.

**`data/` directory:**

- `io.github.virinvictus.Belfry.json` — Flatpak manifest with bundled rubberband + ffmpeg (`--enable-gpl --enable-librubberband`) + mpv (libmpv-only build). sha256 placeholders for the upstream tarballs (filled at v0.5 distribution time).
- `io.github.virinvictus.Belfry.metainfo.xml` — AppStream metainfo with the v0.0.1 release entry.
- `io.github.virinvictus.Belfry.desktop` — desktop file.
- `io.github.virinvictus.Belfry.gschema.xml` — populated GSettings schema. Every key the spec mentions: library-path, default-speed, smart-speed-default, voice-boost-default, skip-forward-seconds, skip-back-seconds, resume-offset-seconds, smart-speed tuning trio, sleep-timer-default-minutes + tap-extend-window, inbox-policy-default (enum), max-concurrent-downloads, auto-download-new, delete-after-played + delay, default-fetch-interval-seconds, fetch-jitter-percent, notifications (off by default at high subscription counts), color-scheme (system/light/dark), window state.

**`.github/workflows/ci.yml`** — fmt + clippy + test on Ubuntu against the system GTK4 / libadwaita / libmpv / SQLite headers. Three matrix actions; warnings-as-errors.

**`scripts/regression.sh`** — local fmt + clippy + test gate, executable, matches the Atrium pattern.

**Docs stubs** in `docs/`: `keymap.md` (skeleton with planned shortcuts from spec §3 and Phase 5/8/13/15 work), `perf-notes.md` (heaptrack-budget table awaiting first measurements), `podcast-namespace-coverage.md` (matrix of `<podcast:*>` elements with their roadmap phase and status — transcripts explicitly listed as out of scope per commitment #1).

### Architectural decisions baked in at Phase 0

These choices land at scaffolding time so they shape every later phase:

- **GPL-3.0-or-later** as the source license. Forced by librubberband's GPL-2-or-later in the Smart Speed filter chain. Documented in `ATTRIBUTIONS.md` with the full chain analysis. Going more permissive would require dropping pitch-preserving stretch — which violates spec commitment #2 ("playback intelligence is the product"). **Updated at v0.0.1**: spec commitments restructured to four (adding "desktop-first, Calibre-shaped library UX") — the playback-intelligence point is now folded into commitment #1.
- **Castro Inbox / Queue / Played** as the primary triage model. The Calibre throughline that drove the original spec was wrong as a *frame* (Calibre is now back as a *UX vocabulary* — see below). Belfry is "a listening device, not a searching device"; transcripts demoted to maybe-2.x.
- **Desktop-first design** with Calibre's library-as-database UX layered on top of Castro's triage states. Every list view in Belfry is a queryable database — sortable columns, filter expressions, multi-select bulk actions, saved Perspectives. Spec §3 restructured around this ("Whitespace with respect; Colour with grace; The library is a queryable database; Every action visible, every UI control keyboard-accessible").
- **Four crates, not three.** Added `belfry-search` to the originally-three-crate plan. Atrium's `atrium-search` is the reference but Belfry doesn't depend on it as a Cargo crate — the evaluator and SQL translator are typed against domains, and forcing a generic API on Atrium's stable v0.13 code (or adding podcast fields to a project that doesn't need them) was the wrong direction. Port the parser shape; ship our own engine. Codified as a feedback rule in memory ("port the shape, don't depend on the crate").
- **HTTP Basic auth in v0.1, not post-1.0.** Real-world podcast feeds use it; Brandon's own OPML has at least one private/authenticated Substack feed. The credential lives in libsecret via `oo7`; the DB stores only a reference.

### v0.0.1 doc updates landed alongside Phase 0

The v0.0.1 patchnotes entry coincides with substantial spec / roadmap restructuring driven by Brandon's "desktop-first, Calibre as library UX" framing:

- **`spec.md` §1** — four commitments instead of three; reference-apps list grew (Calibre and Hermitage added; both tied to specific concrete features Belfry borrows).
- **`spec.md` §3** — restructured from five subsections to seven. New §3.1 *Design Principles* (whitespace, colour, library-as-database, every-action-visible). New §3.5 *Episode List* (sortable columns, multi-select, filter bar, keyboard nav — the Calibre interaction model on the list pane). Renamed §3.7 from "Library Search" to *Filtering, Search, and Perspectives* — filter and search are the same surface; saved filters become Perspectives; *every list is a filterable view*.
- **`spec.md` §1 reference apps** — added Calibre (the library-as-queryable-database mental model) and Hermitage (visual aesthetic + accent extraction).
- **`roadmap.md`** — full rewrite as a 20-phase plan (Phases 0–19), keyed to the six-version milestone arc. Each phase has goal / deliverables / tests (headless before GUI, GUI after) / maintenance notes / performance gate / release tasks. Added explicit *Lanes* section for cross-phase commitments (debug harness, test corpus, performance budgets, documentation hygiene, maintenance lanes). Added explicit *Release discipline* section codifying the VERSION + Cargo.toml + patchnotes + AppStream + roadmap-checkbox flow.
- **`ATTRIBUTIONS.md`** — added Calibre as a UX-and-vocabulary influence (separate from the search-grammar lineage already credited via Atrium).

### Tests

`cargo check / clippy / fmt / test` all green; zero unit tests so far (every crate has a `lib.rs` with module declarations and empty submodule files). `scripts/regression.sh` passes clean. The first unit tests land at Phase 1 with the SQLite worker.

### Known issues / scope cuts

- **`logo.svg`** referenced from `README.md` and `data/` but not yet created. Brandon will provide.
- **Flatpak manifest sha256 placeholders** — `TODO-fill-on-first-build` for the rubberband + ffmpeg + mpv tarballs. Filled at Phase 18 (v0.5.0 distribution).
- **`belfry-search` modules empty.** All five (`lex`, `parse`, `ast`, `eval`, `sql`) declared but contain only doc comments. Real implementation lands in Phases 5 (substring + boolean) and 16 (full Calibre grammar).
- **`belfry --debug` flag wired but no surface yet.** The debug pane grows phase by phase per the *Lanes* section in `roadmap.md`.

### Release tasks

- [x] `VERSION` → `0.0.1`
- [x] `Cargo.toml` `[workspace.package]` `version = "0.0.1"`
- [x] `patchnotes.md` — this entry
- [x] `data/io.github.virinvictus.Belfry.metainfo.xml` — `<release>` for v0.0.1
- [x] `roadmap.md` — Phase 0 marked closed
