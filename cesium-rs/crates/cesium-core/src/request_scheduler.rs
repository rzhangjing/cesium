//! Ported from `packages/engine/Source/Core/RequestScheduler.js`.
//!
//! Manages and prioritizes requests. Skeleton implementation.

/// The request scheduler is used to manage and prioritize requests.
pub struct RequestScheduler {
    /// Maximum number of simultaneous requests.
    pub maximum_requests: usize,
}

impl RequestScheduler {
    /// Creates a new RequestScheduler.
    pub fn new() -> Self {
        Self {
            maximum_requests: 20,
        }
    }
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new()
    }
}
