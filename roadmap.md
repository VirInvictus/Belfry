# Belfry — Roadmap

What's done, what's next, what's deferred. Twenty phases (0–19) mapping the journey from empty repo to v1.0. Current release: **v0.0.1** — Phase 0 closed.

The headline milestones:

- **v0.1 — Walking skeleton** (Phases 1–9). GTK shell, SQLite worker, basic libmpv playback, OPML import, MPRIS2, suspend inhibitor, HTTP Basic auth, filter bar with substring + boolean grammar.
- **v0.2 — The audio engine** (Phases 10–12). Smart Speed, Voice Boost, listening sessions, time-saved counter. The reason this app exists.
- **v0.3 — Triage and Now Playing polish** (Phases 13–15). Castro Inbox/Queue/Played model fully wired, per-show inbox policy, chapter rendering, dynamic accent, sleep timer with tap-to-extend, show notes pane.
- **v0.4 — Library polish at scale** (Phases 16–17). Full Calibre-grammar filtering with Perspectives, tags, bulk actions, listening-history view, storage budget, Continue Playing.
- **v0.5 — Distribution** (Phase 18). Flatpak manifest, Flathub submission, accessibility audit, localization scaffolding.
- **v1.0 — Ship** (Phase 19). Documentation, release notes, rescan-contract verification, schema-stability commitment.

Each phase ends with a `heaptrack` / `massif` checkpoint against the spec §12 budget. Every phase that adds a third-party crate calls it out — *no third-party deps without prior sign-off*.

---

## Lanes

Cross-phase commitments that grow alongside the features. None of these is a single phase's deliverable; each is a discipline maintained across the whole 20-phase arc.

### Debug harness

`belfry --debug` opens a debug surface inside the running application. Skeleton in Phase 0; grows phase by phase:

- **Phase 0** — `--debug` flag wired (no surface yet).
- **Phase 1** — `--debug fixture small/medium/large/stress` synthesizes 100 / 1K / 10K / 50K-episode databases.
- **Phase 2** — IO instrumentation: every SQLite statement logged via `tracing` at TRACE; `RUST_LOG=trace` reveals each statement plus elapsed wall time.
- **Phase 6** — libmpv property + filter telemetry surfaced in the debug pane.
- **Phase 9** — live RSS / heap sampling; "drop caches" affordance to expose retained allocations.
- **Phase 10** — Smart Speed real-time savings telemetry.
- **Phase 14** — accent extraction + chapter parser instrumentation.

The debug surface is **gated on `--debug`** so end users never see it. The integration test suite reuses the same fixture generators — there is no separate test-only fork.

### Test corpus

Synthetic + real fixtures, grown across phases:

- **Phase 1** — stress fixture generator (100 / 1K / 10K / 50K episodes).
- **Phase 2** — synthetic feed XML files for parser tests; recorded HTTP responses for conditional-GET scenarios.
- **Phase 3** — Brandon's actual 72-feed Overcast OPML enters the test suite; round-trip verified end-to-end.
- **Phase 14** — chapter-bearing real feeds (Hardcore History, Reconcilable Differences, etc.) for chapter parser fixtures.
- **Phase 16** — search expression parity test suite (in-memory ↔ SQL paths, ~50 cases minimum).

### Performance budgets

`heaptrack` capture at the end of every phase against the spec §12 budgets:

- Idle (no playback) < 100 MB
- Playback active, Now Playing open < 200 MB
- Cold start to first interactive frame on a 100-show library < 250 ms
- Position-write latency (pause → DB committed) < 50 ms

Captures land in `docs/perf-notes.md`. Features that miss budget get gated or revised before the phase closes.

### Documentation hygiene

Each phase updates the docs that change with it:

- `docs/keymap.md` — touched when shortcuts ship.
- `docs/podcast-namespace-coverage.md` — touched when namespace handling extends.
- `docs/perf-notes.md` — every phase appends a measurement row.
- `README.md` — features list updated at major version releases (v0.1, v0.2, v0.3, v0.4, v0.5, v1.0).
- `spec.md` — touched only when semantics change (rare and deliberate; see release discipline below).
- `ATTRIBUTIONS.md` — touched when new influences or licenses enter.

### Maintenance lanes

Code quality is a discipline, not a phase. Brandon's standing rules:

