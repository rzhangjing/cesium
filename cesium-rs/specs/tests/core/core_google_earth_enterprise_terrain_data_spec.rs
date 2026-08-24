//! Mirror of `packages/engine/Specs/Core/GoogleEarthEnterpriseTerrainDataSpec.js`
//! (570 lines).
//!
//! The `getBuffer` fixture builder is ported verbatim (little-endian quad
//! layout, 4 corner points + 2 faces per quad).
//!
//! # Skipped JS tests (DEVIATION)
//! - "requires tilingScheme" / "requires x" / "requires y" / "requires
//!   level" (createMesh) and "requires thisX/thisY/childX/childY"
//!   (isChildAvailable): the parameters are required `&dyn TilingScheme` /
//!   `i32` fields in Rust, so the DeveloperError paths are compile-time.
//! - "upsample works for all four children of a simple quad": `#[ignore]`d
//!   until the `upsampleQuantizedTerrainMesh` worker port lands (module
//!   DEVIATION 4 of `google_earth_enterprise_terrain_data`).

use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::google_earth_enterprise_terrain_data::{
    CreateMeshOptions, GoogleEarthEnterpriseTerrainData, GoogleEarthEnterpriseTerrainDataOptions,
};
use cesium_core::math::CesiumMath;
use cesium_core::rectangle::Rectangle;
use cesium_core::terrain_data::TerrainData;
use cesium_core::tiling_scheme::TilingScheme;

const TO_EARTH_RADII: f64 = 1.0 / 6371010.0;

// ── LE buffer writers (DataView set* equivalents) ──────────────────────

fn push_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}
fn push_i32(buf: &mut Vec<u8>, value: i32) {
    buf.extend_from_slice(&value.to_le_bytes());
}
fn push_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}
fn push_f32(buf: &mut Vec<u8>, value: f32) {
    buf.extend_from_slice(&value.to_le_bytes());
}
fn push_f64(buf: &mut Vec<u8>, value: f64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Mirrors the spec's `getBuffer(tilingScheme, x, y, level)`.
fn get_buffer(tiling_scheme: &GeographicTilingScheme, x: i32, y: i32, level: i32) -> Vec<u8> {
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut rectangle);
    let center = Rectangle::center(&rectangle);
    let southwest = Rectangle::southwest(&rectangle);
    let step_x = CesiumMath::to_degrees(rectangle.width() / 2.0) / 180.0;
    let step_y = CesiumMath::to_degrees(rectangle.height() / 2.0) / 180.0;

    const SIZE_OF_UINT8: usize = 1;
    const SIZE_OF_UINT16: usize = 2;
    const SIZE_OF_INT32: usize = 4;
    const SIZE_OF_UINT32: usize = 4;
    const SIZE_OF_FLOAT: usize = 4;
    const SIZE_OF_DOUBLE: usize = 8;

    // 2 Uint8s: x and y values in units of step
    let point_size = 2 * SIZE_OF_UINT8 + SIZE_OF_FLOAT;
    // 3 shorts
    let face_size = 3 * SIZE_OF_UINT16;
    // Doubles: OriginX, OriginY, SizeX, SizeY
    // Int32s: numPoints, numFaces, level
    // 4 corner points
    // 2 face (3 shorts)
    let quad_size =
        4 * SIZE_OF_DOUBLE + 3 * SIZE_OF_INT32 + 4 * point_size + 2 * face_size;

    // QuadSize + size of each quad
    let total_size = 4 * (quad_size + SIZE_OF_UINT32);
    let mut buf = Vec::with_capacity(total_size);

    for i in 0..4u32 {
        let mut altitude_start = 0.0f64;
        push_u32(&mut buf, quad_size as u32);

        // Origin
        let mut x_origin = southwest.longitude;
        let mut y_origin = southwest.latitude;

        if (i & 2) != 0 {
            // Top row
            if (i & 1) == 0 {
                // NE
                x_origin = center.longitude;
                altitude_start = 10.0;
            }
            y_origin = center.latitude;
        } else if (i & 1) != 0 {
            // SE
            x_origin = center.longitude;
            altitude_start = 10.0;
        }

        push_f64(&mut buf, CesiumMath::to_degrees(x_origin) / 180.0);
        push_f64(&mut buf, CesiumMath::to_degrees(y_origin) / 180.0);

        // Step - Each step is a degree
        push_f64(&mut buf, step_x);
        push_f64(&mut buf, step_y);

        // NumPoints
        push_i32(&mut buf, 4);
        // NumFaces
        push_i32(&mut buf, 2);
        // Level
        push_i32(&mut buf, 0);

        // Points
        for j in 0..4u32 {
            let mut x_pos = 0u8;
            let mut y_pos = 0u8;
            let mut altitude = altitude_start;
            if (j & 1) != 0 {
                x_pos += 1;
                altitude += 10.0;
            }
            if (j & 2) != 0 {
                y_pos += 1;
            }

            buf.push(x_pos);
            buf.push(y_pos);
            push_f32(&mut buf, (altitude * TO_EARTH_RADII) as f32);
        }

        // Faces
        for index in [0u16, 1, 2, 1, 3, 2] {
            push_u16(&mut buf, index);
        }
    }

    buf
}

