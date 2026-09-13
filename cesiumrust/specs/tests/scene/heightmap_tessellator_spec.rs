//! Ported from CesiumJS `Specs/Scene/HeightmapTessellatorSpec.js`
//!
//! Covers: computeVertices without skirt, with skirt, quantized mesh,
//! web mercator, multi-element little/big endian heights.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::projection::{MapProjection, WebMercatorProjection};
use cesium_geospatial::rectangle::Rectangle;
use cesium_terrain::heightmap_tessellator::{
    compute_vertices, ComputeVerticesOptions, HeightmapStructure,
};
use glam::{DVec2, DVec3};

const POS_EPS: f64 = 1.0;
const UV_EPS: f64 = 1e-7;

fn check_expected_vertex(
    native_rectangle: &Rectangle,
    i: usize,
    j: usize,
    width: usize,
    height: usize,
    index: usize,
    is_edge: bool,
    vertices: &[f64],
    heightmap: &[f64],
    ellipsoid: &Ellipsoid,
    skirt_height: f64,
) {
    let latitude = math_utils::lerp(
        native_rectangle.north,
        native_rectangle.south,
        j as f64 / (height - 1) as f64,
    );
    let latitude = math_utils::to_radians(latitude);
    let longitude = math_utils::lerp(
        native_rectangle.west,
        native_rectangle.east,
        i as f64 / (width - 1) as f64,
    );
    let longitude = math_utils::to_radians(longitude);

    let mut height_sample = heightmap[j * width + i];
    if is_edge {
        height_sample -= skirt_height;
    }

    let carto = Cartographic::from_radians(longitude, latitude, height_sample);
    let expected = ellipsoid.cartographic_to_cartesian(&carto);

    let base = index * 6;
    let vertex_pos = DVec3::new(vertices[base], vertices[base + 1], vertices[base + 2]);

    assert!(
        (vertex_pos.x - expected.x).abs() < POS_EPS
            && (vertex_pos.y - expected.y).abs() < POS_EPS
            && (vertex_pos.z - expected.z).abs() < POS_EPS,
        "vertex[{index}] position mismatch: got ({:.2},{:.2},{:.2}), expected ({:.2},{:.2},{:.2})",
        vertex_pos.x, vertex_pos.y, vertex_pos.z,
        expected.x, expected.y, expected.z,
    );
    assert!(
        (vertices[base + 3] - height_sample).abs() < 1e-10,
        "vertex[{index}] height: got {}, expected {}",
        vertices[base + 3],
        height_sample,
    );
    let expected_u = i as f64 / (width - 1) as f64;
    let expected_v = 1.0 - j as f64 / (height - 1) as f64;
    assert!(
        (vertices[base + 4] - expected_u).abs() < UV_EPS,
        "vertex[{index}] u: got {}, expected {}",
        vertices[base + 4],
        expected_u,
    );
    assert!(
        (vertices[base + 5] - expected_v).abs() < UV_EPS,
        "vertex[{index}] v: got {}, expected {}",
        vertices[base + 5],
        expected_v,
    );
}

fn check_expected_quantized_vertex(
    native_rectangle: &Rectangle,
    i: usize,
    j: usize,
    width: usize,
    height: usize,
    index: usize,
    is_edge: bool,
    vertices: &[f64],
    heightmap: &[f64],
    ellipsoid: &Ellipsoid,
    skirt_height: f64,
    encoding: &cesium_terrain::TerrainEncoding,
) {
    let latitude = math_utils::lerp(
        native_rectangle.north,
        native_rectangle.south,
        j as f64 / (height - 1) as f64,
    );
    let latitude = math_utils::to_radians(latitude);
    let longitude = math_utils::lerp(
        native_rectangle.west,
        native_rectangle.east,
        i as f64 / (width - 1) as f64,
    );
    let longitude = math_utils::to_radians(longitude);

    let mut height_sample = heightmap[j * width + i];
    if is_edge {
        height_sample -= skirt_height;
    }

    let carto = Cartographic::from_radians(longitude, latitude, height_sample);
    let expected = ellipsoid.cartographic_to_cartesian(&carto);

    let decoded = encoding.decode_position(vertices, index);
    assert!(
        (decoded.x - expected.x).abs() < POS_EPS
            && (decoded.y - expected.y).abs() < POS_EPS
            && (decoded.z - expected.z).abs() < POS_EPS,
        "quantized vertex[{index}] mismatch: got ({:.2},{:.2},{:.2}), expected ({:.2},{:.2},{:.2})",
        decoded.x, decoded.y, decoded.z,
        expected.x, expected.y, expected.z,
    );
}

