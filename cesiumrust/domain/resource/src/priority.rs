//! Pluggable priority function for request scheduling.
//!
//! Maps to CesiumJS `Request.priorityFunction` / `RequestScheduler`'s priority
//! heap ordering and the `priorityFunction` callback pattern used in
//! `Cesium3DTileset`, `QuadtreePrimitive`, and `GlobeSurfaceTileProvider`.
//!
//! In CesiumJS, the priority function is a callback on each `Request` that
//! returns a numeric priority (lower = higher priority). The scheduler uses a
//! min-heap ordered by this value to decide which pending requests to promote.
//!
//! This module provides:
//! - A [`PriorityFunction`] trait for pluggable priority computation.
//! - A [`SsedPriority`] default implementation based on Screen-Space Error
//!   Distance (SSED) — the same metric CesiumJS uses for 3D Tiles and terrain
//!   request prioritization.
//! - A [`DistanceDecayPriority`] alternative that uses simple distance falloff.
//! - A [`CompositePriority`] that combines multiple priority signals.
//!
//! **Pure domain logic** — no IO, no framework dependency, f64 throughout.

use std::fmt::Debug;

/// Trait for computing request priority.
///
/// The scheduler calls this once per frame for each pending/throttled request
/// to determine promotion order. Lower values = higher priority (matching
/// CesiumJS `RequestScheduler` min-heap semantics).
///
/// Maps to CesiumJS `request.priorityFunction`:
/// ```js
/// request.priorityFunction = function() {
///   return tile.priority; // computed from SSE/distance
/// };
/// ```
pub trait PriorityFunction: Send + Sync + Debug {
    /// Computes the priority for a request identified by `key`.
    ///
    /// Lower values indicate higher priority (promoted first from the heap).
    /// The `context` provides frame-state information (camera position, etc.)
    /// needed for screen-space computations.
    ///
    /// Returns `f64::MAX` to deprioritize a request effectively to the bottom
    /// of the heap (e.g. for requests that are no longer relevant).
    fn compute_priority(&self, key: &PriorityKey, context: &FrameContext) -> f64;

    /// Returns the name of this priority function (for diagnostics).
    fn name(&self) -> &str;
}

/// Identifies a request for priority computation.
///
/// Generic enough to cover terrain tiles, imagery tiles, and 3D Tiles content.
#[derive(Debug, Clone, PartialEq)]
pub struct PriorityKey {
    /// Tile coordinates (x, y) at the given zoom level.
    pub x: u32,
    pub y: u32,
    /// Zoom level (0 = root).
    pub zoom: u32,

    /// Bounding volume center in world coordinates (ECEF meters, f64).
    /// Used for distance-based priority computations.
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,

    /// Geometric error of this tile in meters (used for SSE computation).
    /// Maps to CesiumJS `tile._geometricError` / `tileset._maximumScreenSpaceError`.
    pub geometric_error: f64,

    /// Additional user-defined weight multiplier (default 1.0).
    /// Allows hosts to bias certain tiles (e.g. center-of-view boost).
    pub weight: f64,
}

impl PriorityKey {
    /// Creates a minimal priority key with just tile coordinates.
    pub fn new(x: u32, y: u32, zoom: u32) -> Self {
        Self {
            x,
            y,
            zoom,
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            geometric_error: 0.0,
            weight: 1.0,
        }
    }

    /// Sets the bounding volume center (ECEF meters).
    pub fn with_center(mut self, x: f64, y: f64, z: f64) -> Self {
        self.center_x = x;
        self.center_y = y;
        self.center_z = z;
        self
    }

    /// Sets the geometric error in meters.
    pub fn with_geometric_error(mut self, error: f64) -> Self {
        self.geometric_error = error;
        self
    }

    /// Sets the weight multiplier.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Per-frame context for priority computations.
///
/// Contains camera state and viewport dimensions needed for screen-space
/// error calculations. Updated once per frame by the host (Bevy system).
#[derive(Debug, Clone)]
pub struct FrameContext {
    /// Camera position in world coordinates (ECEF meters, f64).
    pub camera_x: f64,
    pub camera_y: f64,
    pub camera_z: f64,

    /// Viewport width in pixels.
    pub viewport_width: f64,

    /// Viewport height in pixels.
    pub viewport_height: f64,

    /// Vertical field of view in radians.
    pub fov_y: f64,

    /// Maximum screen-space error threshold (pixels).
    /// Tiles with SSE above this are refined; below this they're sufficient.
    ///
    /// Maps to CesiumJS `Cesium3DTileset.maximumScreenSpaceError` (default 16).
    pub maximum_screen_space_error: f64,

