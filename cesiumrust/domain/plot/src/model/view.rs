//! View descriptor the pure visibility evaluator reads.
//!
//! The core owns the canonical [`ViewMode`] (the bridge aliases it, so there is
//! one definition across the workspace) and a [`ViewContext`] carrying just the
//! view metrics the multi-dimensional visibility rules need. Keeping it here
//! means `eval_visibility` is a pure function of `(Document, ViewContext,
//! Filters)` with no engine types — deterministically unit-testable.

use serde::{Deserialize, Serialize};

/// Which projection the overlay is being evaluated against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ViewMode {
    /// 3D globe (WGS84 / ECEF).
    #[default]
    Globe,
    /// 2D flat map (equirectangular).
    Flat,
}

/// How "zoomed in" the current view is, expressed two equivalent ways so each
/// scale dimension of the visibility rules can pick whichever is natural:
///  * [`ViewContext::pixels_per_world`] drives the 2D `min/max_zoom_px` band
///    (a bigger value == closer in);
///  * [`ViewContext::meters_per_pixel`] drives the metres band (bigger ==
///    further out). The bridge fills at least one; the other may be `0.0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewContext {
    /// Active projection mode.
    pub mode: ViewMode,
    /// Pixels per world unit (2D scale metric). `0.0` when unknown.
    pub pixels_per_world: f64,
    /// Ground metres covered by one screen pixel (3D scale metric). `0.0` when
    /// unknown; smaller == closer to the surface.
    pub meters_per_pixel: f64,
    /// Screen width in logical pixels (unused by M1 rules; carried for later).
    pub screen_w: f64,
    /// Screen height in logical pixels.
    pub screen_h: f64,
    /// Current animation time in seconds since epoch. `0.0` until the time
    /// dimension is wired (M9); the time window rule ignores it while the
    /// window is unset.
    pub time_s: f64,
}

impl Default for ViewContext {
    fn default() -> Self {
        Self {
            mode: ViewMode::default(),
            pixels_per_world: 0.0,
            meters_per_pixel: 0.0,
            screen_w: 0.0,
            screen_h: 0.0,
            time_s: 0.0,
        }
    }
}
