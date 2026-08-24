//! Mirror of the CPU-portable portion of
//! `packages/engine/Specs/Scene/Cesium3DTilesetSpec.js`: tileset.json
//! parsing, tile hierarchy construction, bounding volumes, refine and
//! geometric-error inheritance, screen space error math, traversal helpers
//! and statistics.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix4::Matrix4;
use cesium_core::rectangle::Rectangle;

use cesium_scene::cesium3_d_tile::{
    create_bounding_volume, create_box, create_region, create_sphere, screen_space_error,
    screen_space_error_orthographic, BoundingVolumeHeader, Cesium3DTile, Cesium3DTileHeader,
    ContentHeader, ParentTileContext,
};
use cesium_scene::cesium3_d_tile_refine::Cesium3DTileRefine;
use cesium_scene::cesium3_d_tile_content_state::Cesium3DTileContentState;
use cesium_scene::cesium3_d_tileset::Cesium3DTileset;
use cesium_scene::cesium3_d_tileset_statistics::{
    Cesium3DTilesetStatistics, TileContentCounts,
};
use cesium_scene::cesium3_d_tileset_traversal::Cesium3DTilesetTraversal;
use cesium_scene::tile_bounding_volume::TileBoundingVolume;

fn load_tileset(relative: &str) -> Cesium3DTileset {
    let path = cesium_specs::data_path(relative);
    assert!(path.exists(), "fixture missing: {}", path.display());
    let json = std::fs::read_to_string(&path).unwrap();
    let mut tileset = Cesium3DTileset::new();
    tileset.load_tileset_json(&json).unwrap();
    tileset
}

// Tilesets/Tileset fixture: hierarchy with region bounding volumes
// (mirrors the `tileset` fixture used throughout Cesium3DTilesetSpec).
#[test]
fn loads_tileset_fixture_hierarchy() {
    let tileset = load_tileset("Cesium3DTiles/Tilesets/Tileset/tileset.json");

    assert_eq!(tileset.geometric_error(), 240.0);
    let root_index = tileset.root().expect("root tile");
    assert_eq!(tileset.tiles().len(), 5);
    assert_eq!(tileset.statistics().number_of_tiles_total, 5);

    let root = &tileset.tiles()[root_index];
    assert_eq!(root.geometric_error, 70.0);
    assert_eq!(root.refine, Cesium3DTileRefine::Add);
    assert_eq!(root.depth, 0);
    assert_eq!(root.content_uri.as_deref(), Some("parent.b3dm"));
    assert!(matches!(
        root.bounding_volume.as_ref().unwrap(),
        TileBoundingVolume::Region { .. }
    ));
    // The root content has a tight-fit content bounding volume.
    assert!(root.content_bounding_volume.is_some());
    assert_eq!(root.children.len(), 4);

    // Children inherit the ADD refine from the parent and have depth 1.
    for &child_index in &root.children {
        let child = &tileset.tiles()[child_index];
        assert_eq!(child.refine, Cesium3DTileRefine::Add);
        assert_eq!(child.depth, 1);
        assert_eq!(child.parent, Some(root_index));
        assert_eq!(child.geometric_error, 0.0);
        assert!(child.content_uri.is_some());
    }
}

// BatchedWithBoundingSphere fixture: sphere bounding volume.
#[test]
fn loads_tileset_fixture_with_bounding_sphere() {
    let tileset =
        load_tileset("Cesium3DTiles/Batched/BatchedWithBoundingSphere/tileset.json");

    let root_index = tileset.root().unwrap();
    let root = &tileset.tiles()[root_index];
    match root.bounding_volume.as_ref().unwrap() {
        TileBoundingVolume::Sphere { center, radius } => {
            assert!((center.x - 1215011.9317263428).abs() < 1e-9);
            assert!((center.y - -4736309.3434217675).abs() < 1e-9);
            assert!((center.z - 4081612.0044800863).abs() < 1e-9);
            assert!((radius - 141.4214).abs() < 1e-9);
        }
        other => panic!("expected sphere, got {other:?}"),
    }
    assert_eq!(root.geometric_error, 0.0);
}

