//! 3D Tile node definition.
//!
//! Maps to CesiumJS `Scene/Cesium3DTile.js`

use crate::bounding_volume::BoundingVolume;
use serde::{Deserialize, Serialize};

/// The refinement strategy for a tile.
///
/// Maps to CesiumJS `Scene/Cesium3DTileRefine.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TileRefine {
    /// Child tiles replace the parent when rendered.
    #[default]
    Replace,
    /// Child tiles are added to the parent when rendered.
    Add,
}

/// The loading state of tile content.
///
/// Maps to CesiumJS `Scene/Cesium3DTileContentState.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileContentState {
    /// Content has not been requested.
    #[default]
    Unloaded,
    /// Content request is in progress.
    Loading,
    /// Content is being processed (decoded, GPU upload).
    Processing,
    /// Content is ready for rendering.
    Ready,
    /// Content failed to load.
    Failed,
    /// Content has been explicitly unloaded.
    Expired,
}

impl TileContentState {
    /// Returns true if the content is ready for rendering.
    pub fn is_renderable(&self) -> bool {
        matches!(self, TileContentState::Ready)
    }

    /// Returns true if a request should be made.
    pub fn should_request(&self) -> bool {
        matches!(self, TileContentState::Unloaded | TileContentState::Failed)
    }
}

/// Content reference for a tile.
///
/// Maps to the `content` property in tileset.json
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileContent {
    /// URI to the tile content (glTF, b3dm, pnts, etc.)
    pub uri: String,

    /// Optional bounding volume for the content (tighter than tile bounds).
    #[serde(default)]
    pub bounding_volume: Option<BoundingVolume>,

    /// Optional group ID for multiple contents.
    #[serde(default)]
    pub group: Option<u32>,
}

