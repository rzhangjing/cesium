//! cesium-resource: Resource management and request scheduling.
//! Domain layer - pure Rust, no framework dependency.
//!
//! CesiumJS mapping: `packages/engine/Source/Core/Resource.js`, `RequestScheduler.js`, `Request.js`

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

/// The type of request.
/// Maps to CesiumJS `RequestType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RequestType {
    /// Terrain request.
    Terrain,
    /// Imagery request.
    Imagery,
    /// 3D Tiles request.
    Tiles3D,
    /// Other request type.
    #[default]
    Other,
}

/// The state of a request.
/// Maps to CesiumJS `RequestState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RequestState {
    /// Initial state, not yet issued.
    #[default]
    Unissued,
    /// Issued but not yet active.
    Issued,
    /// Actively being processed.
    Active,
    /// Received response, processing.
    Received,
    /// Request failed.
    Failed,
    /// Request was cancelled.
    Cancelled,
}

/// A unique identifier for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// Stores information for making a request.
/// Maps to CesiumJS `Request`
#[derive(Debug, Clone)]
pub struct Request {
    /// Unique identifier.
    pub id: RequestId,
    /// The URL to request.
    pub url: String,
    /// Priority (lower = higher priority).
    pub priority: f64,
    /// Whether to throttle and prioritize the request.
    pub throttle: bool,
    /// Whether to throttle by server.
    pub throttle_by_server: bool,
    /// Type of request.
    pub request_type: RequestType,
    /// Current state.
    pub state: RequestState,
    /// Server key for throttling.
    pub server_key: String,
}

impl Request {
    /// Creates a new request.
    pub fn new(url: String, request_type: RequestType) -> Self {
        let server_key = extract_server_key(&url);
        Self {
            id: RequestId(0),
            url,
            priority: 0.0,
            throttle: false,
            throttle_by_server: false,
            request_type,
            state: RequestState::Unissued,
            server_key,
        }
    }

    /// Creates a throttled request.
    pub fn throttled(url: String, request_type: RequestType, priority: f64) -> Self {
        let server_key = extract_server_key(&url);
        Self {
            id: RequestId(0),
            url,
            priority,
            throttle: true,
            throttle_by_server: true,
            request_type,
            state: RequestState::Unissued,
            server_key,
        }
    }
}

/// Wrapper for priority queue ordering (min-heap by priority).
#[derive(Debug, Clone)]
struct PrioritizedRequest {
    id: RequestId,
    priority: f64,
}

impl PartialEq for PrioritizedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PrioritizedRequest {}

impl PartialOrd for PrioritizedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lower priority value = higher priority)
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
    }
}

/// Manages request throttling and prioritization.
/// Maps to CesiumJS `RequestScheduler`
#[derive(Debug)]
pub struct RequestScheduler {
    /// Maximum number of simultaneous active requests.
    pub maximum_requests: usize,
    /// Maximum number of simultaneous active requests per server.
    pub maximum_requests_per_server: usize,
    /// Per-server overrides for max requests.
    pub requests_by_server: HashMap<String, usize>,
    /// Whether to throttle requests.
    pub throttle_requests: bool,
    /// Maximum size of the priority heap.
    pub priority_heap_length: usize,

    // Internal state
    active_requests: HashMap<RequestId, Request>,
    pending_heap: BinaryHeap<PrioritizedRequest>,
    active_count_by_server: HashMap<String, usize>,
    next_id: u64,
}

impl RequestScheduler {
    /// Creates a new RequestScheduler with default settings.
    pub fn new() -> Self {
        Self {
            maximum_requests: 50,
            maximum_requests_per_server: 18,
            requests_by_server: HashMap::new(),
            throttle_requests: true,
            priority_heap_length: 20,
            active_requests: HashMap::new(),
            pending_heap: BinaryHeap::new(),
            active_count_by_server: HashMap::new(),
            next_id: 0,
        }
    }

    /// Returns the number of active requests.
    pub fn active_request_count(&self) -> usize {
        self.active_requests.len()
    }

    /// Returns the number of pending requests.
    pub fn pending_request_count(&self) -> usize {
        self.pending_heap.len()
    }

