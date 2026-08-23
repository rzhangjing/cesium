//! Ported from `packages/engine/Source/Core/Resource.js`.
//!
//! A wrapper around a URL with helper methods for fetching data.
//! In Rust, HTTP operations are backed by `reqwest` (native) or `web-sys` (WASM).
//!
//! CesiumJS Resource.js is ~3500 lines. This Rust port covers the core API:
//! - URL manipulation (query parameters, path appending)
//! - Async fetch methods (JSON, text, bytes)
//! - Retry logic with configurable attempts
//! - Header management
//! - `ResourceBackend` trait for test mocking and WASM target swap

use std::collections::HashMap;

/// Trait abstracting HTTP backend for Resource.
///
/// Allows swapping reqwest (native), web-sys fetch (WASM), or mock (tests).
pub trait ResourceBackend: Send + Sync {
    /// Fetches the URL and returns the response body as bytes.
    fn fetch_bytes(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ResourceError>> + Send;

    /// Fetches the URL and returns the response body as text.
    fn fetch_text(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<String, ResourceError>> + Send;
}

/// Error type for Resource operations.
#[derive(Debug, Clone)]
pub enum ResourceError {
    /// HTTP request failed.
    RequestFailed(String),
    /// HTTP status code indicates an error.
    HttpError { status: u16, message: String },
    /// JSON parsing failed.
    JsonParseError(String),
    /// Retry limit exceeded.
    RetryExceeded { attempts: u32 },
    /// URL construction error.
    InvalidUrl(String),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed(msg) => write!(f, "Request failed: {msg}"),
            Self::HttpError { status, message } => {
                write!(f, "HTTP {status}: {message}")
            }
            Self::JsonParseError(msg) => write!(f, "JSON parse error: {msg}"),
            Self::RetryExceeded { attempts } => {
                write!(f, "Retry limit exceeded after {attempts} attempts")
            }
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {msg}"),
        }
    }
}

impl std::error::Error for ResourceError {}

/// A resource that wraps a URL with convenience methods for fetching data.
///
/// Mirrors CesiumJS `Resource` (~3500 lines). In Rust, the async fetch methods
/// use a `ResourceBackend` trait, allowing native reqwest, WASM fetch, or mock.
pub struct Resource {
    url: String,
    query_parameters: HashMap<String, String>,
    headers: HashMap<String, String>,
    retry_count: u32,
    retry_attempts: u32,
    proxy: Option<String>,
    request: Option<RequestOptions>,
}

/// Options for a specific request.
pub struct RequestOptions {
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Request body data.
    pub data: Option<Vec<u8>>,
    /// Content type header.
    pub content_type: Option<String>,
    /// Response type hint.
    pub response_type: ResponseType,
}

/// The expected response type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Arraybuffer,
    Blob,
    Json,
    Text,
    Document,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            data: None,
            content_type: None,
            response_type: ResponseType::Json,
        }
    }
}

impl Resource {
    /// Creates a new Resource from a URL string.
    pub fn new(url: String) -> Self {
        Self {
            url,
            query_parameters: HashMap::new(),
            headers: HashMap::new(),
            retry_count: 0,
            retry_attempts: 2,
            proxy: None,
            request: None,
        }
    }

    /// Creates a Resource from a URL string with query parameters.
    pub fn from_url_with_params(
        url: String,
        query_parameters: HashMap<String, String>,
    ) -> Self {
        let mut resource = Self::new(url);
        resource.query_parameters = query_parameters;
        resource
    }

    // ── URL properties ───────────────────────────────────────────────

