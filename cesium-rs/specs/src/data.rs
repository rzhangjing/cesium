//! Test-data root resolution, mirroring the semantics of the CesiumJS
//! `Specs/absolutize.js` helper (resolve a relative spec asset URL against
//! the suite root) for the filesystem.
//!
//! Resolution order for the data root:
//! 1. the `CESIUM_SPECS_DATA` environment variable, when set;
//! 2. otherwise, walk upwards from this crate's manifest dir (`cesium-rs/specs`)
//!    looking for a `Specs/Data` directory — in the monorepo layout this
//!    resolves to `<workspace>/../Specs/Data`
//!    (i.e. `d:/Rust/cesium/Specs/Data`). The original Jasmine data set is
//!    referenced read-only and is never copied.

use std::env;
use std::path::{Path, PathBuf};

/// Environment variable that overrides the spec data root.
pub const SPECS_DATA_ENV: &str = "CESIUM_SPECS_DATA";

/// Returns the root directory containing the mirrored CesiumJS test data
/// (`Specs/Data` in the CesiumJS repository).
///
/// The directory is not required to exist when this function is called;
/// individual tests that need data files should assert existence of the
/// concrete file they read.
///
/// # Panics
/// Panics only if the crate manifest directory is unavailable (cannot
/// happen under `cargo`).
pub fn specs_data_root() -> PathBuf {
    if let Ok(dir) = env::var(SPECS_DATA_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }

    // CARGO_MANIFEST_DIR == <monorepo>/cesium-rs/specs
    let manifest = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    );

    // Walk upwards until a `Specs/Data` directory is found. This supports
    // both the in-monorepo layout and relocated checkouts that carry their
    // own copy of the data.
    let mut current: &Path = manifest.as_path();
    loop {
        let candidate = current.join("Specs").join("Data");
        if candidate.is_dir() {
            return candidate;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                // Fallback: assume the canonical monorepo layout
                // (<workspace>/../Specs/Data) even if not yet present.
                return manifest.join("..").join("Specs").join("Data");
            }
        }
    }
}

/// Joins a path relative to [`specs_data_root`], the filesystem analogue of
/// `absolutize(url)` in `Specs/absolutize.js`.
///
/// ```no_run
/// let czml = cesium_specs::data_path("simple.czml");
/// ```
pub fn data_path(relative: impl AsRef<Path>) -> PathBuf {
    specs_data_root().join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_path_is_rooted_at_specs_data() {
        let p = data_path("CZML/simple.czml");
        assert!(p.to_string_lossy().contains("Specs"));
        assert!(p.ends_with(Path::new("CZML/simple.czml")));
    }
}
