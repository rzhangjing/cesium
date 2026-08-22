//! Ported from packages/engine/Source/Core/getBaseUri.js

use crate::developer_error::throw_developer_error;

/// Given a URI, returns the base path of the URI.
///
/// Port of CesiumJS `getBaseUri(uri, includeQuery)`.
///
/// # Example
/// ```ignore
/// // basePath will be "/Gallery/";
/// let base_path = get_base_uri(Some("/Gallery/simple.czml?value=true&example=false"), None);
///
/// // basePath will be "/Gallery/?value=true&example=false";
/// let base_path = get_base_uri(Some("/Gallery/simple.czml?value=true&example=false"), Some(true));
/// ```
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `uri` is `None`.
#[must_use]
pub fn get_base_uri(uri: Option<&str>, include_query: Option<bool>) -> String {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && uri.is_none() {
        throw_developer_error("uri is required.");
    }
    // >>includeEnd('debug')
    let Some(uri) = uri else {
        return String::new();
    };
    let include_query = include_query.unwrap_or(false);

    let mut base_path = String::new();
    if let Some(i) = uri.rfind('/') {
        base_path.push_str(&uri[..i + 1]);
    }

    if !include_query {
        return base_path;
    }

    // Port of `new Uri(uri)`: query = text after '?' up to '#', fragment =
    // text after '#'.
    let hash_index = uri.find('#');
    let query = match uri.find('?') {
        Some(q) => {
            let end = hash_index.filter(|&h| h > q).unwrap_or(uri.len());
            &uri[q + 1..end]
        }
        None => "",
    };
    let fragment = hash_index.map(|h| &uri[h + 1..]).unwrap_or("");

    if !query.is_empty() {
        base_path.push('?');
        base_path.push_str(query);
    }
    if !fragment.is_empty() {
        base_path.push('#');
        base_path.push_str(fragment);
    }

    base_path
}