fn make_data(buffer: Vec<u8>, child_tile_mask: Option<u32>) -> GoogleEarthEnterpriseTerrainData {
    GoogleEarthEnterpriseTerrainData::new(GoogleEarthEnterpriseTerrainDataOptions {
        buffer: Some(buffer),
        child_tile_mask,
        negative_altitude_exponent_bias: Some(32.0),
        negative_elevation_threshold: Some(CesiumMath::EPSILON12),
        ..Default::default()
    })
}

#[test]
fn conforms_to_terrain_data_interface() {
    fn assert_terrain_data<T: TerrainData>(_: &T) {}
    // Type-level assertion: the trait implementation exists (JS
    // `toConformToInterface`). The trait is not dyn-compatible (associated
    // const), so a generic bound is used instead of `&dyn TerrainData`.
    let _f: fn(&GoogleEarthEnterpriseTerrainData) = |d| assert_terrain_data(d);
}

// ── upsample ────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "DEVIATION 4: upsample requires the upsampleQuantizedTerrainMesh worker port"]
async fn upsample_works_for_all_four_children_of_a_simple_quad() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let buffer = get_buffer(&tiling_scheme, 0, 0, 0);
    let mut data = make_data(buffer, Some(15));

    let fut = data
        .create_mesh(CreateMeshOptions {
            tiling_scheme: &tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: None,
            exaggeration_relative_height: None,
            throttle: None,
        })
        .expect("mesh creation must not be throttled");
    fut.await;

    for (descendant_x, descendant_y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let upsampled = data.upsample(&tiling_scheme, 0, 0, 0, descendant_x, descendant_y, 1);
        assert!(upsampled.is_some());
    }
}

