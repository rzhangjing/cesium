//! Ported from `packages/engine/Source/Core/Resource.js`.
//!
//! A wrapper around a URL with helper methods for fetching data.
//! Full implementation deferred - this provides the core structure.

/// A resource that wrapss a URL with convenience methods for fetching data.
pub struct Resource {
    url: String,
    query_parameters: std::collections::HashMap<String, String>,
    headers: std::collections::HashMap<String, String>,
    retry_count: u32,
    retry_attempts: u32,
}

impl Resource {
    /// Creates a new Resource from a URL string.
    pub fn new(url: String) -> Self {
        Self {
            url,
            query_parameters: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            retry_count: 0,
            retry_attempts: 0,
        }
    }

    /// Gets the URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Sets a header value.
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Sets a query parameter.
    pub fn set_query_parameter(&mut self, key: String, value: String) {
        self.query_parameters.insert(key, value);
    }

    /// Gets the retry count.
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Sets the number of retry attempts.
    pub fn set_retry_attempts(&mut self, attempts: u32) {
        self.retry_attempts = attempts;
    }

    /// Gets a copy of this resource.
    pub fn clone_resource(&self) -> Self {
        Self {
            url: self.url.clone(),
            query_parameters: self.query_parameters.clone(),
            headers: self.headers.clone(),
            retry_count: self.retry_count,
            retry_attempts: self.retry_attempts,
        }
    }

    /// Gets the URL with query parameters appended.
    pub fn get_url_with_query_parameters(&self) -> String {
        if self.query_parameters.is_empty() {
            return self.url.clone();
        }
        let mut url = self.url.clone();
        let separator = if url.contains('?') { "&" } else { "?" };
        let params: Vec<String> = self
            .query_parameters
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        url.push_str(separator);
        url.push_str(&params.join("&"));
        url
    }
}
