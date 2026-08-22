//! Ported from packages/engine/Source/Core/getFilenameFromUri.js

use crate::developer_error::throw_developer_error;
use crate::urijs;

/// Given a URI, returns the last segment of the URI, removing any path or
/// query information.
///
/// Port of CesiumJS `getFilenameFromUri(uri)`.
///
/// # Panics
/// Panics with `DeveloperError` when `uri` is `None`.
///
/// # Example
/// ```ignore
/// // fileName will be "simple.czml"
/// let file_name = get_filename_from_uri(Some("/Gallery/simple.czml?value=true&example=false"));
/// ```
#[must_use]
pub fn get_filename_from_uri(uri: Option<&str>) -> String {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && uri.is_none() {
        throw_developer_error("uri is required.");
    }
    // >>includeEnd('debug')
    let Some(uri) = uri else {
        return String::new();
    };

    let mut path = urijs::normalize_path(uri);
    if let Some(index) = path.rfind('/') {
        path = path[index + 1..].to_owned();
    }
    path
}
