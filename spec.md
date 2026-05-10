# Belfry

A podcast client for GNOME 50. Overcast's playback intelligence on top of a
Calibre-style library, with a libmpv engine and a filesystem you can actually
read.

## Status

Specification draft. No code yet.

## License

GPLv3 (matches Framework, keeps the C/Python toolchain coherent).

## Philosophy

Three commitments, in priority order:

1. **The filesystem is real.** A user with `find`, `grep`, and `mpv` should be
   able to use their library without Belfry running. The database is an index,
   not the source of truth. If the DB is deleted, a rescan reconstructs it.
2. **Playback intelligence is the product.** Smart Speed and Voice Boost are
   not optional polish. They are why this exists instead of gPodder.
3. **Episodes are documents.** Transcripts, chapters, show notes, and metadata
   are first-class citizens, not afterthoughts. The Calibre lineage is literal:
   an episode is a record with attachments.

Non-goals: cloud sync (deferred), social features, recommendations, a store.
Belfry subscribes to feeds. It does not discover them for you.

## Stack

| Layer       | Choice                          | Why                                     |
|-------------|---------------------------------|------------------------------------------|
| Language    | Python 3.12+                    | Matches Hermitage. Fast enough for this. |
| UI          | GTK4 + libadwaita               | GNOME 50 native. Adwaita 1.6+.           |
| Playback    | libmpv (via python-mpv)         | Filter graph access for Smart Speed.     |
| Database    | SQLite (single file)            | Calibre pattern. WAL mode.               |
| Feeds       | feedparser + podcastparser      | feedparser for Atom/RSS, podcastparser for namespace correctness. |
| HTTP        | httpx                           | Async, HTTP/2, sane defaults.            |
| Async       | asyncio + GLib mainloop bridge  | Standard GNOME pattern.                  |
| Packaging   | Flatpak (org.gnome.Belfry)      | GNOME 50 distribution reality.           |

Python is the deliberate choice over Rust here. Hermitage already lives in
Python; sharing patterns matters more than micro-optimization for an app whose
hot path is libmpv (C) anyway.

## Library layout

```
~/Podcasts/                          # XDG_DATA_HOME/belfry by default
├── belfry.db                        # SQLite, single file, WAL mode
├── belfry.db.backup                 # Rotated nightly
├── metadata/
│   └── <show-slug>/
│       ├── feed.xml                 # Last-fetched raw feed (cache)
│       ├── cover.jpg
│       └── opml.xml                 # Per-show OPML fragment
└── shows/
    └── <show-slug>/
        ├── show.json                # Denormalized show metadata
        └── <YYYY-MM-DD>--<episode-slug>/
            ├── episode.json         # Title, GUID, pub date, duration, etc.
            ├── audio.<ext>          # mp3/m4a/ogg as published
            ├── chapters.json        # Parsed from podcast:chapters or ID3
            ├── transcript.vtt       # Or .srt, normalized to VTT on import
            ├── shownotes.html       # Sanitized
            └── cover.jpg            # Episode art if present, else show art
```

Slugs are filesystem-safe ASCII. Original titles preserved in DB. The folder
structure is what `dired`, `nautilus`, and a backup script see. The DB sees
the same thing plus indexes.

**Rescan contract:** delete `belfry.db`, run `belfry rescan`, get the same
library back minus playback state. Playback state is the only DB-exclusive
data, and it lives in a separate table that's trivial to export to JSON.

## Database schema (sketch)