    /// Checks if a server has open slots for more requests.
    /// Maps to `RequestScheduler.serverHasOpenSlots`
    pub fn server_has_open_slots(&self, server_key: &str, desired_requests: usize) -> bool {
        let max_requests = self
            .requests_by_server
            .get(server_key)
            .copied()
            .unwrap_or(self.maximum_requests_per_server);
        let current = self.active_count_by_server.get(server_key).copied().unwrap_or(0);
        current + desired_requests <= max_requests
    }

    /// Checks if the priority heap has open slots.
    /// Maps to `RequestScheduler.heapHasOpenSlots`
    pub fn heap_has_open_slots(&self, desired_requests: usize) -> bool {
        self.pending_heap.len() + desired_requests <= self.priority_heap_length
    }

    /// Schedules a request. Returns the request ID if accepted.
    /// Maps to `RequestScheduler.request`
    pub fn schedule(&mut self, mut request: Request) -> Option<RequestId> {
        // Assign ID
        let id = RequestId(self.next_id);
        self.next_id += 1;
        request.id = id;

        // If not throttling, immediately activate
        if !self.throttle_requests || !request.throttle {
            self.activate_request(request);
            return Some(id);
        }

        // Check if we can activate immediately
        if self.can_activate(&request) {
            self.activate_request(request);
            return Some(id);
        }

        // Add to pending heap if there's room
        if self.pending_heap.len() < self.priority_heap_length {
            self.pending_heap.push(PrioritizedRequest {
                id,
                priority: request.priority,
            });
            self.active_requests.insert(id, request);
            Some(id)
        } else {
            // Heap is full, reject the request
            None
        }
    }

    /// Cancels a request.
    pub fn cancel(&mut self, id: RequestId) -> bool {
        if let Some(request) = self.active_requests.get_mut(&id) {
            request.state = RequestState::Cancelled;
            self.deactivate_request(id);
            true
        } else {
            false
        }
    }

    /// Marks a request as completed.
    pub fn complete(&mut self, id: RequestId) -> bool {
        if let Some(request) = self.active_requests.get_mut(&id) {
            request.state = RequestState::Received;
            self.deactivate_request(id);
            true
        } else {
            false
        }
    }

    /// Updates priorities and activates pending requests.
    /// Should be called once per frame.
    pub fn update(&mut self) {
        // Try to activate pending requests
        while let Some(prioritized) = self.pending_heap.peek() {
            let id = prioritized.id;
            if let Some(request) = self.active_requests.get(&id) {
                if self.can_activate(request) {
                    let request = self.active_requests.get_mut(&id).unwrap();
                    self.pending_heap.pop();
                    request.state = RequestState::Active;
                    let server_key = request.server_key.clone();
                    *self.active_count_by_server.entry(server_key).or_insert(0) += 1;
                } else {
                    break;
                }
            } else {
                self.pending_heap.pop();
            }
        }
    }

    /// Gets a request by ID.
    pub fn get_request(&self, id: RequestId) -> Option<&Request> {
        self.active_requests.get(&id)
    }

    // Internal helpers

    fn can_activate(&self, request: &Request) -> bool {
        // Check global limit
        let active_count = self
            .active_count_by_server
            .values()
            .sum::<usize>();
        if active_count >= self.maximum_requests {
            return false;
        }

        // Check per-server limit if throttling by server
        if request.throttle_by_server
            && !self.server_has_open_slots(&request.server_key, 1)
        {
            return false;
        }

        true
    }

    fn activate_request(&mut self, mut request: Request) {
        request.state = RequestState::Active;
        let server_key = request.server_key.clone();
        *self.active_count_by_server.entry(server_key).or_insert(0) += 1;
        self.active_requests.insert(request.id, request);
    }

    fn deactivate_request(&mut self, id: RequestId) {
        if let Some(request) = self.active_requests.remove(&id) {
            if let Some(count) = self.active_count_by_server.get_mut(&request.server_key) {
                *count = count.saturating_sub(1);
            }
        }
    }
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the server key from a URL.
fn extract_server_key(url: &str) -> String {
    // Simple extraction: get host:port from URL
    if let Some(start) = url.find("://") {
        let after_scheme = &url[start + 3..];
        let end = after_scheme.find('/').unwrap_or(after_scheme.len());
        after_scheme[..end].to_string()
    } else {
        url.to_string()
    }
}

/// A resource URL template with query parameters.
/// Maps to CesiumJS `Resource` (simplified)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// The base URL.
    pub url: String,
    /// Query parameters.
    pub query_parameters: HashMap<String, String>,
    /// HTTP headers.
    pub headers: HashMap<String, String>,
}

