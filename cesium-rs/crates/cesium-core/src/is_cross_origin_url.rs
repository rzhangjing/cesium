//! Ported from `packages/engine/Source/Core/isCrossOriginUrl.js`.
//!
//! DEVIATION: This function relies on browser DOM APIs (document.createElement, window.location).
//! In Rust we provide a simplified version that compares URL origins.

/// Given a URL, determine whether that URL is considered cross-origin
/// relative to a base URL.
pub fn is_cross_origin_url(url: &str, base_url: &str) -> bool {
    let base_origin = extract_origin(base_url);
    let url_origin = extract_origin(url);
    base_origin != url_origin
}

fn extract_origin(url: &str) -> String {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        if let Some(end) = after_scheme.find('/') {
            after_scheme[..end].to_string()
        } else {
            after_scheme.to_string()
        }
    } else {
        String::new()
    }
}
