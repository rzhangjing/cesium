//! 面向 CesiumRust 领域类型的 Bevy ECS 组件。
//!
//! 在“混合模式”下，这些领域组件结构体被直接用作渲染的
//! Bevy Component，而 IO 则使用 port trait。

use bevy::prelude::*;

/// 主地球实体的标记组件（所有地球子实体的根）。
#[derive(Component)]
pub struct CesiumGlobe;

/// 地形瓦片实体的组件。
#[derive(Component)]
pub struct CesiumTerrainTile {
    pub x: u32,
    pub y: u32,
    pub level: u32,
}

/// 3D Tiles 瓦片集根实体的组件。
#[derive(Component)]
pub struct CesiumTilesetRoot {
    pub url: String,
    pub loading_state: TilesetLoadingState,
}

/// 瓦片集的加载状态。
pub enum TilesetLoadingState {
    NotLoaded,
    Loading,
    Ready,
    Failed(String),
}

/// 单个 3D Tiles 瓦片实体的组件。
#[derive(Component)]
pub struct CesiumTileNode {
    pub path: Vec<usize>,
    pub screen_space_error: f64,
    pub geometric_error: f64,
    pub state: TileContentState,
    pub bounding_sphere_center: Option<glam::DVec3>,
    pub bounding_sphere_radius: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileContentState {
    Unloaded,
    Loading,
    Ready,
    Failed,
    Refined,
}

/// 已加载瓦片内容的组件（mesh + texture）。
#[derive(Component)]
pub struct TileContent {
    pub mesh_handle: Option<Handle<Mesh>>,
    pub material_handle: Option<Handle<StandardMaterial>>,
    pub has_batch_table: bool,
}

/// 影像图层实体的组件（地球的子节点）。
#[derive(Component)]
pub struct CesiumImageryLayer {
    pub layer_index: u32,
    pub opacity: f32,
    pub visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cesium_terrain_tile_defaults() {
        let tile = CesiumTerrainTile {
            x: 0,
            y: 0,
            level: 0,
        };
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
        assert_eq!(tile.level, 0);
    }

    #[test]
    fn test_tileset_loading_states() {
        let root = CesiumTilesetRoot {
            url: "https://example.com/tileset.json".into(),
            loading_state: TilesetLoadingState::NotLoaded,
        };
        assert_eq!(root.url, "https://example.com/tileset.json");
        match root.loading_state {
            TilesetLoadingState::NotLoaded => {}
            _ => panic!("Expected NotLoaded"),
        }
    }

    #[test]
    fn test_cesium_tile_node() {
        let node = CesiumTileNode {
            path: vec![0, 2, 1],
            screen_space_error: 16.0,
            geometric_error: 100.0,
            state: TileContentState::Ready,
            bounding_sphere_center: Some(glam::DVec3::new(1.0, 2.0, 3.0)),
            bounding_sphere_radius: Some(100.0),
        };
        assert_eq!(node.path, vec![0, 2, 1]);
        assert!((node.screen_space_error - 16.0).abs() < 1e-10);
        assert!((node.geometric_error - 100.0).abs() < 1e-10);
        match node.state {
            TileContentState::Ready => {}
            _ => panic!("Expected Ready"),
        }
        assert!(node.bounding_sphere_center.is_some());
        assert!((node.bounding_sphere_radius.unwrap() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_tile_content_states() {
        assert!(matches!(TileContentState::Unloaded, TileContentState::Unloaded));
        assert!(matches!(TileContentState::Refined, TileContentState::Refined));
        assert!(matches!(TileContentState::Failed, TileContentState::Failed));
    }

    #[test]
    fn test_imagery_layer() {
        let layer = CesiumImageryLayer {
            layer_index: 2,
            opacity: 0.75,
            visible: true,
        };
        assert_eq!(layer.layer_index, 2);
        assert!((layer.opacity - 0.75).abs() < 1e-6);
        assert!(layer.visible);
    }
}
