//! Bevy ECS resources for CesiumRust global configuration and state.
//!
//! In "hybrid mode", domain configuration and state serve as Bevy Resources
//! directly for rendering, while IO uses port traits.

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;

/// Render-scale factor: domain works in meters (f64), GPU renders in f32.
/// Earth-scale coordinates (~6.4e6 m) exceed f32 depth/frustum precision,
/// so the adapter scales the world down to a unit sphere for rendering.
/// 1 render unit = 6378137 meters (WGS84 semi-major axis).
pub const METERS_PER_RENDER_UNIT: f64 = 6378137.0;

/// Render scale: 1 render unit = METERS_PER_RENDER_UNIT meters.
#[derive(Resource, Clone)]
pub struct RenderScale(pub f64);

impl Default for RenderScale {
    fn default() -> Self {
        Self(6378137.0)
    }
}

/// Global globe configuration.
#[derive(Resource)]
pub struct GlobeConfig {
    pub ellipsoid: Ellipsoid,
    pub terrain_provider_url: Option<String>,
    pub imagery_providers: Vec<String>,
}

impl Default for GlobeConfig {
    fn default() -> Self {
        Self {
            ellipsoid: Ellipsoid::WGS84,
            terrain_provider_url: None,
            imagery_providers: Vec::new(),
        }
    }
}

/// Statistics about loaded tiles.
#[derive(Resource, Default)]
pub struct TileLoadStats {
    pub tiles_loaded: u32,
    pub tiles_failed: u32,
    pub tiles_pending: u32,
    pub bytes_downloaded: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meters_per_render_unit() {
        assert_eq!(METERS_PER_RENDER_UNIT, 6378137.0);
    }

    #[test]
    fn test_render_scale_default() {
        let scale = RenderScale::default();
        assert_eq!(scale.0, 6378137.0);
    }

    #[test]
    fn test_globe_config_default() {
        let config = GlobeConfig::default();
        assert_eq!(config.ellipsoid, Ellipsoid::WGS84);
        assert_eq!(config.terrain_provider_url, None);
        assert!(config.imagery_providers.is_empty());
    }

    #[test]
    fn test_globe_config_custom() {
        let config = GlobeConfig {
            ellipsoid: Ellipsoid::WGS84,
            terrain_provider_url: Some("https://tiles.example.com".into()),
            imagery_providers: vec!["https://imagery.example.com".into()],
        };
        assert_eq!(
            config.terrain_provider_url.as_deref(),
            Some("https://tiles.example.com")
        );
        assert_eq!(config.imagery_providers.len(), 1);
    }

    #[test]
    fn test_tile_load_stats_default() {
        let stats = TileLoadStats::default();
        assert_eq!(stats.tiles_loaded, 0);
        assert_eq!(stats.tiles_failed, 0);
        assert_eq!(stats.tiles_pending, 0);
        assert_eq!(stats.bytes_downloaded, 0);
    }

    #[test]
    fn test_tile_load_stats_accumulation() {
        let mut stats = TileLoadStats::default();
        stats.tiles_loaded += 42;
        stats.tiles_failed += 3;
        stats.tiles_pending = 5;
        stats.bytes_downloaded += 1_000_000;
        assert_eq!(stats.tiles_loaded, 42);
        assert_eq!(stats.tiles_failed, 3);
        assert_eq!(stats.tiles_pending, 5);
        assert_eq!(stats.bytes_downloaded, 1_000_000);
    }
}
