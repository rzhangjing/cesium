use bevy::prelude::*;
use cesium_tileset::styling::TileStyle;

use crate::components::{CesiumTileNode, TileContent};

pub fn tile_style_system(
    loaded: Option<Res<crate::tileset::loader::LoadedTileset>>,
    tile_query: Query<(&CesiumTileNode, &TileContent)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    let style = match find_style_in_extras(&tileset_json.extras) {
        Some(s) => s,
        None => return,
    };

    let default_props = std::collections::HashMap::new();

    for (_node, content) in tile_query.iter() {
        let show = style.evaluate_show(&default_props);
        let color = style.evaluate_color(&default_props);

        if let Some(ref material_handle) = content.material_handle {
            if let Some(material) = materials.get_mut(material_handle) {
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

fn find_style_in_extras(extras: &Option<serde_json::Value>) -> Option<TileStyle> {
    let extras = extras.as_ref()?;
    let style_value = extras.get("style")?;
    Some(TileStyle::from_json(style_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn test_style_default_values() {
        let style = TileStyle::default();
        let props = std::collections::HashMap::new();
        assert!(style.evaluate_show(&props));
        assert_eq!(style.evaluate_color(&props), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_find_style_missing() {
        assert!(find_style_in_extras(&None).is_none());
        assert!(
            find_style_in_extras(&Some(serde_json::json!({"other": "data"}))).is_none()
        );
    }
}
