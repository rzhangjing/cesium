//! Ported from `packages/engine/Source/Workers/createVerticesFromQuantizedTerrainMesh.js`
//! (550 lines), inlined into cesium-core (DEVIATION: the JS web-worker hop is
//! replaced by an in-process call, mirroring the `TaskProcessor` rayon
//! substitution used elsewhere in this port).
//!
//! ## Function-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `createVerticesFromQuantizedTerrainMesh` | [`create_vertices_from_quantized_terrain_mesh`] | |
//! | `findMinMaxSkirts` | [`find_min_max_skirts`] | |
//! | `addSkirt` | [`add_skirt`] | |
//! | `copyAndSort` | inline `sort_by` copies | |
//!
//! # DEVIATIONS
//! 1. `includeWebMercatorT` / `includeGeodeticSurfaceNormals` are not modeled
//!    (the simplified [`TerrainEncoding`] of this port has no such slots), so
//!    the related worker parameters are ignored.
//! 2. Positions are stored as absolute ECEF (`[X, Y, Z, H, U, V]` layout),
//!    not relative-to-center; the AxisAlignedBoundingBox / ENU extents only
//!    feed the JS `TerrainEncoding` constructor and are not materialized.
//! 3. `occludeePointInScaledSpace` recomputation
//!    (`EllipsoidalOccluder.computeHorizonCullingPointPossiblyUnderEllipsoid`)
//!    is not ported; the result is left at `Cartesian3::ZERO`.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;
use crate::terrain_encoding::TerrainEncoding;
use crate::terrain_mesh::TerrainMesh;
use crate::terrain_provider::add_skirt_indices;

const MAX_SHORT: f64 = 32767.0;

/// Mirrors the `parameters` object scheduled on the JS worker.
pub struct CreateVerticesParams {
    pub minimum_height: f64,
    pub maximum_height: f64,
    /// `u` values, then `v` values, then quantized heights.
    pub quantized_vertices: Vec<u16>,
    /// Per-vertex oct-encoded normals (2 bytes each), if any.
    pub oct_encoded_normals: Option<Vec<u8>>,
    pub indices: Vec<u32>,
    pub west_indices: Vec<u32>,
    pub south_indices: Vec<u32>,
    pub east_indices: Vec<u32>,
    pub north_indices: Vec<u32>,
    pub west_skirt_height: f64,
    pub south_skirt_height: f64,
    pub east_skirt_height: f64,
    pub north_skirt_height: f64,
    pub rectangle: Rectangle,
    /// `relativeToCenter` in JS (the bounding-sphere center).
    pub center: Cartesian3,
    pub ellipsoid: Ellipsoid,
    pub exaggeration: f64,
    pub exaggeration_relative_height: f64,
}