    /// Current frame index (monotonic, for staleness heuristics).
    pub frame_index: u64,
}

impl FrameContext {
    /// Creates a default frame context with typical values.
    pub fn new() -> Self {
        Self {
            camera_x: 0.0,
            camera_y: 0.0,
            camera_z: 6_378_137.0, // Default: above equator at 1 Earth radius
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            fov_y: std::f64::consts::FRAC_PI_3, // 60 degrees
            maximum_screen_space_error: 16.0,
            frame_index: 0,
        }
    }

    /// Sets the camera position.
    pub fn with_camera(mut self, x: f64, y: f64, z: f64) -> Self {
        self.camera_x = x;
        self.camera_y = y;
        self.camera_z = z;
        self
    }

    /// Sets the viewport dimensions.
    pub fn with_viewport(mut self, width: f64, height: f64) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    /// Sets the field of view.
    pub fn with_fov(mut self, fov_y: f64) -> Self {
        self.fov_y = fov_y;
        self
    }

    /// Sets the maximum screen-space error.
    pub fn with_max_sse(mut self, sse: f64) -> Self {
        self.maximum_screen_space_error = sse;
        self
    }

    /// Computes the distance from the camera to a point.
    pub fn distance_to(&self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.camera_x - x;
        let dy = self.camera_y - y;
        let dz = self.camera_z - z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Computes the screen-space error for a tile at the given distance.
    ///
    /// Maps to CesiumJS `Cesium3DTileset.prototype._computeScreenSpaceError`:
    /// ```js
    /// sse = (geometricError * screenHeight) / (distance * 2 * tan(fovY / 2))
    /// ```
    ///
    /// This is the simplified form assuming perspective projection where the
    /// tile's geometric error is a length in world space.
    pub fn screen_space_error(&self, geometric_error: f64, distance: f64) -> f64 {
        if distance <= 0.0 || geometric_error <= 0.0 {
            return f64::MAX;
        }
        let sse_denom = 2.0 * (self.fov_y / 2.0).tan();
        if sse_denom <= 0.0 {
            return f64::MAX;
        }
        (geometric_error * self.viewport_height) / (distance * sse_denom)
    }
}

impl Default for FrameContext {
    fn default() -> Self {
        Self::new()
    }
}

// ── SSED Priority (Screen-Space Error Distance) ─────────────────────────────

/// Default priority function based on Screen-Space Error Distance (SSED).
///
/// This is the same metric CesiumJS uses for 3D Tiles and terrain tile
/// prioritization: tiles with higher screen-space error (i.e. more visually
/// impactful refinement) get higher priority (lower numeric value).
///
/// The priority formula:
/// ```text
/// priority = max(0, maximumSSE - computedSSE) * weight
/// ```
///
/// - If `computedSSE >= maximumSSE`: priority = 0 (highest — tile MUST refine)
/// - If `computedSSE < maximumSSE`: priority > 0 (tile is already sufficient,
///   but we still load it for future camera movements; lower SSE = lower
///   priority)
///
/// Maps to CesiumJS `QuadtreePrimitive._prioritizeTiles` and
/// `Cesium3DTileset._processScreenSpaceError`.
#[derive(Debug, Clone)]
pub struct SsedPriority {
    /// Multiplier applied to the computed priority (default 1.0).
    pub scale: f64,
}

impl SsedPriority {
    /// Creates a new SSED priority function with default scale.
    pub fn new() -> Self {
        Self { scale: 1.0 }
    }

    /// Creates with a custom scale factor.
    pub fn with_scale(scale: f64) -> Self {
        Self { scale }
    }
}

impl Default for SsedPriority {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityFunction for SsedPriority {
    fn compute_priority(&self, key: &PriorityKey, context: &FrameContext) -> f64 {
        // Compute distance from camera to tile center.
        let distance = context.distance_to(key.center_x, key.center_y, key.center_z);

        // Compute screen-space error at this distance.
        let sse = context.screen_space_error(key.geometric_error, distance);

        // Priority: how much the SSE exceeds the threshold.
        // Higher excess → lower priority value → promoted first.
        let excess = context.maximum_screen_space_error - sse;
        let priority = if excess <= 0.0 {
            // SSE exceeds threshold: must refine, highest priority.
            0.0
        } else {
            // SSE below threshold: priority proportional to how far below.
            excess
        };

        priority * self.scale * key.weight
    }

