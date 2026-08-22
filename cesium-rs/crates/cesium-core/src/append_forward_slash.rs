//! Ported from packages/engine/Source/Core/appendForwardSlash.js

/// @private
///
/// Port of CesiumJS `appendForwardSlash(url)`: appends a trailing `/` when
/// the URL is empty or does not already end with one.
#[must_use]
pub fn append_forward_slash(url: &str) -> String {
    if url.is_empty() || !url.ends_with('/') {
        format!("{url}/")
    } else {
        url.to_owned()
    }
}
