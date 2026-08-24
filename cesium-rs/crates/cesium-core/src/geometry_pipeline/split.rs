//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`.
//!
//! This module covers the `splitLongitude` section of `GeometryPipeline.js`
//! (roughly lines 1630–3290): the indexing helpers for non-indexed primitives
//! (`indexTriangles`/`indexTriangleFan`/`indexTriangleStrip`/`indexLines`/
//! `indexLineStrip`/`indexLineLoop`/`indexPrimitive`), the XZ-plane offset and
//! intersection utilities, `splitTriangle`, the barycentric attribute
//! interpolation helpers, and the per-primitive-type longitude splitters
//! (`splitLongitudeTriangles`, `splitLongitudeLines`,
//! `splitLongitudePolyline`).
//!
//! DEVIATION: JS builds the split geometries by packing attribute values at
//! absolute indices into arrays (shared scratch reuse). Rust ownership forbids
//! those overlapping references, so this port clones the source attribute
//! values up front and grows the split attribute arrays with `push`. The
//! produced values are identical: in JS, array "holes" created when a
//! barycentric interpolation is skipped are filled with `0` when the values
//! are re-created as typed arrays — this port pushes the zeros explicitly.

use std::collections::HashMap;

use crate::barycentric_coordinates::barycentric_coordinates_3d;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::developer_error::throw_developer_error;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::intersect::Intersect;
use crate::intersection_tests::IntersectionTests;
use crate::math::CesiumMath;
use crate::plane::Plane;
use crate::primitive_type::PrimitiveType;

// ---------------------------------------------------------------------------
// Index helpers
// ---------------------------------------------------------------------------

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}

/// Reads the `vertex_index`-th position from a packed position array.
fn cartesian3_at(values: &[f64], vertex_index: usize) -> Cartesian3 {
    let mut out = Cartesian3::ZERO;
    Cartesian3::from_array(values, Some(vertex_index * 3), &mut out);
    out
}

/// `indexTriangles` – creates an identity index list for non-indexed triangles.
fn index_triangles(geometry: &mut Geometry) {
    if geometry.indices.is_some() {
        return;
    }
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) {
        if number_of_vertices < 3 {
            throw_developer_error("The number of vertices must be at least three.");
        }
        if number_of_vertices % 3 != 0 {
            throw_developer_error("The number of vertices must be a multiple of three.");
        }
    }

    let mut indices = IndexDatatype::create_typed_array(number_of_vertices, number_of_vertices);
    for i in 0..number_of_vertices {
        write_index(&mut indices, i, i as u32);
    }

    geometry.indices = Some(indices);
}

/// `indexTriangleFan` – converts a triangle fan to indexed triangles.
fn index_triangle_fan(geometry: &mut Geometry) {
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) && number_of_vertices < 3 {
        throw_developer_error("The number of vertices must be at least three.");
    }

    let mut indices =
        IndexDatatype::create_typed_array(number_of_vertices, (number_of_vertices - 2) * 3);
    write_index(&mut indices, 0, 1);
    write_index(&mut indices, 1, 0);
    write_index(&mut indices, 2, 2);

    let mut indices_index = 3;
    for i in 3..number_of_vertices {
        write_index(&mut indices, indices_index, (i - 1) as u32);
        indices_index += 1;
        write_index(&mut indices, indices_index, 0);
        indices_index += 1;
        write_index(&mut indices, indices_index, i as u32);
        indices_index += 1;
    }

    geometry.indices = Some(indices);
    geometry.primitive_type = PrimitiveType::Triangles;
}

/// `indexTriangleStrip` – converts a triangle strip to indexed triangles.
fn index_triangle_strip(geometry: &mut Geometry) {
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) && number_of_vertices < 3 {
        throw_developer_error("The number of vertices must be at least 3.");
    }

    let mut indices =
        IndexDatatype::create_typed_array(number_of_vertices, (number_of_vertices - 2) * 3);
    write_index(&mut indices, 0, 0);
    write_index(&mut indices, 1, 1);
    write_index(&mut indices, 2, 2);

    if number_of_vertices > 3 {
        write_index(&mut indices, 3, 0);
        write_index(&mut indices, 4, 2);
        write_index(&mut indices, 5, 3);
    }

    let mut indices_index = 6;
    let mut i = 3usize;
    while i < number_of_vertices - 1 {
        write_index(&mut indices, indices_index, i as u32);
        indices_index += 1;
        write_index(&mut indices, indices_index, (i - 1) as u32);
        indices_index += 1;
        write_index(&mut indices, indices_index, (i + 1) as u32);
        indices_index += 1;

        if i + 2 < number_of_vertices {
            write_index(&mut indices, indices_index, i as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, (i + 1) as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, (i + 2) as u32);
            indices_index += 1;
        }
        i += 2;
    }

    geometry.indices = Some(indices);
    geometry.primitive_type = PrimitiveType::Triangles;
}

/// `indexLines` – creates an identity index list for non-indexed lines.
fn index_lines(geometry: &mut Geometry) {
    if geometry.indices.is_some() {
        return;
    }
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) {
        if number_of_vertices < 2 {
            throw_developer_error("The number of vertices must be at least two.");
        }
        if number_of_vertices % 2 != 0 {
            throw_developer_error("The number of vertices must be a multiple of 2.");
        }
    }

    let mut indices = IndexDatatype::create_typed_array(number_of_vertices, number_of_vertices);
    for i in 0..number_of_vertices {
        write_index(&mut indices, i, i as u32);
    }

    geometry.indices = Some(indices);
}

/// `indexLineStrip` – converts a line strip to indexed lines.
fn index_line_strip(geometry: &mut Geometry) {
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) && number_of_vertices < 2 {
        throw_developer_error("The number of vertices must be at least two.");
    }

    let mut indices =
        IndexDatatype::create_typed_array(number_of_vertices, (number_of_vertices - 1) * 2);
    write_index(&mut indices, 0, 0);
    write_index(&mut indices, 1, 1);
    let mut indices_index = 2;
    for i in 2..number_of_vertices {
        write_index(&mut indices, indices_index, (i - 1) as u32);
        indices_index += 1;
        write_index(&mut indices, indices_index, i as u32);
        indices_index += 1;
    }

    geometry.indices = Some(indices);
    geometry.primitive_type = PrimitiveType::Lines;
}

