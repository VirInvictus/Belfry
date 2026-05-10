# Belfry — Application Specification

**Version:** 0.0.2 (Phase 1 — SQLite worker + read pool + fixtures shipped; no UI integration yet)
**Target:** GNOME 50+, GTK4 ≥ 4.16, libadwaita ≥ 1.7
**Language:** Rust (2024 Edition)
**Build System:** Cargo workspace (`belfry-core` + `belfry-search` + `belfry-cli` + `belfry`) / Meson wrapper for Flatpak packaging
**License:** GNU GPL v3.0 or later (forced by librubberband's GPL-2-or-later in the Smart Speed filter chain — see `ATTRIBUTIONS.md`. The most permissive license compatible with shipping pitch-preserving time-stretch; aligns with the GNOME ecosystem; matches Framework.)

---

## 1. Mission Statement

Belfry is a podcast client for GNOME 50: **Overcast's audio engine and Castro's triage model, with Calibre's library-as-database UX, on a filesystem you can `ls`.**

Four commitments, in priority order:

1. **Belfry is a listening device, not a searching device.** Playback ergonomics over library archeology. Smart Speed and Voice Boost are calibrated like Overcast; the Castro **Inbox → Queue → Played** flow is the daily metaphor; the Now Playing surface gets disproportionate polish. Library search exists to find episodes you've already heard, not to discover new ones. Belfry subscribes to feeds — it does not discover them for you.
2. **Desktop-first, Calibre-shaped library UX.** Belfry uses desktop whitespace and colour with discipline (Hermitage's accent extraction; libadwaita's restrained palette). Three panes by default; mobile collapse is the *exception*, not the goal. Every list view is a queryable database — sortable columns, filter expressions, multi-select for bulk actions, saved Perspectives. Calibre's interaction model layered on top of Castro's triage states. This is not a phone app stretched to a desktop window.
3. **The filesystem is real.** A user with `find`, `grep`, and `mpv` can use their library without Belfry running. The database is an index, not the source of truth. If `belfry.db` is deleted, `belfry-cli rescan` reconstructs the library minus playback state.
4. **No second-class media.** Per-show overrides for speed, smart speed, voice boost, skip intro/outro, retention, and inbox policy. A 4-hour Hardcore History episode and a 25-minute Cortex episode have nothing in common except the playback engine; their settings are independent.

Reference apps:

- **Overcast** (Marco Arment) — the audio-engine bar. Smart Speed + Voice Boost calibration, per-show priority within lists, time-saved counter as the daily retention hook.
- **Castro** — the **Inbox → Queue → Archive** triage model. Central insight: with hundreds of subscriptions, the daily question is "what's *next*?" not "what's in my library?" Per-show "always queue / always archive" rules let the user write the rules once and stop triaging the same shows every week.
- **Calibre** (Kovid Goyal et al.) — the library-as-queryable-database mental model. Every list view in Belfry has a filter bar with the full expression grammar; saved filters become first-class Perspectives in the sidebar. Sortable columns, multi-select bulk actions, and Wings-style virtual collections are all Calibre's gift, ported into a podcast-shaped vocabulary.
- **NetNewsWire** (Brent Simmons, via Viaduct) — single-writer SQLite worker, conditional GET, the architectural twin pattern. Belfry is to podcasts what Viaduct is to RSS.
- **Hermitage** (Brandon's own) — the visual aesthetic. Cover art is the visual unit; per-show accent extracted from cover hue (median-cut quantizer); the Codex full-bleed detail surface.

Non-goals: cloud sync, social features, recommendations, content discovery, a built-in directory, transcripts (a 2.x maybe at most). See §14.

---

## 2. Architecture

### 2.1 Single-Writer SQLite Worker

A dedicated tokio task owns the writable `rusqlite::Connection`. The GTK thread holds an `mpsc::Sender<Command>` and never touches the writable connection directly. Reads use a separate read-only connection pool that the worker does not own. WAL mode is mandatory.

This mirrors the pattern shipped in Viaduct and Atrium. It eliminates an entire class of UI-thread-blocking and write-conflict bugs, which matters more here than in either of those apps because Belfry has *four* independent producers writing concurrently: the fetch loop (new episodes, ETag updates), the playback loop (position persistence every 30 s), the listening-session recorder (one row per play span, drives time-saved + history), and the user (subscriptions, queue edits, inbox triage).

```text
┌─────────────────────────────────────┐
│        Belfry Engine (Rust)         │
│      (tokio multi-thread runtime)   │
├─────────────────────────────────────┤
│  [Fetch Coordinator]                │
│   ├─ Per-show interval scheduler    │
│   ├─ reqwest pool (conditional GET, │
│   │   HTTP Basic auth)              │
│   └─ feed-rs + podcast: ns handler  │
├─────────────────────────────────────┤
│  [Playback Engine]                  │
│   ├─ libmpv host (libmpv2 crate)    │
│   ├─ Smart Speed filter chain       │
│   ├─ Voice Boost filter chain       │
│   └─ Session recorder               │
├─────────────────────────────────────┤
│  [Data Layer]                       │
│   ├─ Writer task (rusqlite, WAL)    │
│   ├─ Read-only pool                 │
│   └─ Library FTS5 (titles + descs)  │
└──────────┬──────────────────────────┘
           │ (tokio mpsc + glib channel)
    ┌──────┴────────────────────┐
    │  GTK4 Main UI Thread      │
    └───────────────────────────┘
```

`LibraryChanges` and `PlaybackChanges` are coalescing batch types delivered through a `glib::MainContext::channel`. UI updates apply as deltas, never full reloads.

### 2.2 Crate Layout

The workspace ships four crates:

- **`belfry-core`** — headless data layer. SQLite worker + read pool, feed fetch + parse pipeline, libmpv host + filter chains, Smart Speed session recorder, chapter normalizer, OPML import / export. GUI-free; the foundation for every surface.
- **`belfry-search`** — Calibre-shaped search expression language (lex / parse / AST / evaluator / SQL translator), typed against Belfry's domain (Episode / Show). The grammar shape is ported from Atrium's `atrium-search`; the implementation is independent so the two projects evolve without coupling. See `ATTRIBUTIONS.md`.
- **`belfry-cli`** — headless binary that exposes subscriptions, queue, downloads, triage, listening stats, and rescan from the shell. See §8.
- **`belfry`** — the GTK4 binary. Depends on all three above.

The architectural commitment, copied from Atrium: every non-GUI surface stays CLI-testable. The GTK binary is a frontend; the engine is fully exercisable without it. A future post-1.0 sync server could become a fourth crate without disturbing the core.

### 2.3 Widget Tree

Three-pane responsive layout, collapses to single-pane on mobile widths.

```text
AdwApplicationWindow
├── AdwBreakpoint (max-width 900sp → inner_split_view.collapsed)
├── AdwBreakpoint (max-width 600sp → both split_views.collapsed)
└── AdwToastOverlay
    └── AdwNavigationSplitView outer_split_view (sidebar 220–320 px)
        ├── [sidebar] AdwNavigationPage "Library"
        │   └── AdwToolbarView
        │       ├── AdwHeaderBar (add feed, settings)
        │       └── GtkScrolledWindow
        │           └── GtkListView sidebar_list_view
        │               (TreeListModel: triage lists, Shows, Tags)
        └── [content] AdwNavigationSplitView inner_split_view (sidebar 320–480 px)
            ├── [sidebar] AdwNavigationPage "Episodes"
            │   └── AdwToolbarView
            │       ├── AdwHeaderBar + GtkSearchBar
            │       └── GtkListView (recycled, capped natural width)
            └── [content] AdwNavigationPage "Now Playing / Detail"
                └── AdwToolbarView
                    ├── AdwHeaderBar
                    └── GtkStack
                        ├── now_playing: cover art + scrubber + show notes
                        └── empty:       AdwStatusPage "No episode selected"
└── AdwBin (now_bar — persistent transport at window bottom)
```

---

## 3. User Interface

### 3.1 Design Principles

Belfry is a desktop-first GNOME application. Every UI decision falls out of these principles:

- **Whitespace with respect.** Generous margins. Type sized for desktop reading distances, not phone arm's-length. Three panes by default; AdwClamp-bounded list widths so rows don't stretch into runway on ultrawide displays. Mobile responsive collapse at narrow widths is the *exception*, not the goal — a 14" laptop screen is the design target.
- **Colour with grace.** Restrained palette anchored on libadwaita's accent system. Per-show accent extracted from cover art (median-cut quantizer; the Hermitage pattern, see §3.6) blends with the system accent rather than fighting it. No saturated brand colours; no "pop" tones for their own sake.
- **The library is a queryable database.** Calibre's gift: every list view has a filter bar with the full expression grammar (§3.7), sortable columns, multi-select for bulk actions, and the option to save the current filter as a Perspective. Castro's Inbox → Queue → Played flow gives the *triage states*; Calibre's filter vocabulary lets the user shape *what they see within each state*.
- **Every action visible, every UI control keyboard-accessible.** Framework's discipline. No hidden gestures, no vim modes, no chord sequences. If a swipe does something, there's a menu item that does the same thing. Keyboard-first works.

### 3.2 Layout

GNOME HIG. Adwaita 1.7+. Three-pane responsive with libadwaita `NavigationSplitView` + `NavigationView`, plus a persistent **Now-bar** at the window bottom whenever audio is loaded.

```text
┌─────────────────────────────────────────────────────────────┐
│ [≡]  Belfry                                    [+] [⚙]      │
├──────────┬──────────────────┬──────────────────────────────┤
│ Sidebar  │ Episode list     │ Now Playing / Episode detail │
│          │ ────────────     │                              │
│ • Inbox  │ [filter ___ ]    │ ┌─────────────┐              │
│ • Queue  │ Title↑ Show Date │ │  cover art  │              │
│ • Played │ ▶ Ep 1234   ··   │ └─────────────┘              │
│ • Saved  │   Ep 1233   ··   │                              │
│ • Down.  │   Ep 1232   ··   │ Title here                   │
│ ─────    │   ...            │ Show · Date · 47:23          │
│ Shows    │                  │                              │
│ • Show A │                  │ ─[chapters]─[show notes]──   │
│ • Show B │                  │                              │
│ • Show C │                  │ Show notes...                │
│ ─────    │                  │                              │
│ Tags     │                  │                              │
│ ─────    │                  │                              │
│ Persp.   │                  │                              │
│ ─────    ┴──────────────────┤                              │
│ Settings                    │                              │
├─────────────────────────────┴──────────────────────────────┤
│ [▶] cover  Title — Show       ◀◀ 15  ▶  30 ▶▶  1.2× ⚡ 21:04│
└─────────────────────────────────────────────────────────────┘
```

The **Now-bar** (bottom strip) persists whenever audio is loaded, regardless of which view is open — minimum-viable transport (cover thumb, title, play/pause, scrubber, chapter forward, speed) that expands to the full Now Playing view on tap.

Density: row heights, gutter margins, font sizes target desktop reading distance, not mobile arm's-length. AdwClamp constrains the episode list at 920 px max so rows don't stretch into runway on ultrawide displays — the same trick Atrium uses for its task list.

### 3.3 Triage: Inbox → Queue → Played

The Castro model. Every episode is in exactly one of these states at any time, plus an orthogonal Saved flag:

- **Inbox** — newly fetched, awaiting triage. The user reads the description and decides: queue it, save it, archive it, or do nothing (it stays).
- **Queue** — the active playback list. Ordered (drag to reorder; per-show priority breaks ties on auto-insert).
- **Played** — the listening logbook. Episodes that have been listened through, or marked-as-played without listening, live here. Date-band grouped (Today / Yesterday / This Week / etc., the Atrium logbook pattern).
- **Saved** — orthogonal flag. A starred episode is *also* in one of {Inbox, Queue, Played}, but is excluded from auto-deletion regardless of retention policy.

**Per-show `inbox_policy`:**

- `'inbox'` (default) — new episodes land in Inbox for triage.
- `'always_queue'` — new episodes skip Inbox and append to Queue automatically. For shows you always listen to (the daily news, the weekly podcast you never miss).
- `'always_archive'` — new episodes skip Inbox and Queue, marked as played-without-listening. For shows you stay subscribed to but rarely actually listen to.

The contract: this is the *only* organizational hierarchy for *episodes*. Tags organize *shows*, not episodes. The user never thinks about "where" an episode lives — only "is it next, later, or done."

The state mapping in the schema (§4.1) is:

| State | Predicate |
|---|---|
| Inbox | `played = 0 AND in_queue = 0` |
| Queue | `in_queue = 1` |
| Played | `played IN (2, 3)` |
| Saved | `starred = 1` (cross-cuts the above) |

### 3.4 Sidebar

Ordered top-to-bottom:

- **Inbox** — episodes awaiting triage. Live count badge.
- **Queue** — the playback list. First item is what plays next. Live count badge.
- **Played** — listening logbook, date-banded.
- **Saved** — starred episodes (cross-cuts the three above). Live count badge.
- **Downloads** — currently downloading + recently completed.
- ─── separator ───
- **Shows** — subscribed feeds, sorted by user-defined priority then alpha.
- **Tags** — user-defined categories on shows. Calibre loanword; secondary organization, not primary.
- ─── separator ───
- **Perspectives** — saved filter expressions (see §3.7). Section appears only when ≥1 saved.
- ─── separator ───
- **Settings**

No "All Episodes" view — that's the filter bar's job. No "New" view — that's Inbox.

### 3.5 Episode List

The middle pane in any list-bearing view (Inbox, Queue, Played, Saved, Downloads, single-show episode list, search results, Perspective). Calibre-shaped:

- **Sortable columns.** Click header to sort ascending; click again for descending; shift-click for secondary sort. Default columns: Status glyph, Title, Show, Date, Duration. Right-click any header for a column-visibility menu. Sort + visibility persisted per-list in GSettings.
- **Multi-select.** Ctrl-click toggle; Shift-click range; Ctrl-A select all. Bulk actions: queue, archive, star, mark-played, delete, set tag, set show priority. Confirmation toast surfaces what changed; Ctrl-Z invokes the toast for undo.
- **Filter bar above the list.** Persistent (not a popover). Accepts the full expression grammar (§3.7). The filter bar IS the search bar; there is no separate search mode.
- **Keyboard nav.** Arrow keys move focus between rows; Enter opens detail; Space plays the focused row; Q queues; F toggles starred; A archives; Delete removes (with toast).
- **Cover thumb in the row** (~32 px square; the visual unit even in dense list mode).
- **Status glyphs at row start** (played / in-progress / unplayed / queued / starred / downloaded), tinted with the show's accent.
- **Hover state** — subtle accent-tinted background, libadwaita-native (matches Atrium's hover-row "lift" cue at v0.5.0).

This is the surface that Brandon's "the way my brain works — a database I can filter however I want" maps to. Castro gives the *states*; Calibre gives the *grammar inside each state*.

### 3.6 Now Playing

The Codex moment, lifted from Hermitage. This is where the user spends 95% of their time once an episode starts; it gets disproportionate polish.

- **Cover art** is the visual unit. Full-bleed on mobile widths; sidebar-anchored on desktop.
- **Dynamic accent color.** Dominant hue extracted from the cover (median-cut quantizer, the same approach Hermitage uses for book covers) propagates to the scrubber, chapter ticks, queue insertion indicator, and Smart Speed lightning bolt. With 70+ shows, this is what makes them feel visually distinct in a list at a glance.
- **Scrubber** with chapter markers as ticks; click a tick to jump to that chapter.
- **Chapter list** in a side panel; click any title to seek to its start.
- **Chapter image** renders into the cover slot when the active chapter has an image; falls back to episode art, then show art.
- **Show notes** pane, sanitized via `ammonia` (the Viaduct sanitizer recipe). Links open externally through `xdg-open`.
- **Speed control** (0.5×–3.0×, 0.05 increments, double-tap to reset). Persists per-show.
- **Smart Speed indicator** (lightning bolt; glows when active; tooltip shows time saved this episode).
- **Voice Boost indicator**.
- **Skip controls** with configurable intervals (default 15 s back / 30 s forward; per-show override available).
- **Sleep timer** (15 / 30 / 45 / 60 min, end of episode, end of queue). **Tap-to-extend**: if the timer fires and the user taps Play within 30 seconds, extend by the same interval — Castro's beloved touch.
- **Output device picker** — PipeWire sink selection. The Linux AirPlay equivalent.
- **Queue tail** (peek): the next 1–3 queue items below the show notes, so the user knows what's coming next without leaving Now Playing.

### 3.7 Filtering, Search, and Perspectives

Filter and search are the same surface in Belfry. The filter bar above every list view (§3.5) accepts the full expression grammar — typing `tag:tech` filters in place; typing `aether AND duration:<30` works the same way; clearing the bar restores the full list. **There is no separate search mode and no popover-only search. Every list is a filterable view.**

`Ctrl+F` focuses the filter bar.

Calibre-shaped expression grammar — the same shape Atrium's `atrium-search` crate exposes, with podcast-relevant fields:

| Field | Example |
|---|---|
| `show:`, `author:` | `show:"This American Life"` |
| `title:`, `note:` | `title:"smart speed"` |
| `tag:` | `tag:tech` |
| `is:played`, `is:starred`, `is:downloaded`, `is:in_queue`, `is:in_inbox` | `is:starred AND tag:dev` |
| `duration:>30`, `duration:<10..20` | `duration:<30` (everything under 30 min) |
| `pub:thisweek`, `pub:>2024-01-01` | `pub:thisweek` |

Boolean (`AND` / `OR` / `NOT`), match modifiers (`tag:work` substring, `tag:=Work` exact, `tag:~regex`, `tag:?fuzzy`), comparison + range (`>=`, `lo..hi`), date keywords (`today`, `thisweek`, `Ndaysago`), sort modifiers (`sort:KEY`, `sort:-KEY`). The grammar shape is ported from Atrium's `atrium-search` crate (see `ATTRIBUTIONS.md`); Belfry ships its own implementation in `belfry-search` so the two projects can evolve independently — Atrium's evaluator is typed against tasks, Belfry's is typed against episodes and shows.

**Perspectives.** A saved filter expression with a name (Atrium's term, ported from Calibre's saved searches and Wings). Save the current filter via the primary menu's *Save filter as Perspective…*; perspectives appear in the sidebar (§3.4) and apply to the full episode set. The expression text itself is what's stored — re-parsed on every load — so a Perspective written against v0.4's grammar inherits operator additions in v0.5+ for free.

Forgiving parser: malformed input degrades to substring match on the literal text; unknown field names fall through as freeform terms with a yellow tint on the filter bar (no error). New operators can land in minor releases without breaking saved Perspectives.

**Out of scope, forever:** directory search, iTunes/Apple Podcasts lookup, fyyd, "discover" tabs, transcript search, content-discovery search of any kind. Belfry searches your library, not the world.

---

## 4. Data Model

### 4.1 Tables

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
    accent_rgb      INTEGER,           -- precomputed dominant hue (packed RGB)
    apple_podcasts_id TEXT,            -- preserved on OPML round-trip
    last_fetched    INTEGER,           -- unix epoch
    last_modified   TEXT,              -- HTTP header for conditional GET
    etag            TEXT,
    fetch_interval  INTEGER DEFAULT 3600,
    auth_user       TEXT,              -- HTTP Basic; NULL = anonymous
    auth_pass_ref   TEXT,              -- libsecret schema name; NEVER stored inline
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
    description     TEXT,
    pub_date        INTEGER,
    duration        INTEGER,           -- seconds
    file_size       INTEGER,
    audio_url       TEXT,
    audio_path      TEXT,              -- NULL until downloaded; streaming uses URL
    folder_path     TEXT NOT NULL,
    mime_type       TEXT,
    season          INTEGER,
    episode_number  INTEGER,
    episode_type    TEXT,              -- full/trailer/bonus
    UNIQUE (show_id, guid)
);

-- Triage state. Inbox / Queue / Played derived; Saved is the starred flag.
-- played sentinel: 0=unplayed, 1=in-progress, 2=played-fully, 3=archived-unlistened
CREATE TABLE playback (
    episode_id      INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
    position        REAL DEFAULT 0,    -- seconds, supports sub-second seek
    played          INTEGER DEFAULT 0,
    last_played     INTEGER,
    play_count      INTEGER DEFAULT 0,
    starred         INTEGER DEFAULT 0,
    in_queue        INTEGER DEFAULT 0,
    queue_position  INTEGER
);

-- Per-show overrides (Overcast pattern + Castro inbox policy)
CREATE TABLE show_settings (
    show_id         INTEGER PRIMARY KEY REFERENCES shows(id) ON DELETE CASCADE,
    playback_speed  REAL DEFAULT 1.0,
    smart_speed     INTEGER DEFAULT 1,
    voice_boost     INTEGER DEFAULT 0,
    skip_intro      INTEGER DEFAULT 0, -- seconds shaved from episode start
    skip_outro      INTEGER DEFAULT 0,
    skip_forward    INTEGER,           -- NULL = inherit global
    skip_back       INTEGER,           -- NULL = inherit global
    inbox_policy    TEXT DEFAULT 'inbox'  -- 'inbox' | 'always_queue' | 'always_archive'
);

-- One row per playback session — drives the listening-history view and the
-- Smart Speed time-saved counter. real_seconds = wall-clock listen time;
-- audio_seconds = audio time covered (= real_seconds × speed); the difference
-- attributable to silence-skip is recorded in smart_speed_saved.
CREATE TABLE listening_sessions (
    id              INTEGER PRIMARY KEY,
    episode_id      INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER NOT NULL,
    real_seconds    REAL NOT NULL,
    audio_seconds   REAL NOT NULL,
    smart_speed_saved REAL DEFAULT 0
);
CREATE INDEX idx_sessions_episode ON listening_sessions(episode_id);
CREATE INDEX idx_sessions_started ON listening_sessions(started_at);

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

-- Tags on shows (not episodes). Calibre loanword; secondary organization.
CREATE TABLE tags (
    id              INTEGER PRIMARY KEY,
    name            TEXT UNIQUE NOT NULL
);
CREATE TABLE show_tags (
    show_id         INTEGER REFERENCES shows(id) ON DELETE CASCADE,
    tag_id          INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (show_id, tag_id)
);

-- Library FTS: titles + descriptions only. NOT transcripts.
CREATE VIRTUAL TABLE episode_fts USING fts5(
    title, description, content='episodes', content_rowid='id'
);
CREATE VIRTUAL TABLE show_fts USING fts5(
    title, author, description, content='shows', content_rowid='id'
);
```

Triggers keep the two FTS tables in sync with their content tables on INSERT / UPDATE / DELETE.

### 4.2 SQLite Configuration

WAL mode, foreign keys on, `synchronous=NORMAL`. The defaults Calibre learned the hard way.

- **Single writer:** A dedicated tokio task owns the writable connection and serializes all writes. The GTK thread holds only a `Sender`.
- **FTS5:** Enabled on episode and show titles + descriptions. *Not* on transcripts (out of scope per §1).
- **Page-cache size and mmap:** kept conservative. Viaduct's RSS post-mortem found large mmap windows balloon resident memory; Belfry inherits the same caps.

### 4.3 Migrations

Schema versioned via SQLite `user_version` PRAGMA. Migrations live in `belfry-core/src/db/migrations/<NNNN>_*.sql`. v0.1 ships `0001_initial.sql`. Post-1.0 the discipline is **append-only and backwards-compatible**, matching Atrium's practice — `ALTER TABLE … ADD COLUMN` is fine; renames or drops are major-bump-only.

---

## 5. Playback Engine

A single libmpv instance, kept alive across episodes. The Rust binding is `libmpv2`, which exposes the property API directly.

### 5.1 Smart Speed

Detect silence longer than a threshold; time-stretch through it. libmpv's `af` (audio filter) chain expresses this via `silenceremove` (from ffmpeg's filter library) combined with `rubberband` for pitch-preserving stretch:

```
af=lavfi=[silenceremove=stop_periods=-1:stop_duration=0.3:stop_threshold=-40dB:stop_silence=0.15]
```

That's the rough shape. Tunables: silence threshold (-40 dB default), minimum silence duration before compression (0.3 s), residual silence kept (0.15 s for naturalness). Per-show override via `show_settings.smart_speed`.

The Overcast trick is not the filter — it's the calibration. Belfry exposes threshold and duration as advanced settings and ships sensible defaults. Time saved per episode is recorded in `listening_sessions.smart_speed_saved` and surfaced ("Smart Speed saved 4m 12s"). That's the daily retention hook.

### 5.2 Voice Boost

Compression + EQ tuned for spoken word over phone speakers and earbuds. Two-stage filter:

```
af=lavfi=[acompressor=threshold=-18dB:ratio=3:attack=5:release=50,
         equalizer=f=200:t=q:w=1:g=-3,
         equalizer=f=3000:t=q:w=1:g=4,
         loudnorm=I=-16:TP=-1.5:LRA=11]
```

Cuts mud at 200 Hz, lifts presence at 3 kHz, evens loudness to broadcast standard (-16 LUFS, EBU R128 — the spec podcast platforms target). Per-show toggle.

### 5.3 Combined Chain

When both filters are on, `silenceremove` runs first (cheap), then compression/EQ. Order matters: you don't want to compress silence before you remove it.

### 5.4 Session Recorder

The Playback Engine emits `listening_sessions` rows. A session opens on play-start (or resume after pause > 30 s) and closes on pause / seek-out / episode-end / app-quit. Each session stores wall-clock listen time, audio time covered, and Smart Speed savings (computed from `silenceremove` filter telemetry). The history view (§3) and Smart Speed time-saved counter both read from this table.

### 5.5 State Persistence

Position is written to DB on:

- Pause.
- Seek (debounced to 500 ms).
- Episode end.
- App quit.
- Every 30 seconds during playback (cheap insurance against crashes).

Resume offset is `position - 3` seconds for context — Overcast convention.

### 5.6 System Integration

- **MPRIS2 D-Bus interface** (`org.mpris.MediaPlayer2`) — full metadata (title / artist / album-as-show / cover URL / chapter-aware position), play / pause / next / previous / seek. Exposes Belfry to GNOME's media overlay, lock screen, and Bluetooth headset buttons (via `mpris-proxy`). v0.1 requirement.
- **Suspend inhibitor** during active playback (`org.freedesktop.login1` `Inhibit`). Listening with the lid closed must work.
- **PipeWire output sink picker** — list available sinks, route audio to the chosen one. The Linux AirPlay equivalent.
- **Headphone-button play/pause** via MPRIS forwarding.

---

## 6. Library & Filesystem Layout

### 6.1 On-Disk Layout

```text
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
            ├── audio.<ext>          # mp3/m4a/ogg as published; absent for stream-only
            ├── chapters.json        # Parsed from podcast:chapters or ID3
            ├── shownotes.html       # Sanitized
            └── cover.jpg            # Episode art if present, else show art
```

Slugs are filesystem-safe ASCII. Original titles preserved in DB. The folder structure is what `dired`, `nautilus`, and a backup script see. The DB sees the same thing plus indexes.

### 6.2 Streaming

Episodes can play before download completes (or without download at all): if `audio_path IS NULL` and `audio_url` is set, libmpv streams via HTTP with range-request support so resume-from-anywhere works. The download (when triggered) is a separate background job that writes to disk; the user never has to wait for a download to start listening.

GNOME Podcasts's "must download to listen" limitation is the gap here; Belfry does not repeat it.

### 6.3 Rescan Contract

Delete `belfry.db`, run `belfry-cli rescan`, get the same library back minus playback state (positions, listening sessions). **The DB-exclusive data is exactly: playback state, listening sessions, queue ordering, and triage state** — all in tables that export to JSON.

This contract is the data-sovereignty test. The integration suite verifies it end-to-end: build a fixture library, snapshot the DB, delete it, rescan, diff. Any drift in the reconstructable subset (shows, episodes, chapters, tags) is a release-blocker bug.

---

## 7. Feed Handling

### 7.1 Polling

Per-show interval (default 1 hour), jittered ±10% to avoid thundering herd. Conditional GET via `If-Modified-Since` and `If-None-Match`. HTTP 304 short-circuits the entire pipeline.

A cooldown list throttles recently-errored feeds, lifted directly from Viaduct's NetNewsWire-shaped DownloadSession analog. HTTP 429 honours `Retry-After`.

### 7.2 Authentication

HTTP Basic auth ships in v0.1. Real-world podcast feeds use it: Brandon's own OPML has at least one Substack-style private feed with a UUID-token URL, and Patreon-tier shows commonly require Basic. The credential lives in `libsecret` (via the `oo7` crate); the DB stores only a reference (`shows.auth_pass_ref`), never the password inline.

OAuth and signed-URL auth are post-1.0 (§13.4).

### 7.3 Parsing

`feed-rs` handles the RSS / Atom / JSON Feed core. The Podcast Index `podcast:` namespace (`<podcast:chapters>`, `<podcast:person>`, `<podcast:locked>`, `<podcast:funding>`) is layered on top via a hand-rolled `quick-xml` pass against the raw XML. There is no turn-key Rust analog of Python's `podcastparser`; the namespace handler is project-owned code.

### 7.4 Episode Identity

`guid` is canonical. Episodes are deduplicated by `(show_id, guid)`. If a publisher rotates GUIDs (it happens), Belfry surfaces a warning and lets the user merge manually. Belfry does not silently dedupe by title or URL — that hides bugs.

### 7.5 Chapters

Three sources, in precedence order:

1. `<podcast:chapters>` JSON (richest — supports images and URLs).
2. ID3v2 CHAP frames (parsed via the `id3` crate on download).
3. Show notes timestamp parsing (regex, last resort, marked as inferred).

Chapter images render in the Now Playing cover slot when the active chapter has one.

### 7.6 OPML Round-Trip

Import: parse `<outline type="rss" ...>` elements; preserve `applePodcastsID` and any namespace-extension attributes verbatim into `shows.apple_podcasts_id` (and a generic side-bag). Most apps drop hierarchy on import; Belfry's tag round-trip preserves user organization.

Export: emit Belfry's tags as `<outline category="tag1,tag2">` (the Pocket Casts convention). Other apps that ignore the attribute still get a flat-but-correct list.

---

## 8. CLI

`belfry-cli` ships alongside the GUI binary. Hermitage and CalibreQuarry already established the pattern: GUI for browsing, CLI for batch ops.

```text
belfry-cli add <feed-url>           # Subscribe
belfry-cli remove <show-slug>       # Unsubscribe
belfry-cli list inbox|queue|played|saved|downloads|shows
belfry-cli refresh [show-slug]      # Force fetch (defaults: all shows)
belfry-cli download <episode-spec>  # Manual download
belfry-cli queue add|remove|reorder <episode-spec>
belfry-cli triage <episode-spec> --queue|--archive|--star
belfry-cli play <episode-spec>      # Hand off to standalone mpv
belfry-cli export-opml > out.opml
belfry-cli import-opml < in.opml
belfry-cli rescan                   # Rebuild DB from filesystem
belfry-cli stats                    # Listening time, time saved, top shows
belfry-cli auth set <show-slug>     # Prompt for HTTP Basic credentials → libsecret
```

`<episode-spec>` is `<show-slug>/<episode-slug>` or partial-match prefix.

Read commands open the database read-only as a process-level safety guarantee. Write commands spin up the worker on a current-thread tokio runtime, send commands via a `WorkerHandle`, and shut down cleanly — same pattern Atrium uses.

Output formats (mutually exclusive global flags): `--tsv` (default, header row, `cut`/`grep`-friendly), `--json` (jq-friendly), `--human` (terminal viewing).

---

## 9. Configuration

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
delete_after_played_delay_days = 0

[fetch]
default_interval_seconds = 3600
jitter_percent = 10
user_agent = "Belfry/0.1 (+https://github.com/virinvictus/belfry)"

[notifications]
new_episodes = false   # off by default — at 70+ feeds, on-by-default is spam
download_complete = false
```

---

## 10. Dependencies

### Rust crates (backend)

- `tokio` — async runtime; multi-thread, with worker / blocking caps lifted from Viaduct's RSS post-mortem.
- `reqwest` (with `rustls-tls`, `gzip`, `brotli`) — HTTP client; conditional GET; rate-limit handling; HTTP Basic auth.
- `oo7` — libsecret credential storage (HTTP Basic per-show credentials).
- `rusqlite` (bundled, FTS5) — SQLite bindings.
- `feed-rs` — RSS / Atom / JSON Feed parsing.
- `quick-xml` — namespace-aware XML pass for the `podcast:` namespace fields feed-rs doesn't model.
- `libmpv2` — Rust bindings for libmpv; property API + filter graph.
- `ammonia` — HTML sanitization for show notes (same crate Viaduct uses for article HTML).
- `id3` — ID3v2 CHAP frame parsing for chapter fallback.
- `image` — cover decode for the median-cut accent extractor.
- `serde` + `serde_json` + `toml` — config + persistence.
- `regex` — show-notes timestamp inference.
- `tracing` + `tracing-subscriber` — structured logging.
- `zbus` — MPRIS2 D-Bus interface + suspend inhibitor.

### C / GTK libraries (frontend)

- `gtk4` (via `gtk4-rs`) — minimum 4.16.
- `libadwaita` (via `libadwaita-rs`) — minimum 1.7.
- `libmpv` (system library) — Rust binding above; libmpv 0.36+ for current `af` filter graph behaviour.
- `ffmpeg` filter library (transitive via libmpv) — `silenceremove`, `rubberband`, `acompressor`, `equalizer`, `loudnorm`.
- `libsecret` (transitive via `oo7`) — credential storage backend.

No third-party crate or system library lands without prior sign-off — Brandon's standing rule.

---

## 11. Flatpak Distribution

Belfry is packaged as a Flatpak-first application. App ID: `org.gnome.Belfry` if accepted into GNOME Circle, otherwise `io.github.virinvictus.Belfry`.

- **Permissions:** kept tight.
  - `network` — required for feed fetching and audio download / streaming.
  - `xdg-run/dconf` — required for GNOME settings.
  - `pulseaudio` (PipeWire-compatible socket) — playback.
  - `org.freedesktop.secrets` — libsecret access for HTTP Basic credentials.
  - File chooser interactions for OPML import / export run through `org.freedesktop.portal.FileChooser`.
- **Background:** the app uses portal-mediated background execution so periodic feed refresh continues when the UI is closed. This is post-1.0 polish; v0.1 polls only while the app is running.

---

## 12. Memory & Performance Targets

Belfry has no WebKit (Viaduct's floor-pinning dependency), so the targets are tighter than Viaduct's:

- **Idle (no playback):** < 100 MB after warm fetch + image-cache warm.
- **Playback active, Now Playing open:** < 200 MB.
- **Cold start to first interactive frame on a 100-show library:** < 250 ms.
- **Position-write latency (pause → DB committed):** < 50 ms.

GTK4 + libadwaita pull in a ~150 MB C-side anon floor regardless of Rust-allocator choice (Viaduct §11 measured this). The targets above account for that floor.

Each phase ends with a `heaptrack` / `massif` measurement note. Features that miss budget get gated or revised.

---

## 13. Risks and Unknowns

1. **libmpv filter graph latency.** `silenceremove` followed by `rubberband` can introduce noticeable lag on speed changes. Needs prototyping before committing the v0.2 Smart Speed scope.
2. **Smart Speed time-saved attribution.** The `silenceremove` filter doesn't expose per-call telemetry directly; the session recorder has to compute savings from the rate of audio-time-emitted vs wall-clock-time-elapsed while the filter is active. Plan validates in the Phase 2 spike.
3. **Podcast namespace coverage in Rust.** Unlike Python's `podcastparser`, there is no turn-key Rust crate for the Podcast Index namespace. Belfry owns that parser; expect to grow it as feeds in the wild surface new edge cases.
4. **Feed authentication beyond Basic.** Premium podcasts use OAuth tokens or signed URLs as well as Basic. v0.1 supports Basic; the rest is post-1.0 (a planned `auth_kind` column on `shows` keeps the schema future-proof).
5. **Database growth.** Listening sessions at scale (one row per session, multiple per episode possible) will dwarf any other table after a year of use. SQLite handles it; rebuild times on rescan need measuring; periodic pruning of sessions older than N years is a setting.
6. **GTK4 + libmpv embedding.** GL render contexts in GTK4 are workable but fiddly. Audio-only playback dodges the worst of it. If video is ever added (some podcasts ship video), this becomes load-bearing.

---

## 14. Out of Scope, Forever

- Recommendations, "discover" tabs, charts.
- Social features (sharing, comments, ratings, friend activity).
- A built-in podcast directory beyond OPML import.
- Cloud anything that isn't a sync protocol.
- DRM.
- **Transcripts** — ingestion, display, search. The original spec listed transcript FTS as the killer post-1.0 feature; on reflection, that violates commitment #1 (listening device, not searching device). If it ships, it ships in 2.x as a maybe.
- Video podcasts as a first-class format (audio extraction only).
- Windows / macOS support — GNOME-native, deliberately.

---

## 15. Naming and Branding

The bell tower. Episodes are bells; subscriptions are the rope-pull schedule; the library is the chamber where they hang. Icon should evoke architecture, not audio waveforms — no headphones, no microphones, no play triangles in the logo. A belfry silhouette in libadwaita accent colour does the work.

App ID: `org.gnome.Belfry` if accepted into GNOME Circle, else `io.github.virinvictus.Belfry`.

---

## 16. Project Conventions

Standard layout:

- `README.md`, `spec.md` (this file), `roadmap.md`, `patchnotes.md`, `CLAUDE.md`, `ATTRIBUTIONS.md` (design lineage + dependency licenses + license-chain analysis).
- `VERSION` is the single source of truth; `Cargo.toml` (workspace + each member) matches.
- `LICENSE` (GPL-3.0-or-later), `logo.svg`.
- `data/` — `.ui` XML files, icons, GSettings schema, AppStream metainfo, Flatpak manifest, **bundled fonts** (registered via fontconfig at first run; never assume host fonts).
- `belfry-core/`, `belfry-search/`, `belfry-cli/`, `belfry/` — Cargo workspace members.
- `tests/` — integration tests (alongside in-crate unit tests).
- `docs/` — schema, keymap, perf notes, libmpv filter chain reference, Podcast Index namespace coverage matrix.

CI matches Viaduct: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` on Linux. Tests required from day one.
