//! Read pool integration tests.

use std::sync::Arc;

use belfry_core::db::{ReadPool, Show, spawn_worker};
use chrono::Utc;
use tempfile::tempdir;

fn make_show(slug: &str) -> Show {
    Show {
        id: 0,
        slug: slug.to_string(),
        feed_url: format!("https://example.com/{slug}.rss"),
        title: format!("Show {slug}"),
        author: None,
        description: None,
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
async fn pool_validates_path_on_construct() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");

    // Spawn the worker so the DB exists.
    let handle = spawn_worker(db_path.clone()).unwrap();
    drop(handle);

    let pool = ReadPool::new(db_path, 1).unwrap();
    let _conn = pool.open().unwrap();
}

#[tokio::test]
async fn pool_reads_committed_writes() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path.clone()).unwrap();

    handle.insert_show(make_show("test")).await.unwrap();

    let pool = ReadPool::new(db_path, 1).unwrap();
    let conn = pool.open().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn pool_reads_concurrent_with_writer() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path.clone()).unwrap();

    let pool = Arc::new(ReadPool::new(db_path, 4).unwrap());

    // Spawn a writer task that streams 20 inserts through the worker.
    let writer_handle = handle.clone();
    let writer = tokio::spawn(async move {
        for i in 0..20 {
            let mut s = make_show(&format!("slug-{i}"));
            s.feed_url = format!("https://example.com/{i}.rss");
            writer_handle.insert_show(s).await.unwrap();
        }
    });

    // Spawn a reader on the blocking pool that hits the read-only path repeatedly.
    let pool_for_reader = Arc::clone(&pool);
    let reader = tokio::task::spawn_blocking(move || {
        for _ in 0..50 {
            let conn = pool_for_reader.open().unwrap();
            let _: i64 = conn
                .query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
                .unwrap();
        }
    });

    let (w, r) = tokio::join!(writer, reader);
    w.unwrap();
    r.unwrap();

    // Final check: all 20 shows landed; the writer wasn't blocked by the reader.
    let conn = pool.open().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 20);
}

#[tokio::test]
async fn pool_open_returns_independent_handles() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let _handle = spawn_worker(db_path.clone()).unwrap();

    let pool = ReadPool::new(db_path, 1).unwrap();

    // Two handles should both read independently — different prepared
    // statements don't conflict.
    let conn1 = pool.open().unwrap();
    let conn2 = pool.open().unwrap();

    let c1: i64 = conn1
        .query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
        .unwrap();
    let c2: i64 = conn2
        .query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(c1, c2);
}
