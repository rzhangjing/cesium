//! 3D Tileset definition and tileset.json parsing.
//!
//! Maps to CesiumJS `Scene/Cesium3DTileset.js`

use crate::tile::Tile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Asset metadata for a tileset.
///
/// Maps to the `asset` property in tileset.json
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TilesetAsset {
    /// The 3D Tiles version (e.g., "1.0" or "1.1").
    pub version: String,

    /// Optional tileset version for cache busting.
    #[serde(default)]
    pub tileset_version: Option<String>,

    /// Optional generator information.
    #[serde(default)]
    pub generator: Option<String>,

    /// Optional copyright information.
    #[serde(default)]
    pub copyright: Option<String>,
}

/// Property statistics for batch table properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyStats {
    /// Minimum value of the property.
    pub minimum: f64,
    /// Maximum value of the property.
    pub maximum: f64,
}

/// The root tileset structure parsed from tileset.json.
///
/// Maps to CesiumJS `Cesium3DTileset`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TilesetJson {
    /// Asset metadata.
    pub asset: TilesetAsset,

    /// The geometric error when the tileset is not rendered at all.
    pub geometric_error: f64,

    /// The root tile of the tileset.
    pub root: Tile,

    /// Optional property statistics.
    #[serde(default)]
    pub properties: Option<HashMap<String, PropertyStats>>,

    /// Optional extensions used by this tileset.
    #[serde(default)]
    pub extensions_used: Option<Vec<String>>,

    /// Optional extensions required by this tileset.
    #[serde(default)]
    pub extensions_required: Option<Vec<String>>,

    /// Optional extras (application-specific data).
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl TilesetJson {
    /// Parses a tileset from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Parses a tileset from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Serializes the tileset to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Returns the total number of tiles in the tileset (including root).
    pub fn tile_count(&self) -> usize {
        1 + self.root.descendant_count()
    }

    /// Returns all content URIs in the tileset.
    pub fn all_content_uris(&self) -> Vec<String> {
        let mut uris = Vec::new();
        collect_content_uris(&self.root, &mut uris);
        uris
    }

    /// Returns the maximum geometric error in the tileset.
    pub fn max_geometric_error(&self) -> f64 {
        self.geometric_error.max(find_max_geometric_error(&self.root))
    }
}

/// Recursively collects all content URIs from a tile tree.
fn collect_content_uris(tile: &Tile, uris: &mut Vec<String>) {
    for uri in tile.content_uris() {
        uris.push(uri.to_string());
    }
    for child in &tile.children {
        collect_content_uris(child, uris);
    }
}

/// Recursively finds the maximum geometric error in a tile tree.
fn find_max_geometric_error(tile: &Tile) -> f64 {
    let mut max_error = tile.geometric_error;
    for child in &tile.children {
        max_error = max_error.max(find_max_geometric_error(child));
    }
    max_error
}

/// Runtime state for a tileset.
#[derive(Debug, Clone)]
pub struct TilesetState {
    /// Maximum screen space error threshold (default: 16).
    pub maximum_screen_space_error: f64,

    /// Maximum memory usage in bytes (default: 512 MB).
    pub maximum_memory_bytes: u64,

    /// Current memory usage in bytes.
    pub current_memory_bytes: u64,

    /// Number of tiles currently selected for rendering.
    pub selected_tiles_count: usize,

    /// Number of tiles currently loading.
    pub loading_tiles_count: usize,

    /// Total number of tiles visited in the last frame.
    pub visited_tiles_count: usize,

    /// The base path for resolving relative URIs.
    pub base_path: String,
}

impl Default for TilesetState {
    fn default() -> Self {
        Self {
            maximum_screen_space_error: 16.0,
            maximum_memory_bytes: 512 * 1024 * 1024,
            current_memory_bytes: 0,
            selected_tiles_count: 0,
            loading_tiles_count: 0,
            visited_tiles_count: 0,
            base_path: String::new(),
        }
    }
}

