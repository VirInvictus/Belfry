<p align="center">
  <img src="logo.svg" alt="Belfry" width="240">
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Language-Rust-blue" alt="Language: Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg" alt="License: GPL-3.0-or-later"></a>
  <img src="https://img.shields.io/badge/GNOME-50%2B-4a86cf" alt="GNOME 50+">
  <img src="https://img.shields.io/badge/status-specification%20draft-orange" alt="Status: specification draft">
</p>

---

# Belfry

**Overcast's audio engine and Castro's triage model on a filesystem you can `ls`.**

A native GNOME 50 podcast client built on libmpv with Smart Speed and Voice Boost calibrated against Overcast, the Castro **Inbox → Queue → Played** triage flow as the daily metaphor, and a filesystem-canonical library that survives the database. v0.0.0 — specification draft, no code yet.

## Why this exists

There is no podcast client on Linux with proper silence-skipping or voice-targeted EQ. GNOME Podcasts ships three speed presets and no chapters. gPodder is utilitarian. AntennaPod is Android-only. The closest iOS-quality experience available on a desktop Linux machine is "open Overcast in Safari." Belfry is the answer that should already exist.

Three commitments, in priority order:

1. **Belfry is a listening device, not a searching device.** Playback ergonomics over library archeology. The Now Playing surface gets disproportionate polish; library search exists to find episodes you've already heard.
2. **The filesystem is real.** A user with `find`, `grep`, and `mpv` can use their library without Belfry running. The database is an index; deleting it triggers a rescan.
3. **No second-class media.** Per-show overrides for everything that affects playback — speed, smart speed, voice boost, skip intro/outro, retention, inbox policy.

**Author's Note:** I'm a college student in my late thirties with no professional industry experience yet — Belfry is one in a string of native Linux desktop apps I'm building to learn the craft and assemble a portfolio. I came from iOS, where Overcast is my daily driver. Belfry is the Linux replacement I want to exist. I work on Fedora 44 on a ThinkPad T14s AMD Gen 6; that's the environment it'll be tested against. I welcome contributions but can only honestly support my own setup.

## Status

Specification draft; no code yet. v0.0.0.

- [`spec.md`](spec.md) — the contract.
- [`roadmap.md`](roadmap.md) — six-milestone plan from empty repo to 1.0.
- [`patchnotes.md`](patchnotes.md) — release notes (newest at top).
- [`ATTRIBUTIONS.md`](ATTRIBUTIONS.md) — design lineage, dependency licenses, and the GPL-3-via-rubberband chain.

## Stack

- **Rust 2024 Edition**
- **GTK 4.16+ / libadwaita 1.7+**
- **SQLite** via `rusqlite` (bundled, FTS5) — single-writer worker, WAL mode
- **`tokio`** runtime; **`reqwest`** for HTTP; **`feed-rs`** + **`quick-xml`** for feeds
- **libmpv** via `libmpv2` + ffmpeg's `silenceremove` / `acompressor` / `equalizer` / `loudnorm` / `rubberband` filters
- **`oo7`** for libsecret credential storage (HTTP Basic per-show auth)
- **Meson** wrapper over Cargo for Flatpak packaging
- **Memory budget:** < 100 MB idle, < 200 MB active (see [`spec.md`](spec.md) §12)

## Build (placeholder; real build instructions land at v0.1)

The workspace skeleton currently builds clean but does nothing.

```bash
# Native (development)
cargo check --workspace
cargo build --workspace

# Regression gate (fmt + clippy + tests)
scripts/regression.sh
```

System build dependencies (Fedora 44):

```bash
sudo dnf install gtk4-devel libadwaita-devel mpv-libs-devel sqlite-devel
# For Smart Speed (rubberband filter): RPM Fusion's ffmpeg-libs (not ffmpeg-free-libs)
sudo dnf install --setopt=install_weak_deps=False ffmpeg-libs
```

## License

GPL-3.0-or-later. The license is forced by librubberband's GPL-2-or-later via Smart Speed's pitch-preserving stretch — see [`ATTRIBUTIONS.md`](ATTRIBUTIONS.md) for the full chain.
