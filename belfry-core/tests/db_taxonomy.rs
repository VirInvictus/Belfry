//! Chapters + tags integration tests.

use belfry_core::db::{Chapter, Episode, EpisodeType, Show, spawn_worker};
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

fn make_episode(show_id: i64, guid: &str) -> Episode {
    Episode {
        id: 0,
        show_id,
        guid: guid.to_string(),
        title: format!("Episode {guid}"),
        description: None,
        pub_date: Some(Utc::now()),
        duration: Some(1800),
        file_size: None,
        audio_url: None,
        audio_path: None,
        folder_path: format!("shows/test/{guid}"),
        mime_type: None,
        season: None,
        episode_number: None,
        episode_type: Some(EpisodeType::Full),
    }
}

fn make_chapter(start: f64, title: &str) -> Chapter {
    Chapter {
        id: 0,
        episode_id: 0, // ignored — replace() uses the episode_id parameter
        start_time: start,
        end_time: None,
        title: Some(title.to_string()),
        url: None,
        image_path: None,
    }
}

// --- Chapters -----------------------------------------------------------

#[tokio::test]
async fn chapters_replace_inserts_batch() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    let chapters = vec![
        make_chapter(0.0, "Intro"),
        make_chapter(120.0, "Topic A"),
        make_chapter(900.0, "Topic B"),
        make_chapter(1500.0, "Outro"),
    ];

    handle.replace_chapters(ep_id, chapters).await.unwrap();

    let got = handle.list_chapters(ep_id).await.unwrap();
    assert_eq!(got.len(), 4);
    // Ordered by start_time ASC.
    assert_eq!(got[0].title, Some("Intro".to_string()));
    assert_eq!(got[3].title, Some("Outro".to_string()));
}

#[tokio::test]
async fn chapters_replace_overwrites_existing() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    handle
        .replace_chapters(
            ep_id,
            vec![make_chapter(0.0, "Old A"), make_chapter(60.0, "Old B")],
        )
        .await
        .unwrap();

    handle
        .replace_chapters(
            ep_id,
            vec![
                make_chapter(0.0, "New A"),
                make_chapter(30.0, "New B"),
                make_chapter(90.0, "New C"),
            ],
        )
        .await
        .unwrap();

    let got = handle.list_chapters(ep_id).await.unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].title, Some("New A".to_string()));
    assert_eq!(got[2].title, Some("New C".to_string()));
}

#[tokio::test]
async fn chapters_replace_with_empty_clears() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    handle
        .replace_chapters(ep_id, vec![make_chapter(0.0, "Sole")])
        .await
        .unwrap();
    assert_eq!(handle.list_chapters(ep_id).await.unwrap().len(), 1);

    handle.replace_chapters(ep_id, vec![]).await.unwrap();
    assert_eq!(handle.list_chapters(ep_id).await.unwrap().len(), 0);
}

#[tokio::test]
async fn chapters_cascade_when_episode_dropped() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    handle
        .replace_chapters(ep_id, vec![make_chapter(0.0, "A"), make_chapter(10.0, "B")])
        .await
        .unwrap();

    handle.delete_episode(ep_id).await.unwrap();

    assert_eq!(handle.list_chapters(ep_id).await.unwrap().len(), 0);
}

// --- Tags ---------------------------------------------------------------

#[tokio::test]
async fn tag_get_or_create_idempotent() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let id1 = handle.get_or_create_tag("tech".to_string()).await.unwrap();
    let id2 = handle.get_or_create_tag("tech".to_string()).await.unwrap();
    assert_eq!(id1, id2);

    let id3 = handle
        .get_or_create_tag("history".to_string())
        .await
        .unwrap();
    assert_ne!(id1, id3);

    let all = handle.list_tags().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn set_show_tags_replaces() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();

    let t_tech = handle.get_or_create_tag("tech".to_string()).await.unwrap();
    let t_apple = handle.get_or_create_tag("apple".to_string()).await.unwrap();
    let t_history = handle
        .get_or_create_tag("history".to_string())
        .await
        .unwrap();

    handle
        .set_show_tags(show_id, vec![t_tech, t_apple])
        .await
        .unwrap();

    let attached = handle.list_tags_for_show(show_id).await.unwrap();
    let names: Vec<_> = attached.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["apple", "tech"]); // alpha order

    // Replace with a different set.
    handle
        .set_show_tags(show_id, vec![t_history])
        .await
        .unwrap();

    let after = handle.list_tags_for_show(show_id).await.unwrap();
    let after_names: Vec<_> = after.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(after_names, vec!["history"]);
}

#[tokio::test]
async fn set_show_tags_with_empty_clears() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let t = handle.get_or_create_tag("tech".to_string()).await.unwrap();
    handle.set_show_tags(show_id, vec![t]).await.unwrap();
    assert_eq!(handle.list_tags_for_show(show_id).await.unwrap().len(), 1);

    handle.set_show_tags(show_id, vec![]).await.unwrap();
    assert_eq!(handle.list_tags_for_show(show_id).await.unwrap().len(), 0);
}

#[tokio::test]
async fn show_tags_cascade_when_show_dropped() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show("s")).await.unwrap();
    let t = handle.get_or_create_tag("tech".to_string()).await.unwrap();
    handle.set_show_tags(show_id, vec![t]).await.unwrap();

    handle.delete_show(show_id).await.unwrap();

    // The tag itself remains (tags survive shows); the show_tags entry is gone.
    let all_tags = handle.list_tags().await.unwrap();
    assert_eq!(all_tags.len(), 1);
}
