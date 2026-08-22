//! Ported from packages/engine/Source/Core/getExtensionFromUri.js

use crate::developer_error::throw_developer_error;
use crate::urijs;

/// Given a URI, returns the extension of the URI.
///
/// Port of CesiumJS `getExtensionFromUri(uri)`.
///
/// # Panics
/// Panics with `DeveloperError` when `uri` is `None`.
///
/// # Example
/// ```ignore
/// // extension will be "czml"
/// let extension = get_extension_from_uri(Some("/Gallery/simple.czml?value=true&example=false"));
/// ```
#[must_use]
pub fn get_extension_from_uri(uri: Option<&str>) -> String {
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
    path = match path.rfind('.') {
        Some(index) => path[index + 1..].to_owned(),
        None => String::new(),
    };
    path
}
