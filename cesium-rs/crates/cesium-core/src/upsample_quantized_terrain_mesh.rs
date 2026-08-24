//! Ported from `packages/engine/Source/Workers/upsampleQuantizedTerrainMesh.js`
//! (681 lines), inlined into cesium-core (DEVIATION: the JS web-worker hop is
//! replaced by an in-process call).
//!
//! ## Function-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `upsampleQuantizedTerrainMesh` | [`upsample_quantized_terrain_mesh`] | |
//! | `Vertex` (prototype) | [`Vertex`] | `clone`/`initializeIndexed`/`initializeFromClipResult`/`getKey`/`getU`/`getV`/`getH`/`getNormalX`/`getNormalY` all mirrored |
//! | `lerpOctEncodedNormal` | [`lerp_oct_encoded_normal`] | the JS `depth` scratch recursion is plain Rust recursion |
//! | `addClippedPolygon` | [`add_clipped_polygon`] | |
//!
//! # DEVIATIONS
//! 1. The horizon occlusion point
//!    (`EllipsoidalOccluder.computeHorizonCullingPointFromVerticesPossiblyUnderEllipsoid`)
//!    is not ported; the result field is `Cartesian3::ZERO`.
//! 2. The parent vertex buffer uses this port's simplified `TerrainEncoding`
//!    layout (`[X, Y, Z, H, U, V]` + optional oct-encoded normal pair), so
//!    decoding is done with the fixed slot offsets instead of the JS
//!    `TerrainEncoding` methods.
//! 3. JS `vertexMap` object keys (numbers or `JSON.stringify` strings) are
//!    modeled as `HashMap<String, usize>`; keys are only compared against
//!    each other, so the exact string form does not need to match JS.

use std::collections::HashMap;

use crate::attribute_compression::AttributeCompression;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::intersections2d::Intersections2D;
use crate::math::CesiumMath;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::rectangle::Rectangle;

const MAX_SHORT: f64 = 32767.0;
const HALF_MAX_SHORT: f64 = (32767.0 / 2.0) as i64 as f64; // (maxShort / 2) | 0 = 16383

/// Mirrors the `parameters` object scheduled on the JS worker (the parent
/// mesh fields are cloned by the caller, mirroring the worker transfer).
pub struct UpsampleQuantizedMeshParams {
    /// Parent mesh vertex buffer (simplified encoding layout).
    pub vertices: Vec<f32>,
    pub vertex_count_without_skirts: usize,
    /// Parent mesh indices (including skirt indices; only the first
    /// `index_count_without_skirts` entries are used).
    pub indices: Vec<u32>,
    pub index_count_without_skirts: usize,
    /// Stride of the parent vertex buffer.
    pub stride: usize,
    /// Whether the parent buffer carries oct-encoded normals (slots 6/7).
    pub has_vertex_normals: bool,
    pub minimum_height: f64,
    pub maximum_height: f64,
    pub is_east_child: bool,
    pub is_north_child: bool,
    pub child_rectangle: Rectangle,
    pub ellipsoid: Ellipsoid,
}

/// The upsampled tile data, mirroring the worker's transferable result.
pub struct UpsampledQuantizedMeshResult {
    /// `u` values, then `v` values, then quantized heights.
    pub quantized_vertices: Vec<u16>,
    pub indices: Vec<u32>,
    pub encoded_normals: Option<Vec<u8>>,
    pub minimum_height: f64,
    pub maximum_height: f64,
    pub west_indices: Vec<u32>,
    pub south_indices: Vec<u32>,
    pub east_indices: Vec<u32>,
    pub north_indices: Vec<u32>,
    pub bounding_sphere: BoundingSphere,
    pub oriented_bounding_box: OrientedBoundingBox,
    /// DEVIATION 1: always `Cartesian3::ZERO`.
    pub horizon_occlusion_point: Cartesian3,
}

/// Decoded parent-vertex attributes (mirrors `parentUBuffer` & friends).
struct ParentBuffers {
    u: Vec<f64>,
    v: Vec<f64>,
    height: Vec<f64>,
    /// Two f64 entries (0..255) per vertex; `None` without normals.
    normals: Option<Vec<f64>>,
}

/// Mirrors the JS `Vertex` prototype: either an indexed parent vertex or an
/// interpolated point between two vertices.
#[derive(Clone)]
enum Vertex {
    Indexed { index: usize },
    Lerp {
        first: Box<Vertex>,
        second: Box<Vertex>,
        ratio: f64,
    },
}

