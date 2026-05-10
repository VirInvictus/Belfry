//! Fixture generator integration tests.
//!
//! Large-scale + perf-gate tests are `#[ignore]` — run with
//! `cargo test -- --ignored` to exercise.

use belfry_core::db::fixtures::{FixtureScale, generate};
use belfry_core::db::spawn_worker;
use tempfile::tempdir;

#[tokio::test]
async fn small_fixture_has_correct_counts() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    generate(&handle, FixtureScale::Small).await.unwrap();

    let shows = handle.list_shows().await.unwrap();
    assert_eq!(shows.len(), 10);

    let first_show = &shows[0];
    let eps = handle
        .list_episodes_for_show(first_show.id, 100)
        .await
        .unwrap();
    assert_eq!(eps.len(), 10);
}

#[tokio::test]
async fn medium_fixture_has_correct_counts() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    generate(&handle, FixtureScale::Medium).await.unwrap();

    let shows = handle.list_shows().await.unwrap();
    assert_eq!(shows.len(), 50);

    let first_show = &shows[0];
    let eps = handle
        .list_episodes_for_show(first_show.id, 100)
        .await
        .unwrap();
    assert_eq!(eps.len(), 20);
}

#[tokio::test]
#[ignore = "slow — run with `cargo test -- --ignored fixture_large`"]
async fn fixture_large_has_correct_counts() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    generate(&handle, FixtureScale::Large).await.unwrap();

    let shows = handle.list_shows().await.unwrap();
    assert_eq!(shows.len(), 100);
}

/// Phase 1 perf gate: cold-start time on a 10K-episode DB.
///
/// The spec §12 budget is < 250 ms cold start to first interactive frame
/// on a 100-show library; this measures schema-apply + worker spin-up
/// against the v0.1 walking-skeleton fixture.
#[tokio::test]
#[ignore = "perf gate — run with `cargo test -- --ignored cold_start`"]
async fn cold_start_on_10k_episode_db() {
    use std::time::Instant;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");

    // Seed the fixture once.
    {
        let handle = spawn_worker(db_path.clone()).unwrap();
        generate(&handle, FixtureScale::Large).await.unwrap();
    }

    // Measure cold-start time on the existing DB. Tokio runtime is
    // already running; we measure spawn_worker time only.
    let start = Instant::now();
    let _handle = spawn_worker(db_path).unwrap();
    let elapsed = start.elapsed();

    eprintln!("cold start on 10K-episode DB: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 250,
        "cold start took {elapsed:?} (budget 250ms)"
    );
}
