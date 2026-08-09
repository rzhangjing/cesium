//! Bevy ECS components for CesiumRust domain types.
//!
//! In "hybrid mode", these domain-component structs are used directly as
//! Bevy Components for rendering, while IO uses port traits.

use bevy::prelude::*;

/// Marker component for the main globe entity (root of all globe children).
#[derive(Component)]
pub struct CesiumGlobe;

/// Component for terrain tile entities.
#[derive(Component)]
pub struct CesiumTerrainTile {
    pub x: u32,
    pub y: u32,
    pub level: u32,
}

/// Component for 3D Tiles tileset root entity.
#[derive(Component)]
pub struct CesiumTilesetRoot {
    pub url: String,
    pub loading_state: TilesetLoadingState,
}

/// Loading states for tilesets.
pub enum TilesetLoadingState {
    NotLoaded,
    Loading,
    Ready,
    Failed(String),
}

/// Component for individual 3D Tiles tile entities.
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

/// Component for loaded tile content (mesh + texture).
#[derive(Component)]
pub struct TileContent {
    pub mesh_handle: Option<Handle<Mesh>>,
    pub material_handle: Option<Handle<StandardMaterial>>,
    pub has_batch_table: bool,
}

/// Component for imagery layer entities (children of globe).
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