// ─── creates mesh without skirt ─────────────────────────────────────────────

#[test]
fn creates_mesh_without_skirt() {
    let width = 3;
    let height = 3;
    let heightmap = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    // native_rectangle is in raw degrees (not radians)
    let native_rectangle = Rectangle {
        west: 10.0,
        south: 30.0,
        east: 20.0,
        north: 40.0,
    };

    let options = ComputeVerticesOptions {
        rectangle: Some(Rectangle::from_degrees(10.0, 30.0, 20.0, 40.0)),
        ..ComputeVerticesOptions::new(heightmap.clone(), width, height, 0.0, native_rectangle)
    };

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let ellipsoid = Ellipsoid::WGS84;

    let mut index = 0;
    for j in 0..height {
        for i in 0..width {
            check_expected_vertex(
                &native_rectangle, i, j, width, height, index, false,
                vertices, &heightmap, &ellipsoid, 0.0,
            );
            index += 1;
        }
    }
}

// ─── creates mesh with skirt ────────────────────────────────────────────────

#[test]
fn creates_mesh_with_skirt() {
    let width = 3;
    let height = 3;
    let heightmap = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    // native_rectangle is in raw degrees
    let native_rectangle = Rectangle {
        west: 10.0,
        south: 30.0,
        east: 20.0,
        north: 40.0,
    };
    let skirt_height = 10.0;

    let options = ComputeVerticesOptions::new(
        heightmap.clone(), width, height, skirt_height, native_rectangle,
    );

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let ellipsoid = Ellipsoid::WGS84;

    let mut index = 0;

    // Grid vertices
    for j in 0..height {
        for i in 0..width {
            check_expected_vertex(
                &native_rectangle, i, j, width, height, index, false,
                vertices, &heightmap, &ellipsoid, skirt_height,
            );
            index += 1;
        }
    }

    // West edge: south to north
    for j in 0..height {
        check_expected_vertex(
            &native_rectangle, 0, height - 1 - j, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height,
        );
        index += 1;
    }

    // South edge: east to west
    for i in 0..height {
        check_expected_vertex(
            &native_rectangle, width - 1 - i, height - 1, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height,
        );
        index += 1;
    }

    // East edge: north to south
    for j in 0..height {
        check_expected_vertex(
            &native_rectangle, width - 1, j, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height,
        );
        index += 1;
    }

    // North edge: west to east
    for i in 0..height {
        check_expected_vertex(
            &native_rectangle, i, 0, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height,
        );
        index += 1;
    }
}

// ─── creates quantized mesh ─────────────────────────────────────────────────

#[test]
fn creates_quantized_mesh() {
    let width = 3;
    let height = 3;
    let heightmap = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    // Very small rectangle to trigger Bits12 quantization
    let native_rectangle = Rectangle {
        west: 0.01,
        south: 0.01,
        east: 0.02,
        north: 0.02,
    };
    let skirt_height = 10.0;

    let options = ComputeVerticesOptions::new(
        heightmap.clone(), width, height, skirt_height, native_rectangle,
    );

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let encoding = &results.encoding;
    let ellipsoid = Ellipsoid::WGS84;

    let mut index = 0;

    // Grid vertices
    for j in 0..height {
        for i in 0..width {
            check_expected_quantized_vertex(
                &native_rectangle, i, j, width, height, index, false,
                vertices, &heightmap, &ellipsoid, skirt_height, encoding,
            );
            index += 1;
        }
    }

    // West edge: south to north
    for j in 0..height {
        check_expected_quantized_vertex(
            &native_rectangle, 0, height - 1 - j, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height, encoding,
        );
        index += 1;
    }

    // South edge: east to west
    for i in 0..height {
        check_expected_quantized_vertex(
            &native_rectangle, width - 1 - i, height - 1, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height, encoding,
        );
        index += 1;
    }

    // East edge: north to south
    for j in 0..height {
        check_expected_quantized_vertex(
            &native_rectangle, width - 1, j, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height, encoding,
        );
        index += 1;
    }

    // North edge: west to east
    for i in 0..height {
        check_expected_quantized_vertex(
            &native_rectangle, i, 0, width, height, index, true,
            vertices, &heightmap, &ellipsoid, skirt_height, encoding,
        );
        index += 1;
    }
}

// ─── tessellates web mercator heightmaps ────────────────────────────────────