// BatchedColors fixture: region bounding volume.
#[test]
fn loads_tileset_fixture_with_region() {
    let tileset = load_tileset("Cesium3DTiles/Batched/BatchedColors/tileset.json");

    let root_index = tileset.root().unwrap();
    let root = &tileset.tiles()[root_index];
    match root.bounding_volume.as_ref().unwrap() {
        TileBoundingVolume::Region {
            rectangle,
            minimum_height,
            maximum_height,
        } => {
            assert!((rectangle.west - -1.3197004795898053).abs() < 1e-9);
            assert!((rectangle.south - 0.6988582109).abs() < 1e-9);
            assert_eq!(*minimum_height, 0.0);
            assert_eq!(*maximum_height, 20.0);
        }
        other => panic!("expected region, got {other:?}"),
    }
}

// createBox: transforms the center and half axes.
#[test]
fn create_box_applies_transform() {
    let box_values = vec![
        1.0, 2.0, 3.0, // center
        1.0, 0.0, 0.0, // half axis x
        0.0, 2.0, 0.0, // half axis y
        0.0, 0.0, 3.0, // half axis z
    ];
    let mut transform = Matrix4::IDENTITY;
    // Translation of (10, 20, 30)
    transform.elements[12] = 10.0;
    transform.elements[13] = 20.0;
    transform.elements[14] = 30.0;

    match create_box(&box_values, &transform) {
        TileBoundingVolume::Box { center, half_axes } => {
            assert!((center.x - 11.0).abs() < 1e-12);
            assert!((center.y - 22.0).abs() < 1e-12);
            assert!((center.z - 33.0).abs() < 1e-12);
            // Identity rotation, so half axes are unchanged.
            assert!((half_axes.elements[0] - 1.0).abs() < 1e-12);
            assert!((half_axes.elements[4] - 2.0).abs() < 1e-12);
            assert!((half_axes.elements[8] - 3.0).abs() < 1e-12);
        }
        other => panic!("expected box, got {other:?}"),
    }
}

// createSphere: scales the radius by the maximum scale component.
#[test]
fn create_sphere_applies_uniform_scale() {
    let sphere = vec![1.0, 2.0, 3.0, 10.0];
    let mut transform = Matrix4::IDENTITY;
    transform.elements[0] = 2.0; // scale x by 2
    transform.elements[5] = 3.0; // scale y by 3

    match create_sphere(&sphere, &transform) {
        TileBoundingVolume::Sphere { center, radius } => {
            assert!((center.x - 2.0).abs() < 1e-12);
            assert!((center.y - 6.0).abs() < 1e-12);
            assert!((center.z - 3.0).abs() < 1e-12);
            // uniformScale = maximumComponent(scale) = 3
            assert!((radius - 30.0).abs() < 1e-12);
        }
        other => panic!("expected sphere, got {other:?}"),
    }
}

// createRegion: unpacks the rectangle and heights.
#[test]
fn create_region_unpacks_rectangle() {
    let region = vec![-1.0, 0.5, -0.5, 1.0, 10.0, 40.0];
    match create_region(&region, &Matrix4::IDENTITY, &Matrix4::IDENTITY) {
        TileBoundingVolume::Region {
            rectangle,
            minimum_height,
            maximum_height,
        } => {
            assert_eq!(rectangle, Rectangle::new(-1.0, 0.5, -0.5, 1.0));
            assert_eq!(minimum_height, 10.0);
            assert_eq!(maximum_height, 40.0);
        }
        other => panic!("expected region, got {other:?}"),
    }
}

// createBoundingVolume: "boundingVolume must be defined"
#[test]
fn create_bounding_volume_throws_when_missing() {
    let error = create_bounding_volume(None, &Matrix4::IDENTITY, &Matrix4::IDENTITY)
        .unwrap_err();
    assert_eq!(error.message, "boundingVolume must be defined");
}

// createBoundingVolume: "boundingVolume must contain a sphere, region, or box"
#[test]
fn create_bounding_volume_throws_when_empty() {
    let header = BoundingVolumeHeader::default();
    let error = create_bounding_volume(
        Some(&header),
        &Matrix4::IDENTITY,
        &Matrix4::IDENTITY,
    )
    .unwrap_err();
    assert_eq!(
        error.message,
        "boundingVolume must contain a sphere, region, or box"
    );
}