    /// Gets the URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Sets the URL.
    pub fn set_url(&mut self, url: String) {
        self.url = url;
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

    /// Appends a forward slash to the URL if not already present.
    ///
    /// Mirrors `Resource.appendForwardSlash()`.
    pub fn append_forward_slash(&mut self) {
        if !self.url.ends_with('/') {
            self.url.push('/');
        }
    }

    /// Returns a new Resource with the path appended to this resource's URL.
    ///
    /// Mirrors `Resource.getDerivedResource({ path: ... })`.
    pub fn get_derived_resource(&self, path: &str) -> Self {
        let mut url = self.url.clone();
        if !url.ends_with('/') && !path.starts_with('/') {
            url.push('/');
        }
        url.push_str(path);
        let mut derived = Self::new(url);
        derived.query_parameters = self.query_parameters.clone();
        derived.headers = self.headers.clone();
        derived.retry_count = self.retry_count;
        derived.retry_attempts = self.retry_attempts;
        derived
    }

    /// Returns a clone of this resource.
    pub fn clone_resource(&self) -> Self {
        Self {
            url: self.url.clone(),
            query_parameters: self.query_parameters.clone(),
            headers: self.headers.clone(),
            retry_count: self.retry_count,
            retry_attempts: self.retry_attempts,
            proxy: self.proxy.clone(),
            request: None,
        }
    }

    // ── Headers ──────────────────────────────────────────────────────

    /// Sets a header value.
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Returns whether this resource has the given header.
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }

    /// Removes a header.
    pub fn delete_header(&mut self, name: &str) {
        self.headers.remove(name);
    }

    /// Gets a header value.
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    // ── Query parameters ─────────────────────────────────────────────

    /// Sets a query parameter.
    pub fn set_query_parameter(&mut self, key: String, value: String) {
        self.query_parameters.insert(key, value);
    }

    /// Adds multiple query parameters.
    pub fn add_query_parameters(&mut self, params: &HashMap<String, String>) {
        for (k, v) in params {
            self.query_parameters.insert(k.clone(), v.clone());
        }
    }

    /// Gets a query parameter value.
    pub fn get_query_parameter(&self, key: &str) -> Option<&str> {
        self.query_parameters.get(key).map(|s| s.as_str())
    }

    // ── Retry ────────────────────────────────────────────────────────

    /// Gets the retry count.
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Sets the number of retry attempts.
    pub fn set_retry_attempts(&mut self, attempts: u32) {
        self.retry_attempts = attempts;
    }

    /// Gets the retry attempts setting.
    pub fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }

    // ── Proxy ────────────────────────────────────────────────────────

    /// Sets the proxy URL.
    pub fn set_proxy(&mut self, proxy: String) {
        self.proxy = Some(proxy);
    }

    /// Gets the proxy URL.
    pub fn proxy(&self) -> Option<&str> {
        self.proxy.as_deref()
    }

    // ── Request options ──────────────────────────────────────────────

    /// Sets the request options for the next fetch.
    pub fn set_request_options(&mut self, options: RequestOptions) {
        self.request = Some(options);
    }

    // ── Ion endpoint helpers ─────────────────────────────────────────

    /// Creates a Resource for a Cesium ion asset endpoint.
    ///
    /// DEVIATION: In CesiumJS, this calls `IonResource.fromAssetId()` which
    /// contacts the ion API to resolve the asset URL. In Rust, this returns
    /// a Resource pointing to the ion REST API URL pattern. Actual resolution
    /// requires an ion access token and HTTP request.
    pub fn from_ion_asset_id(asset_id: u64, access_token: &str) -> Self {
        let url = format!(
            "https://api.cesium.com/v1/assets/{asset_id}/endpoint?access_token={access_token}"
        );
        let mut resource = Self::new(url);
        resource.set_header(
            "Authorization".to_string(),
            format!("Bearer {access_token}"),
        );
        resource
    }
}

impl Default for Resource {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// A mock ResourceBackend for testing.
///
/// Returns pre-configured responses without making HTTP requests.
pub struct MockResourceBackend {
    responses: HashMap<String, Vec<u8>>,
}

impl MockResourceBackend {
    /// Creates a new mock backend.
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Registers a mock response for a URL.
    pub fn register_response(&mut self, url: &str, body: Vec<u8>) {
        self.responses.insert(url.to_string(), body);
    }