```sql
-- Subscriptions
CREATE TABLE shows (
    id              INTEGER PRIMARY KEY,
    slug            TEXT UNIQUE NOT NULL,
    feed_url        TEXT UNIQUE NOT NULL,
    title           TEXT NOT NULL,
    author          TEXT,
    description     TEXT,
    homepage_url    TEXT,
    cover_path      TEXT,
    last_fetched    INTEGER,           -- unix epoch
    last_modified   TEXT,              -- HTTP header for conditional GET
    etag            TEXT,
    fetch_interval  INTEGER DEFAULT 3600,
    auto_download   INTEGER DEFAULT 1,
    keep_count      INTEGER DEFAULT 0, -- 0 = keep all
    priority        INTEGER DEFAULT 0, -- Overcast-style ordering
    folder_path     TEXT NOT NULL
);

CREATE TABLE episodes (
    id              INTEGER PRIMARY KEY,
    show_id         INTEGER NOT NULL REFERENCES shows(id) ON DELETE CASCADE,
    guid            TEXT NOT NULL,     -- canonical identity
    title           TEXT NOT NULL,
    pub_date        INTEGER,
    duration        INTEGER,           -- seconds
    file_size       INTEGER,
    audio_url       TEXT,
    audio_path      TEXT,              -- NULL until downloaded
    folder_path     TEXT NOT NULL,
    mime_type       TEXT,
    season          INTEGER,
    episode_number  INTEGER,
    episode_type    TEXT,              -- full/trailer/bonus
    UNIQUE (show_id, guid)
);

-- Playback state (the only non-reconstructible data)
CREATE TABLE playback (
    episode_id      INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
    position        REAL DEFAULT 0,    -- seconds, supports sub-second seek
    played          INTEGER DEFAULT 0, -- 0=unplayed, 1=in-progress, 2=finished
    last_played     INTEGER,
    play_count      INTEGER DEFAULT 0,
    starred         INTEGER DEFAULT 0,
    in_queue        INTEGER DEFAULT 0,
    queue_position  INTEGER
);

-- Per-show playback overrides (Overcast pattern)
CREATE TABLE show_settings (
    show_id         INTEGER PRIMARY KEY REFERENCES shows(id) ON DELETE CASCADE,
    playback_speed  REAL DEFAULT 1.0,
    smart_speed     INTEGER DEFAULT 1,
    voice_boost     INTEGER DEFAULT 0,
    skip_intro      INTEGER DEFAULT 0, -- seconds
    skip_outro      INTEGER DEFAULT 0
);

-- Chapters (podcast:chapters or ID3 CHAP)
CREATE TABLE chapters (
    id              INTEGER PRIMARY KEY,
    episode_id      INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    start_time      REAL NOT NULL,     -- seconds
    end_time        REAL,
    title           TEXT,
    url             TEXT,
    image_path      TEXT
);
CREATE INDEX idx_chapters_episode ON chapters(episode_id, start_time);

-- Transcript segments (for search + follow-along)
CREATE TABLE transcript_segments (
    id              INTEGER PRIMARY KEY,
    episode_id      INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    start_time      REAL NOT NULL,
    end_time        REAL,
    speaker         TEXT,
    text            TEXT NOT NULL
);
CREATE VIRTUAL TABLE transcript_fts USING fts5(
    text, content='transcript_segments', content_rowid='id'
);

-- Tags (Calibre pattern: many-to-many)
CREATE TABLE tags (
    id              INTEGER PRIMARY KEY,
    name            TEXT UNIQUE NOT NULL
);
CREATE TABLE show_tags (
    show_id         INTEGER REFERENCES shows(id) ON DELETE CASCADE,
    tag_id          INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (show_id, tag_id)
);
```

WAL mode, foreign keys on, `synchronous=NORMAL`. Same defaults Calibre learned
the hard way.

## Playback engine

Single libmpv instance, kept alive across episodes. python-mpv exposes the
property API directly; we use it.

### Smart Speed

Detect silence longer than a threshold, time-stretch through it. libmpv's
`af` (audio filter) chain supports this via `silenceremove` (from ffmpeg's
filter library) combined with `rubberband` for pitch-preserving stretch:

```
af=lavfi=[silenceremove=stop_periods=-1:stop_duration=0.3:stop_threshold=-40dB:stop_silence=0.15]
```

That's the rough shape. Tunables: silence threshold (-40dB default), minimum
silence duration before compression (0.3s), residual silence kept (0.15s for
naturalness). Per-show override via `show_settings.smart_speed`.

The Overcast trick is not the filter — it's the calibration. We expose
threshold and duration as advanced settings and ship sensible defaults.
Measure time saved per episode and surface it ("Smart Speed saved 4m 12s").
That's the hook.

### Voice Boost

Compression + EQ tuned for spoken word over phone speakers and earbuds.
Two-stage filter:

```
af=lavfi=[acompressor=threshold=-18dB:ratio=3:attack=5:release=50,
         equalizer=f=200:t=q:w=1:g=-3,
         equalizer=f=3000:t=q:w=1:g=4,
         loudnorm=I=-16:TP=-1.5:LRA=11]
```

Cuts mud at 200Hz, lifts presence at 3kHz, evens loudness to broadcast
standard (-16 LUFS, EBU R128 spec for podcast platforms). Per-show toggle.

### Combined chain

When both are on, silenceremove runs first (cheap), then compression/EQ.
Order matters: you don't want to compress silence before you remove it.

