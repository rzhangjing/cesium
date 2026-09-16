//! Default staleness policy — three-state result classification.
//!
//! Mirrors the dispatch at `dynamic_globe.rs:1052-1098`. The three failure
//! states are **semantically distinct and must never be merged**:
//!
//! - `Aborted` (L1052-1060): tile left the wanted set mid-flight. Only clears
//!   `in_flight` + reupload guard and counts a `stale_skip`. No permanent
//!   state change — the tile may be re-requested next frame.
//! - `Failed` (L1062-1072): all worker retries exhausted (transient
//!   throttle/timeout). Enters the `retry_after` cooldown (10 s). NOT
//!   permanent no-data.
//! - `Placeholder` (L1074-1098): no usable imagery (e.g. Bing gradient JPEG).
//!   Stamps **permanent** no-data and inherits ancestor coverage via UV
//!   upsample. Never retried.
//!
//! A fourth `Fresh` verdict represents a successful download with payload.

use std::time::Duration;

use cesium_ports_driven::{StalenessPolicy, StalenessVerdict};

/// Default staleness policy matching `dynamic_globe.rs:1052-1098`.
#[derive(Debug, Clone, Copy)]
pub struct DefaultStaleness;

impl DefaultStaleness {
    /// `dynamic_globe.rs:1070` — retry cooldown after a `Failed` verdict.
    pub const RETRY_COOLDOWN: Duration = Duration::from_secs(10);
}

impl Default for DefaultStaleness {
    fn default() -> Self {
        Self
    }
}

impl StalenessPolicy for DefaultStaleness {
    /// Classify a download result into one of the four verdicts.
    ///
    /// Precedence matches `dynamic_globe.rs` dispatch order (L1052 → L1098):
    /// `aborted` is checked first (the wanted-set gate at L2161 fires before
    /// any decode), then `failed` (retries exhausted), then `placeholder`
    /// (decode returned no usable imagery). If none apply the tile is `Fresh`.
    fn classify(&self, aborted: bool, failed: bool, placeholder: bool) -> StalenessVerdict {
        if aborted {
            StalenessVerdict::Aborted
        } else if failed {
            StalenessVerdict::Failed
        } else if placeholder {
            StalenessVerdict::Placeholder
        } else {
            StalenessVerdict::Fresh
        }
    }

    #[inline]
    fn retry_cooldown(&self) -> Duration {
        Self::RETRY_COOLDOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_states_are_not_merged() {
        let s = DefaultStaleness;
        // Each single-flag input maps to a distinct verdict.
        assert_eq!(s.classify(true, false, false), StalenessVerdict::Aborted);
        assert_eq!(s.classify(false, true, false), StalenessVerdict::Failed);
        assert_eq!(s.classify(false, false, true), StalenessVerdict::Placeholder);
        assert_eq!(s.classify(false, false, false), StalenessVerdict::Fresh);

        // All four verdicts are pairwise distinct.
        let verdicts = [
            s.classify(true, false, false),
            s.classify(false, true, false),
            s.classify(false, false, true),
            s.classify(false, false, false),
        ];
        for (i, a) in verdicts.iter().enumerate() {
            for (j, b) in verdicts.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "verdicts {i} and {j} must differ");
                }
            }
        }
    }

    #[test]
    fn aborted_takes_precedence() {
        // The wanted-set gate (L2161) fires before decode, so `aborted` wins
        // even if other flags are coincidentally set.
        let s = DefaultStaleness;
        assert_eq!(s.classify(true, true, true), StalenessVerdict::Aborted);
        assert_eq!(s.classify(false, true, true), StalenessVerdict::Failed);
    }

    #[test]
    fn retry_cooldown_is_10s() {
        // L1070: `retry_after.insert(key, now + 10s)`
        let s = DefaultStaleness;
        assert_eq!(s.retry_cooldown(), Duration::from_secs(10));
    }

    #[test]
    fn staleness_policy_is_dyn_compatible() {
        let boxed: Box<dyn StalenessPolicy> = Box::new(DefaultStaleness);
        assert_eq!(boxed.classify(false, false, true), StalenessVerdict::Placeholder);
    }
}
