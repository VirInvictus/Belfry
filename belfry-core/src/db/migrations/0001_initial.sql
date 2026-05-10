-- Belfry — initial schema (v0.1).
-- See spec.md §4.1 for the full data model.

PRAGMA user_version = 1;

-- Subscriptions ----------------------------------------------------------
CREATE TABLE shows (
    id                INTEGER PRIMARY KEY,
    slug              TEXT UNIQUE NOT NULL,
    feed_url          TEXT UNIQUE NOT NULL,
    title             TEXT NOT NULL,
    author            TEXT,
    description       TEXT,
    homepage_url      TEXT,
    cover_path        TEXT,
    accent_rgb        INTEGER,                      -- precomputed dominant hue
    apple_podcasts_id TEXT,                         -- preserved on OPML round-trip
    last_fetched      INTEGER,                      -- unix epoch
    last_modified     TEXT,                         -- HTTP If-Modified-Since
    etag              TEXT,                         -- HTTP If-None-Match
    fetch_interval    INTEGER NOT NULL DEFAULT 3600,
    auth_user         TEXT,                         -- HTTP Basic; NULL = anonymous
    auth_pass_ref     TEXT,                         -- libsecret schema name; never inline
    auto_download     INTEGER NOT NULL DEFAULT 1,
    keep_count        INTEGER NOT NULL DEFAULT 0,   -- 0 = keep all
    priority          INTEGER NOT NULL DEFAULT 0,   -- Overcast-style ordering
    folder_path       TEXT NOT NULL
);

CREATE TABLE episodes (
    id              INTEGER PRIMARY KEY,
    show_id         INTEGER NOT NULL REFERENCES shows(id) ON DELETE CASCADE,
    guid            TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    pub_date        INTEGER,
    duration        INTEGER,
    file_size       INTEGER,
    audio_url       TEXT,
    audio_path      TEXT,                           -- NULL until downloaded
    folder_path     TEXT NOT NULL,
    mime_type       TEXT,
    season          INTEGER,
    episode_number  INTEGER,
    episode_type    TEXT,                           -- full / trailer / bonus
    UNIQUE (show_id, guid)
);
CREATE INDEX idx_episodes_show_pub ON episodes(show_id, pub_date DESC);

-- Triage state. Inbox / Queue / Played derived from these columns.
-- played: 0=unplayed, 1=in-progress, 2=played-fully, 3=archived-unlistened
CREATE TABLE playback (
    episode_id      INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
    position        REAL NOT NULL DEFAULT 0,
    played          INTEGER NOT NULL DEFAULT 0,
    last_played     INTEGER,
    play_count      INTEGER NOT NULL DEFAULT 0,
    starred         INTEGER NOT NULL DEFAULT 0,
    in_queue        INTEGER NOT NULL DEFAULT 0,
    queue_position  INTEGER
);
CREATE INDEX idx_playback_inbox    ON playback(played, in_queue) WHERE played = 0 AND in_queue = 0;
CREATE INDEX idx_playback_queue    ON playback(in_queue, queue_position) WHERE in_queue = 1;
CREATE INDEX idx_playback_starred  ON playback(starred) WHERE starred = 1;

-- Per-show overrides (Overcast pattern + Castro inbox policy)
CREATE TABLE show_settings (
    show_id         INTEGER PRIMARY KEY REFERENCES shows(id) ON DELETE CASCADE,
    playback_speed  REAL NOT NULL DEFAULT 1.0,
    smart_speed     INTEGER NOT NULL DEFAULT 1,
    voice_boost     INTEGER NOT NULL DEFAULT 0,
    skip_intro      INTEGER NOT NULL DEFAULT 0,     -- seconds shaved from start
    skip_outro      INTEGER NOT NULL DEFAULT 0,
    skip_forward    INTEGER,                        -- NULL = inherit global
    skip_back       INTEGER,                        -- NULL = inherit global
    inbox_policy    TEXT NOT NULL DEFAULT 'inbox'   -- 'inbox' | 'always_queue' | 'always_archive'
);

-- One row per playback session — drives history view + Smart Speed time-saved
CREATE TABLE listening_sessions (
    id                INTEGER PRIMARY KEY,
    episode_id        INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER NOT NULL,
    real_seconds      REAL NOT NULL,                -- wall-clock listen time
    audio_seconds     REAL NOT NULL,                -- audio time covered
    smart_speed_saved REAL NOT NULL DEFAULT 0
);
CREATE INDEX idx_sessions_episode ON listening_sessions(episode_id);
CREATE INDEX idx_sessions_started ON listening_sessions(started_at);

-- Chapters (podcast:chapters or ID3 CHAP)
CREATE TABLE chapters (
    id          INTEGER PRIMARY KEY,
    episode_id  INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    start_time  REAL NOT NULL,
    end_time    REAL,
    title       TEXT,
    url         TEXT,
    image_path  TEXT
);
CREATE INDEX idx_chapters_episode ON chapters(episode_id, start_time);

-- Tags on shows (not episodes). Calibre loanword; secondary organization.
CREATE TABLE tags (
    id    INTEGER PRIMARY KEY,
    name  TEXT UNIQUE NOT NULL
);
CREATE TABLE show_tags (
    show_id  INTEGER REFERENCES shows(id) ON DELETE CASCADE,
    tag_id   INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (show_id, tag_id)
);

-- Library FTS: titles + descriptions only. NOT transcripts.
CREATE VIRTUAL TABLE episode_fts USING fts5(
    title,
    description,
    content='episodes',
    content_rowid='id'
);
CREATE VIRTUAL TABLE show_fts USING fts5(
    title,
    author,
    description,
    content='shows',
    content_rowid='id'
);

-- FTS sync triggers (episodes)
CREATE TRIGGER episodes_ai AFTER INSERT ON episodes BEGIN
    INSERT INTO episode_fts(rowid, title, description) VALUES (new.id, new.title, new.description);
END;
CREATE TRIGGER episodes_ad AFTER DELETE ON episodes BEGIN
    INSERT INTO episode_fts(episode_fts, rowid, title, description) VALUES ('delete', old.id, old.title, old.description);
END;
CREATE TRIGGER episodes_au AFTER UPDATE ON episodes BEGIN
    INSERT INTO episode_fts(episode_fts, rowid, title, description) VALUES ('delete', old.id, old.title, old.description);
    INSERT INTO episode_fts(rowid, title, description) VALUES (new.id, new.title, new.description);
END;

-- FTS sync triggers (shows)
CREATE TRIGGER shows_ai AFTER INSERT ON shows BEGIN
    INSERT INTO show_fts(rowid, title, author, description) VALUES (new.id, new.title, new.author, new.description);
END;
CREATE TRIGGER shows_ad AFTER DELETE ON shows BEGIN
    INSERT INTO show_fts(show_fts, rowid, title, author, description) VALUES ('delete', old.id, old.title, old.author, old.description);
END;
CREATE TRIGGER shows_au AFTER UPDATE ON shows BEGIN
    INSERT INTO show_fts(show_fts, rowid, title, author, description) VALUES ('delete', old.id, old.title, old.author, old.description);
    INSERT INTO show_fts(rowid, title, author, description) VALUES (new.id, new.title, new.author, new.description);
END;