/// Mirrors `createVerticesFromQuantizedTerrainMesh`.
pub fn create_vertices_from_quantized_terrain_mesh(params: &CreateVerticesParams) -> TerrainMesh {
    let quantized_vertices = &params.quantized_vertices;
    let quantized_vertex_count = quantized_vertices.len() / 3;
    let oct_encoded_normals = params.oct_encoded_normals.as_deref();
    let edge_vertex_count = params.west_indices.len()
        + params.east_indices.len()
        + params.south_indices.len()
        + params.north_indices.len();
    let has_vertex_normals = oct_encoded_normals.is_some();

    let rectangle = params.rectangle;
    let west = rectangle.west;
    let south = rectangle.south;
    let east = rectangle.east;
    let north = rectangle.north;

    let ellipsoid = params.ellipsoid;

    let minimum_height = params.minimum_height;
    let maximum_height = params.maximum_height;

    let center = params.center;

    let u_buffer = &quantized_vertices[0..quantized_vertex_count];
    let v_buffer = &quantized_vertices[quantized_vertex_count..2 * quantized_vertex_count];
    let height_buffer = &quantized_vertices[2 * quantized_vertex_count..3 * quantized_vertex_count];

    let mut uvs: Vec<Cartesian2> = Vec::with_capacity(quantized_vertex_count);
    let mut heights: Vec<f64> = Vec::with_capacity(quantized_vertex_count);
    let mut positions: Vec<Cartesian3> = Vec::with_capacity(quantized_vertex_count);

    let mut min_longitude = f64::INFINITY;
    let mut max_longitude = f64::NEG_INFINITY;
    let mut min_latitude = f64::INFINITY;
    let mut max_latitude = f64::NEG_INFINITY;

    let mut cartographic = Cartographic::default();
    for i in 0..quantized_vertex_count {
        let u = u_buffer[i] as f64 / MAX_SHORT;
        let v = v_buffer[i] as f64 / MAX_SHORT;
        let height = CesiumMath::lerp(
            minimum_height,
            maximum_height,
            height_buffer[i] as f64 / MAX_SHORT,
        );

        cartographic.longitude = CesiumMath::lerp(west, east, u);
        cartographic.latitude = CesiumMath::lerp(south, north, v);
        cartographic.height = height;

        min_longitude = min_longitude.min(cartographic.longitude);
        max_longitude = max_longitude.max(cartographic.longitude);
        min_latitude = min_latitude.min(cartographic.latitude);
        max_latitude = max_latitude.max(cartographic.latitude);

        let mut position = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut position);

        uvs.push(Cartesian2::new(u, v));
        heights.push(height);
        positions.push(position);
    }

    // Mirrors `copyAndSort` (JS sort is stable-ish; f64 total order is fine
    // for these edge orderings).
    let mut west_indices_south_to_north = params.west_indices.clone();
    west_indices_south_to_north.sort_by(|a, b| {
        uvs[*a as usize]
            .y
            .partial_cmp(&uvs[*b as usize].y)
            .unwrap()
    });
    let mut east_indices_north_to_south = params.east_indices.clone();
    east_indices_north_to_south.sort_by(|a, b| {
        uvs[*b as usize]
            .y
            .partial_cmp(&uvs[*a as usize].y)
            .unwrap()
    });
    let mut south_indices_east_to_west = params.south_indices.clone();
    south_indices_east_to_west.sort_by(|a, b| {
        uvs[*b as usize]
            .x
            .partial_cmp(&uvs[*a as usize].x)
            .unwrap()
    });
    let mut north_indices_west_to_east = params.north_indices.clone();
    north_indices_west_to_east.sort_by(|a, b| {
        uvs[*a as usize]
            .x
            .partial_cmp(&uvs[*b as usize].x)
            .unwrap()
    });

    // DEVIATION 3: the JS recomputes the horizon culling point when
    // `minimumHeight < 0`; left at ZERO.
    let occludee_point_in_scaled_space = Cartesian3::default();

    // DEVIATION 2: the ENU-space aaBox (`minimum`/`maximum`) only feeds the
    // JS TerrainEncoding constructor; track nothing here.
    let mut h_min = minimum_height;
    h_min = h_min.min(find_min_max_skirts(
        &params.west_indices,
        params.west_skirt_height,
        &heights,
        &uvs,
        &rectangle,
        &ellipsoid,
    ));
    h_min = h_min.min(find_min_max_skirts(
        &params.south_indices,
        params.south_skirt_height,
        &heights,
        &uvs,
        &rectangle,
        &ellipsoid,
    ));
    h_min = h_min.min(find_min_max_skirts(
        &params.east_indices,
        params.east_skirt_height,
        &heights,
        &uvs,
        &rectangle,
        &ellipsoid,
    ));
    h_min = h_min.min(find_min_max_skirts(
        &params.north_indices,
        params.north_skirt_height,
        &heights,
        &uvs,
        &rectangle,
        &ellipsoid,
    ));

    let encoding = TerrainEncoding::new(
        has_vertex_normals,
        false,
        params.exaggeration,
        params.exaggeration_relative_height,
    );
    let vertex_stride = encoding.stride;

    let mut vertices: Vec<f32> =
        Vec::with_capacity((quantized_vertex_count + edge_vertex_count) * vertex_stride);
    for j in 0..quantized_vertex_count {
        encode_vertex(
            &mut vertices,
            &positions[j],
            &uvs[j],
            heights[j],
            oct_encoded_normals.map(|n| (n[j * 2], n[j * 2 + 1])),
            &encoding,
        );
    }

    let edge_triangle_count = ((edge_vertex_count as i64 - 4) * 2).max(0) as usize;
    let index_buffer_length = params.indices.len() + edge_triangle_count * 3;
    let mut index_buffer: Vec<u32> = Vec::with_capacity(index_buffer_length);
    index_buffer.extend_from_slice(&params.indices);
    let index_count_without_skirts = params.indices.len();

    let percentage = 0.0001;
    let lon_offset = (max_longitude - min_longitude) * percentage;
    let lat_offset = (max_latitude - min_latitude) * percentage;

    // Add skirts (JS order: west, south, east, north).
    add_skirt(
        &mut vertices,
        &west_indices_south_to_north,
        &encoding,
        &heights,
        &uvs,
        oct_encoded_normals,
        &ellipsoid,
        &rectangle,
        params.west_skirt_height,
        -lon_offset,
        0.0,
    );
    add_skirt(
        &mut vertices,
        &south_indices_east_to_west,
        &encoding,
        &heights,
        &uvs,
        oct_encoded_normals,
        &ellipsoid,
        &rectangle,
        params.south_skirt_height,
        0.0,
        -lat_offset,
    );
    add_skirt(
        &mut vertices,
        &east_indices_north_to_south,
        &encoding,
        &heights,
        &uvs,
        oct_encoded_normals,
        &ellipsoid,
        &rectangle,
        params.east_skirt_height,
        lon_offset,
        0.0,
    );
    add_skirt(
        &mut vertices,
        &north_indices_west_to_east,
        &encoding,
        &heights,
        &uvs,
        oct_encoded_normals,
        &ellipsoid,
        &rectangle,
        params.north_skirt_height,
        0.0,
        lat_offset,
    );

    {
        let west_i32: Vec<i32> = west_indices_south_to_north.iter().map(|v| *v as i32).collect();
        let south_i32: Vec<i32> = south_indices_east_to_west.iter().map(|v| *v as i32).collect();
        let east_i32: Vec<i32> = east_indices_north_to_south.iter().map(|v| *v as i32).collect();
        let north_i32: Vec<i32> = north_indices_west_to_east.iter().map(|v| *v as i32).collect();
        index_buffer.resize(index_buffer_length, 0);
        add_skirt_indices(
            &west_i32,
            &south_i32,
            &east_i32,
            &north_i32,
            quantized_vertex_count,
            &mut index_buffer,
            index_count_without_skirts,
        );
    }

    TerrainMesh {
        center,
        vertices,
        stride: vertex_stride,
        indices: index_buffer,
        index_count_without_skirts,
        vertex_count_without_skirts: quantized_vertex_count,
        minimum_height,
        maximum_height,
        rectangle,
        bounding_sphere_3d: crate::bounding_sphere::BoundingSphere::default(),
        occludee_point_in_scaled_space,
        encoding,
        oriented_bounding_box: None,
        west_indices_south_to_north,
        south_indices_east_to_west,
        east_indices_north_to_south,
        north_indices_west_to_east,
    }
}