impl Vertex {
    /// Mirrors `initializeFromClipResult`: consumes one entry (a reference
    /// to a triangle vertex) or four entries (`-1, i, j, ratio` → an
    /// interpolated vertex) from the clip result.
    fn from_clip_result(clip_result: &[f64], index: usize, vertices: &[Vertex; 3]) -> (Vertex, usize) {
        let mut next_index = index + 1;
        let vertex = if clip_result[index] != -1.0 {
            vertices[clip_result[index] as usize].clone()
        } else {
            let first = vertices[clip_result[next_index] as usize].clone();
            next_index += 1;
            let second = vertices[clip_result[next_index] as usize].clone();
            next_index += 1;
            let ratio = clip_result[next_index];
            next_index += 1;
            Vertex::Lerp {
                first: Box::new(first),
                second: Box::new(second),
                ratio,
            }
        };
        (vertex, next_index)
    }

    /// Mirrors `getKey` (JS: index number or `JSON.stringify` of the lerp
    /// triple; DEVIATION 3 — only self-consistency matters).
    fn key(&self, parent: &ParentBuffers) -> String {
        match self {
            Vertex::Indexed { index } => index.to_string(),
            Vertex::Lerp { first, second, ratio } => format!(
                "{{\"first\":{},\"second\":{},\"ratio\":{:?}}}",
                first.key(parent),
                second.key(parent),
                ratio
            ),
        }
    }

    fn is_indexed(&self) -> bool {
        matches!(self, Vertex::Indexed { .. })
    }

    /// Mirrors `getH` (parent heights are quantized 0..maxShort here).
    fn get_h(&self, parent: &ParentBuffers) -> f64 {
        match self {
            Vertex::Indexed { index } => parent.height[*index],
            Vertex::Lerp { first, second, ratio } => {
                CesiumMath::lerp(first.get_h(parent), second.get_h(parent), *ratio)
            }
        }
    }

    /// Mirrors `getU`.
    fn get_u(&self, parent: &ParentBuffers) -> f64 {
        match self {
            Vertex::Indexed { index } => parent.u[*index],
            Vertex::Lerp { first, second, ratio } => {
                CesiumMath::lerp(first.get_u(parent), second.get_u(parent), *ratio)
            }
        }
    }

    /// Mirrors `getV`.
    fn get_v(&self, parent: &ParentBuffers) -> f64 {
        match self {
            Vertex::Indexed { index } => parent.v[*index],
            Vertex::Lerp { first, second, ratio } => {
                CesiumMath::lerp(first.get_v(parent), second.get_v(parent), *ratio)
            }
        }
    }

    /// Mirrors `getNormalX`.
    fn get_normal_x(&self, parent: &ParentBuffers) -> f64 {
        match self {
            Vertex::Indexed { index } => parent.normals.as_ref().unwrap()[*index * 2],
            Vertex::Lerp { .. } => lerp_oct_encoded_normal(self, parent).x,
        }
    }

    /// Mirrors `getNormalY`.
    fn get_normal_y(&self, parent: &ParentBuffers) -> f64 {
        match self {
            Vertex::Indexed { index } => parent.normals.as_ref().unwrap()[*index * 2 + 1],
            Vertex::Lerp { .. } => lerp_oct_encoded_normal(self, parent).y,
        }
    }
}

/// Mirrors `lerpOctEncodedNormal` (the JS `depth`-indexed scratch buffers
/// exist only because JS lacks recursion-local storage).
fn lerp_oct_encoded_normal(vertex: &Vertex, parent: &ParentBuffers) -> Cartesian2 {
    let (first, second, ratio) = match vertex {
        Vertex::Lerp { first, second, ratio } => (first, second, *ratio),
        Vertex::Indexed { .. } => unreachable!("lerpOctEncodedNormal on an indexed vertex"),
    };

    let mut first_decoded = Cartesian3::default();
    AttributeCompression::oct_decode(
        first.get_normal_x(parent),
        first.get_normal_y(parent),
        &mut first_decoded,
    );
    let mut second_decoded = Cartesian3::default();
    AttributeCompression::oct_decode(
        second.get_normal_x(parent),
        second.get_normal_y(parent),
        &mut second_decoded,
    );

    let mut lerped = Cartesian3::default();
    Cartesian3::lerp(&first_decoded, &second_decoded, ratio, &mut lerped);
    lerped = Cartesian3::normalize_new(&lerped);

    let mut result = Cartesian2::default();
    AttributeCompression::oct_encode(&lerped, &mut result);
    result
}

