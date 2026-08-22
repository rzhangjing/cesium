//! Ported from `packages/engine/Source/Core/queryToObject.js`.

use std::collections::HashMap;

use crate::object_to_query::QueryValue;

/// Parses a query string into an object.
pub fn query_to_object(query_string: &str) -> HashMap<String, QueryValue> {
    let mut result: HashMap<String, QueryValue> = HashMap::new();

    if query_string.is_empty() {
        return result;
    }

    let replaced = query_string.replace('+', "%20");
    let parts: Vec<&str> = replaced
        .split(|c| c == '&' || c == ';')
        .collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }
        let subparts: Vec<&str> = part.splitn(2, '=').collect();
        let name = url_decode(subparts[0]);
        let value = if subparts.len() > 1 {
            url_decode(subparts[1])
        } else {
            String::new()
        };

        match result.get_mut(&name) {
            Some(QueryValue::Single(existing)) => {
                let old = existing.clone();
                result.insert(name, QueryValue::Multiple(vec![old, value]));
            }
            Some(QueryValue::Multiple(arr)) => {
                arr.push(value);
            }
            None => {
                result.insert(name, QueryValue::Single(value));
            }
        }
    }

    result
}

/// Simple percent-decoding for URL components.
fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}