impl TilesetState {
    /// Creates a new tileset state with the given base path.
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Default::default()
        }
    }

    /// Resolves a relative URI against the base path.
    pub fn resolve_uri(&self, uri: &str) -> String {
        // Absolute URLs or empty base path: return as-is
        if uri.starts_with("http://") || uri.starts_with("https://") || self.base_path.is_empty() {
            return uri.to_string();
        }
        format!("{}/{}", self.base_path.trim_end_matches('/'), uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounding_volume::BoundingVolume;

    fn create_sample_tileset_json() -> &'static str {
        r#"{
            "asset": {
                "version": "1.0",
                "tilesetVersion": "1.2.3"
            },
            "geometricError": 240,
            "root": {
                "boundingVolume": {
                    "region": [-1.32, 0.69, -1.31, 0.70, 0, 88]
                },
                "geometricError": 70,
                "refine": "ADD",
                "content": {
                    "uri": "parent.b3dm"
                },
                "children": [
                    {
                        "boundingVolume": {
                            "region": [-1.32, 0.69, -1.315, 0.695, 0, 20]
                        },
                        "geometricError": 0,
                        "content": {
                            "uri": "ll.b3dm"
                        }
                    },
                    {
                        "boundingVolume": {
                            "sphere": [0, 0, 0, 100]
                        },
                        "geometricError": 0,
                        "content": {
                            "uri": "lr.b3dm"
                        }
                    }
                ]
            }
        }"#
    }

    #[test]
    fn test_parse_tileset_json() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        assert_eq!(tileset.asset.version, "1.0");
        assert_eq!(tileset.asset.tileset_version, Some("1.2.3".to_string()));
        assert_eq!(tileset.geometric_error, 240.0);
    }

    #[test]
    fn test_tile_count() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        // root + 2 children = 3
        assert_eq!(tileset.tile_count(), 3);
    }

    #[test]
    fn test_all_content_uris() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        let uris = tileset.all_content_uris();
        assert_eq!(uris.len(), 3);
        assert!(uris.contains(&"parent.b3dm".to_string()));
        assert!(uris.contains(&"ll.b3dm".to_string()));
        assert!(uris.contains(&"lr.b3dm".to_string()));
    }

    #[test]
    fn test_max_geometric_error() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        assert_eq!(tileset.max_geometric_error(), 240.0);
    }

    #[test]
    fn test_root_tile_properties() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        assert_eq!(tileset.root.geometric_error, 70.0);
        assert_eq!(tileset.root.refine, Some(crate::tile::TileRefine::Add));
        assert!(tileset.root.has_content());
        assert_eq!(tileset.root.children.len(), 2);
    }

    #[test]
    fn test_bounding_volume_parsing() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        // Root has region bounding volume
        assert!(matches!(tileset.root.bounding_volume, BoundingVolume::Region(_)));

        // Second child has sphere bounding volume
        assert!(matches!(
            tileset.root.children[1].bounding_volume,
            BoundingVolume::Sphere(_)
        ));
    }

    #[test]
    fn test_tileset_state_resolve_uri() {
        let state = TilesetState::new("https://example.com/tilesets");

        assert_eq!(
            state.resolve_uri("tile.b3dm"),
            "https://example.com/tilesets/tile.b3dm"
        );
        assert_eq!(
            state.resolve_uri("https://other.com/tile.b3dm"),
            "https://other.com/tile.b3dm"
        );
    }

    #[test]
    fn test_tileset_serde_roundtrip() {
        let json = create_sample_tileset_json();
        let tileset = TilesetJson::from_json(json).unwrap();

        let serialized = tileset.to_json().unwrap();
        let reparsed = TilesetJson::from_json(&serialized).unwrap();

        assert_eq!(tileset.geometric_error, reparsed.geometric_error);
        assert_eq!(tileset.tile_count(), reparsed.tile_count());
    }

    #[test]
    fn test_tileset_with_properties() {
        let json = r#"{
            "asset": { "version": "1.0" },
            "geometricError": 100,
            "properties": {
                "height": { "minimum": 0, "maximum": 100 }
            },
            "root": {
                "boundingVolume": { "sphere": [0, 0, 0, 50] },
                "geometricError": 10
            }
        }"#;

        let tileset = TilesetJson::from_json(json).unwrap();
        assert!(tileset.properties.is_some());
        let props = tileset.properties.unwrap();
        assert!(props.contains_key("height"));
    }

    #[test]
    fn test_tileset_with_extras() {
        let json = r#"{
            "asset": { "version": "1.0" },
            "geometricError": 100,
            "extras": { "name": "Test Tileset", "author": "Test" },
            "root": {
                "boundingVolume": { "sphere": [0, 0, 0, 50] },
                "geometricError": 10
            }
        }"#;

        let tileset = TilesetJson::from_json(json).unwrap();
        assert!(tileset.extras.is_some());
    }
}