/// Mirrors `upsampleQuantizedTerrainMesh`.
pub fn upsample_quantized_terrain_mesh(
    parameters: &UpsampleQuantizedMeshParams,
) -> UpsampledQuantizedMeshResult {
    let is_east_child = parameters.is_east_child;
    let is_north_child = parameters.is_north_child;

    let min_u = if is_east_child { HALF_MAX_SHORT } else { 0.0 };
    let max_u = if is_east_child { MAX_SHORT } else { HALF_MAX_SHORT };
    let min_v = if is_north_child { HALF_MAX_SHORT } else { 0.0 };
    let max_v = if is_north_child { MAX_SHORT } else { HALF_MAX_SHORT };

    let mut u_buffer: Vec<f64> = Vec::new();
    let mut v_buffer: Vec<f64> = Vec::new();
    let mut height_buffer: Vec<f64> = Vec::new();
    let mut normal_buffer: Vec<f64> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut vertex_map: HashMap<String, usize> = HashMap::new();

    let parent_vertices = &parameters.vertices;
    let parent_indices = &parameters.indices[0..parameters.index_count_without_skirts];

    let has_vertex_normals = parameters.has_vertex_normals;
    let stride = parameters.stride;

    let quantized_vertex_count = parameters.vertex_count_without_skirts;

    let parent_minimum_height = parameters.minimum_height;
    let parent_maximum_height = parameters.maximum_height;

    let mut parent = ParentBuffers {
        u: vec![0.0; quantized_vertex_count],
        v: vec![0.0; quantized_vertex_count],
        height: vec![0.0; quantized_vertex_count],
        normals: has_vertex_normals.then(|| vec![0.0; quantized_vertex_count * 2]),
    };

    let threshold = 20.0;

    for i in 0..quantized_vertex_count {
        // DEVIATION 2: fixed slot offsets of the simplified encoding.
        let base = i * stride;
        let tex_coord_x = parent_vertices[base + 4] as f64;
        let tex_coord_y = parent_vertices[base + 5] as f64;
        let height = parent_vertices[base + 3] as f64;

        let mut u = CesiumMath::clamp(
            (tex_coord_x * MAX_SHORT) as i64 as f64,
            0.0,
            MAX_SHORT,
        );
        let mut v = CesiumMath::clamp(
            (tex_coord_y * MAX_SHORT) as i64 as f64,
            0.0,
            MAX_SHORT,
        );
        parent.height[i] = CesiumMath::clamp(
            ((height - parent_minimum_height) / (parent_maximum_height - parent_minimum_height)
                * MAX_SHORT) as i64 as f64,
            0.0,
            MAX_SHORT,
        );

        if u < threshold {
            u = 0.0;
        }
        if v < threshold {
            v = 0.0;
        }
        if MAX_SHORT - u < threshold {
            u = MAX_SHORT;
        }
        if MAX_SHORT - v < threshold {
            v = MAX_SHORT;
        }

        parent.u[i] = u;
        parent.v[i] = v;

        if let Some(normals) = parent.normals.as_mut() {
            normals[i * 2] = parent_vertices[base + 6] as f64;
            normals[i * 2 + 1] = parent_vertices[base + 7] as f64;
        }

        if ((is_east_child && u >= HALF_MAX_SHORT) || (!is_east_child && u <= HALF_MAX_SHORT))
            && ((is_north_child && v >= HALF_MAX_SHORT) || (!is_north_child && v <= HALF_MAX_SHORT))
        {
            vertex_map.insert(i.to_string(), u_buffer.len());
            u_buffer.push(u);
            v_buffer.push(v);
            height_buffer.push(parent.height[i]);
            if let Some(normals) = parent.normals.as_ref() {
                normal_buffer.push(normals[i * 2]);
                normal_buffer.push(normals[i * 2 + 1]);
            }
        }
    }

    for i in (0..parent_indices.len()).step_by(3) {
        let i0 = parent_indices[i] as usize;
        let i1 = parent_indices[i + 1] as usize;
        let i2 = parent_indices[i + 2] as usize;

        let u0 = parent.u[i0];
        let u1 = parent.u[i1];
        let u2 = parent.u[i2];

        let triangle_vertices = [
            Vertex::Indexed { index: i0 },
            Vertex::Indexed { index: i1 },
            Vertex::Indexed { index: i2 },
        ];

        // Clip triangle on the east-west boundary.
        let clipped = Intersections2D::clip_triangle_at_axis_aligned_threshold(
            HALF_MAX_SHORT,
            is_east_child,
            u0,
            u1,
            u2,
        );

        // Get the first clipped triangle, if any.
        let mut clipped_index = 0usize;
        if clipped_index >= clipped.len() {
            continue;
        }
        let (c0, next) = Vertex::from_clip_result(&clipped, clipped_index, &triangle_vertices);
        clipped_index = next;
        if clipped_index >= clipped.len() {
            continue;
        }
        let (c1, next) = Vertex::from_clip_result(&clipped, clipped_index, &triangle_vertices);
        clipped_index = next;
        if clipped_index >= clipped.len() {
            continue;
        }
        let (c2, _) = Vertex::from_clip_result(&clipped, clipped_index, &triangle_vertices);

        let mut clipped_triangle_vertices = [c0, c1, c2];

        // Clip the triangle against the north-south boundary.
        let clipped2 = Intersections2D::clip_triangle_at_axis_aligned_threshold(
            HALF_MAX_SHORT,
            is_north_child,
            clipped_triangle_vertices[0].get_v(&parent),
            clipped_triangle_vertices[1].get_v(&parent),
            clipped_triangle_vertices[2].get_v(&parent),
        );
        add_clipped_polygon(
            &mut u_buffer,
            &mut v_buffer,
            &mut height_buffer,
            &mut normal_buffer,
            &mut indices,
            &mut vertex_map,
            &clipped2,
            &clipped_triangle_vertices,
            &parent,
            has_vertex_normals,
        );

        // If there's another vertex in the original clipped result, it forms
        // a second triangle. Clip it as well.
        if clipped_index < clipped.len() {
            // JS clones [1] into [2] and then reinitializes [2] from the
            // clip result (the clone is fully overwritten).
            let (c4, _) = Vertex::from_clip_result(&clipped, clipped_index, &triangle_vertices);
            clipped_triangle_vertices[2] = c4;

            let clipped2 = Intersections2D::clip_triangle_at_axis_aligned_threshold(
                HALF_MAX_SHORT,
                is_north_child,
                clipped_triangle_vertices[0].get_v(&parent),
                clipped_triangle_vertices[1].get_v(&parent),
                clipped_triangle_vertices[2].get_v(&parent),
            );
            add_clipped_polygon(
                &mut u_buffer,
                &mut v_buffer,
                &mut height_buffer,
                &mut normal_buffer,
                &mut indices,
                &mut vertex_map,
                &clipped2,
                &clipped_triangle_vertices,
                &parent,
                has_vertex_normals,
            );
        }
    }

    let u_offset = if is_east_child { -MAX_SHORT } else { 0.0 };
    let v_offset = if is_north_child { -MAX_SHORT } else { 0.0 };

    let mut west_indices: Vec<u32> = Vec::new();
    let mut south_indices: Vec<u32> = Vec::new();
    let mut east_indices: Vec<u32> = Vec::new();
    let mut north_indices: Vec<u32> = Vec::new();

    let mut minimum_height = f64::MAX;
    let mut maximum_height = -f64::MAX;

    let mut cartesian_vertices: Vec<f64> = Vec::with_capacity(u_buffer.len() * 3);

    let ellipsoid = parameters.ellipsoid;
    let rectangle = parameters.child_rectangle;

    let north = rectangle.north;
    let south = rectangle.south;
    let mut east = rectangle.east;
    let west = rectangle.west;

    if east < west {
        east += CesiumMath::TWO_PI;
    }

    let mut cartographic = Cartographic::default();
    let mut cartesian = Cartesian3::default();
    for i in 0..u_buffer.len() {
        let mut u = u_buffer[i].round();
        if u <= min_u {
            west_indices.push(i as u32);
            u = 0.0;
        } else if u >= max_u {
            east_indices.push(i as u32);
            u = MAX_SHORT;
        } else {
            u = u * 2.0 + u_offset;
        }
        u_buffer[i] = u;

        let mut v = v_buffer[i].round();
        if v <= min_v {
            south_indices.push(i as u32);
            v = 0.0;
        } else if v >= max_v {
            north_indices.push(i as u32);
            v = MAX_SHORT;
        } else {
            v = v * 2.0 + v_offset;
        }
        v_buffer[i] = v;

        let height = CesiumMath::lerp(
            parent_minimum_height,
            parent_maximum_height,
            height_buffer[i] / MAX_SHORT,
        );
        if height < minimum_height {
            minimum_height = height;
        }
        if height > maximum_height {
            maximum_height = height;
        }
        height_buffer[i] = height;

        cartographic.longitude = CesiumMath::lerp(west, east, u / MAX_SHORT);
        cartographic.latitude = CesiumMath::lerp(south, north, v / MAX_SHORT);
        cartographic.height = height;

        ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);

        cartesian_vertices.push(cartesian.x);
        cartesian_vertices.push(cartesian.y);
        cartesian_vertices.push(cartesian.z);
    }

    let bounding_sphere =
        BoundingSphere::from_vertices(&cartesian_vertices, Some(&Cartesian3::ZERO), Some(3), None);
    let oriented_bounding_box = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(minimum_height),
        Some(maximum_height),
        Some(ellipsoid),
        None,
    );

    // DEVIATION 1: the JS horizon occlusion point comes from
    // EllipsoidalOccluder.computeHorizonCullingPointFromVerticesPossiblyUnderEllipsoid.
    let horizon_occlusion_point = Cartesian3::default();

    let height_range = maximum_height - minimum_height;

    let vertex_count = u_buffer.len();
    let mut vertices: Vec<u16> = Vec::with_capacity(vertex_count * 3);
    for i in 0..vertex_count {
        vertices.push(u_buffer[i] as u16);
    }
    for i in 0..vertex_count {
        vertices.push(v_buffer[i] as u16);
    }
    for i in 0..vertex_count {
        // JS: storing NaN into a Uint16Array yields 0 (flat terrain).
        let quantized = if height_range == 0.0 {
            0u16
        } else {
            ((MAX_SHORT * (height_buffer[i] - minimum_height)) / height_range) as u16
        };
        vertices.push(quantized);
    }

    let encoded_normals = if has_vertex_normals {
        Some(normal_buffer.iter().map(|n| *n as u8).collect())
    } else {
        None
    };

    UpsampledQuantizedMeshResult {
        quantized_vertices: vertices,
        indices,
        encoded_normals,
        minimum_height,
        maximum_height,
        west_indices,
        south_indices,
        east_indices,
        north_indices,
        bounding_sphere,
        oriented_bounding_box,
        horizon_occlusion_point,
    }
}