/// Pushes one `[X, Y, Z, H, U, V(, NX, NY)]` vertex (DEVIATION 2: absolute
/// ECEF positions; oct-encoded normal pair appended when present).
fn encode_vertex(
    vertices: &mut Vec<f32>,
    position: &Cartesian3,
    uv: &Cartesian2,
    height: f64,
    normal: Option<(u8, u8)>,
    encoding: &TerrainEncoding,
) {
    vertices.push(position.x as f32);
    vertices.push(position.y as f32);
    vertices.push(position.z as f32);
    vertices.push(height as f32);
    vertices.push(uv.x as f32);
    vertices.push(uv.y as f32);
    if encoding.has_vertex_normals {
        let (nx, ny) = normal.unwrap_or((0, 0));
        vertices.push(nx as f32);
        vertices.push(ny as f32);
    }
}

/// Mirrors `findMinMaxSkirts` (returns only `hMin`; the JS also updates the
/// ENU-space aaBox, which is not materialized here — DEVIATION 2).
fn find_min_max_skirts(
    edge_indices: &[u32],
    edge_height: f64,
    heights: &[f64],
    uvs: &[Cartesian2],
    rectangle: &Rectangle,
    ellipsoid: &Ellipsoid,
) -> f64 {
    let mut h_min = f64::INFINITY;

    let north = rectangle.north;
    let south = rectangle.south;
    let mut east = rectangle.east;
    let west = rectangle.west;

    if east < west {
        east += CesiumMath::TWO_PI;
    }

    let mut cartographic = Cartographic::default();
    let mut position = Cartesian3::default();
    for &index in edge_indices {
        let index = index as usize;
        let h = heights[index];
        let uv = &uvs[index];

        cartographic.longitude = CesiumMath::lerp(west, east, uv.x);
        cartographic.latitude = CesiumMath::lerp(south, north, uv.y);
        cartographic.height = h - edge_height;

        ellipsoid.cartographic_to_cartesian(&cartographic, &mut position);

        h_min = h_min.min(cartographic.height);
    }
    h_min
}

/// Mirrors `addSkirt` (DEVIATION 1: no webMercatorT / geodetic surface
/// normals; DEVIATION 2: absolute ECEF positions).
#[allow(clippy::too_many_arguments)]
fn add_skirt(
    vertices: &mut Vec<f32>,
    edge_vertices: &[u32],
    encoding: &TerrainEncoding,
    heights: &[f64],
    uvs: &[Cartesian2],
    oct_encoded_normals: Option<&[u8]>,
    ellipsoid: &Ellipsoid,
    rectangle: &Rectangle,
    skirt_length: f64,
    longitude_offset: f64,
    latitude_offset: f64,
) {
    let north = rectangle.north;
    let south = rectangle.south;
    let mut east = rectangle.east;
    let west = rectangle.west;

    if east < west {
        east += CesiumMath::TWO_PI;
    }

    let mut cartographic = Cartographic::default();
    let mut position = Cartesian3::default();
    for &index in edge_vertices {
        let index = index as usize;
        let h = heights[index];
        let uv = &uvs[index];

        cartographic.longitude = CesiumMath::lerp(west, east, uv.x) + longitude_offset;
        cartographic.latitude = CesiumMath::lerp(south, north, uv.y) + latitude_offset;
        cartographic.height = h - skirt_length;

        ellipsoid.cartographic_to_cartesian(&cartographic, &mut position);

        let normal = oct_encoded_normals.map(|n| (n[index * 2], n[index * 2 + 1]));

        encode_vertex(vertices, &position, uv, cartographic.height, normal, encoding);
    }
}