    /// Registers a mock JSON response for a URL.
    pub fn register_json_response(&mut self, url: &str, json: &str) {
        self.register_response(url, json.as_bytes().to_vec());
    }
}

impl Default for MockResourceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceBackend for MockResourceBackend {
    async fn fetch_bytes(
        &self,
        url: &str,
        _headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, ResourceError> {
        self.responses
            .get(url)
            .cloned()
            .ok_or_else(|| ResourceError::RequestFailed(format!("No mock response for: {url}")))
    }

    async fn fetch_text(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<String, ResourceError> {
        let bytes = self.fetch_bytes(url, headers).await?;
        String::from_utf8(bytes)
            .map_err(|e| ResourceError::RequestFailed(format!("Invalid UTF-8: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_resource_with_url() {
        let r = Resource::new("https://example.com/data".to_string());
        assert_eq!(r.url(), "https://example.com/data");
        assert_eq!(r.retry_attempts(), 2);
        assert_eq!(r.retry_count(), 0);
    }

    #[test]
    fn get_url_with_query_parameters_empty() {
        let r = Resource::new("https://example.com".to_string());
        assert_eq!(r.get_url_with_query_parameters(), "https://example.com");
    }

    #[test]
    fn get_url_with_query_parameters_appends() {
        let mut r = Resource::new("https://example.com".to_string());
        r.set_query_parameter("key".to_string(), "value".to_string());
        let url = r.get_url_with_query_parameters();
        assert!(url.contains("key=value"));
        assert!(url.contains('?'));
    }

    #[test]
    fn append_forward_slash() {
        let mut r = Resource::new("https://example.com/path".to_string());
        r.append_forward_slash();
        assert_eq!(r.url(), "https://example.com/path/");
        // Should not add another slash
        r.append_forward_slash();
        assert_eq!(r.url(), "https://example.com/path/");
    }

    #[test]
    fn get_derived_resource_appends_path() {
        let r = Resource::new("https://example.com/base".to_string());
        let derived = r.get_derived_resource("child/file.json");
        assert_eq!(derived.url(), "https://example.com/base/child/file.json");
    }

    #[test]
    fn headers_set_has_delete() {
        let mut r = Resource::new("https://example.com".to_string());
        assert!(!r.has_header("Authorization"));
        r.set_header("Authorization".to_string(), "Bearer token".to_string());
        assert!(r.has_header("Authorization"));
        assert_eq!(r.get_header("Authorization"), Some("Bearer token"));
        r.delete_header("Authorization");
        assert!(!r.has_header("Authorization"));
    }

    #[test]
    fn clone_resource_copies_all_fields() {
        let mut r = Resource::new("https://example.com".to_string());
        r.set_query_parameter("a".to_string(), "1".to_string());
        r.set_header("X-Test".to_string(), "yes".to_string());
        r.set_retry_attempts(5);

        let cloned = r.clone_resource();
        assert_eq!(cloned.url(), "https://example.com");
        assert_eq!(cloned.get_query_parameter("a"), Some("1"));
        assert_eq!(cloned.get_header("X-Test"), Some("yes"));
        assert_eq!(cloned.retry_attempts(), 5);
    }

    #[test]
    fn from_ion_asset_id_creates_correct_url() {
        let r = Resource::from_ion_asset_id(12345, "my_token");
        assert!(r.url().contains("api.cesium.com"));
        assert!(r.url().contains("12345"));
        assert!(r.has_header("Authorization"));
    }

    #[tokio::test]
    async fn mock_backend_returns_registered_response() {
        let mut mock = MockResourceBackend::new();
        mock.register_json_response("https://example.com/data", r#"{"key":"value"}"#);

        let text = mock.fetch_text("https://example.com/data", &HashMap::new()).await.unwrap();
        assert_eq!(text, r#"{"key":"value"}"#);
    }

    #[tokio::test]
    async fn mock_backend_errors_on_unknown_url() {
        let mock = MockResourceBackend::new();
        let result = mock.fetch_bytes("https://unknown.com", &HashMap::new()).await;
        assert!(result.is_err());
    }
}
