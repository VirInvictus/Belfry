//! `<podcast:*>` namespace handler integration tests.
//!
//! Fixture XML files live in `tests/fixtures/feeds/`. The handler is
//! tolerant of malformed input and unknown elements; both behaviours
//! are verified here.

use belfry_core::fetch::namespace::parse;

const PODCAST_NAMESPACE: &str = include_str!("fixtures/feeds/podcast_namespace.xml");
const ITUNES_NAMESPACE: &str = include_str!("fixtures/feeds/itunes_namespace.xml");
const CHAPTER_URL: &str = include_str!("fixtures/feeds/chapter_url.xml");
const BASIC_RSS: &str = include_str!("fixtures/feeds/basic_rss.xml");
const BASIC_ATOM: &str = include_str!("fixtures/feeds/basic_atom.xml");
const MULTI_SEASON: &str = include_str!("fixtures/feeds/multi_season.xml");
const MALFORMED: &str = include_str!("fixtures/feeds/malformed.xml");
const EMPTY_FEED: &str = include_str!("fixtures/feeds/empty.xml");

#[test]
fn extracts_show_guid_from_channel_level() {
    let data = parse(PODCAST_NAMESPACE);
    assert_eq!(data.show_guid.as_deref(), Some("SHOW-GUID-12345"));
}

#[test]
fn extracts_per_item_podcast_guid() {
    let data = parse(PODCAST_NAMESPACE);
    assert_eq!(data.items.len(), 2);
    assert_eq!(data.items[0].podcast_guid.as_deref(), Some("EP-S1E1-GUID"));
    assert_eq!(data.items[1].podcast_guid.as_deref(), Some("EP-S2E5-GUID"));
}

#[test]
fn extracts_season_and_episode_numbers() {
    let data = parse(PODCAST_NAMESPACE);
    assert_eq!(data.items[0].season, Some(1));
    assert_eq!(data.items[0].episode, Some(1));
    assert_eq!(data.items[1].season, Some(2));
    assert_eq!(data.items[1].episode, Some(5));
}

#[test]
fn captures_rss_guid_for_position_merge() {
    let data = parse(PODCAST_NAMESPACE);
    assert_eq!(data.items[0].rss_guid.as_deref(), Some("pn-s1e1"));
    assert_eq!(data.items[1].rss_guid.as_deref(), Some("pn-s2e5"));
}

#[test]
fn extracts_chapters_url_from_empty_element() {
    let data = parse(CHAPTER_URL);
    assert_eq!(data.items.len(), 1);
    assert_eq!(
        data.items[0].chapters_url.as_deref(),
        Some("https://example.com/chapters/ep1-chapters.json")
    );
}

#[test]
fn ignores_itunes_namespace_elements() {
    // The handler is podcast: only — itunes: should be entirely ignored.
    let data = parse(ITUNES_NAMESPACE);
    assert_eq!(data.items.len(), 2);
    assert!(data.show_guid.is_none());
    assert!(data.items.iter().all(|i| i.podcast_guid.is_none()));
    assert!(data.items.iter().all(|i| i.season.is_none()));
    assert!(data.items.iter().all(|i| i.episode.is_none()));
}

#[test]
fn returns_items_for_basic_rss_without_namespace() {
    // basic_rss.xml has no <podcast:*> elements but does have items —
    // the handler still walks the structure and emits empty per-item
    // overlays in source order. Captures rss_guid for cross-check.
    let data = parse(BASIC_RSS);
    assert!(data.show_guid.is_none());
    assert_eq!(data.items.len(), 2);
    assert!(data.items.iter().all(|i| i.podcast_guid.is_none()));
    assert_eq!(data.items[0].rss_guid.as_deref(), Some("basic-rss-ep1"));
    assert_eq!(data.items[1].rss_guid.as_deref(), Some("basic-rss-ep2"));
}

#[test]
fn handles_atom_feed_structure() {
    // Atom uses <feed>/<entry> instead of <channel>/<item>; the handler
    // treats both the same. No podcast: namespace present so per-item
    // overlays are empty.
    let data = parse(BASIC_ATOM);
    assert_eq!(data.items.len(), 2);
}

#[test]
fn multi_season_extracts_each_season_episode_pair() {
    let data = parse(MULTI_SEASON);
    assert_eq!(data.items.len(), 3);
    assert_eq!(data.items[0].season, Some(3));
    assert_eq!(data.items[0].episode, Some(2));
    assert_eq!(data.items[1].season, Some(3));
    assert_eq!(data.items[1].episode, Some(1));
    assert_eq!(data.items[2].season, Some(2));
    assert_eq!(data.items[2].episode, Some(12));
}

#[test]
fn malformed_invalid_season_drops_to_none() {
    // <podcast:season>not-a-number</podcast:season> — season stays None,
    // but <podcast:episode>1</podcast:episode> still parses.
    let data = parse(MALFORMED);
    assert_eq!(data.items.len(), 2);
    assert_eq!(data.items[0].podcast_guid.as_deref(), Some("EP-GOOD-GUID"));
    assert_eq!(data.items[1].season, None);
    assert_eq!(data.items[1].episode, Some(1));
}

#[test]
fn empty_feed_returns_no_items() {
    let data = parse(EMPTY_FEED);
    assert_eq!(data.items.len(), 0);
    assert!(data.show_guid.is_none());
}

#[test]
fn unparseable_input_returns_default() {
    let data = parse("this is not xml at all just words");
    assert!(data.show_guid.is_none());
    assert_eq!(data.items.len(), 0);
}

#[test]
fn empty_input_returns_default() {
    let data = parse("");
    assert!(data.show_guid.is_none());
    assert_eq!(data.items.len(), 0);
}
