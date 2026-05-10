# Belfry — Podcast Index Namespace Coverage Matrix

Which `<podcast:*>` elements Belfry parses and what it does with them. The namespace handler is project-owned code (no turn-key Rust analog of Python's `podcastparser`); the matrix grows as feeds in the wild surface new edge cases.

| Element | Status | Roadmap | Notes |
|---|---|---|---|
| `<podcast:chapters>` | unimplemented | v0.3 | JSON chapter list with images and URLs; preferred over ID3 CHAP. |
| `<podcast:transcript>` | **out of scope** | maybe 2.x | Demoted from killer-feature framing per spec §1 commitment #1. |
| `<podcast:person>` | unimplemented | post-1.0 | Display in show / episode detail. |
| `<podcast:locked>` | unimplemented | post-1.0 | Honor when present (informational). |
| `<podcast:funding>` | unimplemented | post-1.0 | Surface in show detail. |
| `<podcast:value>` | **out of scope** | — | Value4Value / streaming-sats; outside Belfry's scope. |
| `<podcast:soundbite>` | unimplemented | post-1.0 | Cue points; could land as bookmark gestures. |
| `<podcast:season>` / `<podcast:episode>` | unimplemented | v0.1 | Already stored in `episodes.season` / `episodes.episode_number`. |
| `<podcast:guid>` | unimplemented | v0.1 | Falls back to RSS `<guid>`; preferred when present (canonical show identity). |

Conventions:

- The handler runs as a `quick-xml` pass against the raw feed body, layered on top of `feed-rs`'s structured output.
- Unrecognized `podcast:*` elements are logged at TRACE and dropped. Never an error.
- Test fixtures for new feeds go in `belfry-core/tests/fixtures/feeds/`.