    fn name(&self) -> &str {
        "SSED"
    }
}

// ── Distance Decay Priority ─────────────────────────────────────────────────

/// Simple distance-based priority: closer tiles get higher priority.
///
/// Formula: `priority = distance / reference_distance * weight`
///
/// Useful for imagery layers where geometric error is not meaningful (all
/// tiles at a given zoom have the same error) but proximity to the camera
/// determines visual importance.
///
/// Maps to the simpler priority heuristic used in some CesiumJS imagery
/// providers (e.g. `ImageryLayer._createImagerySSEPriorityFunction`).
#[derive(Debug, Clone)]
pub struct DistanceDecayPriority {
    /// Reference distance for normalization (priority = 1.0 at this distance).
    /// Default: 10_000_000.0 meters (~1.5 Earth radii).
    pub reference_distance: f64,
}

impl DistanceDecayPriority {
    /// Creates with default reference distance.
    pub fn new() -> Self {
        Self {
            reference_distance: 10_000_000.0,
        }
    }

    /// Creates with a custom reference distance.
    pub fn with_reference(reference_distance: f64) -> Self {
        Self { reference_distance }
    }
}

impl Default for DistanceDecayPriority {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityFunction for DistanceDecayPriority {
    fn compute_priority(&self, key: &PriorityKey, context: &FrameContext) -> f64 {
        let distance = context.distance_to(key.center_x, key.center_y, key.center_z);
        let normalized = if self.reference_distance > 0.0 {
            distance / self.reference_distance
        } else {
            f64::MAX
        };
        normalized * key.weight
    }

    fn name(&self) -> &str {
        "DistanceDecay"
    }
}

// ── Composite Priority ──────────────────────────────────────────────────────

/// Combines multiple priority functions with weighted blending.
///
/// The final priority is the weighted sum of all constituent priorities:
/// ```text
/// priority = Σ (weight_i * function_i.compute_priority(key, context))
/// ```
///
/// This allows hosts to combine SSED (for visual refinement urgency) with
/// distance decay (for spatial locality) and custom heuristics.
#[derive(Debug)]
pub struct CompositePriority {
    components: Vec<(f64, Box<dyn PriorityFunction>)>,
}

impl CompositePriority {
    /// Creates an empty composite.
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Adds a priority function with the given weight.
    pub fn add(mut self, weight: f64, function: Box<dyn PriorityFunction>) -> Self {
        self.components.push((weight, function));
        self
    }

    /// Returns the number of component functions.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Returns whether the composite has no components.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

impl Default for CompositePriority {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityFunction for CompositePriority {
    fn compute_priority(&self, key: &PriorityKey, context: &FrameContext) -> f64 {
        if self.components.is_empty() {
            return 0.0;
        }
        self.components
            .iter()
            .map(|(weight, func)| weight * func.compute_priority(key, context))
            .sum()
    }

    fn name(&self) -> &str {
        "Composite"
    }
}

// ── Static Priority (for testing / trivial cases) ───────────────────────────

/// A priority function that always returns a fixed value.
///
/// Useful for testing and for requests that should all have equal priority
/// (FIFO ordering within the heap).
#[derive(Debug, Clone)]
pub struct StaticPriority {
    value: f64,
}

impl StaticPriority {
    /// Creates a static priority with the given fixed value.
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

impl PriorityFunction for StaticPriority {
    fn compute_priority(&self, _key: &PriorityKey, _context: &FrameContext) -> f64 {
        self.value
    }

