//! Scene/Cesium3DTilesetSpec.js, Cesium3DTileSpec.js, TileStyleSpec.js,
//! BatchTableSpec.js, FeatureTableSpec.js → Rust integration tests

use cesium_tileset::{
    BoundingVolume, TileRefine, TilesetJson, TileStyle, StyleExpression,
    BatchTable, FeatureTable,
};

// === BoundingVolume ===

#[test]
fn test_bounding_volume_box() {
    let bv = BoundingVolume::Box([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    assert!(matches!(bv, BoundingVolume::Box(_)));
}

#[test]
fn test_bounding_volume_region() {
    let bv = BoundingVolume::Region([-1.0, -0.5, 1.0, 0.5, 0.0, 100.0]);
    assert!(matches!(bv, BoundingVolume::Region(_)));
}

#[test]
fn test_bounding_volume_sphere() {
    let bv = BoundingVolume::Sphere([0.0, 0.0, 0.0, 100.0]);
    assert!(matches!(bv, BoundingVolume::Sphere(_)));
}

// === TileRefine ===

#[test]
fn test_tile_refine_add() {
    assert_eq!(TileRefine::Add as u8, 1);
}

#[test]
fn test_tile_refine_replace() {
    assert_eq!(TileRefine::Replace as u8, 0);
}

// === TilesetJson parsing ===

#[test]
fn test_tileset_json_parse() {
    let json = r#"{
        "asset": {"version": "1.1"},
        "geometricError": 500.0,
        "root": {
            "boundingVolume": {"box": [0,0,0, 1,0,0, 0,1,0, 0,0,1]},
            "geometricError": 100.0,
            "refine": "REPLACE"
        }
    }"#;
    let tileset: TilesetJson = serde_json::from_str(json).unwrap();
    assert_eq!(tileset.asset.version, "1.1");
    assert!((tileset.geometric_error - 500.0).abs() < 1e-10);
}

#[test]
fn test_tileset_json_with_content() {
    let json = r#"{
        "asset": {"version": "1.0"},
        "geometricError": 100.0,
        "root": {
            "boundingVolume": {"sphere": [0, 0, 0, 100]},
            "geometricError": 50.0,
            "content": {"uri": "tile.b3dm"}
        }
    }"#;
    let tileset: TilesetJson = serde_json::from_str(json).unwrap();
    assert!(tileset.root.content.is_some());
}

// === TileStyle ===

#[test]
fn test_tile_style_new() {
    let style = TileStyle::default();
    assert!(style.show.is_none());
    assert!(style.color.is_none());
}

// === StyleExpression ===

#[test]
fn test_style_expression_from_json_bool() {
    let json = serde_json::json!(true);
    let expr = StyleExpression::from_json(&json);
    assert!(expr.is_some());
}

#[test]
fn test_style_expression_from_json_number() {
    let json = serde_json::json!(42);
    let expr = StyleExpression::from_json(&json);
    assert!(expr.is_some());
}

#[test]
fn test_style_expression_from_json_string() {
    let json = serde_json::json!("${height} > 10");
    let expr = StyleExpression::from_json(&json);
    assert!(expr.is_some());
}

// === BatchTable ===

#[test]
fn test_batch_table_new() {
    let bt = BatchTable::new(None, Vec::new(), 0);
    assert_eq!(bt.features_length, 0);
}

#[test]
fn test_batch_table_with_features() {
    let json = serde_json::json!({
        "height": [10.0, 20.0, 30.0],
        "name": ["A", "B", "C"]
    });
    let bt = BatchTable::new(Some(json), Vec::new(), 3);
    assert_eq!(bt.features_length, 3);
}

// === FeatureTable ===

#[test]
fn test_feature_table_new() {
    let ft = FeatureTable::new(None, Vec::new());
    assert_eq!(ft.features_length, 0);
}

#[test]
fn test_feature_table_with_positions() {
    let json = serde_json::json!({"POINTS_LENGTH": 2, "POSITION": [0,0,0, 1,1,1]});
    let ft = FeatureTable::new(Some(json), Vec::new());
    assert_eq!(ft.features_length, 2);
}
