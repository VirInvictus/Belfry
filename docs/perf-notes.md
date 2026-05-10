# Belfry — Performance Notes

`heaptrack` / `massif` baselines, captured per phase against the `spec.md` §12 budget:

- **Idle (no playback):** < 100 MB
- **Playback active, Now Playing open:** < 200 MB
- **Cold start to first interactive frame on a 100-show library:** < 250 ms
- **Position-write latency (pause → DB committed):** < 50 ms

| Phase | Date | Idle RSS | Playback RSS | Cold start | Notes |
|---|---|---|---|---|---|
| 1 (v0.0.2) | 2026-05-09 | n/m | n/m | **1.5 ms** | Engine-only cold start on 10K-episode fixture. `belfry-core::db::spawn_worker` time only — schema-apply is no-op on the 2nd spawn (user_version already 1); this measures the open-connection + check-version path. Release build. RSS measurements ship in Phase 9 once the GTK shell is wired. Perf gate budget 250 ms; landed 1.5 ms (165× under). |

## Method

Cold-start measurement: `belfry-core/tests/db_fixtures.rs::cold_start_on_10k_episode_db`. Run via `cargo test --release -p belfry-core --test db_fixtures cold_start -- --ignored --nocapture`. The test seeds a fixture of 100 shows × 100 episodes via `fixtures::generate(FixtureScale::Large)`, drops the worker handle, then re-spawns the worker against the existing DB and times the spawn.

## What's measured at each phase

- **Phase 1** (v0.0.2): engine cold-start; worker round-trip latency on insert.
- **Phase 6** (v0.0.7): position-write latency.
- **Phase 9** (v0.1.0): full app cold-start with GUI; idle RSS; walking-skeleton end-to-end.
- **Phase 12** (v0.2.0): Smart Speed CPU overhead; session recorder write latency.
- **Phase 17** (v0.4.0): bulk-action commit time; storage-budget render time; FTS query latency.
- **Phase 19** (v1.0.0): full app under realistic load (Brandon's 72-feed library, full episode history).

## Known unknowns / risks

- Smart Speed time-saved attribution method validates in Phase 10's spike; current spec §13.2 flags this as an open question.
- libmpv filter graph latency on speed changes — flagged in spec §13.1, prototype in Phase 10.
- GTK4 + libmpv embedding — audio-only dodges the worst of it; revisited if/when video lands (post-2.x).
