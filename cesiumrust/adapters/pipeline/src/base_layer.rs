//! Base-layer zoom exemption logic.
//!
//! Mirrors `dynamic_globe.rs:1483` — tiles at `z <= BASE_LAYER_ZOOM` form the
//! permanent global fallback layer (CesiumJS's base imagery layer role).
//! They are downloaded once at startup and never despawned or evicted, so
//! fast pans into never-visited regions show blurry imagery instead of the
//! black base sphere while fine tiles load.

/// Determines whether a tile key belongs to the permanently-resident base layer.
///
/// The zoom component is extracted via the provided closure, keeping this
/// generic over key types (e.g. `(u32, u32, u32)` where `.2` is zoom).
///
/// Corresponds to `dynamic_globe.rs:1483`:
/// ```text
/// if old.2 <= BASE_LAYER_ZOOM { continue; }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BaseLayerGuard {
    /// Maximum zoom level that is permanently exempt from eviction.
    /// Default: 3 (`dynamic_globe.rs:70`).
    pub max_zoom: u32,
}

impl BaseLayerGuard {
    /// Create a guard with the default base layer zoom (3).
    pub fn new() -> Self {
        Self { max_zoom: 3 }
    }

    /// Create a guard with a custom base layer zoom.
    pub fn with_zoom(max_zoom: u32) -> Self {
        Self { max_zoom }
    }

    /// Returns true if the given zoom level is within the base layer
    /// (permanently exempt from eviction/despawn).
    #[inline]
    pub fn is_base_layer(&self, zoom: u32) -> bool {
        zoom <= self.max_zoom
    }
}

impl Default for BaseLayerGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_layer_exempt_at_zoom_3_and_below() {
        let guard = BaseLayerGuard::new();
        assert!(guard.is_base_layer(0));
        assert!(guard.is_base_layer(1));
        assert!(guard.is_base_layer(2));
        assert!(guard.is_base_layer(3));
    }

    #[test]
    fn not_base_layer_above_zoom_3() {
        let guard = BaseLayerGuard::new();
        assert!(!guard.is_base_layer(4));
        assert!(!guard.is_base_layer(19));
    }

    #[test]
    fn custom_zoom_level() {
        let guard = BaseLayerGuard::with_zoom(5);
        assert!(guard.is_base_layer(5));
        assert!(!guard.is_base_layer(6));
    }
}
