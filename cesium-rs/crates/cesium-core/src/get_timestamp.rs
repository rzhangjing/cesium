//! Ported from packages/engine/Source/Core/getTimestamp.js

use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

/// Gets a timestamp that can be used in measuring the time between events.
/// Timestamps are expressed in milliseconds, but it is not specified what
/// the milliseconds are measured from.
///
/// Port of CesiumJS `getTimestamp()`: the JS version uses
/// `performance.now()` when available and `Date.now()` otherwise; the native
/// port uses a monotonic clock anchored at first use (equivalent to
/// `performance.now()` semantics).
#[must_use]
pub fn get_timestamp() -> f64 {
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_secs_f64() * 1000.0
}
