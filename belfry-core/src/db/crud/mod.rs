//! CRUD modules — one file per table. The worker dispatches into these;
//! they are not part of the public API.

pub(super) mod chapters;
pub(super) mod episodes;
pub(super) mod playback;
pub(super) mod sessions;
pub(super) mod show_settings;
pub(super) mod shows;
pub(super) mod tags;
