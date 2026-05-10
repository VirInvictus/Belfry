//! Hand-rolled `podcast:` namespace handler.
//!
//! `feed-rs` covers the RSS / Atom / JSON Feed core, but the Podcast Index
//! `<podcast:*>` namespace requires project-owned parsing. This pass walks
//! the raw XML body via `quick-xml`'s event reader and extracts the
//! elements covered by the namespace coverage matrix
//! (see `docs/podcast-namespace-coverage.md`).
//!
//! v0.0.3 covers:
//! - `<podcast:guid>` at channel level (canonical show identity).
//! - `<podcast:guid>` at item level (canonical episode identity).
//! - `<podcast:season>` (number).
//! - `<podcast:episode>` (number).
//! - `<podcast:chapters url="..." type="..."/>` — URL captured;
//!   storage deferred to Phase 14.
//!
//! Unknown `<podcast:*>` elements are logged at TRACE and dropped. The
//! parser is tolerant of malformed XML — recoverable errors are logged at
//! WARN; whatever was parsed cleanly is returned.
//!
//! ## Known limitations (v0.0.3)
//!
//! - Assumes the standard `podcast:` prefix. Feeds declaring the
//!   namespace under a non-standard prefix (e.g., `<itu:guid>` for the
//!   Podcast Index namespace) are not handled. Real-world feeds use
//!   `podcast:`; non-standard prefixes go to the edge-case log.
//! - First-text-wins for split-content text events. CDATA-wrapped
//!   `<podcast:guid>` content with mixed Text+CData events captures only
//!   the first segment. Real feeds don't split namespace text content
//!   this way; deferred unless we encounter a feed that does.

use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, Event};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct NamespaceData {
    /// `<podcast:guid>` at channel level — canonical show identity.
    pub show_guid: Option<String>,
    /// One entry per `<item>` (RSS) or `<entry>` (Atom) in source order.
    /// The parser merges these with feed-rs entries by position.
    pub items: Vec<ItemNamespaceData>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ItemNamespaceData {
    /// RSS `<guid>` for this item — used for cross-checking the
    /// position-based merge with feed-rs's parsed entries.
    pub rss_guid: Option<String>,
    /// `<podcast:guid>` at item level.
    pub podcast_guid: Option<String>,
    /// `<podcast:season>` number.
    pub season: Option<u32>,
    /// `<podcast:episode>` number.
    pub episode: Option<u32>,
    /// `<podcast:chapters url="..." />` URL. Storage deferred to Phase 14.
    pub chapters_url: Option<String>,
}

/// Parse an XML body for `<podcast:*>` namespace elements.
///
/// Tolerant of malformed XML: unrecoverable errors abort the loop and
/// return whatever was parsed cleanly. Returns `NamespaceData::default()`
/// on completely unparseable input.
pub fn parse(xml: &str) -> NamespaceData {
    let mut reader = Reader::from_str(xml);
    let mut data = NamespaceData::default();
    let mut state = ParseState::default();

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => handle_start(&mut state, e),
            Ok(Event::Empty(e)) => handle_empty(&mut state, e),
            Ok(Event::End(e)) => handle_end(&mut state, &mut data, e),
            Ok(Event::Text(t)) => {
                let raw = t.unescape().unwrap_or_default();
                handle_text(&mut state, &mut data, raw.as_ref());
            }
            Ok(Event::CData(t)) => {
                let raw = String::from_utf8_lossy(t.as_ref());
                handle_text(&mut state, &mut data, raw.as_ref());
            }
            Ok(_) => {} // Decl, PI, Comment, DocType — ignored.
            Err(e) => {
                tracing::warn!(?e, "namespace parser: recoverable XML error; skipping");
                continue;
            }
        }
    }

    data
}

#[derive(Debug, Default)]
struct ParseState {
    in_channel: bool,
    in_item: bool,
    current_item: Option<ItemNamespaceData>,
    /// What the next Text/CData event populates. Cleared once consumed
    /// (first-text-wins).
    pending_text: Option<TextTarget>,
}

#[derive(Debug, Clone, Copy)]
enum TextTarget {
    ShowGuid,
    ItemRssGuid,
    ItemPodcastGuid,
    ItemSeason,
    ItemEpisode,
}