fn sphere_header(radius: f64) -> Cesium3DTileHeader {
    Cesium3DTileHeader {
        bounding_volume: Some(BoundingVolumeHeader {
            sphere: Some(vec![0.0, 0.0, 0.0, radius]),
            ..Default::default()
        }),
        geometric_error: Some(10.0),
        ..Default::default()
    }
}

// Cesium3DTile constructor: geometricError falls back to the tileset value
// for the root tile (mirrors the "geometricErrorUndefined" fallback).
#[test]
fn geometric_error_falls_back_to_tileset() {
    let header = Cesium3DTileHeader {
        bounding_volume: Some(BoundingVolumeHeader {
            sphere: Some(vec![0.0, 0.0, 0.0, 1.0]),
            ..Default::default()
        }),
        geometric_error: None,
        ..Default::default()
    };

    let tile =
        Cesium3DTile::from_header(&header, None, &Matrix4::IDENTITY, 240.0).unwrap();
    assert_eq!(tile.geometric_error, 240.0);
}

// Cesium3DTile constructor: refine is inherited from the parent when
// omitted, and lowercase values are accepted.
#[test]
fn refine_inheritance_and_lowercase() {
    let parent_header = sphere_header(1.0);
    let parent = Cesium3DTile::from_header(
        &Cesium3DTileHeader {
            refine: Some("ADD".to_string()),
            ..parent_header.clone()
        },
        None,
        &Matrix4::IDENTITY,
        0.0,
    )
    .unwrap();
    assert_eq!(parent.refine, Cesium3DTileRefine::Add);

    // Child without refine inherits from the parent.
    let child = Cesium3DTile::from_header(
        &sphere_header(1.0),
        Some(&parent.parent_context()),
        &Matrix4::IDENTITY,
        0.0,
    )
    .unwrap();
    assert_eq!(child.refine, Cesium3DTileRefine::Add);

    // Lowercase "replace" is accepted (deprecation tolerated).
    let replace = Cesium3DTile::from_header(
        &Cesium3DTileHeader {
            refine: Some("replace".to_string()),
            ..sphere_header(1.0)
        },
        None,
        &Matrix4::IDENTITY,
        0.0,
    )
    .unwrap();
    assert_eq!(replace.refine, Cesium3DTileRefine::Replace);
}

// Cesium3DTile constructor: an empty content URI yields empty content.
#[test]
fn empty_content_uri_creates_empty_content() {
    let header = Cesium3DTileHeader {
        content: Some(ContentHeader {
            uri: Some(String::new()),
            ..Default::default()
        }),
        ..sphere_header(1.0)
    };

    let tile =
        Cesium3DTile::from_header(&header, None, &Matrix4::IDENTITY, 0.0).unwrap();
    assert!(tile.has_empty_content);
    assert_eq!(tile.content_state, Cesium3DTileContentState::Ready);
    assert_eq!(tile.content_uri, None);
}

// Cesium3DTile constructor: a single-entry `contents` array (3D Tiles 1.1)
// is equivalent to `content`.
#[test]
fn contents_array_single_entry() {
    let header = Cesium3DTileHeader {
        contents: Some(vec![ContentHeader {
            uri: Some("tile.glb".to_string()),
            ..Default::default()
        }]),
        ..sphere_header(1.0)
    };

    let tile =
        Cesium3DTile::from_header(&header, None, &Matrix4::IDENTITY, 0.0).unwrap();
    assert!(!tile.has_multiple_contents);
    assert_eq!(tile.content_uri.as_deref(), Some("tile.glb"));
}

// Cesium3DTile constructor: multiple contents are flagged.
#[test]
fn contents_array_multiple_entries() {
    let header = Cesium3DTileHeader {
        contents: Some(vec![
            ContentHeader {
                uri: Some("a.glb".to_string()),
                ..Default::default()
            },
            ContentHeader {
                uri: Some("b.glb".to_string()),
                ..Default::default()
            },
        ]),
        ..sphere_header(1.0)
    };

    let tile =
        Cesium3DTile::from_header(&header, None, &Matrix4::IDENTITY, 0.0).unwrap();
    assert!(tile.has_multiple_contents);
    assert_eq!(tile.content_state, Cesium3DTileContentState::Unloaded);
}

