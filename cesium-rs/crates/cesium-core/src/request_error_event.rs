//! Ported from `packages/engine/Source/Core/RequestErrorEvent.js`.

use crate::parse_response_headers;
use std::collections::HashMap;

/// An event that is raised when a request encounters an error.
pub struct RequestErrorEvent {
    /// The HTTP error status code.
    pub status_code: Option<u16>,
    /// The response included along with the error.
    pub response: Option<String>,
    /// The response headers as key/value pairs.
    pub response_headers: Option<HashMap<String, String>>,
}

impl RequestErrorEvent {
    /// Creates a new RequestErrorEvent.
    pub fn new(
        status_code: Option<u16>,
        response: Option<String>,
        response_headers: Option<String>,
    ) -> Self {
        let headers = response_headers.map(|h| parse_response_headers::parse_response_headers(&h));
        Self {
            status_code,
            response,
            response_headers: headers,
        }
    }
}

impl std::fmt::Display for RequestErrorEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("Request has failed.");
        if let Some(code) = self.status_code {
            s.push_str(&format!(" Status Code: {code}"));
        }
        write!(f, "{s}")
    }
}
