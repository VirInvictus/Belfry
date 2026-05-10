//! Worker integration tests: spawn, CRUD shows, shutdown.

use belfry_core::db::{Show, spawn_worker};
use chrono::Utc;
use tempfile::tempdir;

fn make_show(slug: &str) -> Show {
    Show {
        id: 0,
        slug: slug.to_string(),
        feed_url: format!("https://example.com/{slug}.rss"),
        title: format!("Show {slug}"),
        author: Some("Test Author".to_string()),
        description: Some("A test show.".to_string()),
        homepage_url: None,
        cover_path: None,
        accent_rgb: None,
        apple_podcasts_id: None,
        last_fetched: Some(Utc::now()),
        last_modified: None,
        etag: None,
        fetch_interval: 3600,
        auth_user: None,
        auth_pass_ref: None,
        auto_download: true,
        keep_count: 0,
        priority: 0,
        folder_path: format!("shows/{slug}"),
    }
}

#[tokio::test]
async fn insert_list_round_trip() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");

    let handle = spawn_worker(db_path).unwrap();

    let id = handle.insert_show(make_show("test-show")).await.unwrap();
    assert!(id > 0);

    let shows = handle.list_shows().await.unwrap();
    assert_eq!(shows.len(), 1);
    assert_eq!(shows[0].slug, "test-show");
    assert_eq!(shows[0].id, id);
}

#[tokio::test]
async fn get_show_by_id() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path).unwrap();

    let id = handle.insert_show(make_show("test")).await.unwrap();
    let fetched = handle.get_show(id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().slug, "test");

    let missing = handle.get_show(999).await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn update_modifies_row() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path).unwrap();

    let id = handle.insert_show(make_show("test")).await.unwrap();
    let mut show = handle.get_show(id).await.unwrap().unwrap();
    show.title = "Renamed".to_string();
    show.priority = 100;
    handle.update_show(show).await.unwrap();

    let fetched = handle.get_show(id).await.unwrap().unwrap();
    assert_eq!(fetched.title, "Renamed");
    assert_eq!(fetched.priority, 100);
}

#[tokio::test]
async fn delete_removes_row() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path).unwrap();

    let id = handle.insert_show(make_show("test")).await.unwrap();
    handle.delete_show(id).await.unwrap();

    assert!(handle.get_show(id).await.unwrap().is_none());
    assert_eq!(handle.list_shows().await.unwrap().len(), 0);
}

#[tokio::test]
async fn list_orders_by_priority_then_title() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path).unwrap();

    let mut a = make_show("a");
    a.priority = 0;
    let mut b = make_show("b");
    b.priority = 10;
    let mut c = make_show("c");
    c.priority = 10;

    handle.insert_show(a).await.unwrap();
    handle.insert_show(b).await.unwrap();
    handle.insert_show(c).await.unwrap();

    let shows = handle.list_shows().await.unwrap();
    assert_eq!(shows.len(), 3);
    // priority DESC, then title ASC: b (p=10, "Show b"), c (p=10, "Show c"), a (p=0, "Show a")
    assert_eq!(shows[0].slug, "b");
    assert_eq!(shows[1].slug, "c");
    assert_eq!(shows[2].slug, "a");
}

#[tokio::test]
async fn migrations_run_on_spawn() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");

    // Spawn the worker; migrations run before WorkerHandle is returned.
    let handle = spawn_worker(db_path.clone()).unwrap();
    let _ = handle.list_shows().await.unwrap();

    // Drop the worker, re-spawn against the same DB. Should be no-op migrate.
    drop(handle);

    let handle2 = spawn_worker(db_path).unwrap();
    let shows = handle2.list_shows().await.unwrap();
    assert_eq!(shows.len(), 0);
}

#[tokio::test]
async fn shutdown_ack_returns() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path).unwrap();

    handle.insert_show(make_show("test")).await.unwrap();
    handle.shutdown_ack().await.unwrap();
    // After ack, the handle is still alive (the worker only exits when all
    // senders drop), but the operation completed cleanly.
}
