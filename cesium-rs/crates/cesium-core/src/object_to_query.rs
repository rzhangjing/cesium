//! Ported from `packages/engine/Source/Core/objectToQuery.js`.

use std::collections::HashMap;

/// Converts an object representing a set of name/value pairs into a query string.
pub fn object_to_query(obj: &HashMap<String, QueryValue>) -> String {
    let mut result = String::new();
    for (key, value) in obj {
        let encoded_key = url_encode(key);
        let part_prefix = format!("{}=", encoded_key);
        match value {
            QueryValue::Single(v) => {
                result.push_str(&part_prefix);
                result.push_str(&url_encode(v));
                result.push('&');
            }
            QueryValue::Multiple(arr) => {
                for v in arr {
                    result.push_str(&part_prefix);
                    result.push_str(&url_encode(v));
                    result.push('&');
                }
            }
        }
    }
    // trim last &
    if result.ends_with('&') {
        result.pop();
    }
    result
}

/// A value in a query string: either a single string or an array of strings.
#[derive(Debug, Clone)]
pub enum QueryValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Simple percent-encoding for URL components.
fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