/// A node in the 3D Tiles tree structure.
///
/// Maps to CesiumJS `Cesium3DTile`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    /// The bounding volume of this tile.
    pub bounding_volume: BoundingVolume,

    /// The geometric error (in meters) introduced if this tile is rendered
    /// and its children are not.
    pub geometric_error: f64,

    /// The refinement strategy (ADD or REPLACE).
    #[serde(default)]
    pub refine: Option<TileRefine>,

    /// Optional 4x4 transform matrix (column-major, 16 elements).
    #[serde(default)]
    pub transform: Option<[f64; 16]>,

    /// The content of this tile (if it has renderable content).
    #[serde(default)]
    pub content: Option<TileContent>,

    /// Multiple contents (for composite tiles).
    #[serde(default)]
    pub contents: Option<Vec<TileContent>>,

    /// Child tiles.
    #[serde(default)]
    pub children: Vec<Tile>,

    /// Optional viewer request volume for prefetching.
    #[serde(default)]
    pub viewer_request_volume: Option<BoundingVolume>,

    /// Optional extras (application-specific data).
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl Tile {
    /// Returns the effective refine mode, inheriting from parent if not specified.
    pub fn effective_refine(&self, parent_refine: TileRefine) -> TileRefine {
        self.refine.unwrap_or(parent_refine)
    }

    /// Returns true if this tile has renderable content.
    pub fn has_content(&self) -> bool {
        self.content.is_some() || self.contents.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// Returns all content URIs for this tile.
    pub fn content_uris(&self) -> Vec<&str> {
        let mut uris = Vec::new();
        if let Some(ref content) = self.content {
            uris.push(content.uri.as_str());
        }
        if let Some(ref contents) = self.contents {
            for c in contents {
                uris.push(c.uri.as_str());
            }
        }
        uris
    }

    /// Returns the number of descendant tiles (recursive).
    pub fn descendant_count(&self) -> usize {
        let mut count = self.children.len();
        for child in &self.children {
            count += child.descendant_count();
        }
        count
    }

    /// Gets the transform matrix as a glam DMat4, or identity if not specified.
    pub fn transform_matrix(&self) -> glam::DMat4 {
        match self.transform {
            Some(data) => glam::DMat4::from_cols_array(&data),
            None => glam::DMat4::IDENTITY,
        }
    }
}

/// Runtime state for a tile during traversal.
///
/// This is separate from the serializable Tile struct to keep
/// the domain model pure and the runtime state mutable.
#[derive(Debug, Clone, Default)]
pub struct TileRuntimeState {
    /// Current content loading state.
    pub content_state: TileContentState,

    /// Distance from the camera to this tile.
    pub distance_to_camera: f64,

    /// Screen space error for this tile.
    pub screen_space_error: f64,

    /// Whether this tile is visible in the current frame.
    pub visible: bool,

    /// Whether this tile was selected for rendering.
    pub selected: bool,

    /// The depth of this tile in the tree.
    pub depth: u32,

    /// Frame number when this tile was last visited.
    pub visited_frame: u64,

    /// Frame number when this tile was last selected.
    pub selected_frame: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    fn create_test_tile() -> Tile {
        Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error: 50.0,
            refine: Some(TileRefine::Replace),
            transform: None,
            content: Some(TileContent {
                uri: "test.b3dm".to_string(),
                bounding_volume: None,
                group: None,
            }),
            contents: None,
            children: vec![],
            viewer_request_volume: None,
            extras: None,
        }
    }

    #[test]
    fn test_tile_has_content() {
        let tile = create_test_tile();
        assert!(tile.has_content());

        let mut empty_tile = create_test_tile();
        empty_tile.content = None;
        assert!(!empty_tile.has_content());
    }

    #[test]
    fn test_tile_content_uris() {
        let tile = create_test_tile();
        let uris = tile.content_uris();
        assert_eq!(uris, vec!["test.b3dm"]);
    }

    #[test]
    fn test_tile_multiple_contents() {
        let mut tile = create_test_tile();
        tile.content = None;
        tile.contents = Some(vec![
            TileContent {
                uri: "a.glb".to_string(),
                bounding_volume: None,
                group: Some(0),
            },
            TileContent {
                uri: "b.glb".to_string(),
                bounding_volume: None,
                group: Some(1),
            },
        ]);

        let uris = tile.content_uris();
        assert_eq!(uris.len(), 2);
        assert!(tile.has_content());
    }

    #[test]
    fn test_effective_refine() {
        let mut tile = create_test_tile();
        tile.refine = None;

        // Should inherit from parent
        assert_eq!(tile.effective_refine(TileRefine::Add), TileRefine::Add);
        assert_eq!(tile.effective_refine(TileRefine::Replace), TileRefine::Replace);

        // Should use own value
        tile.refine = Some(TileRefine::Add);
        assert_eq!(tile.effective_refine(TileRefine::Replace), TileRefine::Add);
    }

    #[test]
    fn test_descendant_count() {
        let mut root = create_test_tile();
        root.children = vec![create_test_tile(), create_test_tile()];
        root.children[0].children = vec![create_test_tile()];

        assert_eq!(root.descendant_count(), 3);
    }

    #[test]
    fn test_transform_matrix_identity() {
        let tile = create_test_tile();
        assert_eq!(tile.transform_matrix(), glam::DMat4::IDENTITY);
    }

    #[test]
    fn test_transform_matrix_custom() {
        let mut tile = create_test_tile();
        let translation = glam::DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
        tile.transform = Some(translation.to_cols_array());

        let matrix = tile.transform_matrix();
        assert_eq!(matrix, translation);
    }

    #[test]
    fn test_content_state() {
        assert!(TileContentState::Ready.is_renderable());
        assert!(!TileContentState::Loading.is_renderable());
        assert!(TileContentState::Unloaded.should_request());
        assert!(TileContentState::Failed.should_request());
        assert!(!TileContentState::Ready.should_request());
    }

    #[test]
    fn test_tile_serde() {
        let tile = create_test_tile();
        let json = serde_json::to_string(&tile).unwrap();
        let parsed: Tile = serde_json::from_str(&json).unwrap();
        assert_eq!(tile.geometric_error, parsed.geometric_error);
    }
}
