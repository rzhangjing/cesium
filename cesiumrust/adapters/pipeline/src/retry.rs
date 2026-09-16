//! Default retry policy — dual-timescale, verbatim from `dynamic_globe.rs`.
//!
//! Mirrors the two independent retry mechanisms that must NOT be conflated:
//!
//! 1. **Worker-level retries** (`download_worker`, L2186-2191): 3 attempts
//!    with exponential backoff `250ms << attempt` → 250 ms, 500 ms, 1000 ms.
//!    Handles momentary 403/429/timeout from the tile server.
//! 2. **Pipeline-level cooldown** (L1068-1071): after all worker retries are
//!    exhausted the tile enters the `retry_after` map with a **10 s** cooldown
//!    before `enqueue_tiles` (L398-400, L419-421) will re-issue the fetch.
//!
//! Both timescales are required: worker retries smooth over transient hiccups;
//! the pipeline cooldown prevents hammering a throttling server every frame.

use std::time::Duration;

use cesium_ports_driven::RetryPolicy;

/// Default retry policy matching `dynamic_globe.rs` exactly.
///
/// | Parameter | Value | Source line |
/// |-----------|-------|-------------|
/// | `max_attempts` | 3 | L2186 |
/// | `backoff_base` | 250 ms | L2189 |
/// | `cooldown` | 10 s | L1070 |
#[derive(Debug, Clone, Copy)]
pub struct DefaultRetry;

impl DefaultRetry {
    /// `dynamic_globe.rs:2186` — worker-level retry attempts (`0..3u32`).
    pub const MAX_ATTEMPTS: u32 = 3;
    /// `dynamic_globe.rs:2189` — base backoff; actual sleep = `base << attempt`.
    pub const BACKOFF_BASE: Duration = Duration::from_millis(250);
    /// `dynamic_globe.rs:1070` — pipeline-level retry cooldown (10 s).
    pub const COOLDOWN: Duration = Duration::from_secs(10);

    /// Compute the backoff sleep for a given zero-based attempt index.
    ///
    /// Corresponds to L2188-2190: `sleep(250ms << attempt)`.
    /// attempt 0 → 250 ms, 1 → 500 ms, 2 → 1000 ms.
    #[inline]
    pub fn backoff_for(attempt: u32) -> Duration {
        Self::BACKOFF_BASE * (1u32 << attempt)
    }
}

impl Default for DefaultRetry {
    fn default() -> Self {
        Self
    }
}

impl RetryPolicy for DefaultRetry {
    #[inline]
    fn max_attempts(&self) -> u32 {
        Self::MAX_ATTEMPTS
    }

    #[inline]
    fn backoff_base(&self) -> Duration {
        Self::BACKOFF_BASE
    }

    #[inline]
    fn cooldown(&self) -> Duration {
        Self::COOLDOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_matches_dynamic_globe() {
        let r = DefaultRetry;
        assert_eq!(r.max_attempts(), 3); // L2186
        assert_eq!(r.backoff_base(), Duration::from_millis(250)); // L2189
        assert_eq!(r.cooldown(), Duration::from_secs(10)); // L1070
    }

    #[test]
    fn worker_backoff_is_exponential() {
        // L2188-2190: 250ms << attempt
        assert_eq!(DefaultRetry::backoff_for(0), Duration::from_millis(250));
        assert_eq!(DefaultRetry::backoff_for(1), Duration::from_millis(500));
        assert_eq!(DefaultRetry::backoff_for(2), Duration::from_millis(1000));
    }

    #[test]
    fn dual_timescale_is_distinct() {
        // The pipeline cooldown (10 s) must be strictly larger than the total
        // worker retry window (250 + 500 + 1000 = 1750 ms). This proves the
        // two timescales are NOT collapsed into one.
        let r = DefaultRetry;
        let worker_window: Duration = (0..r.max_attempts())
            .map(DefaultRetry::backoff_for)
            .sum();
        assert!(
            r.cooldown() > worker_window,
            "cooldown ({:?}) must exceed worker retry window ({:?})",
            r.cooldown(),
            worker_window
        );
    }

    #[test]
    fn retry_policy_is_dyn_compatible() {
        // Q1: the trait must be usable as a trait object once concrete.
        let boxed: Box<dyn RetryPolicy> = Box::new(DefaultRetry);
        assert_eq!(boxed.max_attempts(), 3);
    }
}
