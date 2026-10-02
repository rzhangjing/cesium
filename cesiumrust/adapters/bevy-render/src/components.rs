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
    /// 瓦片列号 x。
    pub x: u32,
    /// 瓦片行号 y。
    pub y: u32,
    /// 瓦片层级（细节级别）。
    pub level: u32,
}

/// 3D Tiles 瓦片集根实体的组件。
#[derive(Component)]
pub struct CesiumTilesetRoot {
    /// tileset.json 的 URL。
    pub url: String,
    /// 当前加载状态。
    pub loading_state: TilesetLoadingState,
}

/// 瓦片集的加载状态。
pub enum TilesetLoadingState {
    /// 尚未开始加载。
    NotLoaded,
    /// 正在加载。
    Loading,
    /// 已就绪可渲染。
    Ready,
    /// 加载失败（附错误信息）。
    Failed(String),
}

/// 单个 3D Tiles 瓦片实体的组件。
#[derive(Component)]
pub struct CesiumTileNode {
    /// 从根到本瓦片的子索引路径。
    pub path: Vec<usize>,
    /// 当前屏幕空间误差（驱动 LOD 选择）。
    pub screen_space_error: f64,
    /// 几何误差（瓦片精度）。
    pub geometric_error: f64,
    /// 内容加载状态。
    pub state: TileContentState,
    /// 包围球中心（世界坐标）。
    pub bounding_sphere_center: Option<glam::DVec3>,
    /// 包围球半径。
    pub bounding_sphere_radius: Option<f64>,
}

/// 瓦片内容的加载生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileContentState {
    /// 未加载。
    Unloaded,
    /// 加载中。
    Loading,
    /// 已就绪。
    Ready,
    /// 加载失败。
    Failed,
    /// 已被子瓦片细化替代。
    Refined,
}

/// 已加载瓦片内容的组件（mesh + texture）。
#[derive(Component)]
pub struct TileContent {
    /// 网格句柄（未就绪时为 None）。
    pub mesh_handle: Option<Handle<Mesh>>,
    /// 材质句柄（未就绪时为 None）。
    pub material_handle: Option<Handle<StandardMaterial>>,
    /// 是否含批量表（batch table，用于样式/拾取）。
    pub has_batch_table: bool,
}

/// 影像图层实体的组件（地球的子节点）。
#[derive(Component)]
pub struct CesiumImageryLayer {
    /// 图层序号（叠放顺序）。
    pub layer_index: u32,
    /// 不透明度 [0,1]。
    pub opacity: f32,
    /// 是否可见。
    pub visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 地形瓦片组件应能按字段构造并读回。
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
    /// 根组件应保存 URL 并默认处于 NotLoaded 状态。
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
    /// 瓦片节点组件应保存路径/误差/包围球等字段。
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
    /// 内容状态枚举变体应可匹配区分。
    fn test_tile_content_states() {
        assert!(matches!(TileContentState::Unloaded, TileContentState::Unloaded));
        assert!(matches!(TileContentState::Refined, TileContentState::Refined));
        assert!(matches!(TileContentState::Failed, TileContentState::Failed));
    }

    #[test]
    /// 影像图层组件应保存序号/不透明度/可见性。
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
