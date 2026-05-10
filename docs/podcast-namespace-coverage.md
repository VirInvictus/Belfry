# Belfry — Podcast Index Namespace Coverage Matrix

Which `<podcast:*>` elements Belfry parses and what it does with them. The namespace handler is project-owned code (no turn-key Rust analog of Python's `podcastparser`); the matrix grows as feeds in the wild surface new edge cases.

| Element | Status | Phase | Notes |
|---|---|---|---|
| `<podcast:guid>` | **planned for v0.0.3** | 2 | Canonical show identity. Preferred over RSS `<guid>` when present; written to `shows.apple_podcasts_id`-adjacent column on import. |
| `<podcast:season>` / `<podcast:episode>` | **planned for v0.0.3** | 2 | Stored in `episodes.season` / `episodes.episode_number`. Falls back to `<itunes:season>` / `<itunes:episode>` when absent. |
| `<podcast:chapters>` | parsed, not stored | 2 (parse) → 14 (store) | URL extracted in v0.0.3 namespace handler; chapter file fetch + storage land at Phase 14 (chapters table population). The handler does **not** download the chapter file in Phase 2. |
| `<podcast:transcript>` | **out of scope** | maybe 2.x | Demoted per spec §1 commitment #1 ("listening device, not searching device"). |
| `<podcast:person>` | unimplemented | post-1.0 | Display in show / episode detail. |
| `<podcast:locked>` | unimplemented | post-1.0 | Honor when present (informational; do not auto-import elsewhere). |
| `<podcast:funding>` | unimplemented | post-1.0 | Surface in show detail. |
| `<podcast:soundbite>` | unimplemented | post-1.0 | Cue points; could land as bookmark gestures. |
| `<podcast:value>` | **out of scope** | — | Value4Value / streaming-sats; outside Belfry's scope. |
| `<podcast:license>` | unimplemented | post-1.0 | Surface in show detail when present. |
| `<podcast:images>` | unimplemented | post-1.0 | Multi-resolution show artwork; we use single cover URL for now. |
| `<podcast:medium>` | unimplemented | post-1.0 | Distinguishes podcast / music / video / film; may inform UI. |
| `<podcast:trailer>` | partial | 2 | Indirectly handled via existing `episode_type = 'trailer'` mapping from itunes namespace; explicit `<podcast:trailer>` element parsing post-1.0. |

## Conventions

- The handler runs as a `quick-xml` event-based pass against the raw feed body, layered on top of `feed-rs`'s structured output for RSS / Atom / JSON Feed core fields.
- **Unrecognized `podcast:*` elements are logged at TRACE and dropped. Never an error.** Forward compatibility with namespace evolution is part of the contract.
- **Tolerant of malformed XML.** Mismatched tags, encoding errors, and recoverable structural issues degrade to TRACE-logged warnings; the handler returns whatever was parsed cleanly. The `malformed.xml` fixture pins this behaviour.
- Test fixtures live in `belfry-core/tests/fixtures/feeds/`. Any new feed shape encountered in the wild gets a synthetic fixture before its handling lands in code.

## v0.0.3 fixture corpus (Phase 2)

Nine synthetic XML feeds in `belfry-core/tests/fixtures/feeds/`:

1. `basic_rss.xml` — RSS 2.0 with `<enclosure>` and `<guid>`.
2. `basic_atom.xml` — Atom 1.0 equivalent.
3. `basic_json.xml` — JSON Feed equivalent.
4. `podcast_namespace.xml` — `<podcast:guid>`, `<podcast:season>`, `<podcast:episode>`.
5. `itunes_namespace.xml` — `<itunes:duration>`, `<itunes:image>`, `<itunes:explicit>`, `<itunes:season>`, `<itunes:episodeType>`.
6. `chapter_url.xml` — `<podcast:chapters url="..." type="application/json+chapters"/>`. URL captured; file not fetched in Phase 2.
7. `multi_season.xml` — multiple seasons; verifies sorting / grouping / season-episode pair handling.
8. `malformed.xml` — mismatched tags + recoverable encoding errors. Verifies tolerance.
9. `empty.xml` — feed with zero items (valid).

## Real-feed verification

Before each release that touches the fetch pipeline, run `belfry-cli refresh --show=<slug>` against feeds drawn from Brandon's own OPML. Any namespace edge case discovered gets a synthetic fixture added to `tests/fixtures/feeds/` and a row in this document's edge-case log below.

## Edge-case log

(Empty until a real feed surfaces something unexpected.)
