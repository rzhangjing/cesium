//! 3D Tile 节点定义。
//!
//! 对应 `Scene/Cesium3DTile`

use crate::bounding_volume::BoundingVolume;
use serde::{Deserialize, Serialize};

/// 瓦片的 refinement 策略。
///
/// 对应 `Scene/Cesium3DTileRefine`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TileRefine {
    /// 渲染时子瓦片替换父瓦片。
    #[default]
    Replace,
    /// 渲染时子瓦片叠加到父瓦片上。
    Add,
}

/// 瓦片内容的加载状态。
///
/// 对应 `Scene/Cesium3DTileContentState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileContentState {
    /// 尚未请求内容。
    #[default]
    Unloaded,
    /// 内容请求正在进行。
    Loading,
    /// 内容正在处理（解码、GPU 上传）。
    Processing,
    /// 内容已就绪可渲染。
    Ready,
    /// 内容加载失败。
    Failed,
    /// 内容已被显式卸载。
    Expired,
}

impl TileContentState {
    /// 若内容已就绪可渲染则返回 true。
    pub fn is_renderable(&self) -> bool {
        matches!(self, TileContentState::Ready)
    }

    /// 若应发起请求则返回 true。
    pub fn should_request(&self) -> bool {
        matches!(self, TileContentState::Unloaded | TileContentState::Failed)
    }
}

/// 瓦片的内容引用。
///
/// 映射到 tileset.json 中的 `content` 属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileContent {
    /// 瓦片内容的 URI（glTF、b3dm、pnts 等）
    pub uri: String,

    /// 可选的内容包围体（比瓦片边界更紧凑）。
    #[serde(default)]
    pub bounding_volume: Option<BoundingVolume>,

    /// 用于多内容的可选 group ID。
    #[serde(default)]
    pub group: Option<u32>,
}

/// 3D Tiles 树结构中的一个节点。
///
/// 对应 `Cesium3DTile`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    /// 本瓦片的包围体。
    pub bounding_volume: BoundingVolume,

    /// 几何误差（以米计）：若渲染本瓦片而其子瓦片未渲染时引入的误差。
    pub geometric_error: f64,

    /// refinement 策略（ADD 或 REPLACE）。
    #[serde(default)]
    pub refine: Option<TileRefine>,

    /// 可选的 4x4 变换矩阵（列主序，16 个元素）。
    #[serde(default)]
    pub transform: Option<[f64; 16]>,

    /// 本瓦片的内容（若它有可渲染内容）。
    #[serde(default)]
    pub content: Option<TileContent>,

    /// 多个内容（用于复合瓦片）。
    #[serde(default)]
    pub contents: Option<Vec<TileContent>>,

    /// 子瓦片。
    #[serde(default)]
    pub children: Vec<Tile>,

    /// 用于预取的可选 viewer request volume。
    #[serde(default)]
    pub viewer_request_volume: Option<BoundingVolume>,

    /// 可选的 extras（应用特定数据）。
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl Tile {
    /// 返回有效的 refine 模式，若未指定则从父瓦片继承。
    pub fn effective_refine(&self, parent_refine: TileRefine) -> TileRefine {
        self.refine.unwrap_or(parent_refine)
    }

    /// 若本瓦片有可渲染内容则返回 true。
    pub fn has_content(&self) -> bool {
        self.content.is_some() || self.contents.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// 返回本瓦片所有内容 URI。
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

    /// 返回后代瓦片的数量（递归）。
    pub fn descendant_count(&self) -> usize {
        let mut count = self.children.len();
        for child in &self.children {
            count += child.descendant_count();
        }
        count
    }

    /// 将变换矩阵作为 glam DMat4 返回，若未指定则返回单位阵。
    pub fn transform_matrix(&self) -> glam::DMat4 {
        match self.transform {
            Some(data) => glam::DMat4::from_cols_array(&data),
            None => glam::DMat4::IDENTITY,
        }
    }
}

/// 遍历期间瓦片的运行时状态。
///
/// 它与可序列化的 Tile 结构分离，以保持 domain 模型纯粹、
/// 运行时状态可变。
#[derive(Debug, Clone, Default)]
pub struct TileRuntimeState {
    /// 当前的内容加载状态。
    pub content_state: TileContentState,

    /// 到本瓦片的相机距离。
    pub distance_to_camera: f64,

    /// 本瓦片的屏幕空间误差。
    pub screen_space_error: f64,

    /// 本瓦片在当前帧中是否可见。
    pub visible: bool,

    /// 本瓦片是否被选中渲染。
    pub selected: bool,

    /// 本瓦片在树中的深度。
    pub depth: u32,

    /// 本瓦片上次被访问时的帧号。
    pub visited_frame: u64,

    /// 本瓦片上次被选中时的帧号。
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

        // 应从父瓦片继承
        assert_eq!(tile.effective_refine(TileRefine::Add), TileRefine::Add);
        assert_eq!(tile.effective_refine(TileRefine::Replace), TileRefine::Replace);

        // 应使用自身值
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
