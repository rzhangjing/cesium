//! 3D Tileset 定义与 tileset.json 解析。
//!
//! 镜像 CesiumJS `Scene/Cesium3DTileset.js`

use crate::tile::Tile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 瓦片集的资产元数据。
///
/// 映射到 tileset.json 中的 `asset` 属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TilesetAsset {
    /// 3D Tiles 版本（例如 "1.0" 或 "1.1"）。
    pub version: String,

    /// 用于缓存刷新的可选瓦片集版本。
    #[serde(default)]
    pub tileset_version: Option<String>,

    /// 可选的生成器信息。
    #[serde(default)]
    pub generator: Option<String>,

    /// 可选的版权信息。
    #[serde(default)]
    pub copyright: Option<String>,
}

/// batch table 属性的统计信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyStats {
    /// 属性的最小值。
    pub minimum: f64,
    /// 属性的最大值。
    pub maximum: f64,
}

/// 从 tileset.json 解析出的根瓦片集结构。
///
/// 镜像 CesiumJS `Cesium3DTileset`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TilesetJson {
    /// 资产元数据。
    pub asset: TilesetAsset,

    /// 瓦片集完全不渲染时的几何误差。
    pub geometric_error: f64,

    /// 瓦片集的根瓦片。
    pub root: Tile,

    /// 可选的属性统计。
    #[serde(default)]
    pub properties: Option<HashMap<String, PropertyStats>>,

    /// 本瓦片集使用的可选扩展。
    #[serde(default)]
    pub extensions_used: Option<Vec<String>>,

    /// 本瓦片集要求的可选扩展。
    #[serde(default)]
    pub extensions_required: Option<Vec<String>>,

    /// 可选的 extras（应用特定数据）。
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl TilesetJson {
    /// 从 JSON 字符串解析瓦片集。
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 从 JSON 字节解析瓦片集。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// 将瓦片集序列化为 JSON 字符串。
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 返回瓦片集中瓦片的总数（包括根）。
    pub fn tile_count(&self) -> usize {
        1 + self.root.descendant_count()
    }

    /// 返回瓦片集中所有内容 URI。
    pub fn all_content_uris(&self) -> Vec<String> {
        let mut uris = Vec::new();
        collect_content_uris(&self.root, &mut uris);
        uris
    }

    /// 返回瓦片集中最大的几何误差。
    pub fn max_geometric_error(&self) -> f64 {
        self.geometric_error.max(find_max_geometric_error(&self.root))
    }
}

/// 从瓦片树递归收集所有内容 URI。
fn collect_content_uris(tile: &Tile, uris: &mut Vec<String>) {
    for uri in tile.content_uris() {
        uris.push(uri.to_string());
    }
    for child in &tile.children {
        collect_content_uris(child, uris);
    }
}

/// 在瓦片树中递归查找最大的几何误差。
fn find_max_geometric_error(tile: &Tile) -> f64 {
    let mut max_error = tile.geometric_error;
    for child in &tile.children {
        max_error = max_error.max(find_max_geometric_error(child));
    }
    max_error
}

/// 瓦片集的运行时状态。
#[derive(Debug, Clone)]
pub struct TilesetState {
    /// 最大屏幕空间误差阈值（默认：16）。
    pub maximum_screen_space_error: f64,

    /// 最大内存使用量（字节，默认：512 MB）。
    pub maximum_memory_bytes: u64,

    /// 当前内存使用量（字节）。
    pub current_memory_bytes: u64,

    /// 当前被选中渲染的瓦片数。
    pub selected_tiles_count: usize,

    /// 当前正在加载的瓦片数。
    pub loading_tiles_count: usize,

    /// 上一帧访问的瓦片总数。
    pub visited_tiles_count: usize,

    /// 解析相对 URI 时使用的基础路径。
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
    /// 创建一个新的、带给定基础路径的瓦片集状态。
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Default::default()
        }
    }

    /// 基于基础路径解析一个相对 URI。
    pub fn resolve_uri(&self, uri: &str) -> String {
        // 绝对 URL 或空基础路径：原样返回
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

        // 根 + 2 个子瓦片 = 3
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

        // 根拥有 region 包围体
        assert!(matches!(tileset.root.bounding_volume, BoundingVolume::Region(_)));

        // 第二个子瓦片拥有 sphere 包围体
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
