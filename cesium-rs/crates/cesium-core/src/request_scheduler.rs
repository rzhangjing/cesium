//! Ported from `packages/engine/Source/Core/RequestScheduler.js`.

/// Schedules and prioritizes network requests.
pub struct RequestScheduler {
    _private: (),
}

impl RequestScheduler {
    /// Creates a new RequestScheduler.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for RequestScheduler {
    fn default() -> Self { Self::new() }
}