// ── createMesh ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_mesh_creates_specified_vertices_plus_skirt_vertices() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(0, 0, 0, &mut rectangle);

    let buffer = get_buffer(&tiling_scheme, 0, 0, 0);
    let mut data = make_data(buffer, Some(15));

    let fut = data
        .create_mesh(CreateMeshOptions {
            tiling_scheme: &tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: None,
            exaggeration_relative_height: None,
            throttle: None,
        })
        .expect("mesh creation must not be throttled");
    fut.await;

    let mesh = data.mesh().expect("mesh must be created");
    // 9 regular + 8 skirt vertices
    assert_eq!(mesh.vertices.len(), 17 * mesh.encoding.stride);
    // 2 regular + 4 skirt triangles per quad
    assert_eq!(mesh.indices.len(), 4 * 6 * 3);
    assert_eq!(mesh.minimum_height, 0.0);
    assert!(
        (mesh.maximum_height - 20.0).abs() < 1e-2,
        "maximumHeight {} must be close to 20",
        mesh.maximum_height
    );

    let encoding = &mesh.encoding;
    let wgs84 = Ellipsoid::WGS84;
    let count = mesh.vertices.len() / encoding.stride;
    for i in 0..count {
        let height = encoding.decode_height(&mesh.vertices, i);
        if i < 9 {
            // Original vertices
            assert!((0.0..=20.0).contains(&height), "height {height} not in 0..20");

            // Only test on original positions as the skirts angle outward
            let base = i * encoding.stride;
            let cartesian = cesium_core::cartesian3::Cartesian3::new(
                mesh.vertices[base] as f64,
                mesh.vertices[base + 1] as f64,
                mesh.vertices[base + 2] as f64,
            );
            let mut cartographic = Cartographic::default();
            wgs84.cartesian_to_cartographic(&cartesian, &mut cartographic);
            cartographic.longitude = CesiumMath::convert_longitude_range(cartographic.longitude);
            assert!(
                Rectangle::contains(&rectangle, &cartographic),
                "vertex {i} must lie in the tile rectangle"
            );
        } else {
            // Skirts
            assert!(
                (-1000.0..=-980.0).contains(&height),
                "skirt height {height} not in -1000..-980"
            );
        }
    }
}

#[tokio::test]
async fn create_mesh_exaggerates_mesh() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let buffer = get_buffer(&tiling_scheme, 0, 0, 0);
    let mut data = make_data(buffer, Some(15));

    let fut = data
        .create_mesh(CreateMeshOptions {
            tiling_scheme: &tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: Some(2.0),
            exaggeration_relative_height: None,
            throttle: None,
        })
        .expect("mesh creation must not be throttled");
    fut.await;

    let mesh = data.mesh().expect("mesh must be created");
    // 9 regular + 8 skirt vertices
    assert_eq!(mesh.vertices.len(), 17 * mesh.encoding.stride);
    // 2 regular + 4 skirt triangles per quad
    assert_eq!(mesh.indices.len(), 4 * 6 * 3);

    // Even though there's exaggeration, it doesn't affect the mesh's
    // height, bounding sphere, or any other bounding volumes.
    // The exaggeration is instead stored in the mesh's TerrainEncoding
    assert_eq!(mesh.minimum_height, 0.0);
    assert!((mesh.maximum_height - 20.0).abs() < 1e-2);
    assert_eq!(mesh.encoding.exaggeration, 2.0);

    let encoding = &mesh.encoding;
    let count = mesh.vertices.len() / encoding.stride;
    for i in 0..count {
        let height = encoding.decode_height(&mesh.vertices, i);
        if i < 9 {
            // Original vertices
            assert!((0.0..=40.0).contains(&height));
        } else {
            // Skirts
            assert!((-1000.0..=-960.0).contains(&height));
        }
    }
}

// ── interpolateHeight ───────────────────────────────────────────────────

#[tokio::test]
async fn interpolate_height_clamps_coordinates_outside_the_mesh() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(7, 6, 5, &mut rectangle);
    let buffer = get_buffer(&tiling_scheme, 7, 6, 5);
    let data = make_data(buffer, Some(15));

    assert_eq!(
        data.interpolate_height(&rectangle, 0.0, 0.0),
        data.interpolate_height(&rectangle, rectangle.east, rectangle.south),
    );
}

#[tokio::test]
async fn interpolate_height_returns_a_height_from_the_correct_triangle() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(7, 6, 5, &mut rectangle);
    let buffer = get_buffer(&tiling_scheme, 7, 6, 5);
    let data = make_data(buffer, Some(15));

    // position in the northwest quadrant of the tile.
    let mut longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.25;
    let mut latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.75;

    let result = data
        .interpolate_height(&rectangle, longitude, latitude)
        .expect("interpolation must succeed");
    assert!((0.0..=10.0).contains(&result), "result {result} not in 0..10");

    // position in the southeast quadrant of the tile.
    longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.75;
    latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.25;

    let result = data
        .interpolate_height(&rectangle, longitude, latitude)
        .expect("interpolation must succeed");
    assert!(
        (10.0..=20.0).contains(&result),
        "result {result} not in 10..20"
    );

    // position on the line between the southwest and northeast corners.
    longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.5;
    latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.5;

    let result = data
        .interpolate_height(&rectangle, longitude, latitude)
        .expect("interpolation must succeed");
    assert!(
        (result - 10.0).abs() < 1e-6,
        "result {result} must equal 10 within 1e-6"
    );
}

