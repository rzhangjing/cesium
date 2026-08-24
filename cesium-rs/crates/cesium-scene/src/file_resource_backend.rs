//! Offline `file://` implementation of [`ResourceBackend`].
//!
//! B4-5 (offline discipline): the globe terrain/imagery demo path must work
//! without network access. This backend resolves `file:///...` URLs (and
//! bare filesystem paths) against the local disk and maps the two failure
//! classes the tile pipeline needs to distinguish (cesiumrust pitfall
//! checkpoint "failed/placeholder"):
//!
//! - **missing file** → [`ResourceError::HttpError`] with `status = 404`:
//!   a DETERMINISTIC no-data signal (the tile may inherit/upsample from its
//!   ancestor and the outcome may be cached permanently);
//! - **any other IO failure** → [`ResourceError::RequestFailed`]: a
//!   TRANSIENT signal that must be retried on a later frame and never
//!   stamped as permanent no-data.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cesium_core::resource::{ResourceBackend, ResourceError};

/// A [`ResourceBackend`] that reads `file://` URLs from the local disk.
#[derive(Default, Clone, Copy)]
pub struct FileResourceBackend;

impl FileResourceBackend {
    /// Creates a new FileResourceBackend.
    pub fn new() -> Self {
        Self
    }
}

/// Maps a `file://` URL (or a bare path) onto a local filesystem path.
fn url_to_path(url: &str) -> PathBuf {
    let remainder = url
        .strip_prefix("file:///")
        .or_else(|| url.strip_prefix("file://"))
        .unwrap_or(url);
    // Windows: "file:///C:/foo" strips down to "C:/foo"; the "/C:/foo"
    // variant (host-less three-slash form already consumed above) needs the
    // leading separator dropped.
    let bytes = remainder.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
        return PathBuf::from(&remainder[1..]);
    }
    Path::new(remainder).to_path_buf()
}

/// Classifies a local read failure into the deterministic (404) vs.
/// transient signal the tile pipeline relies on.
fn io_error_to_resource_error(path: &Path, error: std::io::Error) -> ResourceError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ResourceError::HttpError {
            status: 404,
            message: format!("File not found: {}", path.display()),
        }
    } else {
        ResourceError::RequestFailed(format!(
            "Failed to read {}: {error}",
            path.display()
        ))
    }
}

impl ResourceBackend for FileResourceBackend {
    async fn fetch_bytes(
        &self,
        url: &str,
        _headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, ResourceError> {
        let path = url_to_path(url);
        std::fs::read(&path).map_err(|error| io_error_to_resource_error(&path, error))
    }

    async fn fetch_text(
        &self,
        url: &str,
        _headers: &HashMap<String, String>,
    ) -> Result<String, ResourceError> {
        let path = url_to_path(url);
        std::fs::read_to_string(&path).map_err(|error| io_error_to_resource_error(&path, error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_file_urls_to_local_paths() {
        assert_eq!(
            url_to_path("file:///C:/terrain/layer.json"),
            PathBuf::from("C:/terrain/layer.json")
        );
        assert_eq!(
            url_to_path("file:///D:/Rust/cesium/a.terrain"),
            PathBuf::from("D:/Rust/cesium/a.terrain")
        );
        assert_eq!(
            url_to_path("file://C:/x/y"),
            PathBuf::from("C:/x/y")
        );
        assert_eq!(url_to_path("assets/tiles/0.terrain"), PathBuf::from("assets/tiles/0.terrain"));
    }

    #[test]
    fn missing_file_is_a_deterministic_404() {
        let backend = FileResourceBackend::new();
        let headers = HashMap::new();
        let result = pollster::block_on(backend.fetch_bytes(
            "file:///this/path/does/not/exist.bin",
            &headers,
        ));
        match result {
            Err(ResourceError::HttpError { status, .. }) => assert_eq!(status, 404),
            other => panic!("expected a 404 HttpError, got {other:?}"),
        }
    }
}
