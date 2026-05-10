//! XDG-aware path resolution for the library, config, and cache directories.
//!
//! See `spec.md` §6.1 (on-disk layout).

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::errors::{Error, Result};

const APP_DIR: &str = "belfry";

/// Resolve the library root from explicit values. Pure function; safe to test
/// without env mutation.
fn library_root_resolved(xdg: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf> {
    if let Some(p) = xdg {
        return Ok(PathBuf::from(p).join(APP_DIR));
    }
    if let Some(p) = home {
        return Ok(PathBuf::from(p).join(".local/share").join(APP_DIR));
    }
    Err(Error::NoLibraryPath)
}

/// Returns `$XDG_DATA_HOME/belfry`, falling back to `$HOME/.local/share/belfry`.
pub fn library_root() -> Result<PathBuf> {
    let xdg = env::var_os("XDG_DATA_HOME");
    let home = env::var_os("HOME");
    library_root_resolved(xdg.as_deref(), home.as_deref())
}

/// Returns `<library_root>/belfry.db`.
pub fn database_file() -> Result<PathBuf> {
    Ok(library_root()?.join("belfry.db"))
}

/// Returns `<library_root>/shows`.
pub fn shows_dir() -> Result<PathBuf> {
    Ok(library_root()?.join("shows"))
}

/// Returns `<library_root>/metadata`.
pub fn metadata_dir() -> Result<PathBuf> {
    Ok(library_root()?.join("metadata"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_root_uses_xdg_data_home() {
        let xdg = OsStr::new("/tmp/xdg-test");
        assert_eq!(
            library_root_resolved(Some(xdg), None).unwrap(),
            PathBuf::from("/tmp/xdg-test/belfry"),
        );
    }

    #[test]
    fn library_root_falls_back_to_home() {
        let home = OsStr::new("/home/test");
        assert_eq!(
            library_root_resolved(None, Some(home)).unwrap(),
            PathBuf::from("/home/test/.local/share/belfry"),
        );
    }

    #[test]
    fn library_root_prefers_xdg_over_home() {
        let xdg = OsStr::new("/tmp/xdg");
        let home = OsStr::new("/home/test");
        assert_eq!(
            library_root_resolved(Some(xdg), Some(home)).unwrap(),
            PathBuf::from("/tmp/xdg/belfry"),
        );
    }

    #[test]
    fn library_root_errors_when_neither_set() {
        assert!(matches!(
            library_root_resolved(None, None),
            Err(Error::NoLibraryPath),
        ));
    }
}