#[test]
fn tessellates_web_mercator_heightmaps() {
    let width = 3;
    let height = 3;
    let heightmap = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let native_rectangle = Rectangle {
        west: 1000000.0,
        south: 3000000.0,
        east: 2000000.0,
        north: 4000000.0,
    };

    let options = ComputeVerticesOptions {
        is_geographic: false,
        ..ComputeVerticesOptions::new(heightmap.clone(), width, height, 0.0, native_rectangle)
    };

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let ellipsoid = Ellipsoid::WGS84;
    let projection = WebMercatorProjection::wgs84();

    let geographic_sw = projection.unproject(DVec3::new(
        native_rectangle.west,
        native_rectangle.south,
        0.0,
    ));
    let geographic_ne = projection.unproject(DVec3::new(
        native_rectangle.east,
        native_rectangle.north,
        0.0,
    ));

    for j in 0..height {
        let y = math_utils::lerp(
            native_rectangle.north,
            native_rectangle.south,
            j as f64 / (height - 1) as f64,
        );
        for i in 0..width {
            let x = math_utils::lerp(
                native_rectangle.west,
                native_rectangle.east,
                i as f64 / (width - 1) as f64,
            );

            let lat_lon = projection.unproject(DVec3::new(x, y, 0.0));
            let height_sample = heightmap[j * width + i];

            let carto =
                Cartographic::from_radians(lat_lon.longitude, lat_lon.latitude, height_sample);
            let expected = ellipsoid.cartographic_to_cartesian(&carto);

            let base = (j * width + i) * 6;
            let vertex_pos =
                DVec3::new(vertices[base], vertices[base + 1], vertices[base + 2]);

            assert!(
                (vertex_pos.x - expected.x).abs() < POS_EPS
                    && (vertex_pos.y - expected.y).abs() < POS_EPS
                    && (vertex_pos.z - expected.z).abs() < POS_EPS,
                "web mercator vertex[{j}][{i}] mismatch: got ({:.2},{:.2},{:.2}), expected ({:.2},{:.2},{:.2})",
                vertex_pos.x, vertex_pos.y, vertex_pos.z,
                expected.x, expected.y, expected.z,
            );
            assert!(
                (vertices[base + 3] - height_sample).abs() < 1e-10,
                "height mismatch at [{j}][{i}]",
            );

            let expected_u = (lat_lon.longitude - geographic_sw.longitude)
                / (geographic_ne.longitude - geographic_sw.longitude);
            let expected_v = (lat_lon.latitude - geographic_sw.latitude)
                / (geographic_ne.latitude - geographic_sw.latitude);
            assert!(
                (vertices[base + 4] - expected_u).abs() < UV_EPS,
                "u mismatch at [{j}][{i}]: got {}, expected {}",
                vertices[base + 4],
                expected_u,
            );
            assert!(
                (vertices[base + 5] - expected_v).abs() < UV_EPS,
                "v mismatch at [{j}][{i}]: got {}, expected {}",
                vertices[base + 5],
                expected_v,
            );
        }
    }
}

// ─── supports multi-element little endian heights ───────────────────────────

#[test]
fn supports_multi_element_little_endian_heights() {
    let width = 3;
    let height = 3;
    #[rustfmt::skip]
    let heightmap = vec![
        1.0, 2.0, 100.0,  3.0, 4.0, 100.0,  5.0, 6.0, 100.0,
        7.0, 8.0, 100.0,  9.0, 10.0, 100.0, 11.0, 12.0, 100.0,
        13.0, 14.0, 100.0, 15.0, 16.0, 100.0, 17.0, 18.0, 100.0,
    ];
    // native_rectangle is in raw degrees
    let native_rectangle = Rectangle {
        west: 10.0,
        south: 30.0,
        east: 20.0,
        north: 40.0,
    };

    let options = ComputeVerticesOptions {
        rectangle: Some(Rectangle::from_degrees(10.0, 30.0, 20.0, 40.0)),
        structure: Some(HeightmapStructure {
            stride: 3,
            elements_per_height: 2,
            element_multiplier: 10.0,
            ..Default::default()
        }),
        ..ComputeVerticesOptions::new(heightmap.clone(), width, height, 0.0, native_rectangle)
    };

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let ellipsoid = Ellipsoid::WGS84;

    for j in 0..height {
        let latitude = math_utils::lerp(
            native_rectangle.north,
            native_rectangle.south,
            j as f64 / (height - 1) as f64,
        );
        let latitude = math_utils::to_radians(latitude);
        for i in 0..width {
            let longitude = math_utils::lerp(
                native_rectangle.west,
                native_rectangle.east,
                i as f64 / (width - 1) as f64,
            );
            let longitude = math_utils::to_radians(longitude);

            let height_sample_index = (j * width + i) * 3;
            // Little endian: low element first
            let height_sample = heightmap[height_sample_index]
                + heightmap[height_sample_index + 1] * 10.0;

            let carto = Cartographic::from_radians(longitude, latitude, height_sample);
            let expected = ellipsoid.cartographic_to_cartesian(&carto);

            let base = (j * width + i) * 6;
            let vertex_pos =
                DVec3::new(vertices[base], vertices[base + 1], vertices[base + 2]);

            assert!(
                (vertex_pos.x - expected.x).abs() < POS_EPS
                    && (vertex_pos.y - expected.y).abs() < POS_EPS
                    && (vertex_pos.z - expected.z).abs() < POS_EPS,
                "LE vertex[{j}][{i}] mismatch",
            );
            assert!(
                (vertices[base + 3] - height_sample).abs() < 1e-10,
                "LE height mismatch at [{j}][{i}]: got {}, expected {}",
                vertices[base + 3],
                height_sample,
            );
            let expected_u = i as f64 / (width - 1) as f64;
            let expected_v = 1.0 - j as f64 / (height - 1) as f64;
            assert!(
                (vertices[base + 4] - expected_u).abs() < UV_EPS,
                "LE u mismatch at [{j}][{i}]",
            );
            assert!(
                (vertices[base + 5] - expected_v).abs() < UV_EPS,
                "LE v mismatch at [{j}][{i}]",
            );
        }
    }
}