// ── isChildAvailable ────────────────────────────────────────────────────

#[test]
fn is_child_available_returns_true_for_all_children_when_mask_not_specified() {
    let data = make_data(vec![0u8; 1], None);

    assert!(data.is_child_available(10, 20, 20, 40));
    assert!(data.is_child_available(10, 20, 21, 40));
    assert!(data.is_child_available(10, 20, 20, 41));
    assert!(data.is_child_available(10, 20, 21, 41));
}

#[test]
fn is_child_available_works_when_only_southwest_child_is_available() {
    // Google layout: bit 0 is SW.
    let data = make_data(vec![0u8; 1], Some(1));

    assert!(!data.is_child_available(10, 20, 20, 40));
    assert!(!data.is_child_available(10, 20, 21, 40));
    assert!(data.is_child_available(10, 20, 20, 41));
    assert!(!data.is_child_available(10, 20, 21, 41));
}

#[test]
fn is_child_available_works_when_only_southeast_child_is_available() {
    let data = make_data(vec![0u8; 1], Some(2));

    assert!(!data.is_child_available(10, 20, 20, 40));
    assert!(!data.is_child_available(10, 20, 21, 40));
    assert!(!data.is_child_available(10, 20, 20, 41));
    assert!(data.is_child_available(10, 20, 21, 41));
}

#[test]
fn is_child_available_works_when_only_northeast_child_is_available() {
    let data = make_data(vec![0u8; 1], Some(4));

    assert!(!data.is_child_available(10, 20, 20, 40));
    assert!(data.is_child_available(10, 20, 21, 40));
    assert!(!data.is_child_available(10, 20, 20, 41));
    assert!(!data.is_child_available(10, 20, 21, 41));
}

#[test]
fn is_child_available_works_when_only_northwest_child_is_available() {
    let data = make_data(vec![0u8; 1], Some(8));

    assert!(data.is_child_available(10, 20, 20, 40));
    assert!(!data.is_child_available(10, 20, 21, 40));
    assert!(!data.is_child_available(10, 20, 20, 41));
    assert!(!data.is_child_available(10, 20, 21, 41));
}

// ── constructor checks ──────────────────────────────────────────────────

#[test]
#[should_panic]
fn new_requires_buffer() {
    let _ = GoogleEarthEnterpriseTerrainData::new(GoogleEarthEnterpriseTerrainDataOptions {
        buffer: None,
        child_tile_mask: Some(8),
        negative_altitude_exponent_bias: Some(32.0),
        negative_elevation_threshold: Some(CesiumMath::EPSILON12),
        ..Default::default()
    });
}

#[test]
#[should_panic]
fn new_requires_negative_altitude_exponent_bias() {
    let _ = GoogleEarthEnterpriseTerrainData::new(GoogleEarthEnterpriseTerrainDataOptions {
        buffer: Some(vec![0u8; 1]),
        child_tile_mask: Some(8),
        negative_altitude_exponent_bias: None,
        negative_elevation_threshold: Some(CesiumMath::EPSILON12),
        ..Default::default()
    });
}

#[test]
#[should_panic]
fn new_requires_negative_elevation_threshold() {
    let _ = GoogleEarthEnterpriseTerrainData::new(GoogleEarthEnterpriseTerrainDataOptions {
        buffer: Some(vec![0u8; 1]),
        child_tile_mask: Some(8),
        negative_altitude_exponent_bias: Some(32.0),
        negative_elevation_threshold: None,
        ..Default::default()
    });
}
