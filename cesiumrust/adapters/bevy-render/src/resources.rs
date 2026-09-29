//! 用于 CesiumRust 全局配置与状态的 Bevy ECS 资源。
//!
//! 在“混合模式”下，领域配置与状态直接作为 Bevy Resources
//! 供渲染使用，而 IO 则使用 port trait。

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;

/// 渲染缩放因子：领域以米（f64）工作，GPU 以 f32 渲染。
/// 地球尺度的坐标（~6.4e6 m）超出 f32 的深度/视锥精度，
/// 因此适配器将世界缩小为一个单位球体以供渲染。
/// 1 render unit = 6378137 米（WGS84 半长轴）。
pub const METERS_PER_RENDER_UNIT: f64 = 6378137.0;

/// 渲染缩放：1 render unit = METERS_PER_RENDER_UNIT 米。
#[derive(Resource, Clone)]
pub struct RenderScale(pub f64);

impl Default for RenderScale {
    fn default() -> Self {
        Self(6378137.0)
    }
}

/// 全局地球配置。
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

/// 关于已加载瓦片的统计信息。
#[derive(Resource, Default)]
pub struct TileLoadStats {
    pub tiles_loaded: u32,
    pub tiles_failed: u32,
    pub tiles_pending: u32,
    pub bytes_downloaded: u64,
    /// 在 LOD/culling 选择期被跳过（从未请求）的瓦片数。
    /// 附加字段：通过 `#[derive(Default)]` 默认为 `0`，因此所有现有
    /// 构造点（`TileLoadStats::default()`、`init_resource`）无需
    /// 修改仍保持有效。
    pub tiles_skipped: u32,
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
        assert_eq!(stats.tiles_skipped, 0);
    }

    #[test]
    fn test_tile_load_stats_accumulation() {
        let mut stats = TileLoadStats::default();
        stats.tiles_loaded += 42;
        stats.tiles_failed += 3;
        stats.tiles_pending = 5;
        stats.bytes_downloaded += 1_000_000;
        stats.tiles_skipped += 7;
        assert_eq!(stats.tiles_loaded, 42);
        assert_eq!(stats.tiles_failed, 3);
        assert_eq!(stats.tiles_pending, 5);
        assert_eq!(stats.bytes_downloaded, 1_000_000);
        assert_eq!(stats.tiles_skipped, 7);
    }
}
