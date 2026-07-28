//! URI and HTTP utility functions.
//!
//! Faithful port of CesiumJS `objectToQuery.js`, `queryToObject.js`,
//! `parseResponseHeaders.js`, `getFilenameFromUri.js`, `getExtensionFromUri.js`.

use std::collections::HashMap;

/// Converts an object representing URL parameters into a query string.
///
/// CesiumJS: `objectToQuery(obj)` → `"key1=value1&key2=value2"`
/// Arrays produce repeated keys: `{key: ["a","b"]}` → `"key=a&key=b"`
pub fn object_to_query(obj: &HashMap<String, QueryValue>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in obj {
        match value {
            QueryValue::Single(v) => {
                parts.push(format!("{}={}", key, percent_encode(v)));
            }
            QueryValue::Array(arr) => {
                for item in arr {
                    parts.push(format!("{}={}", key, percent_encode(item)));
                }
            }
        }
    }
    parts.join("&")
}

/// Converts a query string into an object.
///
/// CesiumJS: `queryToObject(queryString)` → `{key1: "value1", key2: ["a","b"]}`
/// Supports both `&` and `;` as separators. `+` is decoded as space.
pub fn query_to_object(query_string: &str) -> HashMap<String, QueryValue> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    if query_string.is_empty() {
        return HashMap::new();
    }

    // Split on & or ;
    let pairs: Vec<&str> = query_string
        .split(|c| c == '&' || c == ';')
        .filter(|s| !s.is_empty())
        .collect();

    for pair in pairs {
        let (key, value) = if let Some(idx) = pair.find('=') {
            (&pair[..idx], &pair[idx + 1..])
        } else {
            (pair, "")
        };
        let decoded_key = percent_decode(key);
        let decoded_value = percent_decode(value);
        result.entry(decoded_key).or_default().push(decoded_value);
    }

    // Convert Vec<String> to QueryValue
    result
        .into_iter()
        .map(|(k, v)| {
            if v.len() == 1 {
                (k, QueryValue::Single(v.into_iter().next().unwrap()))
            } else {
                (k, QueryValue::Array(v))
            }
        })
        .collect()
}

/// Parses HTTP response headers string into a map.
///
/// CesiumJS: `parseResponseHeaders(headerString)` → `{Date: "...", Server: "..."}`
pub fn parse_response_headers(header_string: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if header_string.is_empty() {
        return result;
    }

    for line in header_string.split("\r\n") {
        if let Some(idx) = line.find(": ") {
            let key = &line[..idx];
            let value = &line[idx + 2..];
            result.insert(key.to_string(), value.to_string());
        }
    }
    result
}

/// Gets the filename from a URI (last path segment, without query/fragment).
///
/// CesiumJS: `getFilenameFromUri(uri)`
pub fn get_filename_from_uri(uri: &str) -> String {
    // Remove query string
    let path = uri.split('?').next().unwrap_or(uri);
    // Remove fragment
    let path = path.split('#').next().unwrap_or(path);
    // Get last segment
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Gets the file extension from a URI (without the dot).
///
/// CesiumJS: `getExtensionFromUri(uri)` → `"png"` or `""`
pub fn get_extension_from_uri(uri: &str) -> String {
    let filename = get_filename_from_uri(uri);
    if let Some(idx) = filename.rfind('.') {
        filename[idx + 1..].to_string()
    } else {
        String::new()
    }
}

/// Value type for query parameters.
#[derive(Clone, Debug, PartialEq)]
pub enum QueryValue {
    Single(String),
    Array(Vec<String>),
}

/// Percent-encodes a string (RFC 3986).
/// Encodes all characters except unreserved: A-Z a-z 0-9 - _ . ~
fn percent_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// Decodes a percent-encoded string. Also converts `+` to space.
fn percent_decode(input: &str) -> String {
    let input = input.replace('+', " ");
    let mut result = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &input[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}
