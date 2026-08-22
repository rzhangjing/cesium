//! Ported from packages/engine/Source/Core/isDataUri.js

use crate::check::type_of;

const DATA_URI_PREFIX: &str = "data:";

/// Determines if the specified uri is a data uri.
///
/// Port of CesiumJS `isDataUri(uri)` (`/^data:/i`).
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `uri` is `None`.
#[must_use]
pub fn is_data_uri(uri: Option<&str>) -> bool {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) {
        type_of::string("uri", uri);
    }
    // >>includeEnd('debug')
    let Some(uri) = uri else { return false };
    let bytes = uri.as_bytes();
    bytes.len() >= DATA_URI_PREFIX.len()
        && bytes[..DATA_URI_PREFIX.len()].eq_ignore_ascii_case(DATA_URI_PREFIX.as_bytes())
}