    fn name(&self) -> &str {
        "Static"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> FrameContext {
        FrameContext::new()
            .with_camera(0.0, 0.0, 6_378_137.0)
            .with_viewport(1920.0, 1080.0)
            .with_fov(std::f64::consts::FRAC_PI_3)
    }

    #[test]
    fn ssed_priority_closer_tile_has_higher_priority() {
        let ctx = make_context();
        let ssed = SsedPriority::new();

        // Tile close to camera (small distance → large SSE → low priority value)
        let close = PriorityKey::new(0, 0, 15)
            .with_center(0.0, 0.0, 6_378_137.0 - 1000.0) // 1km below camera
            .with_geometric_error(10.0);

        // Tile far from camera
        let far = PriorityKey::new(0, 0, 5)
            .with_center(0.0, 0.0, 0.0) // center of Earth
            .with_geometric_error(10.0);

        let p_close = ssed.compute_priority(&close, &ctx);
        let p_far = ssed.compute_priority(&far, &ctx);

        // Closer tile should have lower priority value (higher priority)
        assert!(
            p_close < p_far,
            "close priority {} should be < far priority {}",
            p_close,
            p_far
        );
    }

    #[test]
    fn ssed_priority_zero_when_sse_exceeds_threshold() {
        let ctx = make_context().with_max_sse(16.0);
        let ssed = SsedPriority::new();

        // Very close tile with large geometric error → SSE >> threshold
        let key = PriorityKey::new(0, 0, 20)
            .with_center(0.0, 0.0, 6_378_137.0 - 100.0) // 100m below camera
            .with_geometric_error(1000.0);

        let priority = ssed.compute_priority(&key, &ctx);
        assert_eq!(priority, 0.0, "SSE exceeds threshold → priority must be 0");
    }

    #[test]
    fn distance_decay_closer_is_lower() {
        let ctx = make_context();
        let dd = DistanceDecayPriority::new();

        let close = PriorityKey::new(0, 0, 10)
            .with_center(0.0, 0.0, 6_378_137.0 - 100_000.0);
        let far = PriorityKey::new(0, 0, 5)
            .with_center(0.0, 0.0, 0.0);

        let p_close = dd.compute_priority(&close, &ctx);
        let p_far = dd.compute_priority(&far, &ctx);
        assert!(p_close < p_far);
    }

    #[test]
    fn distance_decay_reference_normalization() {
        let ctx = make_context();
        let dd = DistanceDecayPriority::with_reference(1_000_000.0);

        // Tile at exactly 1M meters from camera
        let key = PriorityKey::new(0, 0, 10)
            .with_center(0.0, 0.0, 6_378_137.0 - 1_000_000.0);

        let priority = dd.compute_priority(&key, &ctx);
        // Should be approximately 1.0 (distance / reference_distance)
        assert!(
            (priority - 1.0).abs() < 0.001,
            "expected ~1.0, got {}",
            priority
        );
    }

    #[test]
    fn composite_blends_weights() {
        let ctx = make_context();
        let composite = CompositePriority::new()
            .add(2.0, Box::new(StaticPriority::new(3.0)))
            .add(1.0, Box::new(StaticPriority::new(5.0)));

        let key = PriorityKey::new(0, 0, 0);
        let priority = composite.compute_priority(&key, &ctx);
        // 2.0 * 3.0 + 1.0 * 5.0 = 11.0
        assert!((priority - 11.0).abs() < f64::EPSILON);
    }

    #[test]
    fn static_priority_always_same() {
        let ctx = make_context();
        let sp = StaticPriority::new(42.0);
        let key = PriorityKey::new(1, 2, 3).with_center(100.0, 200.0, 300.0);
        assert_eq!(sp.compute_priority(&key, &ctx), 42.0);
    }

    #[test]
    fn weight_multiplier_affects_priority() {
        let ctx = make_context();
        let dd = DistanceDecayPriority::new();

        let normal = PriorityKey::new(0, 0, 10)
            .with_center(0.0, 0.0, 6_000_000.0)
            .with_weight(1.0);
        let boosted = PriorityKey::new(0, 0, 10)
            .with_center(0.0, 0.0, 6_000_000.0)
            .with_weight(0.5); // Lower weight → lower priority value → higher priority

        let p_normal = dd.compute_priority(&normal, &ctx);
        let p_boosted = dd.compute_priority(&boosted, &ctx);
        assert!(p_boosted < p_normal);
    }

    #[test]
    fn screen_space_error_computation() {
        let ctx = FrameContext::new()
            .with_viewport(1080.0, 1080.0)
            .with_fov(std::f64::consts::FRAC_PI_2); // 90 degrees

        // At distance 1000, geometric error 10:
        // sse = (10 * 1080) / (1000 * 2 * tan(45°)) = 10800 / 2000 = 5.4
        let sse = ctx.screen_space_error(10.0, 1000.0);
        assert!((sse - 5.4).abs() < 0.01, "expected ~5.4, got {}", sse);
    }

    #[test]
    fn screen_space_error_zero_distance_is_max() {
        let ctx = make_context();
        assert_eq!(ctx.screen_space_error(10.0, 0.0), f64::MAX);
    }

    #[test]
    fn frame_context_distance_to() {
        let ctx = FrameContext::new().with_camera(0.0, 0.0, 100.0);
        let dist = ctx.distance_to(0.0, 0.0, 0.0);
        assert!((dist - 100.0).abs() < f64::EPSILON);

        let dist2 = ctx.distance_to(3.0, 4.0, 100.0);
        assert!((dist2 - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn priority_function_names() {
        assert_eq!(SsedPriority::new().name(), "SSED");
        assert_eq!(DistanceDecayPriority::new().name(), "DistanceDecay");
        assert_eq!(StaticPriority::new(0.0).name(), "Static");
        assert_eq!(CompositePriority::new().name(), "Composite");
    }
}