// ─── supports multi-element big endian heights ──────────────────────────────

#[test]
fn supports_multi_element_big_endian_heights() {
    let width = 3;
    let height = 3;
    #[rustfmt::skip]
    let heightmap = vec![
        1.0, 2.0, 100.0,  3.0, 4.0, 100.0,  5.0, 6.0, 100.0,
        7.0, 8.0, 100.0,  9.0, 10.0, 100.0, 11.0, 12.0, 100.0,
        13.0, 14.0, 100.0, 15.0, 16.0, 100.0, 17.0, 18.0, 100.0,
    ];
    // native_rectangle is in raw degrees
    let native_rectangle = Rectangle {
        west: 10.0,
        south: 30.0,
        east: 20.0,
        north: 40.0,
    };

    let options = ComputeVerticesOptions {
        rectangle: Some(Rectangle::from_degrees(10.0, 30.0, 20.0, 40.0)),
        structure: Some(HeightmapStructure {
            stride: 3,
            elements_per_height: 2,
            element_multiplier: 10.0,
            is_big_endian: true,
            ..Default::default()
        }),
        ..ComputeVerticesOptions::new(heightmap.clone(), width, height, 0.0, native_rectangle)
    };

    let results = compute_vertices(&options);
    let vertices = &results.vertices;
    let ellipsoid = Ellipsoid::WGS84;

    for j in 0..height {
        let latitude = math_utils::lerp(
            native_rectangle.north,
            native_rectangle.south,
            j as f64 / (height - 1) as f64,
        );
        let latitude = math_utils::to_radians(latitude);
        for i in 0..width {
            let longitude = math_utils::lerp(
                native_rectangle.west,
                native_rectangle.east,
                i as f64 / (width - 1) as f64,
            );
            let longitude = math_utils::to_radians(longitude);

            let height_sample_index = (j * width + i) * 3;
            // Big endian: high element first
            let height_sample = heightmap[height_sample_index] * 10.0
                + heightmap[height_sample_index + 1];

            let carto = Cartographic::from_radians(longitude, latitude, height_sample);
            let expected = ellipsoid.cartographic_to_cartesian(&carto);

            let base = (j * width + i) * 6;
            let vertex_pos =
                DVec3::new(vertices[base], vertices[base + 1], vertices[base + 2]);

            assert!(
                (vertex_pos.x - expected.x).abs() < POS_EPS
                    && (vertex_pos.y - expected.y).abs() < POS_EPS
                    && (vertex_pos.z - expected.z).abs() < POS_EPS,
                "BE vertex[{j}][{i}] mismatch",
            );
            assert!(
                (vertices[base + 3] - height_sample).abs() < 1e-10,
                "BE height mismatch at [{j}][{i}]: got {}, expected {}",
                vertices[base + 3],
                height_sample,
            );
            let expected_u = i as f64 / (width - 1) as f64;
            let expected_v = 1.0 - j as f64 / (height - 1) as f64;
            assert!(
                (vertices[base + 4] - expected_u).abs() < UV_EPS,
                "BE u mismatch at [{j}][{i}]",
            );
            assert!(
                (vertices[base + 5] - expected_v).abs() < UV_EPS,
                "BE v mismatch at [{j}][{i}]",
            );
        }
    }
}