### State persistence

Position is written to DB on:
- Pause
- Seek (debounced to 500ms)
- Episode end
- App quit
- Every 30 seconds during playback (cheap insurance against crashes)

Resume offset is `position - 3` seconds for context, Overcast convention.

## Feed handling

### Polling

Per-show interval (default 1 hour), jittered ±10% to avoid thundering herd.
Conditional GET via `If-Modified-Since` and `If-None-Match`. 304 short-circuits
the entire pipeline.

### Parsing

`podcastparser` handles the Podcast Index namespace properly:
`<podcast:transcript>`, `<podcast:chapters>`, `<podcast:person>`,
`<podcast:locked>`, `<podcast:funding>`. Fall back to feedparser for
malformed feeds.

### Episode identity

`guid` is canonical. Episodes are deduplicated by `(show_id, guid)`. If a
publisher rotates GUIDs (it happens), we surface a warning and let the user
merge manually. We do not silently dedupe by title or URL — that hides bugs.

### Transcripts

`<podcast:transcript>` URLs are downloaded on episode fetch. Supported
formats: VTT, SRT, JSON (Podcast Index format), HTML (extract text). All
normalize to VTT on disk + segmented rows in `transcript_segments` for FTS.

### Chapters

Three sources, in precedence order:
1. `<podcast:chapters>` JSON (richest, supports images and URLs)
2. ID3v2 CHAP frames (parsed via mutagen on download)
3. Show notes timestamp parsing (regex, last resort, marked as inferred)

## UI

### Layout

GNOME HIG. Three-pane responsive layout, collapses to single-pane on mobile
widths (libadwaita `NavigationSplitView` + `NavigationView`).

```
┌─────────────────────────────────────────────────────────────┐
│ [≡]  Belfry                                    [+] [⚙]      │
├──────────┬──────────────────┬──────────────────────────────┤
│ Sidebar  │ Episode list     │ Now Playing / Episode detail │
│          │                  │                              │
│ • Queue  │ Show: The Daily  │ ┌─────────────┐              │
│ • Inbox  │ ─────────────    │ │  cover art  │              │
│ • Stars  │ ▶ Episode 1234   │ └─────────────┘              │
│ • Down.  │   Episode 1233   │                              │
│ ─────    │   Episode 1232   │ Title here                   │
│ Shows    │   ...            │ Show · Date · 47:23          │
│ • Show A │                  │                              │
│ • Show B │                  │ ─[chapters]─[transcript]──   │
│ • Show C │                  │                              │
│ ─────    │                  │ Show notes...                │
│ Tags     │                  │                              │
│ ─────    ┴──────────────────┤ ◀◀ 15  ▶  30 ▶▶   1.2× ⚡    │
│ Settings                    │ ━━━━━━━━●─────────  21:04    │
└─────────────────────────────┴──────────────────────────────┘
```

### Views

- **Queue**: Ordered playback list. Drag to reorder. Overcast's central metaphor.
- **Inbox**: New unplayed episodes from auto-download. Triage view.
- **Starred**: Saved-forever episodes.
- **Downloads**: Currently downloading + recently completed.
- **Shows**: Subscribed feeds, sorted by priority then alpha.
- **Tags**: User-defined Calibre-style categories.

### Now Playing

- Cover art (full-bleed on mobile, sidebar on desktop)
- Scrubber with chapter markers as ticks
- Speed control (0.5×–3.0×, 0.05 increments, double-tap to reset)
- Smart Speed indicator (lightning bolt, glows when active)
- Voice Boost indicator
- Sleep timer (15/30/45/60 min, end of episode)
- AirPlay equivalent: GNOME network audio sinks via PipeWire
- Chapter list (tappable, jumps to start)
- Transcript pane (scrolls with playback, current segment highlighted, click to seek)
- Show notes (HTML, sanitized, links open externally)

### Search

Cmd-F / Ctrl-F. Searches:
- Show titles and authors
- Episode titles and descriptions
- Transcript FTS (the killer feature; nobody else has this on Linux)

Results grouped by source. Transcript hits show segment context with
timestamp; click to play from that moment.

## CLI

`belfry` ships a CLI alongside the GUI. Hermitage and calibreQuarry already
established the pattern: GUI for browsing, CLI for batch ops.

