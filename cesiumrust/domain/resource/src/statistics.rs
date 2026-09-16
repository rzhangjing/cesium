//! Request statistics aggregation.
//!
//! Maps to CesiumJS `RequestScheduler.statistics` (the private `statistics`
//! object inside `RequestScheduler.js`) and the per-server / per-type counters
//! used for diagnostics and throttling decisions.
//!
//! The statistics are **pure counters** — no IO, no framework dependency.
//! The [`RequestScheduler`] in `lib.rs` drives state transitions that update
//! these counters.

use std::collections::HashMap;

use crate::RequestType;

/// Aggregate request statistics.
///
/// Mirrors CesiumJS `RequestScheduler.statistics`:
/// ```js
/// var statistics = {
///   numberOfAttemptedRequests: 0,
///   numberOfActiveRequests: 0,
///   numberOfCancelledRequests: 0,
///   numberOfCancelledActiveRequests: 0,
///   numberOfFailedRequests: 0,
///   numberOfActiveRequestsEver: 0,
///   lastNumberOfActiveRequests: 0,
/// };
/// ```
///
/// Extended with per-server and per-type breakdowns for richer diagnostics
/// (the per-server breakdown mirrors `numberOfActiveRequestsByServer`).
#[derive(Debug, Clone, Default)]
pub struct RequestStatistics {
    /// Total number of requests that have been attempted (scheduled).
    pub attempted: u64,

    /// Number of currently active requests.
    pub active: u64,

    /// Number of requests cancelled while pending (never activated).
    pub cancelled_pending: u64,

    /// Number of requests cancelled while active.
    pub cancelled_active: u64,

    /// Number of requests that failed (retries exhausted or error).
    pub failed: u64,

    /// Number of requests that completed successfully.
    pub succeeded: u64,

    /// Total number of requests ever made active (monotonic).
    pub active_ever: u64,

    /// Number of active requests at the previous `update()` call.
    /// Used for delta diagnostics (JS `lastNumberOfActiveRequests`).
    pub last_active: u64,

    /// Per-server active request counts.
    ///
    /// Maps to `RequestScheduler.numberOfActiveRequestsByServer`.
    pub active_by_server: HashMap<String, u64>,

    /// Per-server total completed counts.
    pub completed_by_server: HashMap<String, u64>,

    /// Per-server total failed counts.
    pub failed_by_server: HashMap<String, u64>,

    /// Per-type active request counts.
    pub active_by_type: HashMap<RequestType, u64>,

    /// Per-type total completed counts.
    pub completed_by_type: HashMap<RequestType, u64>,

    /// Per-type total failed counts.
    pub failed_by_type: HashMap<RequestType, u64>,
}

impl RequestStatistics {
    /// Creates a new zero-initialized statistics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that a request has been scheduled (attempted).
    pub fn on_scheduled(&mut self) {
        self.attempted += 1;
    }

    /// Records that a request has been activated.
    pub fn on_activated(&mut self, server_key: &str, request_type: RequestType) {
        self.active += 1;
        self.active_ever += 1;
        *self.active_by_server.entry(server_key.to_string()).or_insert(0) += 1;
        *self.active_by_type.entry(request_type).or_insert(0) += 1;
    }

    /// Records that a request completed successfully.
    pub fn on_completed(&mut self, server_key: &str, request_type: RequestType) {
        self.active = self.active.saturating_sub(1);
        self.succeeded += 1;
        if let Some(count) = self.active_by_server.get_mut(server_key) {
            *count = count.saturating_sub(1);
        }
        if let Some(count) = self.active_by_type.get_mut(&request_type) {
            *count = count.saturating_sub(1);
        }
        *self.completed_by_server.entry(server_key.to_string()).or_insert(0) += 1;
        *self.completed_by_type.entry(request_type).or_insert(0) += 1;
    }

    /// Records that a request failed.
    pub fn on_failed(&mut self, server_key: &str, request_type: RequestType) {
        self.active = self.active.saturating_sub(1);
        self.failed += 1;
        if let Some(count) = self.active_by_server.get_mut(server_key) {
            *count = count.saturating_sub(1);
        }
        if let Some(count) = self.active_by_type.get_mut(&request_type) {
            *count = count.saturating_sub(1);
        }
        *self.failed_by_server.entry(server_key.to_string()).or_insert(0) += 1;
        *self.failed_by_type.entry(request_type).or_insert(0) += 1;
    }

    /// Records that a pending (never-activated) request was cancelled.
    pub fn on_cancelled_pending(&mut self) {
        self.cancelled_pending += 1;
    }

    /// Records that an active request was cancelled.
    pub fn on_cancelled_active(&mut self, server_key: &str, request_type: RequestType) {
        self.active = self.active.saturating_sub(1);
        self.cancelled_active += 1;
        if let Some(count) = self.active_by_server.get_mut(server_key) {
            *count = count.saturating_sub(1);
        }
        if let Some(count) = self.active_by_type.get_mut(&request_type) {
            *count = count.saturating_sub(1);
        }
    }

    /// Called at the start of each scheduler `update()` cycle to snapshot the
    /// previous active count for delta diagnostics.
    ///
    /// Maps to JS: `statistics.lastNumberOfActiveRequests = statistics.numberOfActiveRequests`.
    pub fn snapshot_last_active(&mut self) {
        self.last_active = self.active;
    }