- **Mid-phase opportunistic.** When working on a file, if it crosses ~500 LOC, propose a split before adding more. If a comment block has rotted, fix it. If an unused helper turns up, delete it.
- **End-of-version mandatory.** Phases 9, 12, 15, 17, 18, 19 each include an explicit maintenance pass: comment audit (every public function has a one-line doc; no comments restating obvious code), file decomposition (any file >500 LOC reviewed; >800 LOC must split), `cargo clippy -- -W clippy::pedantic` review with action on the obvious wins, dead-code removal (`cargo udeps` for unused deps; manual sweep for unused public APIs).
- **Documentation refresh** at every major version release: README, keymap, perf-notes synchronized.

---

## Release discipline

Every release (every phase that bumps the third digit, plus the .x.0 milestone releases) touches:

1. **`VERSION`** at the repo root.
2. **`Cargo.toml`** `[workspace.package]` `version`.
3. **`patchnotes.md`** — newest entry at top. Atrium-shape: short title, date, narrative paragraph(s), bullet sub-sections (What's there / Tests / Maintenance / Known issues / Release tasks), test count delta when relevant.
4. **`data/io.github.virinvictus.Belfry.metainfo.xml`** — new `<release>` element with version + date + short description.
5. The phase's `roadmap.md` checkbox flipped to `[x]`.

The first three are commit-time; the fourth is required for AppStream validation; the fifth is the contract that the phase is closed.

---

## Shipped

### Phase 0 — Scaffolding (v0.0.1, closed 2026-05-09)

Cargo workspace stood up; four crates (`belfry-core`, `belfry-search`, `belfry-cli`, `belfry`); pinned v0.1 dependency set in `[workspace.dependencies]`; Meson wrapper; GitHub Actions CI (fmt + clippy + test gates); regression.sh; `data/` directory with Flatpak manifest skeleton, AppStream metainfo, .desktop file, populated GSettings schema (every key the spec mentions); `0001_initial.sql` with the full schema from spec §4.1; doc stubs for keymap, perf-notes, namespace coverage; `belfry --debug` flag wired (no surface yet — grows per the lane above).

`cargo check / clippy / fmt / test` all green; `scripts/regression.sh` passes. Workspace builds; no application logic yet.

---

## Plan

### Phase 1 — Schema + Worker (v0.0.2) — closed 2026-05-09

**Goal:** stand up the SQLite spine. Single-writer worker, read pool, schema migrations apply.

- [x] `belfry-core::db::worker` — tokio task that owns the writable `rusqlite::Connection`. UI sends `Command` enum variants via `mpsc::Sender`; replies via oneshot.
- [x] `belfry-core::db::pool` — read-only connection pool (multiple read-only handles).
- [x] `belfry-core::db::migrations` — versioned migrations runner; reads `0001_initial.sql`; applies via `user_version` PRAGMA.
- [x] Domain types: `Show`, `Episode`, `Playback`, `ShowSettings`, `Chapter`, `Tag`, `ListeningSession` — all `Serialize` + `Deserialize`.
- [x] CRUD command set on the worker: insert/update/delete for each domain table.
- [x] WAL + foreign keys ON + `synchronous=NORMAL` enforced at connection open.
- [x] **Debug harness Phase 1:** `belfry --fixture {small,medium,large,stress}` synthesizes 100 / 1K / 10K / 50K-episode databases.

**Tests (headless):** migration runner idempotent; round-trip every domain type; worker shutdown clean (no in-flight writes lost); FTS5 triggers fire on insert/update/delete; stress fixtures generate correct row counts. **65 tests pass.**

**Performance gate:** cold-start time on 10K-episode fixture < 250 ms (schema apply + worker spin-up). **Landed at 1.5 ms (165× under budget).**

**Release tasks:** bump to v0.0.2. ✓

### Phase 2 — Fetch Coordinator (v0.0.3)

**Goal:** feeds in. Conditional GET, podcast namespace handling, error cooldown.

- [ ] `belfry-core::fetch` — submodules: `scheduler`, `client`, `parser`, `namespace`.
- [ ] `feed-rs` integration for RSS / Atom / JSON Feed core.
- [ ] Hand-rolled `podcast:` namespace pass via `quick-xml`. v0.0.3 covers `<podcast:guid>`, `<podcast:season>`, `<podcast:episode>`; `<podcast:chapters>` parsed but not stored until Phase 14. Other namespace elements logged at TRACE and dropped.
- [ ] Conditional GET (`If-Modified-Since`, `If-None-Match`); 304 short-circuits the entire pipeline.
- [ ] HTTP 429 honors `Retry-After`; per-show error cooldown list.
- [ ] Per-show interval scheduler (default 1 hour, ±10% jitter).
- [ ] CLI: `belfry-cli refresh [--show=SLUG]` triggers fetch.
- [ ] **Debug harness Phase 2:** SQLite IO tracing — every statement + elapsed wall time at TRACE level.

**Tests (headless):** parse 10 real feeds drawn from Brandon's OPML (snapshot tests on parsed Episode rows); 304 short-circuit; HTTP 429 with `Retry-After`; synthetic feeds with namespace elements; GUID dedup behavior.

**Maintenance:** `belfry-core::fetch` split into submodules if `fetch.rs` >300 LOC.

**Performance gate:** 100 feed fetches concurrently in ≤ 5 s on warm cache.

**Release tasks:** bump to v0.0.3, namespace coverage matrix updated, perf-notes appended.

### Phase 3 — OPML + Authentication (v0.0.4)

**Goal:** import Brandon's actual OPML. HTTP Basic auth for private feeds.

- [ ] `belfry-core::opml::import` — parses `<outline type="rss">`, preserves `applePodcastsID` (Overcast format), tag round-trip via `category="..."`.
- [ ] `belfry-core::opml::export` — emits Belfry's tags as `category=`. Apps that ignore the attribute see a flat-but-correct list.
- [ ] HTTP Basic auth via `oo7` (libsecret). DB stores only the credential reference (`shows.auth_pass_ref`); password lives in the keyring.
- [ ] `belfry-cli auth set <slug>` — interactive password prompt → libsecret store.
- [ ] OPML import via CLI; GUI integration deferred to Phase 4.

**Tests (headless):** round-trip Brandon's actual 72-feed OPML — import → export → diff against original (every `applePodcastsID` preserved, all 72 entries); OPML with tags (synthetic) — round-trip preserves tags; HTTP Basic auth (mock server requires auth; with valid creds in keyring, fetch succeeds).

**Performance gate:** 72-feed OPML import completes in ≤ 1 s.

**Release tasks:** bump to v0.0.4.

### Phase 4 — GTK Shell + Filter Bar (v0.0.5)

**Goal:** the application window. Three-pane layout. Filter bar on every list (substring stopgap until Phase 5).

- [ ] `adw::Application` with the spec §2.3 widget tree.
- [ ] `AdwNavigationSplitView` outer + inner; AdwBreakpoints for mobile collapse.
- [ ] Sidebar populated with the §3.4 entries (Inbox / Queue / Played / Saved / Downloads / Shows / Tags / Settings) — initially empty.
- [ ] `GtkListView` for the episode list (signal factory, recycled rows).
- [ ] **Filter bar** above the episode list — `GtkSearchEntry` styled per spec. v0.0.5 ships substring-only matching against title + description; Phase 5 swaps in the engine.
- [ ] **Sortable columns** in the episode list (Title / Show / Date / Duration; default sort by Date desc).
- [ ] Show detail view (skeleton).
- [ ] Now Playing view stub (cover + title; transport ships in Phase 7).
- [ ] Wire the worker from `belfry-core` — UI sends commands via `mpsc::Sender`, receives `LibraryChanges` deltas via `glib::MainContext::channel`.
- [ ] OPML import via the GUI (file chooser portal).

**Tests (GUI starts here):** window appears; sidebar populated; episode list scrolls 10K episodes from the stress fixture without lag (snapshot — scrolled view height stays bounded); filter bar substring match works live; OPML import via file chooser — Brandon's 72-feed file imports cleanly; sortable column headers swap order on click.

**Maintenance:** comment audit on `belfry-core` (Phase 1-3 churn) — every public function has a one-line doc.

**Performance gate:** cold start to first interactive frame on 100-show library < 250 ms.

**Release tasks:** bump to v0.0.5; first screenshot in patchnotes.

### Phase 5 — `belfry-search` v1: substring + boolean (v0.0.6)

**Goal:** the filter bar acquires real grammar — boolean composition. Foundation for the full Calibre vocabulary in Phase 16.

- [ ] `belfry-search::lex` — tokens (TEXT, BOOL_OP, GROUP_OPEN, GROUP_CLOSE).
- [ ] `belfry-search::parse` — recursive descent for `expr = or_expr; or_expr = and_expr ("OR" and_expr)*; and_expr = not_expr (("AND")? not_expr)*; not_expr = ("NOT" | "!") not_expr | primary; primary = "(" or_expr ")" | bareword | quoted_string`. Unknown text falls through as substring (forgiving parser).
- [ ] `belfry-search::ast::Expr` — variants `Text`, `And(Box<Expr>, Box<Expr>)`, `Or(...)`, `Not(...)`, `Pass`.
- [ ] `belfry-search::eval` — in-memory walk against a loaded `Vec<Episode>`.
- [ ] `belfry-search::sql::try_translate` — `Expr → Option<(WHERE, params)>` for the boolean-only subset.
- [ ] Wire into Phase 4's filter bars — replace stopgap substring with `belfry-search::eval`.

**Tests (headless):** lexer / parser / AST unit tests (40+ cases); evaluator parity tests (`a AND b OR c` precedence, NOT, parens); SQL translator parity — same 40+ inputs, same id sets returned by both paths.

**Tests (GUI):** type `aether AND tech` in a filter bar; list filters correctly. Type unmatched parens; toast appears, list shows substring fallback.

**Performance gate:** filter response under 16 ms for 10K-episode in-memory eval.

**Release tasks:** bump to v0.0.6.

### Phase 6 — Playback Spine (v0.0.7)

**Goal:** play an episode. No filter chains yet — just spine.

- [ ] `belfry-core::playback::host` — singleton libmpv2 instance, kept alive across episodes.
- [ ] Variable speed (0.5×–3.0× in 0.05 increments; double-tap reset to default-speed from GSettings).
- [ ] Position persistence: write to DB on pause / seek (debounced 500 ms) / episode-end / app-quit / every 30 s during playback.
- [ ] Resume offset (`position - 3` seconds; from `resume-offset-seconds` GSettings key).
- [ ] Streaming — `audio_path IS NULL AND audio_url IS NOT NULL` → libmpv plays via HTTP with range-request resume. Download is a separate background job.
- [ ] CLI: `belfry-cli play <spec>` hands off to standalone mpv (per spec §8).
- [ ] **Debug harness Phase 6:** libmpv property + filter telemetry surfaced.

**Tests (headless):** play a test audio file; assert position written every 30 s; pause / seek / resume; streaming with mock HTTP range server.

**Tests (GUI):** click an episode; play starts; pause works; close app; reopen; episode resumes within 3 s of last position.

**Maintenance:** if `belfry-core::playback` >400 LOC, split into submodules (`host`, `state`, `position`).

**Performance gate:** position-write latency (pause → DB committed) < 50 ms.

**Release tasks:** bump to v0.0.7.

### Phase 7 — Now Playing UI (v0.0.8)

**Goal:** the listening surface. Cover, scrubber, transport, speed control. No chapters / show notes / sleep timer yet.

- [ ] Cover art display (full-bleed on mobile, sidebar-anchored on desktop).
- [ ] Scrubber with click-to-seek; position label updates from worker.
- [ ] Play / pause / skip-forward / skip-back transport.
- [ ] Speed control (slider + double-tap reset).
- [ ] Persistent **Now-bar** at window bottom whenever audio is loaded.
- [ ] Now Playing → episode detail navigation (full-bleed mode).
- [ ] Skip intervals from GSettings.

**Tests (GUI integration):** play from list; Now Playing populates; click scrubber; seek works; speed control changes mpv speed property; Now-bar persists across view switches.

**Performance gate:** Now Playing render < 100 ms after click (cold).

**Release tasks:** bump to v0.0.8.

### Phase 8 — System Integration (v0.0.9)

**Goal:** Belfry behaves like a citizen of GNOME 50. MPRIS, suspend, headphones, PipeWire.

- [ ] **MPRIS2 D-Bus interface** (`org.mpris.MediaPlayer2.Belfry`) — full metadata (title, artist=show, album=show, art-url, length, position), control (play/pause/next/previous/seek/stop/setposition).
- [ ] **Suspend inhibitor** during active playback (`org.freedesktop.login1::Inhibit`, scope=sleep, mode=block).
- [ ] **Headphone-button play/pause** via MPRIS forwarding (works through `mpris-proxy`).
- [ ] **PipeWire output sink picker** in Now Playing.
- [ ] Cover art exposed as a URL via the standard libadwaita pattern.

**Tests (headless):** `dbus-send` calls into MPRIS surface; verify play/pause/seek; suspend inhibitor active during playback (check via `loginctl list-inhibitors`).

**Tests (GUI):** GNOME's media overlay shows Belfry's currently-playing episode; Bluetooth headset play/pause works; output sink picker lists PipeWire sinks; switching mid-playback works.

**Maintenance:** Phase 1-7 comment audit — every public function has a one-line doc; remove obvious comments.

**Performance gate:** MPRIS metadata refresh < 50 ms after track change.

**Release tasks:** bump to v0.0.9.

### Phase 9 — v0.1 Release Gate (v0.1.0)

**Goal:** the walking skeleton ships. Maintenance pass + E2E test + first major-version tag.

- [ ] **Maintenance pass:** review every file >300 LOC for split candidates; comment audit (one-line public docs everywhere); dead-code removal; clippy `-W clippy::pedantic` review (action on the obvious wins).
- [ ] **Documentation refresh:** README features list, keymap reflects shipped shortcuts, `docs/perf-notes.md` populated with first measurements.
- [ ] **Walking-skeleton E2E test** (integration test in `tests/`): import OPML → fetch all 10 fixture feeds → play one episode → verify position persisted across simulated app-quit-and-restart → CLI `stats` reports correct counts.
- [ ] **Memory budget verified:** heaptrack capture meets §12 idle / active targets on 100-show library.
- [ ] **Debug harness Phase 9:** live RSS / heap sampling pane; "drop caches" affordance.
- [ ] Tag v0.1.0.

**Tests (E2E):** walking-skeleton full-flow test passes; rescan-contract partial verification on the v0.1 schema subset.

**Maintenance:** all Phase 1-8 cleanup landed.

**Performance gate:** §12 idle/active budgets met on 100-show library.

**Release tasks:** bump to v0.1.0; **major patchnotes entry** (the walking skeleton ships); AppStream metainfo release element added.

### Phase 10 — Smart Speed (v0.1.1)

**Goal:** silence-skipping with pitch-preserving stretch. The reason this app exists.

- [ ] `silenceremove` filter chain integration (default tunables from spec §5.1 / GSettings).
- [ ] `rubberband` filter for pitch-preserving stretch.
- [ ] Per-show toggle (`show_settings.smart_speed`).
- [ ] Now Playing indicator (lightning bolt; glows when active).
- [ ] **Time-saved attribution spike** — validate the §13.2 risk. Compute savings from rate of audio-time-emitted vs wall-clock-time-elapsed while filter is active. Document method in `docs/perf-notes.md`.
- [ ] **Debug harness Phase 10:** Smart Speed real-time savings telemetry.

**Tests (headless):** A/B audio test — play a fixture file with known silence runs; assert duration after Smart Speed is shorter; spectrum analysis on output rules out pitch artifact. Per-show override — episode from show A (Smart Speed on) plays sped-up; episode from show B (off) doesn't.

**Tests (GUI):** toggle Smart Speed in show detail; restart episode; indicator state matches setting.

**Performance gate:** Smart Speed adds ≤ 10 % CPU overhead on the playback path.

**Release tasks:** bump to v0.1.1; perf-notes appended.

### Phase 11 — Voice Boost (v0.1.2)

**Goal:** broadcast-quality compression + EQ for spoken word.

- [ ] `acompressor + equalizer + loudnorm` filter chain.
- [ ] Per-show toggle (`show_settings.voice_boost`).
- [ ] Now Playing indicator.
- [ ] Combined-chain ordering: silenceremove first, compress/EQ second.

**Tests (headless):** spectrum analysis on test audio (200 Hz cut, 3 kHz lift verified); loudness target (-16 LUFS within ±1 dB); combined chain runs silenceremove before compression.

**Tests (GUI):** toggle Voice Boost; episode restarts with chain active.

**Performance gate:** Voice Boost + Smart Speed combined: ≤ 15 % CPU overhead.

**Release tasks:** bump to v0.1.2.

### Phase 12 — Session Recorder + Time-Saved Counter (v0.2.0)

**Goal:** the audio engine ships. Sessions tracked, time saved surfaced, v0.2 milestone tag.

- [ ] `belfry-core::playback::session_recorder` — opens a `listening_sessions` row on play-start (or resume after pause >30 s); closes on pause / seek-out / episode-end / app-quit.
- [ ] Smart Speed savings attribution stored in each row.
- [ ] **Time-saved counter** in Now Playing footer ("Smart Speed saved 4m 12s this episode") and Settings ("Total time saved by Smart Speed: 14h 32m").
- [ ] **Maintenance pass for v0.2:** review all files added in Phases 10-12; refactor as needed; comment audit on playback engine.

**Tests (headless):** session row counts match playback events 1:1; cumulative time-saved query returns correct sum; sessions append-only schema invariant verified.

**Tests (GUI):** play episode; Now Playing footer updates time-saved live.

**Performance gate:** session recorder writes do not stutter playback (heaptrack on 1-hour playback session).

**Release tasks:** bump to v0.2.0; **major patchnotes entry** (the audio engine ships); AppStream metainfo release.

### Phase 13 — Castro Triage Model (v0.2.1)

**Goal:** Inbox / Queue / Played / Saved fully wired. Drag-reorder. Per-show inbox_policy.

- [ ] Inbox state derivation (`played=0 AND in_queue=0`).
- [ ] Queue management UI: drag-reorder, "Add to Queue" from any list, multi-add via bulk action.
- [ ] Played view date-banded (Today / Yesterday / This Week / Earlier — Atrium logbook pattern).
- [ ] Saved/starred cross-cut sidebar entry (live count).
- [ ] **Per-show `inbox_policy` editor** in show detail (`'inbox' | 'always_queue' | 'always_archive'`).
- [ ] Auto-route new episodes per the show's policy.

**Tests (headless):** state transitions (insert episode → Inbox; queue → Queue; play to end → Played); drag-reorder position assignment consistent (no gaps, no duplicates); inbox policies route new episodes correctly (3 fixtures: inbox / always_queue / always_archive).

**Tests (GUI):** drag an episode from Inbox to Queue; sidebar counts update live.

**Performance gate:** queue reorder of 1000-row queue: < 100 ms commit.

**Release tasks:** bump to v0.2.1.

### Phase 14 — Chapters + Dynamic Accent (v0.2.2)

**Goal:** chapters render, cover-art accent extraction lands.

- [ ] `<podcast:chapters>` JSON parsing on episode fetch (download external chapter file when present).
- [ ] ID3v2 CHAP frame fallback via `id3` crate (parsed when audio is downloaded).
- [ ] Show-notes timestamp regex inference (lowest precedence; marked `inferred` in DB).
- [ ] Chapter ticks on the scrubber.
- [ ] Chapter list panel in Now Playing (click to seek).
- [ ] Chapter image renders into cover slot when active chapter has one.
- [ ] **Dynamic accent extraction** from cover art — median-cut quantizer (port from Hermitage).
- [ ] Accent propagates to scrubber, chapter ticks, queue insertion indicator, Smart Speed lightning bolt.
- [ ] `shows.accent_rgb` populated on cover-image change.
- [ ] **Debug harness Phase 14:** accent extraction + chapter parser instrumentation.

**Tests (headless):** chapter parsing on 5 real feeds (snapshot tests on chapter rows); ID3 fallback on a fixture mp3 with CHAP frames; accent extraction returns sensible RGB for 10 test covers (visual verification).

**Tests (GUI):** play chapter-bearing episode; ticks visible; chapter list populated; switch episodes; accent color changes.

**Maintenance:** namespace coverage matrix updated.

**Performance gate:** accent extraction completes in < 50 ms per cover.

**Release tasks:** bump to v0.2.2.

### Phase 15 — Show Notes + Sleep Timer + Skip (v0.3.0)

**Goal:** Now Playing polish complete. v0.3 milestone tag.

- [ ] **Show notes pane** in Now Playing — `ammonia` sanitization (Viaduct's recipe); links open externally via `xdg-open`.
- [ ] **Sleep timer**: 15 / 30 / 45 / 60 minutes, end-of-episode, end-of-queue.
- [ ] **Tap-to-extend**: timer fires → 30 s window in which tapping Play extends by the same interval (Castro touch).
- [ ] Configurable skip intervals (per-show + global, from GSettings).
- [ ] Configurable resume offset (per-show, defaults from GSettings).
- [ ] **Maintenance pass for v0.3:** comment audit; file decomposition (any file >500 LOC reviewed); clippy pedantic review; `docs/keymap.md` updated with all shipped shortcuts.

**Tests (headless):** sleep timer fires at correct time; tap-to-extend within window reschedules; outside window (>30 s after fire) no extension.

**Tests (GUI):** set sleep timer; UI confirms; timer fires; toast offers extension; tap Play; extension confirmed; show notes link clicked → opens browser.

**Performance gate:** sleep timer doesn't drift > 100 ms over a 60-minute scheduled window.

**Release tasks:** bump to v0.3.0; **major patchnotes entry** (Now Playing complete); AppStream metainfo release.

### Phase 16 — `belfry-search` v2: full Calibre grammar (v0.3.1)

**Goal:** filter bars become powerful. Match modifiers, state predicates, field operators, sort, ranges, fuzzy.

- [ ] Field operators: `show:`, `author:`, `title:`, `note:`, `tag:`, `duration:`, `pub:`.
- [ ] Match modifiers: `tag:work` substring, `tag:=Work` exact, `tag:~regex`, `tag:?fuzzy` (Damerau-Levenshtein, length-aware threshold).
- [ ] State predicates: `is:played`, `is:in_progress`, `is:unplayed`, `is:starred`, `is:downloaded`, `is:in_queue`, `is:in_inbox`, `is:archived`.
- [ ] Date keywords: `today`, `yesterday`, `tomorrow`, `thisweek`, `lastweek`, `thismonth`, `Ndaysago`, `Ndaysout`.
- [ ] Comparison + range: `=`, `!=`, `<`, `<=`, `>`, `>=`, `lo..hi`.
- [ ] Sort modifiers: `sort:KEY`, `sort:-KEY`. Multiple sorts compose.
- [ ] SQL translator: all-or-nothing rule; falls back to in-memory for regex / fuzzy / in_progress / derived states.
- [ ] **Saved Perspectives** — sidebar section (appears when ≥1 saved). Save current filter via primary menu's *Save filter as Perspective…*.
- [ ] Perspective storage: TEXT column, expression re-parsed on every load (matches Atrium's pattern).

**Tests (headless):** comprehensive grammar parser tests (~100 cases covering every feature); in-memory ↔ SQL parity tests (~50 cases); forgiving parser tests (unknown field names, unbalanced quotes, trailing operators); sort modifier composition; Perspective round-trip (save → reload → re-evaluate).

**Tests (GUI):** type complex filter; results correct; save as Perspective; sidebar entry appears; click Perspective; filter applied.

**Performance gate:** filter eval < 16 ms on 10K episodes for the SQL-path subset; < 100 ms for in-memory fallbacks.

**Release tasks:** bump to v0.3.1.

### Phase 17 — Library Polish at Scale (v0.4.0)

**Goal:** v0.4 milestone — managing 70+ subscriptions becomes effortless.

- [ ] Tags + tag-based show filtering.
- [ ] **Bulk actions on shows** in show list — multi-select → set tags / retention / priority / inbox_policy.
- [ ] **Storage budget view** — "Belfry is using 12.4 GB; here are the largest shows" with one-click bulk-archive (Overcast pattern).
- [ ] **Listening-history view** — "this week / month / year, by show, by hour, by day" — read from `listening_sessions`.
- [ ] **Continue Playing** surface on app launch — last episode resumes with one click.
- [ ] OPML round-trip preserves tags via `<outline category="...">` extension.
- [ ] CLI feature parity with GUI (`stats`, `queue`, `triage`, `download`, `play`).
- [ ] **Maintenance pass for v0.4:** comment audit; large-file decomposition; clippy pedantic; dead-code sweep; documentation refresh (README, keymap, perf-notes).

**Tests (headless):** bulk action affects exactly the selected shows; storage budget query returns accurate sizes (vs `du`); listening-history aggregations correct (manual fixture verification).

**Tests (GUI):** multi-select 5 shows; set tag; all 5 reflect; storage view; click "Archive oldest 10 episodes from Show X"; episodes removed; app restart with paused episode → Continue Playing surfaces it.

**Performance gate:** bulk action on 70 shows: < 200 ms commit; storage view: < 500 ms render.

**Release tasks:** bump to v0.4.0; **major patchnotes entry** (library polish ships); AppStream metainfo release.

### Phase 18 — Distribution + Accessibility + Localization (v0.5.0)

**Goal:** Flatpak-distributable. Orca-friendly. Translatable.

- [ ] **Flatpak manifest finalized** — real sha256, real builds against `org.gnome.Platform//50` with bundled rubberband + ffmpeg + mpv.
- [ ] **Flathub submission** prepared (icons / metainfo / verified screenshots).
- [ ] **Localization scaffolding** — gettext, PO template extracted, `data/` updates for translation.
- [ ] **Accessibility audit** — Orca, AT-SPI labels on every interactive widget, focus rings, high-contrast mode.
- [ ] **Bundled fonts** registered via fontconfig at first run (Inter + Atkinson Hyperlegible).
- [ ] Notifications opt-in plumbing (off by default).
- [ ] Keyboard shortcuts everywhere — every action has an accelerator.
- [ ] Right-click context menus on episode rows + sidebar entries.
- [ ] **Distribution maintenance pass:** dead-code sweep; final clippy pedantic pass; documentation audit (README + spec + roadmap consistency).

**Tests:** Flatpak builds clean (`flatpak-builder` against the manifest); Flatpak runs in sandbox; OPML import via portal works; Orca audit (every interactive widget reads correctly); gettext extraction (every visible string is `_("...")` wrapped); high-contrast theme visual snapshot.

**Performance gate:** Flatpak runtime memory < native runtime + 10 % (sandbox overhead).

**Release tasks:** bump to v0.5.0; Flathub PR opened; **major patchnotes entry** (distribution ships).

### Phase 19 — v1.0 Ship (v1.0.0)

**Goal:** the v1.0 milestone tag. Documentation, schema-stability commitment, rescan-contract verified end-to-end.

- [ ] **User documentation** in `docs/` — keymap, FAQ, troubleshooting, the spec rendered for end users.
- [ ] **Release notes for v1.0** in `patchnotes.md` — narrative, not dry list.
- [ ] **Crash-reporting opt-in** (off by default; in-process only).
- [ ] **Rescan-contract verified end-to-end** — build → snapshot → delete → rescan → diff. Any drift outside the explicitly-DB-exclusive subset (playback / sessions / queue / triage) is a release-blocker.
- [ ] **Schema-stability commitment** — post-1.0, append-only and backwards-compatible. Drops/renames are major-bump-only.
- [ ] **Memory budget verified** at every measurement point on §12.
- [ ] **Final maintenance pass:** comment audit (final), file decomposition (final), clippy pedantic (final).
- [ ] Tag v1.0.0.

**Tests (E2E):** full rescan-contract verification; walking-skeleton + Smart Speed + Voice Boost + chapters + sleep timer + Perspectives all pass; all Flatpak permissions verified minimal-but-sufficient.

**Maintenance:** all known issues triaged or closed; documentation synchronized across spec / roadmap / README / patchnotes.

**Performance gate:** all §12 budgets met on Brandon's actual library (72 feeds + their full history loaded).

**Release tasks:** bump to v1.0.0; **flagship patchnotes entry**; AppStream metainfo v1.0.0 release element; Flathub release.

---

## Post-1.0 (deferred)

- **Background fetch via `org.freedesktop.portal.Background`** so closing the window doesn't pause the fetch loop.
- **gpodder.net or Nextcloud-gpodder sync** — opt-in only, never required for triage state. Castro's near-death-by-cloud-DB is the lesson.
- **Premium-podcast auth beyond HTTP Basic** — OAuth tokens, signed URLs. Schema is already future-proofed.
- **Stats dashboard** — top shows by listening time, by month, by year (Overcast Premium parity).
- **Cross-device queue continuity** (requires sync).
- **Right-click "Send to..." menu** (clipboard variants, share targets) — Viaduct's pattern.

## Maybe in 2.x

- **Transcripts** — ingestion (`<podcast:transcript>` for VTT/SRT/JSON/HTML), display, follow-along highlight, FTS search. Demoted from "killer post-1.0 differentiator" because it violates commitment #1 (listening device, not searching device). If it ships, it ships when v1.0 is rock-solid *and* there's a use case beyond search-novelty — most likely the follow-along reading-while-listening surface, not full-text query.
- **Video podcasts** — audio extraction is fine; a real video surface would mean GTK4 GL embedding, which spec §13 flags as load-bearing complexity.
