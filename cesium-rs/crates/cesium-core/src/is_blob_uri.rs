//! Ported from packages/engine/Source/Core/isBlobUri.js

use crate::check::type_of;

const BLOB_URI_PREFIX: &str = "blob:";

/// Determines if the specified uri is a blob uri.
///
/// Port of CesiumJS `isBlobUri(uri)` (`/^blob:/i`).
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `uri` is `None`.
#[must_use]
pub fn is_blob_uri(uri: Option<&str>) -> bool {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) {
        type_of::string("uri", uri);
    }
    // >>includeEnd('debug')
    let Some(uri) = uri else { return false };
    let bytes = uri.as_bytes();
    bytes.len() >= BLOB_URI_PREFIX.len()
        && bytes[..BLOB_URI_PREFIX.len()].eq_ignore_ascii_case(BLOB_URI_PREFIX.as_bytes())
}
