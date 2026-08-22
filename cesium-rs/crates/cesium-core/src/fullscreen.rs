//! Ported from `packages/engine/Source/Core/Fullscreen.js`.
//!
//! Browser-independent fullscreen API. Skeleton for Rust (no DOM).

/// Utilities for working with the fullscreen API.
/// In Rust, fullscreen is managed by the windowing system, not the browser.
pub struct Fullscreen;

impl Fullscreen {
    /// Returns whether fullscreen is supported.
    pub fn supports_fullscreen() -> bool {
        false
    }

    /// Returns whether the application is currently in fullscreen mode.
    pub fn fullscreen() -> bool {
        false
    }
}