// getScreenSpaceError (perspective): geometricError * height /
// (distance * sseDenominator), divided by pixelRatio.
#[test]
fn screen_space_error_perspective_math() {
    let error = screen_space_error(100.0, 1000.0, 1080.0, 2.0, 1.0);
    assert!((error - 54.0).abs() < 1e-12);

    // pixelRatio scales the result down.
    let error = screen_space_error(100.0, 1000.0, 1080.0, 2.0, 2.0);
    assert!((error - 27.0).abs() < 1e-12);
}

// getScreenSpaceError: leaf tiles (zero geometric error) return 0.
#[test]
fn screen_space_error_zero_geometric_error() {
    assert_eq!(screen_space_error(0.0, 1000.0, 1080.0, 2.0, 1.0), 0.0);
}

// getScreenSpaceError: distance is clamped at EPSILON7 when the viewer is
// inside the tile.
#[test]
fn screen_space_error_clamps_distance() {
    let clamped = screen_space_error(1.0, 0.0, 100.0, 1.0, 1.0);
    let expected = 100.0 / cesium_core::math::CesiumMath::EPSILON7;
    assert!((clamped - expected).abs() < 1e-3);
}

// getScreenSpaceError (orthographic / 2D): geometricError / pixelSize.
#[test]
fn screen_space_error_orthographic_math() {
    // pixelSize = max(200, 100) / max(500, 400) = 0.4
    let error = screen_space_error_orthographic(40.0, 100.0, 200.0, 500.0, 400.0, 1.0);
    assert!((error - 100.0).abs() < 1e-12);
    assert_eq!(
        screen_space_error_orthographic(0.0, 100.0, 200.0, 500.0, 400.0, 1.0),
        0.0
    );
}

// canTraverse: leaf tiles are never traversed.
#[test]
fn can_traverse_leaf_is_false() {
    let tile = Cesium3DTile::new();
    assert!(!Cesium3DTilesetTraversal::can_traverse(&tile, 16.0));
}

// canTraverse: traverses when the screen space error exceeds the maximum.
#[test]
fn can_traverse_uses_screen_space_error() {
    let mut tile = Cesium3DTile::new();
    tile.children = vec![1];
    tile.screen_space_error = 20.0;
    assert!(Cesium3DTilesetTraversal::can_traverse(&tile, 16.0));
    assert!(!Cesium3DTilesetTraversal::can_traverse(&tile, 32.0));

    // External tileset content is always traversed to reach its root.
    let mut tileset_tile = Cesium3DTile::new();
    tileset_tile.children = vec![1];
    tileset_tile.has_tileset_content = true;
    tileset_tile.screen_space_error = 0.0;
    assert!(Cesium3DTilesetTraversal::can_traverse(&tileset_tile, 16.0));
}

// sortChildrenByDistanceToCamera: farthest first, centerZDepth tie break.
#[test]
fn sort_children_by_distance_to_camera() {
    let mut a = Cesium3DTile::new();
    let mut b = Cesium3DTile::new();
    a.distance_to_camera = 10.0;
    b.distance_to_camera = 20.0;
    // b is farther, so b sorts first (comparator returns positive).
    assert!(Cesium3DTilesetTraversal::sort_children_by_distance_to_camera(&a, &b) > 0.0);

    // Both at distance zero: tie broken by centerZDepth.
    a.distance_to_camera = 0.0;
    b.distance_to_camera = 0.0;
    a.center_z_depth = 1.0;
    b.center_z_depth = 2.0;
    assert!(Cesium3DTilesetTraversal::sort_children_by_distance_to_camera(&a, &b) > 0.0);
}

// Statistics: clear() resets the per-frame counters only.
#[test]
fn statistics_clear() {
    let mut statistics = Cesium3DTilesetStatistics::new();
    statistics.selected = 3;
    statistics.visited = 5;
    statistics.number_of_commands = 2;
    statistics.number_of_attempted_requests = 1;
    statistics.number_of_features_selected = 7;
    statistics.number_of_points_selected = 8;
    statistics.number_of_triangles_selected = 9;
    statistics.number_of_tiles_styled = 4;
    statistics.number_of_features_styled = 6;
    statistics.number_of_tiles_culled_with_children_union = 2;
    statistics.number_of_tiles_total = 42;

    statistics.clear();

    assert_eq!(statistics.selected, 0);
    assert_eq!(statistics.visited, 0);
    assert_eq!(statistics.number_of_commands, 0);
    assert_eq!(statistics.number_of_attempted_requests, 0);
    assert_eq!(statistics.number_of_features_selected, 0);
    assert_eq!(statistics.number_of_points_selected, 0);
    assert_eq!(statistics.number_of_triangles_selected, 0);
    assert_eq!(statistics.number_of_tiles_styled, 0);
    assert_eq!(statistics.number_of_features_styled, 0);
    assert_eq!(statistics.number_of_tiles_culled_with_children_union, 0);
    // Running totals are not cleared.
    assert_eq!(statistics.number_of_tiles_total, 42);
}

