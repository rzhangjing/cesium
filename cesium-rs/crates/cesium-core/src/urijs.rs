//! Internal stand-in for the `urijs` npm dependency used by the CesiumJS
//! Core URI helpers (`getAbsoluteUri`, `getExtensionFromUri`,
//! `getFilenameFromUri`). Not a mirror of a `Source/Core` file.
//!
//! Only the tiny subset used by the ported Core helpers is implemented:
//! `new Uri(x).normalize().path()` and scheme detection.

/// Returns the path component of `uri` after RFC 3986 style normalization
/// (query/fragment stripped, `.`/`..` path segments resolved, percent
/// decoding of path segments left as-is — matches urijs behavior for the
/// inputs covered by the CesiumJS specs).
#[must_use]
pub fn normalize_path(uri: &str) -> String {
    // Absolute URLs: use the `url` crate (which normalizes dot segments).
    if let Ok(parsed) = url::Url::parse(uri) {
        if !parsed.scheme().is_empty() {
            return parsed.path().to_owned();
        }
    }

    // Relative reference: keep the part before '?' or '#'.
    let mut path = uri;
    for delim in ['?', '#'] {
        if let Some(index) = path.find(delim) {
            path = &path[..index];
        }
    }
    remove_dot_segments(path)
}

/// RFC 3986 §5.2.4 "Remove Dot Segments" for a path without authority.
fn remove_dot_segments(path: &str) -> String {
    let mut input = path;
    let mut output: Vec<&str> = Vec::new();
    let mut absolute = path.starts_with('/');

    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix("./") {
            input = rest;
            continue;
        }
        if input == "." {
            input = "";
            continue;
        }
        if let Some(rest) = input.strip_prefix("../") {
            input = rest;
            absolute = false;
            continue;
        }
        if input == ".." {
            input = "";
            absolute = false;
            continue;
        }
        if input.starts_with("/./") {
            input = &input[2..];
            continue;
        }
        if input == "/." {
            input = "/";
            continue;
        }
        if input.starts_with("/../") {
            input = &input[3..];
            output.pop();
            continue;
        }
        if input == "/.." {
            input = "/";
            output.pop();
            continue;
        }
        if input == "." || input == ".." {
            input = "";
            continue;
        }
        // Move the first path segment (including leading '/', if any) to output.
        let start = if input.starts_with('/') { 1 } else { 0 };
        let end = input[start..].find('/').map_or(input.len(), |i| start + i);
        output.push(&input[..end]);
        input = &input[end..];
    }

    let mut result = output.join("");
    if absolute && !result.starts_with('/') {
        result.insert(0, '/');
    }
    result
}

/// Returns the scheme of `uri` ("" when there is none), matching
/// `new Uri(uri).scheme()` for the inputs covered by the CesiumJS specs.
#[must_use]
#[allow(dead_code)]
pub fn scheme(uri: &str) -> &str {
    let Some(colon) = uri.find(':') else {
        return "";
    };
    let candidate = &uri[..colon];
    // RFC 3986 scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    let mut chars = candidate.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return "",
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
        candidate
    } else {
        ""
    }
}
