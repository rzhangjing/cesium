//! 瓦片样式系统：从 tileset 的 extras 中读取样式定义，逐瓦片应用到材质。
//!
//! [`tile_style_system`] 把 [`TileStyle`] 求值得到的 show/color 写入每个
//! 瓦片内容的 `StandardMaterial`；样式缺失或 tileset 未加载时不作用。
use bevy::prelude::*;
use cesium_tileset::styling::TileStyle;

use crate::components::{CesiumTileNode, TileContent};

/// 逐帧根据瓦片样式更新可见性与颜色的系统。
///
/// # 参数
/// - `loaded`：已加载的 tileset（未就绪则返回）
/// - `tile_query`：瓦片节点与内容（含材质句柄）
/// - `materials`：标准材质资源
pub fn tile_style_system(
    loaded: Option<Res<crate::tileset::loader::LoadedTileset>>,
    tile_query: Query<(&CesiumTileNode, &TileContent)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 逐层前导：无已加载 tileset / 无 tileset.json / 无样式则直接返回。
    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    // 取出 tileset.json（描述根节点与 extras）。
    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    let style = match find_style_in_extras(&tileset_json.extras) {
        Some(s) => s,
        None => return,
    };

    // 目前以空属性集求值（后续可接入每个瓦片的元数据）。
    let default_props = std::collections::HashMap::new();

    for (_node, content) in tile_query.iter() {
        // 求值 show 与 color，据此调整材质基色。
        let show = style.evaluate_show(&default_props);
        let color = style.evaluate_color(&default_props);

        // 仅对已分配材质的瓦片生效（未加载内容的瓦片句柄为 None）。
        if let Some(ref material_handle) = content.material_handle {
            if let Some(material) = materials.get_mut(material_handle) {
                // 不显示时置全透明，否则用样式颜色覆盖基色。
                if !show {
                    material.base_color.set_alpha(0.0);
                } else {
                    material.base_color = Color::srgb(
                        color[0] as f32,
                        color[1] as f32,
                        color[2] as f32,
                    );
                }
            }
        }
    }
}

/// 从 tileset 的 extras JSON 中查找名为 `style` 的字段并解析为 [`TileStyle`]。
///
/// # 参数
/// - `extras`：tileset 的可选 extras 值
fn find_style_in_extras(extras: &Option<serde_json::Value>) -> Option<TileStyle> {
    let extras = extras.as_ref()?;
    let style_value = extras.get("style")?;
    Some(TileStyle::from_json(style_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 应能从 JSON 解析出 show/color/pointSize 并正确求值。
    fn test_parse_style_from_json() {
        let json = serde_json::json!({
            "style": {
                "color": "color('red')",
                "show": true,
                "pointSize": 2.0
            }
        });
        let style_value = json.get("style").unwrap();
        let style = TileStyle::from_json(style_value);

        let props = std::collections::HashMap::new();
        assert!(style.evaluate_show(&props));
        assert_eq!(style.evaluate_color(&props), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(style.evaluate_point_size(&props), 2.0);
    }

    #[test]
    /// 默认样式应显示且基色为不透明白色。
    fn test_style_default_values() {
        let style = TileStyle::default();
        let props = std::collections::HashMap::new();
        assert!(style.evaluate_show(&props));
        assert_eq!(style.evaluate_color(&props), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    /// extras 缺失或无 style 字段时应返回 None。
    fn test_find_style_missing() {
        assert!(find_style_in_extras(&None).is_none());
        assert!(
            find_style_in_extras(&Some(serde_json::json!({"other": "data"}))).is_none()
        );
    }
}