impl Resource {
    /// Creates a new resource with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            query_parameters: HashMap::new(),
            headers: HashMap::new(),
        }
    }

    /// Adds a query parameter.
    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_parameters.insert(key.into(), value.into());
        self
    }

    /// Adds a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Builds the full URL with query parameters.
    pub fn build_url(&self) -> String {
        if self.query_parameters.is_empty() {
            return self.url.clone();
        }

        let params: Vec<String> = self
            .query_parameters
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        if self.url.contains('?') {
            format!("{}&{}", self.url, params.join("&"))
        } else {
            format!("{}?{}", self.url, params.join("&"))
        }
    }

    /// Gets the server key for this resource.
    pub fn server_key(&self) -> String {
        extract_server_key(&self.url)
    }

    /// Creates a derived resource with a relative path appended.
    pub fn derive(&self, relative_path: &str) -> Self {
        let base = if self.url.ends_with('/') {
            &self.url
        } else if let Some(pos) = self.url.rfind('/') {
            &self.url[..=pos]
        } else {
            &self.url
        };

        Self {
            url: format!("{}{}", base, relative_path),
            query_parameters: self.query_parameters.clone(),
            headers: self.headers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_scheduler_basic() {
        let mut scheduler = RequestScheduler::new();
        let request = Request::new(
            "https://example.com/tile.b3dm".to_string(),
            RequestType::Tiles3D,
        );

        let id = scheduler.schedule(request).unwrap();
        assert_eq!(scheduler.active_request_count(), 1);

        scheduler.complete(id);
        assert_eq!(scheduler.active_request_count(), 0);
    }

    #[test]
    fn test_server_throttling() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 2;

        // Schedule 2 requests to the same server
        for i in 0..2 {
            let request = Request::throttled(
                format!("https://example.com/tile{}.b3dm", i),
                RequestType::Tiles3D,
                i as f64,
            );
            scheduler.schedule(request).unwrap();
        }

        // Third request should be pending (not active)
        let request = Request::throttled(
            "https://example.com/tile2.b3dm".to_string(),
            RequestType::Tiles3D,
            2.0,
        );
        scheduler.schedule(request).unwrap();

        // Only 2 should be active
        assert!(scheduler.server_has_open_slots("example.com", 0));
        assert!(!scheduler.server_has_open_slots("example.com", 1));
    }

    #[test]
    fn test_priority_ordering() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 1;
        scheduler.throttle_requests = true;

        // First request activates immediately
        let r1 = Request::throttled(
            "https://a.com/1".to_string(),
            RequestType::Other,
            10.0,
        );
        scheduler.schedule(r1).unwrap();

        // These should be pending
        let r2 = Request::throttled(
            "https://b.com/2".to_string(),
            RequestType::Other,
            5.0, // Higher priority (lower value)
        );
        let r3 = Request::throttled(
            "https://c.com/3".to_string(),
            RequestType::Other,
            1.0, // Highest priority
        );
        scheduler.schedule(r2).unwrap();
        scheduler.schedule(r3).unwrap();

        assert_eq!(scheduler.pending_request_count(), 2);
    }

    #[test]
    fn test_resource_build_url() {
        let resource = Resource::new("https://example.com/api")
            .with_query("key", "value")
            .with_query("format", "json");

        let url = resource.build_url();
        assert!(url.starts_with("https://example.com/api?"));
        assert!(url.contains("key=value"));
        assert!(url.contains("format=json"));
    }

    #[test]
    fn test_resource_derive() {
        let base = Resource::new("https://example.com/tileset.json");
        let derived = base.derive("tiles/tile.b3dm");
        assert_eq!(derived.url, "https://example.com/tiles/tile.b3dm");
    }

    #[test]
    fn test_extract_server_key() {
        assert_eq!(
            extract_server_key("https://example.com:443/path"),
            "example.com:443"
        );
        assert_eq!(
            extract_server_key("http://localhost:8080/api"),
            "localhost:8080"
        );
    }
}