/// Mirrors `addClippedPolygon`.
#[allow(clippy::too_many_arguments)]
fn add_clipped_polygon(
    u_buffer: &mut Vec<f64>,
    v_buffer: &mut Vec<f64>,
    height_buffer: &mut Vec<f64>,
    normal_buffer: &mut Vec<f64>,
    indices: &mut Vec<u32>,
    vertex_map: &mut HashMap<String, usize>,
    clipped: &[f64],
    triangle_vertices: &[Vertex; 3],
    parent: &ParentBuffers,
    has_vertex_normals: bool,
) {
    if clipped.is_empty() {
        return;
    }

    let mut polygon_vertices: Vec<Vertex> = Vec::with_capacity(4);
    let mut clipped_index = 0usize;
    while clipped_index < clipped.len() {
        let (vertex, next) =
            Vertex::from_clip_result(clipped, clipped_index, triangle_vertices);
        polygon_vertices.push(vertex);
        clipped_index = next;
    }

    let num_vertices = polygon_vertices.len();
    let mut new_indices = vec![0usize; num_vertices];
    for (i, polygon_vertex) in polygon_vertices.iter().enumerate() {
        if !polygon_vertex.is_indexed() {
            let key = polygon_vertex.key(parent);
            if let Some(&existing) = vertex_map.get(&key) {
                new_indices[i] = existing;
            } else {
                let new_index = u_buffer.len();
                u_buffer.push(polygon_vertex.get_u(parent));
                v_buffer.push(polygon_vertex.get_v(parent));
                height_buffer.push(polygon_vertex.get_h(parent));
                if has_vertex_normals {
                    normal_buffer.push(polygon_vertex.get_normal_x(parent));
                    normal_buffer.push(polygon_vertex.get_normal_y(parent));
                }
                new_indices[i] = new_index;
                vertex_map.insert(key, new_index);
            }
        } else if let Vertex::Indexed { index } = polygon_vertex {
            // Parent vertices outside the child quadrant were never inserted
            // into the map; JS reads `undefined` there and never uses it
            // (such vertices cannot appear in kept geometry).
            new_indices[i] = vertex_map.get(&index.to_string()).copied().unwrap_or(0);
        }
    }

    if num_vertices == 3 {
        // A triangle.
        indices.push(new_indices[0] as u32);
        indices.push(new_indices[1] as u32);
        indices.push(new_indices[2] as u32);
    } else if num_vertices == 4 {
        // A quad - two triangles.
        indices.push(new_indices[0] as u32);
        indices.push(new_indices[1] as u32);
        indices.push(new_indices[2] as u32);

        indices.push(new_indices[0] as u32);
        indices.push(new_indices[2] as u32);
        indices.push(new_indices[3] as u32);
    }
}