/// `indexLineLoop` – converts a line loop to indexed lines.
fn index_line_loop(geometry: &mut Geometry) {
    let number_of_vertices = geometry.compute_number_of_vertices().unwrap_or(0);

    if cfg!(debug_assertions) && number_of_vertices < 2 {
        throw_developer_error("The number of vertices must be at least two.");
    }

    let mut indices =
        IndexDatatype::create_typed_array(number_of_vertices, number_of_vertices * 2);

    write_index(&mut indices, 0, 0);
    write_index(&mut indices, 1, 1);

    let mut indices_index = 2;
    for i in 2..number_of_vertices {
        write_index(&mut indices, indices_index, (i - 1) as u32);
        indices_index += 1;
        write_index(&mut indices, indices_index, i as u32);
        indices_index += 1;
    }

    write_index(&mut indices, indices_index, (number_of_vertices - 1) as u32);
    indices_index += 1;
    write_index(&mut indices, indices_index, 0);

    geometry.indices = Some(indices);
    geometry.primitive_type = PrimitiveType::Lines;
}

/// `indexPrimitive` – ensures the geometry has an index buffer, converting
/// strip/fan/loop primitive types to indexed `TRIANGLES`/`LINES`.
fn index_primitive(geometry: &mut Geometry) {
    match geometry.primitive_type {
        PrimitiveType::TriangleFan => index_triangle_fan(geometry),
        PrimitiveType::TriangleStrip => index_triangle_strip(geometry),
        PrimitiveType::Triangles => index_triangles(geometry),
        PrimitiveType::LineStrip => index_line_strip(geometry),
        PrimitiveType::LineLoop => index_line_loop(geometry),
        PrimitiveType::Lines => index_lines(geometry),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// XZ-plane offset / intersection utilities
// ---------------------------------------------------------------------------

/// `offsetPointFromXZPlane` – nudges points that lie (nearly) on the XZ plane.
fn offset_point_from_xz_plane(p: &mut Cartesian3, is_behind: bool) {
    if p.y.abs() < CesiumMath::EPSILON6 {
        if is_behind {
            p.y = -CesiumMath::EPSILON6;
        } else {
            p.y = CesiumMath::EPSILON6;
        }
    }
}

/// `offsetTriangleFromXZPlane` – nudges a triangle away from the XZ plane.
fn offset_triangle_from_xz_plane(p0: &mut Cartesian3, p1: &mut Cartesian3, p2: &mut Cartesian3) {
    if p0.y != 0.0 && p1.y != 0.0 && p2.y != 0.0 {
        offset_point_from_xz_plane(p0, p0.y < 0.0);
        offset_point_from_xz_plane(p1, p1.y < 0.0);
        offset_point_from_xz_plane(p2, p2.y < 0.0);
        return;
    }

    let p0y = p0.y.abs();
    let p1y = p1.y.abs();
    let p2y = p2.y.abs();

    let sign;
    if p0y > p1y {
        if p0y > p2y {
            sign = CesiumMath::sign(p0.y);
        } else {
            sign = CesiumMath::sign(p2.y);
        }
    } else if p1y > p2y {
        sign = CesiumMath::sign(p1.y);
    } else {
        sign = CesiumMath::sign(p2.y);
    }

    let is_behind = sign < 0.0;
    offset_point_from_xz_plane(p0, is_behind);
    offset_point_from_xz_plane(p1, is_behind);
    offset_point_from_xz_plane(p2, is_behind);
}

/// `getXZIntersectionOffsetPoints` – computes the intersection of the segment
/// `p`–`p1` with the XZ plane and two nudged copies of it (`u1` behind the
/// plane, `v1` in front of it).
fn get_xz_intersection_offset_points(
    p: &Cartesian3,
    p1: &Cartesian3,
    u1: &mut Cartesian3,
    v1: &mut Cartesian3,
) {
    let mut diff = Cartesian3::ZERO;
    Cartesian3::subtract(p1, p, &mut diff);
    let mut scaled = Cartesian3::ZERO;
    Cartesian3::multiply_by_scalar(&diff, p.y / (p.y - p1.y), &mut scaled);
    Cartesian3::add(p, &scaled, u1);
    *v1 = *u1;
    offset_point_from_xz_plane(u1, true);
    offset_point_from_xz_plane(v1, false);
}

// ---------------------------------------------------------------------------
// splitTriangle
// ---------------------------------------------------------------------------

/// Result of [`split_triangle`]: up to 7 positions (`p0`, `p1`, `p2` plus the
/// intersection/nudged points `u1`, `u2`, `q1`, `q2`) and 9 result indices
/// referencing them.
struct SplitTriangleResult {
    positions: [Cartesian3; 7],
    positions_length: usize,
    indices: [usize; 9],
}

/// `splitTriangle` – splits a triangle by the XZ plane (in IDL-local
/// coordinates) if needed. Returns `None` when the triangle cannot cross the
/// international date line.
fn split_triangle(
    p0: &mut Cartesian3,
    p1: &mut Cartesian3,
    p2: &mut Cartesian3,
) -> Option<SplitTriangleResult> {
    // In ellipsoid coordinates, for a triangle approximately on the
    // ellipsoid to cross the IDL, first it needs to be on the negative side
    // of the plane x = 0.
    if p0.x >= 0.0 || p1.x >= 0.0 || p2.x >= 0.0 {
        return None;
    }

    offset_triangle_from_xz_plane(p0, p1, p2);

    let p0_behind = p0.y < 0.0;
    let p1_behind = p1.y < 0.0;
    let p2_behind = p2.y < 0.0;

    let mut num_behind = 0;
    num_behind += if p0_behind { 1 } else { 0 };
    num_behind += if p1_behind { 1 } else { 0 };
    num_behind += if p2_behind { 1 } else { 0 };

    let mut indices = [0usize; 9];
    let mut u1 = Cartesian3::ZERO;
    let mut u2 = Cartesian3::ZERO;
    let mut q1 = Cartesian3::ZERO;
    let mut q2 = Cartesian3::ZERO;

    if num_behind == 1 {
        indices[1] = 3;
        indices[2] = 4;
        indices[5] = 6;
        indices[7] = 6;
        indices[8] = 5;

        if p0_behind {
            get_xz_intersection_offset_points(p0, p1, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p0, p2, &mut u2, &mut q2);

            indices[0] = 0;
            indices[3] = 1;
            indices[4] = 2;
            indices[6] = 1;
        } else if p1_behind {
            get_xz_intersection_offset_points(p1, p2, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p1, p0, &mut u2, &mut q2);

            indices[0] = 1;
            indices[3] = 2;
            indices[4] = 0;
            indices[6] = 2;
        } else if p2_behind {
            get_xz_intersection_offset_points(p2, p0, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p2, p1, &mut u2, &mut q2);

            indices[0] = 2;
            indices[3] = 0;
            indices[4] = 1;
            indices[6] = 0;
        }
    } else if num_behind == 2 {
        indices[2] = 4;
        indices[4] = 4;
        indices[5] = 3;
        indices[7] = 5;
        indices[8] = 6;

        if !p0_behind {
            get_xz_intersection_offset_points(p0, p1, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p0, p2, &mut u2, &mut q2);

            indices[0] = 1;
            indices[1] = 2;
            indices[3] = 1;
            indices[6] = 0;
        } else if !p1_behind {
            get_xz_intersection_offset_points(p1, p2, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p1, p0, &mut u2, &mut q2);

            indices[0] = 2;
            indices[1] = 0;
            indices[3] = 2;
            indices[6] = 1;
        } else if !p2_behind {
            get_xz_intersection_offset_points(p2, p0, &mut u1, &mut q1);
            get_xz_intersection_offset_points(p2, p1, &mut u2, &mut q2);

            indices[0] = 0;
            indices[1] = 1;
            indices[3] = 0;
            indices[6] = 2;
        }
    }

    let mut positions_length = 3;
    if num_behind == 1 || num_behind == 2 {
        positions_length = 7;
    }

    Some(SplitTriangleResult {
        positions: [*p0, *p1, *p2, u1, u2, q1, q2],
        positions_length,
        indices,
    })
}

// ---------------------------------------------------------------------------
// Split geometry bookkeeping
// ---------------------------------------------------------------------------

/// Intermediate representation of a hemisphere geometry under construction.
/// Attribute values grow with `push`; indices accumulate as `u32` and are
/// narrowed to the appropriate [`IndexStorage`] width in
/// [`update_geometry_after_split`] (mirrors JS `createTypedArray` at the end).
struct SplitGeometry {
    attributes: HashMap<String, GeometryAttribute>,
    indices: Vec<u32>,
    primitive_type: PrimitiveType,
}

/// `copyGeometryForSplit` – creates an empty geometry with the same attribute
/// layout (and primitive type) as the source.
fn copy_geometry_for_split(geometry: &Geometry) -> SplitGeometry {
    let mut copied_attributes = HashMap::new();

    for (name, attribute) in &geometry.attributes {
        copied_attributes.insert(
            name.clone(),
            GeometryAttribute::new(
                attribute.component_datatype,
                attribute.components_per_attribute,
                attribute.normalize,
                Vec::new(),
            ),
        );
    }

    SplitGeometry {
        attributes: copied_attributes,
        indices: Vec::new(),
        primitive_type: geometry.primitive_type,
    }
}

/// `updateGeometryAfterSplit` – finalizes a split geometry: returns `None` if
/// it ended up empty, otherwise converts the accumulated indices to the
/// proper width and optionally recomputes the bounding sphere.
fn update_geometry_after_split(
    geometry: SplitGeometry,
    compute_bounding_sphere: bool,
) -> Option<Geometry> {
    let position_attribute = geometry.attributes.get("position")?;
    if position_attribute.values.is_empty() {
        return None;
    }

    // DEVIATION: JS re-wraps every `attribute.values` with
    // `ComponentDatatype.createTypedArray` here; our `Vec<f64>` domain
    // representation needs no conversion.

    let number_of_vertices =
        position_attribute.values.len() / position_attribute.components_per_attribute as usize;

    let mut indices = IndexDatatype::create_typed_array(number_of_vertices, geometry.indices.len());
    for (k, value) in geometry.indices.iter().enumerate() {
        write_index(&mut indices, k, *value);
    }

    let bounding_sphere = if compute_bounding_sphere {
        let position_values = position_attribute.values.clone();
        Some(BoundingSphere::from_vertices(
            &position_values,
            None,
            None,
            None,
        ))
    } else {
        None
    };

    Some(Geometry::new(
        geometry.attributes,
        Some(indices),
        Some(geometry.primitive_type),
        bounding_sphere,
    ))
}

/// `updateInstanceAfterSplit` – assigns the split results back to the
/// instance.
fn update_instance_after_split(
    instance: &mut GeometryInstance,
    west_geometry: SplitGeometry,
    east_geometry: SplitGeometry,
    compute_bounding_sphere: bool,
) {
    let west_geometry = update_geometry_after_split(west_geometry, compute_bounding_sphere);
    let east_geometry = update_geometry_after_split(east_geometry, compute_bounding_sphere);

    match (west_geometry, east_geometry) {
        (None, Some(east)) => {
            instance.geometry = GeometryInstanceGeometry::Geometry(Box::new(east));
        }
        (Some(west), None) => {
            instance.geometry = GeometryInstanceGeometry::Geometry(Box::new(west));
        }
        (west, east) => {
            // Both defined — or neither defined (mirrors the JS final `else`).
            instance.west_hemisphere_geometry = west;
            instance.east_hemisphere_geometry = east;
            // `instance.geometry` already holds `Placeholder` (JS `undefined`).
        }
    }
}

/// Takes the geometry out of the instance, leaving `Placeholder` (JS
/// `undefined`) behind.
fn take_geometry(instance: &mut GeometryInstance) -> Option<Geometry> {
    match std::mem::replace(
        &mut instance.geometry,
        GeometryInstanceGeometry::Placeholder,
    ) {
        GeometryInstanceGeometry::Geometry(geometry) => Some(*geometry),
        GeometryInstanceGeometry::Placeholder => None,
    }
}

// ---------------------------------------------------------------------------
// Barycentric attribute interpolation
// ---------------------------------------------------------------------------

/// Rust port of the functions generated by
/// `generateBarycentricInterpolateFunction` (`interpolateAndPackCartesian4` /
/// `Cartesian3` / `Cartesian2`), generalized over the component count.
fn interpolate_and_pack(
    i0: usize,
    i1: usize,
    i2: usize,
    coords: &Cartesian3,
    source_values: &[f64],
    current_values: &mut Vec<f64>,
    number_of_components: usize,
    normalize: bool,
) {
    let mut value = [0.0f64; 4];
    for c in 0..number_of_components {
        value[c] = source_values[i0 * number_of_components + c] * coords.x
            + source_values[i1 * number_of_components + c] * coords.y
            + source_values[i2 * number_of_components + c] * coords.z;
    }

    if normalize {
        let mut magnitude_squared = 0.0;
        for c in 0..number_of_components {
            magnitude_squared += value[c] * value[c];
        }
        // Mirrors JS `CartesianX.normalize`, which divides unconditionally.
        let magnitude = magnitude_squared.sqrt();
        for c in 0..number_of_components {
            value[c] /= magnitude;
        }
    }

    current_values.extend_from_slice(&value[..number_of_components]);
}

/// `interpolateAndPackBoolean` – interpolates the single-component
/// `applyOffset` attribute and thresholds it at `EPSILON6`.
fn interpolate_and_pack_boolean(
    i0: usize,
    i1: usize,
    i2: usize,
    coords: &Cartesian3,
    source_values: &[f64],
    current_values: &mut Vec<f64>,
) {
    let v1 = source_values[i0] * coords.x;
    let v2 = source_values[i1] * coords.y;
    let v3 = source_values[i2] * coords.z;
    current_values.push(if v1 + v2 + v3 > CesiumMath::EPSILON6 {
        1.0
    } else {
        0.0
    });
}

/// DEVIATION: when a barycentric interpolation is skipped (undefined
/// coordinates) JS leaves array holes that become `0` when the values are
/// re-created as typed arrays; this push-based port appends explicit zeros so
/// every attribute keeps the same vertex count. `position` was already pushed
/// by [`insert_split_point`].
fn push_skipped_vertex(current: &mut SplitGeometry) {
    for (name, attribute) in current.attributes.iter_mut() {
        if name == "position" {
            continue;
        }
        attribute.values.extend(
            std::iter::repeat(0.0).take(attribute.components_per_attribute as usize),
        );
    }
}

/// `computeTriangleAttributes` – interpolates all per-vertex attributes for a
/// point inserted into a split triangle.
#[allow(clippy::too_many_arguments)]
fn compute_triangle_attributes(
    i0: usize,
    i1: usize,
    i2: usize,
    point: &Cartesian3,
    positions: &[f64],
    normals: Option<&[f64]>,
    tangents: Option<&[f64]>,
    bitangents: Option<&[f64]>,
    tex_coords: Option<&[f64]>,
    extrude_directions: Option<&[f64]>,
    apply_offset: Option<&[f64]>,
    current: &mut SplitGeometry,
    custom_attribute_names: &[String],
    all_attributes: &HashMap<String, GeometryAttribute>,
    inserted_index: usize,
) {
    // Push-based growth keeps the insertion position implicit.
    let _ = inserted_index;

    if normals.is_none()
        && tangents.is_none()
        && bitangents.is_none()
        && tex_coords.is_none()
        && extrude_directions.is_none()
        && custom_attribute_names.is_empty()
    {
        return;
    }

    let mut p0 = Cartesian3::ZERO;
    let mut p1 = Cartesian3::ZERO;
    let mut p2 = Cartesian3::ZERO;
    Cartesian3::from_array(positions, Some(i0 * 3), &mut p0);
    Cartesian3::from_array(positions, Some(i1 * 3), &mut p1);
    Cartesian3::from_array(positions, Some(i2 * 3), &mut p2);

    let Some(coords) = barycentric_coordinates_3d(point, &p0, &p1, &p2) else {
        push_skipped_vertex(current);
        return;
    };

    if let Some(normals) = normals {
        let values = &mut current.attributes.get_mut("normal").unwrap().values;
        interpolate_and_pack(i0, i1, i2, &coords, normals, values, 3, true);
    }

    if let Some(extrude_directions) = extrude_directions {
        let mut d0 = [0.0f64; 3];
        let mut d1 = [0.0f64; 3];
        let mut d2 = [0.0f64; 3];
        for c in 0..3 {
            d0[c] = extrude_directions[i0 * 3 + c] * coords.x;
            d1[c] = extrude_directions[i1 * 3 + c] * coords.y;
            d2[c] = extrude_directions[i2 * 3 + c] * coords.z;
        }

        let mut direction;
        if d0 != [0.0; 3] || d1 != [0.0; 3] || d2 != [0.0; 3] {
            direction = [d0[0] + d1[0] + d2[0], d0[1] + d1[1] + d2[1], d0[2] + d1[2] + d2[2]];
            // Mirrors JS `Cartesian3.normalize`, which divides unconditionally.
            let magnitude =
                (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
                    .sqrt();
            direction[0] /= magnitude;
            direction[1] /= magnitude;
            direction[2] /= magnitude;
        } else {
            direction = [0.0; 3];
        }

        let values = &mut current
            .attributes
            .get_mut("extrudeDirection")
            .unwrap()
            .values;
        values.extend_from_slice(&direction);
    }

    if let Some(apply_offset) = apply_offset {
        let values = &mut current.attributes.get_mut("applyOffset").unwrap().values;
        interpolate_and_pack_boolean(i0, i1, i2, &coords, apply_offset, values);
    }

    if let Some(tangents) = tangents {
        let values = &mut current.attributes.get_mut("tangent").unwrap().values;
        interpolate_and_pack(i0, i1, i2, &coords, tangents, values, 3, true);
    }

    if let Some(bitangents) = bitangents {
        let values = &mut current.attributes.get_mut("bitangent").unwrap().values;
        interpolate_and_pack(i0, i1, i2, &coords, bitangents, values, 3, true);
    }

    if let Some(tex_coords) = tex_coords {
        let values = &mut current.attributes.get_mut("st").unwrap().values;
        interpolate_and_pack(i0, i1, i2, &coords, tex_coords, values, 2, false);
    }

    // DEVIATION: JS iterates custom attribute names in object insertion
    // order; `HashMap` order differs, but interpolation results are
    // independent per attribute.
    for attribute_name in custom_attribute_names {
        generic_interpolate(
            i0,
            i1,
            i2,
            &coords,
            &all_attributes[attribute_name],
            &mut current.attributes.get_mut(attribute_name).unwrap().values,
        );
    }
}

/// `genericInterpolate` – interpolates an arbitrary custom attribute based on
/// its component count.
fn generic_interpolate(
    i0: usize,
    i1: usize,
    i2: usize,
    coords: &Cartesian3,
    source_attribute: &GeometryAttribute,
    current_values: &mut Vec<f64>,
) {
    let components_per_attribute = source_attribute.components_per_attribute;
    let source_values = &source_attribute.values;
    match components_per_attribute {
        4 => interpolate_and_pack(i0, i1, i2, coords, source_values, current_values, 4, false),
        3 => interpolate_and_pack(i0, i1, i2, coords, source_values, current_values, 3, false),
        2 => interpolate_and_pack(i0, i1, i2, coords, source_values, current_values, 2, false),
        _ => {
            current_values.push(
                source_values[i0] * coords.x
                    + source_values[i1] * coords.y
                    + source_values[i2] * coords.z,
            );
        }
    }
}

/// `insertSplitPoint` – adds a point to the split geometry being built (with
/// index remapping for original vertices). Returns the inserted vertex index.
fn insert_split_point(
    current: &mut SplitGeometry,
    index_map: &mut [i64],
    indices: &IndexStorage,
    current_index: isize,
    point: &Cartesian3,
) -> usize {
    let insert_index = current.attributes["position"].values.len() / 3;

    if current_index >= 0 {
        let prev_index = read_index(indices, current_index as usize) as usize;
        let new_index = index_map[prev_index];

        if new_index == -1 {
            index_map[prev_index] = insert_index as i64;
            push_position(current, point);
            current.indices.push(insert_index as u32);
            return insert_index;
        }

        current.indices.push(new_index as u32);
        return new_index as usize;
    }

    push_position(current, point);
    current.indices.push(insert_index as u32);
    insert_index
}

fn push_position(current: &mut SplitGeometry, point: &Cartesian3) {
    let values = &mut current.attributes.get_mut("position").unwrap().values;
    values.push(point.x);
    values.push(point.y);
    values.push(point.z);
}

/// JS `NAMED_ATTRIBUTES` table.
fn is_named_attribute(name: &str) -> bool {
    matches!(
        name,
        "position" | "normal" | "bitangent" | "tangent" | "st" | "extrudeDirection" | "applyOffset"
    )
}

// ---------------------------------------------------------------------------
// splitLongitudeTriangles
// ---------------------------------------------------------------------------

/// `splitLongitudeTriangles` – splits indexed triangle geometry at the IDL.
fn split_longitude_triangles(instance: &mut GeometryInstance, geometry: Geometry) {
    let compute_bounding_sphere = geometry.bounding_sphere.is_some();
    let attributes = &geometry.attributes;
    let positions = attributes["position"].values.clone();
    let normals = attributes.get("normal").map(|a| a.values.clone());
    let bitangents = attributes.get("bitangent").map(|a| a.values.clone());
    let tangents = attributes.get("tangent").map(|a| a.values.clone());
    let tex_coords = attributes.get("st").map(|a| a.values.clone());
    let extrude_directions = attributes.get("extrudeDirection").map(|a| a.values.clone());
    let apply_offset = attributes.get("applyOffset").map(|a| a.values.clone());
    let indices = match geometry.indices.clone() {
        Some(indices) => indices,
        None => {
            if cfg!(debug_assertions) {
                throw_developer_error("geometry.indices is required.");
            }
            return;
        }
    };

    let custom_attribute_names: Vec<String> = attributes
        .keys()
        .filter(|name| !is_named_attribute(name))
        .cloned()
        .collect();

    let mut east_geometry = copy_geometry_for_split(&geometry);
    let mut west_geometry = copy_geometry_for_split(&geometry);

    let vertex_count = positions.len() / 3;
    let mut west_geometry_index_map = vec![-1i64; vertex_count];
    let mut east_geometry_index_map = vec![-1i64; vertex_count];

    let len = indices.len();
    let mut i = 0;
    while i < len {
        let i0 = read_index(&indices, i) as usize;
        let i1 = read_index(&indices, i + 1) as usize;
        let i2 = read_index(&indices, i + 2) as usize;

        let mut p0 = cartesian3_at(&positions, i0);
        let mut p1 = cartesian3_at(&positions, i1);
        let mut p2 = cartesian3_at(&positions, i2);

        match split_triangle(&mut p0, &mut p1, &mut p2) {
            Some(result) if result.positions_length > 3 => {
                for result_index in result.indices {
                    let point = result.positions[result_index];

                    let (current, current_index_map) = if point.y < 0.0 {
                        (&mut west_geometry, &mut west_geometry_index_map)
                    } else {
                        (&mut east_geometry, &mut east_geometry_index_map)
                    };

                    let current_index = if result_index < 3 {
                        (i + result_index) as isize
                    } else {
                        -1
                    };
                    let inserted_index = insert_split_point(
                        current,
                        current_index_map,
                        &indices,
                        current_index,
                        &point,
                    );
                    compute_triangle_attributes(
                        i0,
                        i1,
                        i2,
                        &point,
                        &positions,
                        normals.as_deref(),
                        tangents.as_deref(),
                        bitangents.as_deref(),
                        tex_coords.as_deref(),
                        extrude_directions.as_deref(),
                        apply_offset.as_deref(),
                        current,
                        &custom_attribute_names,
                        attributes,
                        inserted_index,
                    );
                }
            }
            result => {
                // Either no split occurred, or all three vertices landed in
                // the same hemisphere.
                let points = match &result {
                    Some(r) => [r.positions[0], r.positions[1], r.positions[2]],
                    None => [p0, p1, p2],
                };

                let is_west = points[0].y < 0.0;
                let (current, current_index_map) = if is_west {
                    (&mut west_geometry, &mut west_geometry_index_map)
                } else {
                    (&mut east_geometry, &mut east_geometry_index_map)
                };

                for (k, point) in points.iter().enumerate() {
                    let inserted_index = insert_split_point(
                        current,
                        current_index_map,
                        &indices,
                        (i + k) as isize,
                        point,
                    );
                    compute_triangle_attributes(
                        i0,
                        i1,
                        i2,
                        point,
                        &positions,
                        normals.as_deref(),
                        tangents.as_deref(),
                        bitangents.as_deref(),
                        tex_coords.as_deref(),
                        extrude_directions.as_deref(),
                        apply_offset.as_deref(),
                        current,
                        &custom_attribute_names,
                        attributes,
                        inserted_index,
                    );
                }
            }
        }

        i += 3;
    }

    update_instance_after_split(
        instance,
        west_geometry,
        east_geometry,
        compute_bounding_sphere,
    );
}

// ---------------------------------------------------------------------------
// splitLongitudeLines
// ---------------------------------------------------------------------------

/// `computeLineAttributes` – copies the `applyOffset` value of the endpoint
/// closest to the inserted point.
fn compute_line_attributes(
    i0: usize,
    i1: usize,
    point: &Cartesian3,
    positions: &[f64],
    insert_index: usize,
    current: &mut SplitGeometry,
    apply_offset: Option<&[f64]>,
) {
    // Push-based growth keeps the insertion position implicit.
    let _ = insert_index;
    let Some(apply_offset) = apply_offset else {
        return;
    };

    let mut p0 = Cartesian3::ZERO;
    Cartesian3::from_array(positions, Some(i0 * 3), &mut p0);
    let values = &mut current.attributes.get_mut("applyOffset").unwrap().values;
    if Cartesian3::equals_epsilon(
        Some(&p0),
        Some(point),
        Some(CesiumMath::EPSILON10),
        Some(CesiumMath::EPSILON10),
    ) {
        values.push(apply_offset[i0]);
    } else {
        values.push(apply_offset[i1]);
    }
}

/// `splitLongitudeLines` – splits indexed line geometry at the IDL.
fn split_longitude_lines(instance: &mut GeometryInstance, geometry: Geometry) {
    let compute_bounding_sphere = geometry.bounding_sphere.is_some();
    let positions = geometry.attributes["position"].values.clone();
    let apply_offset = geometry
        .attributes
        .get("applyOffset")
        .map(|a| a.values.clone());
    let indices = match geometry.indices.clone() {
        Some(indices) => indices,
        None => {
            if cfg!(debug_assertions) {
                throw_developer_error("geometry.indices is required.");
            }
            return;
        }
    };

    let mut east_geometry = copy_geometry_for_split(&geometry);
    let mut west_geometry = copy_geometry_for_split(&geometry);

    let vertex_count = positions.len() / 3;
    let mut west_geometry_index_map = vec![-1i64; vertex_count];
    let mut east_geometry_index_map = vec![-1i64; vertex_count];

    // JS module-level `xzPlane`.
    let xz_plane = Plane::from_point_normal_new(&Cartesian3::ZERO, &Cartesian3::UNIT_Y);

    let length = indices.len();
    let mut i = 0;
    while i < length {
        let i0 = read_index(&indices, i) as usize;
        let i1 = read_index(&indices, i + 1) as usize;

        let mut p0 = cartesian3_at(&positions, i0);
        let mut p1 = cartesian3_at(&positions, i1);

        if p0.y.abs() < CesiumMath::EPSILON6 {
            p0.y = if p0.y < 0.0 {
                -CesiumMath::EPSILON6
            } else {
                CesiumMath::EPSILON6
            };
        }

        if p1.y.abs() < CesiumMath::EPSILON6 {
            p1.y = if p1.y < 0.0 {
                -CesiumMath::EPSILON6
            } else {
                CesiumMath::EPSILON6
            };
        }

        let intersection = IntersectionTests::line_segment_plane(&p0, &p1, &xz_plane);
        if let Some(intersection) = intersection {
            // Move point on the xz-plane slightly away from the plane.
            let mut offset = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(
                &Cartesian3::UNIT_Y,
                5.0 * CesiumMath::EPSILON9,
                &mut offset,
            );
            let p0_behind = p0.y < 0.0;
            if p0_behind {
                // DEVIATION: JS negates in place; Rust needs a temporary.
                let mut negated = Cartesian3::ZERO;
                Cartesian3::negate(&offset, &mut negated);
                offset = negated;
            }
            let (p0_geometry, p0_index_map, p1_geometry, p1_index_map) = if p0_behind {
                (
                    &mut west_geometry,
                    &mut west_geometry_index_map,
                    &mut east_geometry,
                    &mut east_geometry_index_map,
                )
            } else {
                (
                    &mut east_geometry,
                    &mut east_geometry_index_map,
                    &mut west_geometry,
                    &mut west_geometry_index_map,
                )
            };

            let mut offset_point = Cartesian3::ZERO;
            Cartesian3::add(&intersection, &offset, &mut offset_point);

            let mut insert_index =
                insert_split_point(p0_geometry, p0_index_map, &indices, i as isize, &p0);
            compute_line_attributes(
                i0,
                i1,
                &p0,
                &positions,
                insert_index,
                p0_geometry,
                apply_offset.as_deref(),
            );

            insert_index = insert_split_point(p0_geometry, p0_index_map, &indices, -1, &offset_point);
            compute_line_attributes(
                i0,
                i1,
                &offset_point,
                &positions,
                insert_index,
                p0_geometry,
                apply_offset.as_deref(),
            );

            // DEVIATION: JS negates in place; Rust needs a temporary.
            let mut negated = Cartesian3::ZERO;
            Cartesian3::negate(&offset, &mut negated);
            offset = negated;
            Cartesian3::add(&intersection, &offset, &mut offset_point);
            insert_index = insert_split_point(p1_geometry, p1_index_map, &indices, -1, &offset_point);
            compute_line_attributes(
                i0,
                i1,
                &offset_point,
                &positions,
                insert_index,
                p1_geometry,
                apply_offset.as_deref(),
            );

            insert_index = insert_split_point(
                p1_geometry,
                p1_index_map,
                &indices,
                (i + 1) as isize,
                &p1,
            );
            compute_line_attributes(
                i0,
                i1,
                &p1,
                &positions,
                insert_index,
                p1_geometry,
                apply_offset.as_deref(),
            );
        } else {
            let is_west = p0.y < 0.0;
            let (current, current_index_map) = if is_west {
                (&mut west_geometry, &mut west_geometry_index_map)
            } else {
                (&mut east_geometry, &mut east_geometry_index_map)
            };

            let mut insert_index =
                insert_split_point(current, current_index_map, &indices, i as isize, &p0);
            compute_line_attributes(
                i0,
                i1,
                &p0,
                &positions,
                insert_index,
                current,
                apply_offset.as_deref(),
            );

            insert_index =
                insert_split_point(current, current_index_map, &indices, (i + 1) as isize, &p1);
            compute_line_attributes(
                i0,
                i1,
                &p1,
                &positions,
                insert_index,
                current,
                apply_offset.as_deref(),
            );
        }

        i += 2;
    }

    update_instance_after_split(
        instance,
        west_geometry,
        east_geometry,
        compute_bounding_sphere,
    );
}

// ---------------------------------------------------------------------------
// splitLongitudePolyline
// ---------------------------------------------------------------------------

/// `updateAdjacencyAfterSplit` – fixes `prevPosition`/`nextPosition`
/// attributes of a split polyline that cross the XZ plane.
fn update_adjacency_after_split(geometry: &mut SplitGeometry) {
    let positions = geometry.attributes["position"].values.clone();
    let mut prev_positions = std::mem::take(
        &mut geometry.attributes.get_mut("prevPosition").unwrap().values,
    );
    let mut next_positions = std::mem::take(
        &mut geometry.attributes.get_mut("nextPosition").unwrap().values,
    );

    let length = positions.len();
    let mut j = 0;
    while j < length {
        let position_x = positions[j];
        let position_y = positions[j + 1];
        if position_x > 0.0 {
            j += 3;
            continue;
        }

        let prev_position_y = prev_positions[j + 1];
        if (position_y < 0.0 && prev_position_y > 0.0)
            || (position_y > 0.0 && prev_position_y < 0.0)
        {
            // JS `j - 3 > 0` (strict).
            if j > 3 {
                prev_positions[j] = positions[j - 3];
                prev_positions[j + 1] = positions[j - 2];
                prev_positions[j + 2] = positions[j - 1];
            } else {
                prev_positions[j] = positions[j];
                prev_positions[j + 1] = positions[j + 1];
                prev_positions[j + 2] = positions[j + 2];
            }
        }

        let next_position_y = next_positions[j + 1];
        if (position_y < 0.0 && next_position_y > 0.0)
            || (position_y > 0.0 && next_position_y < 0.0)
        {
            if j + 3 < length {
                next_positions[j] = positions[j + 3];
                next_positions[j + 1] = positions[j + 4];
                next_positions[j + 2] = positions[j + 5];
            } else {
                next_positions[j] = positions[j];
                next_positions[j + 1] = positions[j + 1];
                next_positions[j + 2] = positions[j + 2];
            }
        }

        j += 3;
    }

    geometry.attributes.get_mut("prevPosition").unwrap().values = prev_positions;
    geometry.attributes.get_mut("nextPosition").unwrap().values = next_positions;
}

/// `splitLongitudePolyline` – splits polyline volumes (four vertices per
/// segment) at the IDL.
fn split_longitude_polyline(instance: &mut GeometryInstance, geometry: Geometry) {
    let compute_bounding_sphere = geometry.bounding_sphere.is_some();
    // DEVIATION: JS mutates the instance's own attribute arrays in place for
    // the coplanar offsets below; the original geometry is discarded after
    // the split, so cloning the sources produces identical output.
    let mut positions = geometry.attributes["position"].values.clone();
    let mut prev_positions = geometry.attributes["prevPosition"].values.clone();
    let mut next_positions = geometry.attributes["nextPosition"].values.clone();
    let expand_and_widths = geometry.attributes["expandAndWidth"].values.clone();

    let tex_coords = geometry.attributes.get("st").map(|a| a.values.clone());
    let colors = geometry.attributes.get("color").map(|a| a.values.clone());

    let mut east_geometry = copy_geometry_for_split(&geometry);
    let mut west_geometry = copy_geometry_for_split(&geometry);

    // JS module-level `xzPlane`.
    let xz_plane = Plane::from_point_normal_new(&Cartesian3::ZERO, &Cartesian3::UNIT_Y);

    let offset_scalar = 5.0 * CesiumMath::EPSILON9;
    let coplanar_offset = CesiumMath::EPSILON6;

    let mut intersection_found = false;

    let length = positions.len() / 3;
    let mut i = 0;
    while i < length {
        let i0 = i;
        let i2 = i + 2;

        let mut p0 = cartesian3_at(&positions, i0);
        let mut p2 = cartesian3_at(&positions, i2);

        // Offset points that are close to the 180 longitude and change the
        // previous/next point to be the same offset point so it can be
        // projected to 2D. There is special handling in the shader for when
        // position == prevPosition || position == nextPosition.
        if p0.y.abs() < coplanar_offset {
            p0.y = coplanar_offset * if p2.y < 0.0 { -1.0 } else { 1.0 };
            positions[i * 3 + 1] = p0.y;
            positions[(i + 1) * 3 + 1] = p0.y;

            let mut j = i0 * 3;
            while j < i0 * 3 + 4 * 3 {
                prev_positions[j] = positions[i * 3];
                prev_positions[j + 1] = positions[i * 3 + 1];
                prev_positions[j + 2] = positions[i * 3 + 2];
                j += 3;
            }
        }

        // Do the same but for when the line crosses 180 longitude in the
        // opposite direction.
        if p2.y.abs() < coplanar_offset {
            p2.y = coplanar_offset * if p0.y < 0.0 { -1.0 } else { 1.0 };
            positions[(i + 2) * 3 + 1] = p2.y;
            positions[(i + 3) * 3 + 1] = p2.y;

            let mut j = i0 * 3;
            while j < i0 * 3 + 4 * 3 {
                next_positions[j] = positions[(i + 2) * 3];
                next_positions[j + 1] = positions[(i + 2) * 3 + 1];
                next_positions[j + 2] = positions[(i + 2) * 3 + 2];
                j += 3;
            }
        }

        let intersection = IntersectionTests::line_segment_plane(&p0, &p2, &xz_plane);
        if let Some(intersection) = intersection {
            intersection_found = true;

            // Move point on the xz-plane slightly away from the plane.
            let mut offset = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(&Cartesian3::UNIT_Y, offset_scalar, &mut offset);
            let p0_behind = p0.y < 0.0;
            if p0_behind {
                // DEVIATION: JS negates in place; Rust needs a temporary.
                let mut negated = Cartesian3::ZERO;
                Cartesian3::negate(&offset, &mut negated);
                offset = negated;
            }
            let (p0_geometry, p2_geometry) = if p0_behind {
                (&mut west_geometry, &mut east_geometry)
            } else {
                (&mut east_geometry, &mut west_geometry)
            };

            let mut offset_point = Cartesian3::ZERO;
            Cartesian3::add(&intersection, &offset, &mut offset_point);

            {
                let values = &mut p0_geometry.attributes.get_mut("position").unwrap().values;
                values.extend_from_slice(&[p0.x, p0.y, p0.z, p0.x, p0.y, p0.z]);
                values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
                values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
            }
            {
                let values = &mut p0_geometry.attributes.get_mut("prevPosition").unwrap().values;
                values.extend_from_slice(&prev_positions[i0 * 3..i0 * 3 + 3]);
                values.extend_from_slice(&prev_positions[i0 * 3 + 3..i0 * 3 + 6]);
                values.extend_from_slice(&[p0.x, p0.y, p0.z, p0.x, p0.y, p0.z]);
            }
            {
                let values = &mut p0_geometry.attributes.get_mut("nextPosition").unwrap().values;
                for _ in 0..4 {
                    values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
                }
            }

            // DEVIATION: JS negates in place; Rust needs a temporary.
            let mut negated = Cartesian3::ZERO;
            Cartesian3::negate(&offset, &mut negated);
            offset = negated;
            Cartesian3::add(&intersection, &offset, &mut offset_point);

            {
                let values = &mut p2_geometry.attributes.get_mut("position").unwrap().values;
                values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
                values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
                values.extend_from_slice(&[p2.x, p2.y, p2.z, p2.x, p2.y, p2.z]);
            }
            {
                let values = &mut p2_geometry.attributes.get_mut("prevPosition").unwrap().values;
                for _ in 0..4 {
                    values.extend_from_slice(&[offset_point.x, offset_point.y, offset_point.z]);
                }
            }
            {
                let values = &mut p2_geometry.attributes.get_mut("nextPosition").unwrap().values;
                values.extend_from_slice(&[p2.x, p2.y, p2.z, p2.x, p2.y, p2.z]);
                values.extend_from_slice(&next_positions[i2 * 3..i2 * 3 + 3]);
                values.extend_from_slice(&next_positions[i2 * 3 + 3..i2 * 3 + 6]);
            }

            let width = expand_and_widths[i0 * 2 + 1].abs();

            {
                let values = &mut p0_geometry.attributes.get_mut("expandAndWidth").unwrap().values;
                values.extend_from_slice(&[-1.0, width, 1.0, width]);
                values.extend_from_slice(&[-1.0, -width, 1.0, -width]);
            }
            {
                let values = &mut p2_geometry.attributes.get_mut("expandAndWidth").unwrap().values;
                values.extend_from_slice(&[-1.0, width, 1.0, width]);
                values.extend_from_slice(&[-1.0, -width, 1.0, -width]);
            }

            let mut diff = Cartesian3::ZERO;
            Cartesian3::subtract(&intersection, &p0, &mut diff);
            let mut t = Cartesian3::magnitude_squared(&diff);
            Cartesian3::subtract(&p2, &p0, &mut diff);
            t /= Cartesian3::magnitude_squared(&diff);

            if let Some(colors) = &colors {
                let c0 = &colors[i0 * 4..i0 * 4 + 4];
                let c2 = &colors[i2 * 4..i2 * 4 + 4];

                let r = CesiumMath::lerp(c0[0], c2[0], t);
                let g = CesiumMath::lerp(c0[1], c2[1], t);
                let b = CesiumMath::lerp(c0[2], c2[2], t);
                let a = CesiumMath::lerp(c0[3], c2[3], t);

                {
                    let values = &mut p0_geometry.attributes.get_mut("color").unwrap().values;
                    values.extend_from_slice(&colors[i0 * 4..i0 * 4 + 2 * 4]);
                    values.extend_from_slice(&[r, g, b, a]);
                    values.extend_from_slice(&[r, g, b, a]);
                }
                {
                    let values = &mut p2_geometry.attributes.get_mut("color").unwrap().values;
                    values.extend_from_slice(&[r, g, b, a]);
                    values.extend_from_slice(&[r, g, b, a]);
                    values.extend_from_slice(&colors[i2 * 4..i2 * 4 + 2 * 4]);
                }
            }

            if let Some(tex_coords) = &tex_coords {
                let s0_x = tex_coords[i0 * 2];
                let s0_y = tex_coords[i0 * 2 + 1];
                let s3_x = tex_coords[(i + 3) * 2];
                let s3_y = tex_coords[(i + 3) * 2 + 1];

                let sx = CesiumMath::lerp(s0_x, s3_x, t);

                {
                    let values = &mut p0_geometry.attributes.get_mut("st").unwrap().values;
                    values.extend_from_slice(&tex_coords[i0 * 2..i0 * 2 + 2 * 2]);
                    values.extend_from_slice(&[sx, s0_y]);
                    values.extend_from_slice(&[sx, s3_y]);
                }
                {
                    let values = &mut p2_geometry.attributes.get_mut("st").unwrap().values;
                    values.extend_from_slice(&[sx, s0_y]);
                    values.extend_from_slice(&[sx, s3_y]);
                    values.extend_from_slice(&tex_coords[i2 * 2..i2 * 2 + 2 * 2]);
                }
            }

            let mut index = p0_geometry.attributes["position"].values.len() / 3 - 4;
            p0_geometry
                .indices
                .extend([index, index + 2, index + 1, index + 1, index + 2, index + 3].map(|v| v as u32));

            index = p2_geometry.attributes["position"].values.len() / 3 - 4;
            p2_geometry
                .indices
                .extend([index, index + 2, index + 1, index + 1, index + 2, index + 3].map(|v| v as u32));
        } else {
            let current = if p0.y < 0.0 {
                &mut west_geometry
            } else {
                &mut east_geometry
            };

            {
                let values = &mut current.attributes.get_mut("position").unwrap().values;
                values.extend_from_slice(&[p0.x, p0.y, p0.z]);
                values.extend_from_slice(&[p0.x, p0.y, p0.z]);
                values.extend_from_slice(&[p2.x, p2.y, p2.z]);
                values.extend_from_slice(&[p2.x, p2.y, p2.z]);
            }

            {
                let values = &mut current.attributes.get_mut("prevPosition").unwrap().values;
                values.extend_from_slice(&prev_positions[i * 3..i * 3 + 4 * 3]);
            }
            {
                let values = &mut current.attributes.get_mut("nextPosition").unwrap().values;
                values.extend_from_slice(&next_positions[i * 3..i * 3 + 4 * 3]);
            }

            {
                let values = &mut current.attributes.get_mut("expandAndWidth").unwrap().values;
                values.extend_from_slice(&expand_and_widths[i * 2..i * 2 + 4 * 2]);
            }
            if let Some(tex_coords) = &tex_coords {
                let values = &mut current.attributes.get_mut("st").unwrap().values;
                values.extend_from_slice(&tex_coords[i * 2..i * 2 + 4 * 2]);
            }

            if let Some(colors) = &colors {
                let values = &mut current.attributes.get_mut("color").unwrap().values;
                values.extend_from_slice(&colors[i * 4..i * 4 + 4 * 4]);
            }

            let index = current.attributes["position"].values.len() / 3 - 4;
            current
                .indices
                .extend([index, index + 2, index + 1, index + 1, index + 2, index + 3].map(|v| v as u32));
        }

        i += 4;
    }

    if intersection_found {
        update_adjacency_after_split(&mut west_geometry);
        update_adjacency_after_split(&mut east_geometry);
    }

    update_instance_after_split(
        instance,
        west_geometry,
        east_geometry,
        compute_bounding_sphere,
    );
}

// ---------------------------------------------------------------------------
// splitLongitude (entry point)
// ---------------------------------------------------------------------------

/// Splits the instance's geometry, by introducing new vertices and indices,
/// that intersect the International Date Line and Prime Meridian so that no
/// primitives cross longitude -180/180 degrees. This is not required for 3D
/// drawing, but is required for correcting drawing in 2D and Columbus view.
///
/// Port of `GeometryPipeline.splitLongitude`. The JS `instance is required`
/// debug check is enforced by Rust's type system.
pub fn split_longitude(instance: &mut GeometryInstance) {
    let geometry_type = {
        let Some(geometry) = instance.geometry.as_geometry() else {
            return;
        };

        if let Some(bounding_sphere) = &geometry.bounding_sphere {
            let min_x = bounding_sphere.center.x - bounding_sphere.radius;
            if min_x > 0.0
                || BoundingSphere::intersect_plane(bounding_sphere, &Plane::ORIGIN_ZX_PLANE)
                    != Intersect::Intersecting
            {
                return;
            }
        }

        geometry.geometry_type
    };

    if geometry_type != GeometryType::None {
        let Some(geometry) = take_geometry(instance) else {
            return;
        };
        match geometry_type {
            GeometryType::Polylines => split_longitude_polyline(instance, geometry),
            GeometryType::Triangles => split_longitude_triangles(instance, geometry),
            GeometryType::Lines => split_longitude_lines(instance, geometry),
            GeometryType::None => unreachable!(),
        }
    } else {
        let Some(mut geometry) = take_geometry(instance) else {
            return;
        };
        index_primitive(&mut geometry);
        match geometry.primitive_type {
            PrimitiveType::Triangles => split_longitude_triangles(instance, geometry),
            PrimitiveType::Lines => split_longitude_lines(instance, geometry),
            _ => {
                // JS leaves other primitive types (e.g. POINTS) untouched;
                // restore the (possibly re-indexed) geometry.
                instance.geometry = GeometryInstanceGeometry::Geometry(Box::new(geometry));
            }
        }
    }
}