fn handle_start(state: &mut ParseState, e: BytesStart<'_>) {
    let (prefix_str, local_str) = element_name(&e);

    state.pending_text = match (prefix_str.as_str(), local_str.as_str()) {
        ("", "channel") | ("", "feed") => {
            state.in_channel = true;
            None
        }
        ("", "item") | ("", "entry") => {
            state.in_item = true;
            state.current_item = Some(ItemNamespaceData::default());
            None
        }
        ("", "guid") if state.in_item => Some(TextTarget::ItemRssGuid),
        ("podcast", "guid") if state.in_item => Some(TextTarget::ItemPodcastGuid),
        ("podcast", "guid") if state.in_channel => Some(TextTarget::ShowGuid),
        ("podcast", "season") if state.in_item => Some(TextTarget::ItemSeason),
        ("podcast", "episode") if state.in_item => Some(TextTarget::ItemEpisode),
        ("podcast", other) => {
            tracing::trace!(element = %other, "namespace parser: skipping unknown <podcast:>");
            None
        }
        _ => None,
    };
}

fn handle_empty(state: &mut ParseState, e: BytesStart<'_>) {
    let (prefix_str, local_str) = element_name(&e);

    if prefix_str == "podcast" && local_str == "chapters" {
        if let Some(item) = state.current_item.as_mut() {
            for attr in e.attributes().with_checks(false).flatten() {
                if attr.key.as_ref() == b"url" {
                    // URLs in chapter elements don't typically contain XML
                    // entities; raw UTF-8 conversion is sufficient. If a real
                    // feed surfaces entities here, swap to
                    // `decode_and_unescape_value(reader.decoder())`.
                    let url = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                    item.chapters_url = Some(url);
                }
            }
        }
    }
}

fn handle_text(state: &mut ParseState, data: &mut NamespaceData, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let target = match state.pending_text.take() {
        Some(t) => t,
        None => return,
    };
    match target {
        TextTarget::ShowGuid => data.show_guid = Some(trimmed.to_string()),
        TextTarget::ItemRssGuid => {
            if let Some(item) = state.current_item.as_mut() {
                item.rss_guid = Some(trimmed.to_string());
            }
        }
        TextTarget::ItemPodcastGuid => {
            if let Some(item) = state.current_item.as_mut() {
                item.podcast_guid = Some(trimmed.to_string());
            }
        }
        TextTarget::ItemSeason => {
            if let Some(item) = state.current_item.as_mut() {
                item.season = trimmed.parse().ok();
            }
        }
        TextTarget::ItemEpisode => {
            if let Some(item) = state.current_item.as_mut() {
                item.episode = trimmed.parse().ok();
            }
        }
    }
}

fn handle_end(state: &mut ParseState, data: &mut NamespaceData, e: BytesEnd<'_>) {
    let (prefix_str, local_str) = end_element_name(&e);

    match (prefix_str.as_str(), local_str.as_str()) {
        ("", "channel") | ("", "feed") => state.in_channel = false,
        ("", "item") | ("", "entry") => {
            if let Some(item) = state.current_item.take() {
                data.items.push(item);
            }
            state.in_item = false;
            state.pending_text = None;
        }
        _ => {
            // Pending text consumed by handle_text already; clear in case the
            // element body was empty (no Text event fired between Start/End).
            state.pending_text = None;
        }
    }
}

fn element_name(e: &BytesStart<'_>) -> (String, String) {
    let name = e.name();
    let local_str = String::from_utf8_lossy(name.local_name().as_ref()).into_owned();
    let prefix_str = match name.prefix() {
        Some(p) => String::from_utf8_lossy(p.as_ref()).into_owned(),
        None => String::new(),
    };
    (prefix_str, local_str)
}

fn end_element_name(e: &BytesEnd<'_>) -> (String, String) {
    let name = e.name();
    let local_str = String::from_utf8_lossy(name.local_name().as_ref()).into_owned();
    let prefix_str = match name.prefix() {
        Some(p) => String::from_utf8_lossy(p.as_ref()).into_owned(),
        None => String::new(),
    };
    (prefix_str, local_str)
}
