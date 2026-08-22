//! Ported from `packages/engine/Source/Core/parseResponseHeaders.js`.

use std::collections::HashMap;

/// Parses the result of XMLHttpRequest's `getAllResponseHeaders()` method into
/// a dictionary.
pub fn parse_response_headers(header_string: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    if header_string.is_empty() {
        return headers;
    }

    for header_pair in header_string.split("\r\n") {
        if let Some(index) = header_pair.find(": ") {
            if index > 0 {
                let key = &header_pair[..index];
                let val = &header_pair[index + 2..];
                headers.insert(key.to_string(), val.to_string());
            }
        }
    }

    headers
}
