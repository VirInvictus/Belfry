//! Playback + show_settings + listening_sessions integration tests.

use belfry_core::db::{
    Episode, EpisodeType, InboxPolicy, ListeningSession, Playback, PlayedState, Show, ShowSettings,
    spawn_worker,
};
use chrono::Utc;
use tempfile::tempdir;

fn make_show() -> Show {
    Show {
        id: 0,
        slug: "test".to_string(),
        feed_url: "https://example.com/test.rss".to_string(),
        title: "Test Show".to_string(),
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
        folder_path: "shows/test".to_string(),
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

// --- Playback -----------------------------------------------------------

#[tokio::test]
async fn playback_upsert_creates_then_updates() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    let pb = Playback {
        episode_id: ep_id,
        position: 120.5,
        played: PlayedState::InProgress,
        last_played: Some(Utc::now()),
        play_count: 1,
        starred: false,
        in_queue: true,
        queue_position: Some(0),
    };
    handle.upsert_playback(pb.clone()).await.unwrap();

    let got = handle.get_playback(ep_id).await.unwrap().unwrap();
    assert_eq!(got.position, 120.5);
    assert_eq!(got.played, PlayedState::InProgress);
    assert!(got.in_queue);

    let pb2 = Playback {
        position: 240.0,
        played: PlayedState::PlayedFully,
        play_count: 2,
        starred: true,
        in_queue: false,
        queue_position: None,
        ..pb
    };
    handle.upsert_playback(pb2).await.unwrap();

    let got2 = handle.get_playback(ep_id).await.unwrap().unwrap();
    assert_eq!(got2.position, 240.0);
    assert_eq!(got2.played, PlayedState::PlayedFully);
    assert!(got2.starred);
    assert!(!got2.in_queue);
}

#[tokio::test]
async fn playback_get_returns_none_for_missing() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();
    assert!(handle.get_playback(9999).await.unwrap().is_none());
}

// --- Show settings ------------------------------------------------------

#[tokio::test]
async fn show_settings_upsert_round_trip() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();

    let s = ShowSettings {
        show_id,
        playback_speed: 1.5,
        smart_speed: true,
        voice_boost: true,
        skip_intro: 30,
        skip_outro: 60,
        skip_forward: Some(45),
        skip_back: Some(10),
        inbox_policy: InboxPolicy::AlwaysQueue,
    };
    handle.upsert_show_settings(s.clone()).await.unwrap();

    let got = handle.get_show_settings(show_id).await.unwrap().unwrap();
    assert_eq!(got, s);
}

#[tokio::test]
async fn show_settings_inbox_policy_round_trips() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();

    for policy in [
        InboxPolicy::Inbox,
        InboxPolicy::AlwaysQueue,
        InboxPolicy::AlwaysArchive,
    ] {
        let s = ShowSettings {
            show_id,
            playback_speed: 1.0,
            smart_speed: false,
            voice_boost: false,
            skip_intro: 0,
            skip_outro: 0,
            skip_forward: None,
            skip_back: None,
            inbox_policy: policy,
        };
        handle.upsert_show_settings(s).await.unwrap();
        let got = handle.get_show_settings(show_id).await.unwrap().unwrap();
        assert_eq!(got.inbox_policy, policy);
    }
}

// --- Sessions -----------------------------------------------------------

fn make_session(
    episode_id: i64,
    started_at: i64,
    real_seconds: f64,
    saved: f64,
) -> ListeningSession {
    use chrono::TimeZone;
    ListeningSession {
        id: 0,
        episode_id,
        started_at: Utc.timestamp_opt(started_at, 0).unwrap(),
        ended_at: Utc
            .timestamp_opt(started_at + real_seconds as i64, 0)
            .unwrap(),
        real_seconds,
        audio_seconds: real_seconds + saved,
        smart_speed_saved: saved,
    }
}

#[tokio::test]
async fn session_insert_appends() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    let id1 = handle
        .insert_session(make_session(ep_id, 1_700_000_000, 600.0, 60.0))
        .await
        .unwrap();
    let id2 = handle
        .insert_session(make_session(ep_id, 1_700_001_000, 300.0, 30.0))
        .await
        .unwrap();
    assert_ne!(id1, id2);

    let sessions = handle.list_sessions_for_episode(ep_id).await.unwrap();
    assert_eq!(sessions.len(), 2);
    // Ordered by started_at ASC.
    assert!(sessions[0].started_at < sessions[1].started_at);
}

#[tokio::test]
async fn session_aggregates_in_window() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    handle
        .insert_session(make_session(ep_id, 1_700_000_000, 600.0, 60.0))
        .await
        .unwrap();
    handle
        .insert_session(make_session(ep_id, 1_700_001_000, 300.0, 30.0))
        .await
        .unwrap();
    handle
        .insert_session(make_session(ep_id, 1_700_010_000, 900.0, 90.0))
        .await
        .unwrap();

    // Window covers the first two sessions only.
    let real = handle
        .total_real_seconds(1_700_000_000, 1_700_002_000)
        .await
        .unwrap();
    assert!((real - 900.0).abs() < 0.01);

    let saved = handle
        .total_smart_speed_saved(1_700_000_000, 1_700_002_000)
        .await
        .unwrap();
    assert!((saved - 90.0).abs() < 0.01);

    // Wider window includes all three.
    let total_real = handle
        .total_real_seconds(1_700_000_000, 1_700_999_999)
        .await
        .unwrap();
    assert!((total_real - 1800.0).abs() < 0.01);
}

#[tokio::test]
async fn session_window_excludes_out_of_range() {
    let dir = tempdir().unwrap();
    let handle = spawn_worker(dir.path().join("belfry.db")).unwrap();

    let show_id = handle.insert_show(make_show()).await.unwrap();
    let ep_id = handle
        .insert_episode(make_episode(show_id, "ep-1"))
        .await
        .unwrap();

    handle
        .insert_session(make_session(ep_id, 1_700_000_000, 600.0, 60.0))
        .await
        .unwrap();

    // Window before the session.
    let real_before = handle.total_real_seconds(0, 1_699_999_999).await.unwrap();
    assert_eq!(real_before, 0.0);

    // Window after the session.
    let real_after = handle
        .total_real_seconds(1_700_000_001, 9_999_999_999)
        .await
        .unwrap();
    assert_eq!(real_after, 0.0);
}