    /// Returns the total number of cancelled requests (pending + active).
    pub fn total_cancelled(&self) -> u64 {
        self.cancelled_pending + self.cancelled_active
    }

    /// Returns the total number of finished requests (succeeded + failed + cancelled).
    pub fn total_finished(&self) -> u64 {
        self.succeeded + self.failed + self.total_cancelled()
    }

    /// Returns the number of active requests for a specific server.
    pub fn active_for_server(&self, server_key: &str) -> u64 {
        self.active_by_server.get(server_key).copied().unwrap_or(0)
    }

    /// Returns the number of active requests for a specific type.
    pub fn active_for_type(&self, request_type: RequestType) -> u64 {
        self.active_by_type.get(&request_type).copied().unwrap_or(0)
    }

    /// Resets all counters to zero (used in tests / `clearForSpecs`).
    ///
    /// Maps to `RequestScheduler.clearForSpecs()` which resets the statistics.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Returns a human-readable summary string for diagnostics.
    pub fn summary(&self) -> String {
        format!(
            "attempted={} active={} succeeded={} failed={} cancelled={}",
            self.attempted,
            self.active,
            self.succeeded,
            self.failed,
            self.total_cancelled(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_statistics_are_zeroed() {
        let stats = RequestStatistics::new();
        assert_eq!(stats.attempted, 0);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.succeeded, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.total_cancelled(), 0);
        assert!(stats.active_by_server.is_empty());
    }

    #[test]
    fn schedule_and_activate() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("example.com:443", RequestType::Imagery);
        assert_eq!(stats.attempted, 1);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.active_ever, 1);
        assert_eq!(stats.active_for_server("example.com:443"), 1);
        assert_eq!(stats.active_for_type(RequestType::Imagery), 1);
    }

    #[test]
    fn complete_decrements_active() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("a.com:80", RequestType::Terrain);
        stats.on_completed("a.com:80", RequestType::Terrain);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.succeeded, 1);
        assert_eq!(stats.active_for_server("a.com:80"), 0);
        assert_eq!(*stats.completed_by_server.get("a.com:80").unwrap(), 1);
    }

    #[test]
    fn fail_decrements_active() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("b.com:443", RequestType::Tiles3D);
        stats.on_failed("b.com:443", RequestType::Tiles3D);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.failed, 1);
        assert_eq!(*stats.failed_by_type.get(&RequestType::Tiles3D).unwrap(), 1);
    }

    #[test]
    fn cancel_pending_does_not_affect_active() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_cancelled_pending();
        assert_eq!(stats.active, 0);
        assert_eq!(stats.cancelled_pending, 1);
        assert_eq!(stats.total_cancelled(), 1);
    }

    #[test]
    fn cancel_active_decrements() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("c.com:80", RequestType::Other);
        stats.on_cancelled_active("c.com:80", RequestType::Other);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.cancelled_active, 1);
        assert_eq!(stats.active_for_server("c.com:80"), 0);
    }

    #[test]
    fn multiple_servers_tracked_independently() {
        let mut stats = RequestStatistics::new();
        stats.on_activated("a.com:443", RequestType::Imagery);
        stats.on_activated("a.com:443", RequestType::Imagery);
        stats.on_activated("b.com:443", RequestType::Terrain);
        assert_eq!(stats.active, 3);
        assert_eq!(stats.active_for_server("a.com:443"), 2);
        assert_eq!(stats.active_for_server("b.com:443"), 1);
        assert_eq!(stats.active_for_type(RequestType::Imagery), 2);
        assert_eq!(stats.active_for_type(RequestType::Terrain), 1);
    }

    #[test]
    fn snapshot_last_active() {
        let mut stats = RequestStatistics::new();
        stats.on_activated("x.com:80", RequestType::Other);
        stats.on_activated("x.com:80", RequestType::Other);
        stats.snapshot_last_active();
        assert_eq!(stats.last_active, 2);
        stats.on_completed("x.com:80", RequestType::Other);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.last_active, 2); // unchanged until next snapshot
    }

    #[test]
    fn reset_clears_everything() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("d.com:80", RequestType::Imagery);
        stats.on_completed("d.com:80", RequestType::Imagery);
        stats.reset();
        assert_eq!(stats.attempted, 0);
        assert_eq!(stats.succeeded, 0);
        assert!(stats.active_by_server.is_empty());
    }

    #[test]
    fn total_finished_accounts_for_all_outcomes() {
        let mut stats = RequestStatistics::new();
        // 1 succeeded, 1 failed, 1 cancelled_pending, 1 cancelled_active
        stats.on_activated("e.com:80", RequestType::Other);
        stats.on_completed("e.com:80", RequestType::Other);
        stats.on_activated("e.com:80", RequestType::Other);
        stats.on_failed("e.com:80", RequestType::Other);
        stats.on_cancelled_pending();
        stats.on_activated("e.com:80", RequestType::Other);
        stats.on_cancelled_active("e.com:80", RequestType::Other);
        assert_eq!(stats.total_finished(), 4);
    }

    #[test]
    fn summary_format() {
        let mut stats = RequestStatistics::new();
        stats.on_scheduled();
        stats.on_activated("f.com:80", RequestType::Other);
        let s = stats.summary();
        assert!(s.contains("attempted=1"));
        assert!(s.contains("active=1"));
    }
}
