//! cesium-network: HTTP network adapter
//!
//! Implements the TileFetcher port using async HTTP requests.
//! Uses tokio for async runtime.

use cesium_ports_driven::{PortError, PortResult, TileFetcher};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Network errors
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Request cancelled")]
    Cancelled,
}

/// HTTP-based tile fetcher implementation.
///
/// This is a mock implementation for now. In a real implementation,
/// this would use reqwest or another HTTP client.
#[allow(dead_code)]
pub struct HttpTileFetcher {
    /// Base URL for tile requests
    base_url: String,

    /// Request headers
    headers: HashMap<String, String>,

    /// Active request count per server
    active_requests: Arc<Mutex<HashMap<String, usize>>>,

    /// Maximum concurrent requests per server
    max_requests_per_server: usize,

    /// Cancelled URLs
    cancelled: Arc<Mutex<Vec<String>>>,
}

impl HttpTileFetcher {
    /// Creates a new HTTP tile fetcher.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            headers: HashMap::new(),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_server: 6,
            cancelled: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Sets a request header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets the maximum concurrent requests per server.
    pub fn with_max_requests_per_server(mut self, max: usize) -> Self {
        self.max_requests_per_server = max;
        self
    }

    /// Extracts the server key from a URL.
    #[allow(dead_code)]
    fn extract_server_key(url: &str) -> String {
        // Simple extraction: use the host as the server key
        if let Some(start) = url.find("://") {
            let rest = &url[start + 3..];
            if let Some(end) = rest.find('/') {
                return rest[..end].to_string();
            }
            return rest.to_string();
        }
        url.to_string()
    }
}

impl TileFetcher for HttpTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        _priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        Box::pin(async move {
            // Check if cancelled
            {
                let cancelled = self.cancelled.lock().await;
                if cancelled.contains(&url.to_string()) {
                    return Err(PortError::Cancelled);
                }
            }

            // In a real implementation, this would make an async HTTP request
            // For now, return an error indicating this is a mock
            Err(PortError::Network(format!(
                "HttpTileFetcher::fetch not implemented for URL: {}. \
                 This is a mock implementation.",
                url
            )))
        })
    }

    fn cancel(&self, url: &str) {
        let cancelled = self.cancelled.clone();
        let url = url.to_string();
        tokio::spawn(async move {
            let mut cancelled = cancelled.lock().await;
            cancelled.push(url);
        });
    }
}

/// A mock tile fetcher for testing that returns predefined data.
pub struct MockTileFetcher {
    /// Predefined responses keyed by URL
    responses: HashMap<String, Vec<u8>>,
}

impl MockTileFetcher {
    /// Creates a new mock tile fetcher.
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Adds a predefined response for a URL.
    pub fn with_response(mut self, url: &str, data: Vec<u8>) -> Self {
        self.responses.insert(url.to_string(), data);
        self
    }
}

impl Default for MockTileFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl TileFetcher for MockTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        _priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        let result = self
            .responses
            .get(url)
            .cloned()
            .ok_or_else(|| PortError::NotFound(format!("No mock response for URL: {}", url)));

        Box::pin(async move { result })
    }

    fn cancel(&self, _url: &str) {
        // Mock fetcher doesn't need cancellation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_server_key() {
        assert_eq!(
            HttpTileFetcher::extract_server_key("https://example.com/tiles/0/0/0.terrain"),
            "example.com"
        );
        assert_eq!(
            HttpTileFetcher::extract_server_key("http://localhost:8080/api"),
            "localhost:8080"
        );
    }

    #[tokio::test]
    async fn test_mock_tile_fetcher() {
        let fetcher = MockTileFetcher::new()
            .with_response("http://test.com/tile.terrain", vec![1, 2, 3, 4]);

        let result = fetcher.fetch("http://test.com/tile.terrain", 1.0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);

        let result = fetcher.fetch("http://test.com/missing.terrain", 1.0).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_http_tile_fetcher_builder() {
        let fetcher = HttpTileFetcher::new("https://assets.cesium.com")
            .with_header("Authorization", "Bearer token")
            .with_max_requests_per_server(10);

        assert_eq!(fetcher.base_url, "https://assets.cesium.com");
        assert_eq!(fetcher.max_requests_per_server, 10);
        assert!(fetcher.headers.contains_key("Authorization"));
    }

    #[tokio::test]
    async fn test_http_tile_fetcher_mock() {
        let fetcher = HttpTileFetcher::new("https://assets.cesium.com");
        let result = fetcher.fetch("https://assets.cesium.com/tile.terrain", 1.0).await;
        // Should return an error since this is a mock
        assert!(result.is_err());
    }
}