```
belfry add <feed-url>           # Subscribe
belfry remove <show-slug>       # Unsubscribe
belfry list [--shows|--queue]   # List
belfry refresh [show-slug]      # Force fetch
belfry download <episode-spec>  # Manual download
belfry play <episode-spec>      # Hand off to standalone mpv
belfry export-opml > out.opml
belfry import-opml < in.opml
belfry rescan                   # Rebuild DB from filesystem
belfry stats                    # Time saved, episodes played, etc.
```

`<episode-spec>` is `<show-slug>/<episode-slug>` or partial-match prefix.

## Configuration

`~/.config/belfry/config.toml`. Sane defaults; the file is optional.

```toml
[library]
path = "~/Podcasts"

[playback]
default_speed = 1.0
smart_speed = true
voice_boost = false
skip_forward = 30
skip_back = 15
resume_offset = 3

[smart_speed]
silence_threshold_db = -40
min_silence_duration = 0.3
residual_silence = 0.15

[downloads]
max_concurrent = 3
auto_download_new = true
keep_unplayed = true
delete_after_played = false

[fetch]
default_interval_seconds = 3600
jitter_percent = 10
user_agent = "Belfry/0.1 (+https://github.com/vrnvctss/belfry)"
```

## Roadmap

### v0.1 — Walking skeleton

- [ ] GTK4 shell with three-pane layout
- [ ] SQLite schema, migrations
- [ ] Add/remove feeds, OPML import
- [ ] Episode list, basic playback (libmpv, no filters)
- [ ] Position persistence
- [ ] Filesystem mirror on download

### v0.2 — Playback intelligence

- [ ] Smart Speed filter chain
- [ ] Voice Boost filter chain
- [ ] Per-show settings overrides
- [ ] Time-saved tracking
- [ ] Sleep timer

### v0.3 — First-class metadata

- [ ] `<podcast:chapters>` parsing and display
- [ ] ID3 chapter fallback
- [ ] `<podcast:transcript>` download and parsing
- [ ] Transcript pane with follow-along highlighting
- [ ] FTS search across transcripts

### v0.4 — Library polish

- [ ] Tags + virtual collections
- [ ] Queue management with drag-reorder
- [ ] Starred episodes
- [ ] Per-show priority
- [ ] CLI feature parity

### v0.5 — Distribution

- [ ] Flatpak manifest
- [ ] Flathub submission
- [ ] Localization scaffolding (GNU gettext)
- [ ] Keyboard shortcuts everywhere
- [ ] Accessibility audit (Orca, high contrast)

### v1.0 — Ship

- [ ] User documentation
- [ ] Release notes
- [ ] Crash reporting opt-in
- [ ] Rescan contract verified end-to-end

### Post-1.0 (deferred)

- gpodder.net sync (add subscriptions + episode actions)
- Self-hosted sync server (separate repo)
- Cross-device queue continuity
- Statistics dashboard (Overcast clone)

## Risks and unknowns

1. **libmpv filter graph latency.** `silenceremove` followed by `rubberband`
   can introduce noticeable lag on speed changes. Needs prototyping before
   committing the v0.2 scope.
2. **Transcript format chaos.** The Podcast Index spec is young. JSON, VTT,
   SRT, and HTML all appear in the wild with varying compliance. Expect to
   write a normalizer with edge cases.
3. **Feed authentication.** Premium podcasts use HTTP Basic auth, OAuth
   tokens, or signed URLs. v1 supports Basic only; the rest is post-1.0.
4. **GTK4 + libmpv embedding.** GL render contexts in GTK4 are workable but
   fiddly. Audio-only playback dodges the worst of it. If we ever add video
   (some podcasts ship video), this becomes load-bearing.
5. **Database growth.** Transcripts at scale are not trivial. A talk-heavy
   library could hit 100MB of FTS index. SQLite handles it, but rebuild times
   on rescan need measuring.

## Out of scope, forever

- Recommendations, "discover" tabs, charts
- Social features (sharing, comments, ratings)
- A built-in podcast directory beyond OPML import
- Cloud anything that isn't a sync protocol
- DRM
- Video podcasts as a first-class format (audio extraction only)
- Windows/macOS support (GNOME-native, deliberately)

## Naming and branding

The bell tower. Episodes are bells; subscriptions are the rope-pull schedule;
the library is the chamber where they hang. Icon should evoke architecture,
not audio waveforms. No headphones, no microphones, no play triangles in the
logo. A belfry silhouette in libadwaita accent color does the work.

App ID: `org.gnome.Belfry` if accepted into GNOME Circle, else
`io.github.vrnvctss.Belfry`.
