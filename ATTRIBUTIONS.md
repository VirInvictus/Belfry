# Attributions

Belfry stands on three traditions: an iOS-shaped audio engine (Overcast), a triage philosophy (Castro), and a Linux architectural lineage (NetNewsWire via Viaduct, plus the rest of Brandon's portfolio). Plus the system libraries and Rust crates that make any of it possible.

**No source is copied from any of the design-lineage projects below.** The debts are conceptual: ideas, vocabulary, calibration targets, UI discipline. Every borrow is named here. Direct code dependencies (Rust crates, system libraries) are listed under *Technical foundations* with their licenses.

---

## Design lineage

### Overcast — the audio engine reference
**Marco Arment** · iOS · proprietary · <https://overcast.fm/>

Belfry's audio engine is calibrated against Overcast. The features we owe:

- **Smart Speed** — silence-skipping with pitch-preserving stretch. The filter chain is ours (`silenceremove` + `rubberband` via libmpv); the calibration target — natural-sounding, Overcast-quality time saved without distortion — is Marco's.
- **Voice Boost** — broadcast-quality compression + EQ for spoken word. Same story: chain is ours, calibration is Overcast's.
- **Time-saved counter** — "Smart Speed saved 4m 12s." The retention hook nobody else copies properly.
- **Per-show priority** within playlists/queue — the Overcast pattern that lets daily news float above weekly long-form without manual intervention.
- **Resume offset** — `position - 3` seconds on resume for context. Tiny detail; users notice instantly when missing.

If you listen on iOS, [install Overcast](https://overcast.fm/). It is and remains the gold standard.

### Calibre — the library-as-database mental model
**Kovid Goyal et al.** · cross-platform · GPL-3.0 · <https://calibre-ebook.com/>

The way Brandon's brain works — "a database I can filter however I want." Calibre is the reason Belfry treats every list view as a queryable database rather than a fixed scroll-of-rows. Specific debts:

- **Sortable columns on every list.** Click header → sort; shift-click → secondary sort. Belfry's episode list (spec §3.5) is Calibre's interaction model layered on top of Castro's triage states.
- **Filter bar as the primary interaction surface.** Calibre's search bar is *always there*, always live; typing in it filters in place. Belfry's filter bar (spec §3.7) is the same — there is no separate "search mode."
- **Saved filters as first-class library entries.** Calibre calls them Wings (virtual libraries) and saved searches; Belfry calls them Perspectives (Atrium's term). They appear in the sidebar alongside the standard triage entries; clicking applies the filter.
- **Multi-select bulk actions.** Ctrl-click / Shift-click / Ctrl-A → set tag, change retention, change priority on many shows at once. The 70+-subscription user's quality-of-life feature.
- **Search expression vocabulary.** Match modifiers (`tag:work` / `tag:=Work` / `tag:~regex` / `tag:?fuzzy`), state predicates (`is:NAME`), comparison + range, sort modifiers — the full grammar. Reached Belfry through Atrium's `atrium-search`, but the original is Calibre's. Belfry credits both.

If you have a large Calibre library, you already think the way Belfry's filter UX expects. Belfry brings that interaction model to podcasts.

### Castro — the triage model
**Bluck Apps** (formerly Supertop) · iOS · proprietary · <https://castro.fm/>

Belfry's organization model — **Inbox → Queue → Archive** with per-show "always queue" / "always archive" overrides — is Castro's. The central insight that with hundreds of subscriptions the daily question is "what's *next*?" not "what's in my library?" reshapes everything from sidebar order to schema columns. Specific debts:

- The Inbox / Queue / Archive triage flow (Belfry: Inbox / Queue / Played).
- Per-show `inbox_policy` (`always_queue` / `always_archive`).
- Tap-to-extend sleep timer — if the timer fires and you tap Play within 30 s, extend by the same interval. The single most-loved detail in the Castro reviews.
- The architectural commitment that triage state is local-first. Castro's near-death-by-cloud-DB-rot (2023) is the cautionary tale that drove this decision.

### NetNewsWire — the architectural twin pattern
**Brent Simmons / Ranchero Software** · macOS, iOS · MIT · <https://netnewswire.com/>

Reached Belfry indirectly, through [Viaduct](https://github.com/virinvictus/Viaduct) (Brandon's NetNewsWire-port-to-Linux RSS reader). The patterns we copy:

- Single-writer SQLite worker — every write funnels through one tokio task; the GTK thread never blocks on I/O.
- Conditional GET via `If-Modified-Since` / `If-None-Match`. HTTP 304 short-circuits the entire pipeline.
- OPML on disk as a peer to the SQL store; not a sync mechanism.
- The "port don't invent" discipline — when in doubt, look at how NNW does it.

Viaduct's ATTRIBUTIONS document Brent and the Ranchero team in detail; Belfry inherits the lineage without re-litigating it.

### Hermitage — visual direction and accent extraction
**Brandon LaRocque** · GTK4 / Python · GPL-3.0 · <https://github.com/virinvictus/Hermitage>

Hermitage is the cover-art-first Calibre browser. Belfry's Now Playing surface is a podcast-shaped Codex:

- Cover art as the visual unit — full-bleed on mobile, sidebar-anchored on desktop.
- **Dynamic accent color** extracted from the cover via median-cut quantization. Hermitage uses this for book covers; Belfry uses the same approach (and likely ports the implementation directly) for episode covers, propagating the dominant hue to the scrubber, chapter ticks, and Smart Speed indicator.
- The Codex aesthetic — blurred-cover hero, opinionated typography, click-anywhere-on-cover-to-play.

### Atrium — search vocabulary and crate split
**Brandon LaRocque** · Rust / GTK4 · MIT · <https://github.com/virinvictus/Atrium>

Atrium pioneered the Calibre-shaped search expression language for portfolio Rust apps. Belfry's library-search grammar (`tag:`, `is:played`, `duration:>30`, `pub:thisweek`, boolean + match modifiers + sort) is the same shape, scoped to podcast-relevant fields.

**Belfry ports the parser shape; it does not depend on `atrium-search` as a Cargo crate.** atrium-search's evaluator and SQL translator are typed against Atrium's `Task` / `ScheduledFor` domain — sharing the binary would either require generic-ifying Atrium's stable code or adding podcast fields to a project that doesn't need them. Cleaner to study the lex / parse / AST shape (the domain-agnostic ~60% of atrium-search) and ship a podcast-specific `belfry-search` evaluator alongside it. The two projects credit each other and evolve independently — the Framework pattern, not the Viaduct pattern.

Belfry also inherits Atrium's three-crate workspace split (`-core` / `-cli` / GTK binary) and its debug harness pattern (`--debug` flag, stress fixture generators, IO instrumentation).

### Framework — UI discipline
**Brandon LaRocque** · C / GTK4 · GPL-3.0-or-later · <https://github.com/virinvictus/Framework>

Framework's design north star — *"Every action has a visible UI control. Every UI control has a keyboard shortcut. No vim bindings, no modal interfaces, no hidden commands."* — is Belfry's, scaled down to a player. Specifically:

- Keyboard shortcuts for everything (Space / J / K / L / comma / period / S).
- No invisible gestures — if a swipe does something, there's a menu item that does the same thing.
- No vim modes, no chord sequences, no opinionated keystroke schemes the user has to learn.

### Lattice — filesystem-canonical ethos
**Brandon LaRocque** · Python · MIT · <https://github.com/virinvictus/Lattice>

Lattice's discipline: the filesystem is the source of truth; the database is an index. Belfry inherits this for the podcast library — `belfry.db` is regenerable from `~/Podcasts/` plus the rescan contract.

### AntennaPod — retention-policy reference
**Daniel Oeh and contributors** · Android · MIT · <https://antennapod.org/>

The FOSS gold standard for podcast clients. Belfry copies AntennaPod's per-show retention vocabulary: keep N unplayed, delete-after-played delay, never-delete-if-starred. AntennaPod is the implementation reference any time the podcast feature space asks "how should this work?"

### Apps surveyed but not borrowed from

- **Apple Podcasts**, **Pocket Casts**, **Player FM**, **GNOME Podcasts**, **Kasts**, **gPodder** — surveyed during the design phase to establish the feature floor and identify the gaps a polished GTK4 client could fill. None contributed specific patterns Belfry copies.
- **Spotify**, **YouTube Music** — surveyed only to understand what the average user expects to see; explicitly *not* a model.

---

## Technical foundations

### libmpv — the playback engine
[mpv project](https://mpv.io/) · LGPL-2.1+ default; **GPLv3+ in Fedora's `mpv-libs` package** as built with `--enable-gpl --enable-version3` · system library

The Rust binding is [`libmpv2`](https://crates.io/crates/libmpv2). Belfry uses libmpv's property API and `af` (audio filter) chain directly — no custom audio path. This is the largest single dependency and the one that ties Belfry to GPL.

### librubberband — pitch-preserving time-stretch
[Particular Programs Ltd](https://breakfastquay.com/rubberband/) · **GPL-2.0-or-later** · pulled in via libmpv → ffmpeg `rubberband` filter

**This is the license-driver.** Smart Speed's pitch-preservation depends on librubberband. Without it, sped-up audio sounds chipmunk-y — the exact failure mode Overcast was built to avoid. Because librubberband is GPL-2-or-later, the effective combined work is GPL-2-or-later at minimum, and Belfry chooses GPL-3-or-later as the most permissive license compatible with this chain (and with the GNOME ecosystem). See *Licensing summary* below.

### ffmpeg / libavfilter — the filter graph
[FFmpeg developers](https://ffmpeg.org/) · LGPL-2.1+ (stock filters) or GPL-2+ (with `--enable-gpl`) · pulled in via libmpv

The filters Belfry composes:

- `silenceremove` — Smart Speed's silence-skip stage. Stock LGPL.
- `rubberband` — Smart Speed's pitch-preserving stretch. **Requires `--enable-gpl --enable-librubberband` at ffmpeg build time.** This is the GPL-2 link in the chain.
- `acompressor`, `equalizer`, `loudnorm` — Voice Boost stages. All stock LGPL.

Stock Fedora `ffmpeg-free-libs` (LGPL+) lacks the rubberband filter; users will need RPM Fusion's `ffmpeg-libs` or to build with `--enable-gpl --enable-librubberband`. The Flatpak bundle (post-1.0) ships a properly-built ffmpeg.

### GTK4
[GNOME](https://www.gtk.org/) (≥ 4.16) · LGPL-2.1+ · system library

UI toolkit. Bound via [`gtk4-rs`](https://crates.io/crates/gtk4) (MIT).

### libadwaita
[GNOME](https://gnome.pages.gitlab.gnome.org/libadwaita/) (≥ 1.7) · LGPL-2.1+ · system library

GNOME design language. Bound via [`libadwaita-rs`](https://crates.io/crates/libadwaita) (MIT).

### SQLite + FTS5
[D. Richard Hipp et al.](https://www.sqlite.org/) · Public Domain · linked via `rusqlite` (bundled feature)

The data layer. WAL mode mandatory; FTS5 backs library search.

### libsecret (via the `oo7` crate)
[GNOME](https://gnome.pages.gitlab.gnome.org/libsecret/) · LGPL-2.1+ · system library

Stores HTTP Basic credentials (per-show feed auth) in the user's keyring. Belfry's DB only stores a reference; no plaintext credential ever lands in `belfry.db`.

---

## Rust crates

All crates below are MIT or MIT/Apache-2.0 — both permissive and cleanly compatible with GPL-3.

| Crate | Role | License |
|---|---|---|
| `tokio` | Async runtime | MIT |
| `reqwest` | HTTP client (rustls-tls, gzip, brotli) | MIT/Apache-2.0 |
| `oo7` | libsecret credential storage | MIT/Apache-2.0 |
| `rusqlite` (bundled, FTS5) | SQLite bindings | MIT |
| `feed-rs` | RSS / Atom / JSON Feed parsing | MIT/Apache-2.0 |
| `quick-xml` | Namespace-aware XML pass for `podcast:` namespace | MIT |
| `libmpv2` | Rust binding for libmpv | MIT |
| `ammonia` | HTML sanitization for show notes | MIT/Apache-2.0 |
| `id3` | ID3v2 CHAP frame parsing for chapter fallback | MIT |
| `image` | Cover decode for the median-cut accent extractor | MIT/Apache-2.0 |
| `serde` + `serde_json` + `toml` | Config + persistence | MIT/Apache-2.0 |
| `regex` | Show-notes timestamp inference | MIT/Apache-2.0 |
| `tracing` + `tracing-subscriber` | Structured logging | MIT |
| `zbus` | MPRIS2 D-Bus interface + suspend inhibitor | MIT |
| `gtk4-rs` | GTK4 Rust bindings | MIT |
| `libadwaita-rs` | libadwaita Rust bindings | MIT |

If [`atrium-search`](https://github.com/virinvictus/Atrium) is depended on directly for the library search grammar, it inherits its parent's MIT license.

No third-party crate or system library lands without prior sign-off — Brandon's standing rule. Each addition is a checkbox in `roadmap.md`.

---

## Licensing summary

| Layer | License | Note |
|---|---|---|
| Belfry source | **GPL-3.0-or-later** (chosen) | The most permissive license compatible with the chain below. |
| Rust crates | MIT or MIT/Apache-2.0 | Permissive; compatible with GPL-3. |
| GNOME stack (GTK4, libadwaita, libsecret) | LGPL-2.1+ | Compatible. |
| SQLite | Public Domain | Compatible. |
| libmpv (Fedora `mpv-libs`) | **GPLv3+ as built** | Forces GPL-3+ on the linked binary. |
| ffmpeg stock filters (`silenceremove`, `acompressor`, `equalizer`, `loudnorm`) | LGPL-2.1+ | Compatible. |
| **librubberband** (via ffmpeg `rubberband` filter) | **GPL-2-or-later** | The license-driver. Smart Speed depends on it. |

**Combined effective license of the shipping binary: GPL-3.0-or-later.**

Belfry's source remains GPL-3-or-later; recipients distributing a build that includes Smart Speed must also distribute (or offer) corresponding source for the GPL-licensed dependencies, per GPL § 6. This is the standard GPL chain — no surprises, just the unavoidable consequence of using the right tool (rubberband) for Smart Speed.

If a future version drops Smart Speed entirely (or replaces rubberband with a non-GPL pitch-preserving stretcher — none currently exists at the same quality), the license could be relaxed. Until then, GPL-3-or-later is correct.
