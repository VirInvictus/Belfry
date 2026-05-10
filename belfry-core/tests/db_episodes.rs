//! Episodes CRUD + FTS5 trigger verification.

use belfry_core::db::{Episode, EpisodeType, Show, spawn_worker};
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
        last_fetched: None,
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

fn make_episode(show_id: i64, guid: &str, title: &str) -> Episode {
    Episode {
        id: 0,
        show_id,
        guid: guid.to_string(),
        title: title.to_string(),
        description: Some(format!("description of {title}")),
        pub_date: Some(Utc::now()),
        duration: Some(3600),
        file_size: Some(50_000_000),
        audio_url: Some(format!("https://example.com/{guid}.mp3")),
        audio_path: None,
        folder_path: format!("shows/test/{guid}"),
        mime_type: Some("audio/mpeg".to_string()),
        season: None,
        episode_number: None,
        episode_type: Some(EpisodeType::Full),
    }
}

#[tokio::test]
async fn insert_get_round_trip() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let id = handle
        .insert_episode(make_episode(show_id, "ep-1", "Episode One"))
        .await
        .unwrap();

    let ep = handle.get_episode(id).await.unwrap().unwrap();
    assert_eq!(ep.title, "Episode One");
    assert_eq!(ep.guid, "ep-1");
    assert_eq!(ep.episode_type, Some(EpisodeType::Full));
}

#[tokio::test]
async fn dedup_by_show_guid() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    handle
        .insert_episode(make_episode(show_id, "ep-1", "First"))
        .await
        .unwrap();

    // Same (show_id, guid) — should fail with unique constraint.
    let dup = handle
        .insert_episode(make_episode(show_id, "ep-1", "Second"))
        .await;
    assert!(dup.is_err());

    // Different guid succeeds.
    handle
        .insert_episode(make_episode(show_id, "ep-2", "Second"))
        .await
        .unwrap();

    let by_guid = handle
        .get_episode_by_guid(show_id, "ep-1".to_string())
        .await
        .unwrap();
    assert!(by_guid.is_some());
    assert_eq!(by_guid.unwrap().title, "First");
}

#[tokio::test]
async fn list_for_show_orders_by_pub_date_desc() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();

    let mut ep1 = make_episode(show_id, "ep-1", "Old");
    ep1.pub_date = Some(Utc::now() - chrono::Duration::days(2));
    let mut ep2 = make_episode(show_id, "ep-2", "New");
    ep2.pub_date = Some(Utc::now());

    handle.insert_episode(ep1).await.unwrap();
    handle.insert_episode(ep2).await.unwrap();

    let list = handle.list_episodes_for_show(show_id, 10).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].guid, "ep-2"); // newest first
    assert_eq!(list[1].guid, "ep-1");
}

#[tokio::test]
async fn update_modifies_episode() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let id = handle
        .insert_episode(make_episode(show_id, "ep-1", "Original"))
        .await
        .unwrap();

    let mut ep = handle.get_episode(id).await.unwrap().unwrap();
    ep.title = "Renamed".to_string();
    ep.duration = Some(7200);
    handle.update_episode(ep).await.unwrap();

    let after = handle.get_episode(id).await.unwrap().unwrap();
    assert_eq!(after.title, "Renamed");
    assert_eq!(after.duration, Some(7200));
}

#[tokio::test]
async fn delete_removes_episode() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let id = handle
        .insert_episode(make_episode(show_id, "ep-1", "Doomed"))
        .await
        .unwrap();

    handle.delete_episode(id).await.unwrap();
    assert!(handle.get_episode(id).await.unwrap().is_none());
}

#[tokio::test]
async fn cascade_delete_when_show_dropped() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1", "Bound"))
        .await
        .unwrap();

    handle.delete_show(show_id).await.unwrap();
    assert!(handle.get_episode(ep_id).await.unwrap().is_none());
}

#[tokio::test]
async fn fts_trigger_inserts_into_episode_fts() {
    use belfry_core::db::pool::ReadPool;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path.clone()).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    handle
        .insert_episode(make_episode(show_id, "ep-1", "Smart Speed Calibration"))
        .await
        .unwrap();
    handle
        .insert_episode(make_episode(show_id, "ep-2", "Voice Boost Tuning"))
        .await
        .unwrap();

    // Read directly via the read pool to verify the FTS5 trigger landed.
    let pool = ReadPool::new(db_path, 1).unwrap();
    let conn = pool.open().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episode_fts WHERE episode_fts MATCH 'smart'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let voice_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episode_fts WHERE episode_fts MATCH 'voice'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(voice_count, 1);
}

#[tokio::test]
async fn fts_trigger_clears_on_delete() {
    use belfry_core::db::pool::ReadPool;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path.clone()).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1", "Doomed Title Here"))
        .await
        .unwrap();

    handle.delete_episode(ep_id).await.unwrap();

    let pool = ReadPool::new(db_path, 1).unwrap();
    let conn = pool.open().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episode_fts WHERE episode_fts MATCH 'doomed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn fts_trigger_reflects_update() {
    use belfry_core::db::pool::ReadPool;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("belfry.db");
    let handle = spawn_worker(db_path.clone()).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();

    // Use a description that doesn't echo the title — otherwise the FTS
    // match would still hit on the description column after the title change.
    let mut ep = make_episode(show_id, "ep-1", "OriginalTitleHere");
    ep.description = Some("static unrelated content".to_string());
    let ep_id = handle.insert_episode(ep).await.unwrap();

    let mut ep = handle.get_episode(ep_id).await.unwrap().unwrap();
    ep.title = "Renamed Episode".to_string();
    handle.update_episode(ep).await.unwrap();

    let pool = ReadPool::new(db_path, 1).unwrap();
    let conn = pool.open().unwrap();

    let original_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episode_fts WHERE episode_fts MATCH 'originaltitlehere'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(original_count, 0);

    let renamed_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episode_fts WHERE episode_fts MATCH 'renamed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(renamed_count, 1);
}