// Statistics: increment/decrement load counts (including inner contents).
#[test]
fn statistics_increment_and_decrement_load_counts() {
    let mut statistics = Cesium3DTilesetStatistics::new();
    let content = TileContentCounts {
        features_length: 5,
        points_length: 100,
        triangles_length: 10,
        geometry_byte_length: 1024,
        batch_table_byte_length: 256,
        textures_byte_length: 2048,
        inner_contents: vec![TileContentCounts {
            features_length: 1,
            points_length: 2,
            ..Default::default()
        }],
    };

    statistics.increment_load_counts(&content);
    assert_eq!(statistics.number_of_features_loaded, 6);
    assert_eq!(statistics.number_of_points_loaded, 102);
    assert_eq!(statistics.geometry_byte_length, 1024);
    assert_eq!(statistics.batch_table_byte_length, 256);
    assert_eq!(statistics.textures_byte_length, 2048);

    statistics.increment_selection_counts(&content);
    assert_eq!(statistics.number_of_features_selected, 6);
    assert_eq!(statistics.number_of_points_selected, 102);
    assert_eq!(statistics.number_of_triangles_selected, 10);

    statistics.decrement_load_counts(&content);
    assert_eq!(statistics.number_of_features_loaded, 0);
    assert_eq!(statistics.number_of_points_loaded, 0);
    assert_eq!(statistics.geometry_byte_length, 0);
    assert_eq!(statistics.textures_byte_length, 0);
}

// Statistics: clone(statistics, result).
#[test]
fn statistics_clone_into() {
    let mut source = Cesium3DTilesetStatistics::new();
    source.selected = 1;
    source.visited = 2;
    source.number_of_tiles_total = 3;
    source.textures_reference_counter_by_id
        .insert("texture-0".to_string(), 4);

    let mut result = Cesium3DTilesetStatistics::new();
    Cesium3DTilesetStatistics::clone_into(&source, &mut result);

    assert_eq!(result.selected, 1);
    assert_eq!(result.visited, 2);
    assert_eq!(result.number_of_tiles_total, 3);
    assert_eq!(
        result.textures_reference_counter_by_id.get("texture-0"),
        Some(&4)
    );
}

// TileBoundingVolume: sphere distance mirrors TileBoundingSphere
// (distance to the center).
#[test]
fn bounding_volume_sphere_distance() {
    let volume = TileBoundingVolume::new_sphere(Cartesian3::new(0.0, 0.0, 0.0), 5.0);
    let point = Cartesian3::new(3.0, 4.0, 0.0);
    assert!((volume.distance_to_point(&point) - 5.0).abs() < 1e-12);
}

// TileBoundingVolume: box bounding sphere radius is the corner distance.
#[test]
fn bounding_volume_box_bounding_sphere() {
    let half_axes = cesium_core::matrix3::Matrix3::from_array_new(
        &[2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0],
        0,
    );
    let volume = TileBoundingVolume::new_box(Cartesian3::ZERO, half_axes);
    let sphere = volume.bounding_sphere();
    assert_eq!(sphere.center, Cartesian3::ZERO);
    // |(2,3,4)| = sqrt(29)
    assert!((sphere.radius - 29.0_f64.sqrt()).abs() < 1e-12);

    // Box distance: point outside along +x.
    let point = Cartesian3::new(5.0, 0.0, 0.0);
    assert!((volume.distance_to_point(&point) - 3.0).abs() < 1e-12);
}

// Tileset: invalid JSON returns a RuntimeError.
#[test]
fn load_tileset_json_throws_for_invalid_json() {
    let mut tileset = Cesium3DTileset::new();
    let error = tileset.load_tileset_json("{ not json").unwrap_err();
    assert!(error.message.starts_with("Failed to load tileset JSON:"));
}
