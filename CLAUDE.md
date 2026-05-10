# CLAUDE.md (Belfry)

Per-project guidance. Overrides `~/.claude/CLAUDE.md` only where they conflict.

## What Belfry is

A GNOME 50 podcast client. Reference apps: **Overcast** (audio engine — Smart Speed, Voice Boost, time-saved counter), **Castro** (Inbox → Queue → Played triage, per-show always-queue/archive policy, tap-to-extend sleep timer), **NetNewsWire** (architectural twin pattern, via Viaduct).

Read `spec.md` before changing semantics. Read `roadmap.md` before scoping work. Read `ATTRIBUTIONS.md` before adding deps.

## Hard rules specific to Belfry

- **Listening device, not searching device.** Playback ergonomics > library archeology. Transcripts are 2.x at the earliest; library FTS over titles + descriptions only.
- **The filesystem is the contract.** `~/Podcasts/` is the source of truth; `belfry.db` is regenerable. Any change to the on-disk layout in spec §6.1 is a major-bump operation.
- **GPL-3-or-later is non-negotiable.** Driven by librubberband (Smart Speed). No proposing license relaxation without proposing rubberband replacement.
- **Per-show overrides everywhere.** A 4-hour Hardcore History episode and a 25-minute Cortex have nothing in common except the engine. If a setting is global-only, that's a bug.
- **Queue is shaped data.** Drag-reorderable, with per-show priority breaking ties on auto-insert. Position changes are first-class, not derived.

## Workspace

Four crates: `belfry-core` (headless engine), `belfry-search` (library-search grammar; ports `atrium-search` shape but *does not depend* on it — see ATTRIBUTIONS.md), `belfry-cli`, `belfry` (GTK4 binary).

The architectural twin is **Viaduct**, not Hermitage. Same single-writer SQLite worker, same conditional GET, same OPML-on-disk discipline. The aesthetic twin is **Hermitage** (full-bleed cover art, dynamic accent extracted from cover hue, Codex-style detail surface).

## Architectural commitments to preserve

- **Single-writer SQLite worker.** A dedicated tokio task owns the writable connection; the GTK thread holds an `mpsc::Sender<Command>`. No exceptions; no `RwLock<Connection>`; no second writer.
- **Read commands open the DB read-only at the process level.** `belfry-cli stats` cannot accidentally write because the connection forbids it.
- **OPML round-trip preserves `applePodcastsID` and tags via `<outline>` attributes.** Other apps that ignore the attributes still see a flat-but-correct list; round-trip back into Belfry is lossless.
- **Listening sessions are append-only.** One row per playback span; never updated, never re-attributed.
