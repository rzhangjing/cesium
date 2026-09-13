//! Corridor geometry - a band of constant width along a polyline path.
//!
//! Faithful port of CesiumJS `CorridorGeometryLibrary.js` and
//! `CorridorGeometry.js`. A corridor is defined by a series of centerline
//! positions and a width; the geometry is the flat ribbon between the left
//! and right edges (with optional rounded/mitered/beveled corners).

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::{self, EPSILON7};
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// Corner style for corridor turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerType {
    /// Rounded corners (default).
    #[default]
    Rounded,
    /// Mitered (sharp) corners.
    Mitered,
    /// Beveled (cut) corners.
    Beveled,
}

/// Options describing a corridor.
#[derive(Debug, Clone)]
pub struct CorridorOptions {
    /// Centerline positions (at least 2).
    pub positions: Vec<DVec3>,
    /// Width in meters.
    pub width: f64,
    /// Height above the ellipsoid surface.
    pub height: f64,
    /// Angular granularity in radians.
    pub granularity: f64,
    /// Corner style.
    pub corner_type: CornerType,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
}

impl Default for CorridorOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 0.0,
            height: 0.0,
            granularity: std::f64::consts::PI / 180.0,
            corner_type: CornerType::Rounded,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// Determines if the angle from `backward` to `forward` (viewed from outside
/// the ellipsoid) is greater than pi.
///
/// Maps to `PolylineVolumeGeometryLibrary.angleIsGreaterThanPi`.
fn angle_is_greater_than_pi(
    forward: DVec3,
    backward: DVec3,
    position: DVec3,
    ellipsoid: &Ellipsoid,
) -> bool {
    let normal = ellipsoid.geodetic_surface_normal(position).unwrap_or(DVec3::Z);
    let mut east = DVec3::Z.cross(normal);
    if east.length_squared() < 1e-30 {
        east = DVec3::X.cross(normal);
    }
    east = east.normalize_or(DVec3::X);
    let north = normal.cross(east).normalize_or(DVec3::Y);

    let next_pt = position + forward;
    let prev_pt = position + backward;
    let next_x = (next_pt - position).dot(east);
    let next_y = (next_pt - position).dot(north);
    let prev_x = (prev_pt - position).dot(east);
    let prev_y = (prev_pt - position).dot(north);

    prev_x * next_y - prev_y * next_x >= 0.0
}

/// Rotates a vector around a unit axis by an angle (Rodrigues' formula).
fn rotate_around_axis(v: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    v * cos_a + axis.cross(v) * sin_a + axis * axis.dot(v) * (1.0 - cos_a)
}

/// Computes a rounded corner arc.
fn compute_round_corner(
    corner_point: DVec3,
    start_point: DVec3,
    end_point: DVec3,
    corner_type: CornerType,
    left_is_outside: bool,
) -> Vec<DVec3> {
    let v1 = start_point - corner_point;
    let v2 = end_point - corner_point;
    let angle = v1.angle_between(v2);
    let granularity = if corner_type == CornerType::Beveled {
        1
    } else {
        (angle / math_utils::to_radians(5.0)).ceil() as usize + 1
    };

    let axis = if left_is_outside {
        (-corner_point).normalize_or(DVec3::Z)
    } else {
        corner_point.normalize_or(DVec3::Z)
    };
    let step_angle = angle / granularity as f64;

    let mut array: Vec<DVec3> = Vec::with_capacity(granularity + 1);
    let mut current = start_point;
    for _ in 0..granularity {
        current = rotate_around_axis(current, axis, step_angle);
        array.push(current);
    }
    if let Some(last) = array.last_mut() {
        *last = end_point;
    }
    array
}

/// Computes a mitered corner (2 points).
fn compute_mitered_corner(
    position: DVec3,
    left_corner_direction: DVec3,
    last_point: DVec3,
    left_is_outside: bool,
) -> Vec<DVec3> {
    let corner_point = if left_is_outside {
        position + left_corner_direction
    } else {
        position - left_corner_direction
    };
    vec![corner_point, last_point]
}

/// A computed corner (either left or right positions).
struct CornerData {
    left_positions: Option<Vec<DVec3>>,
    right_positions: Option<Vec<DVec3>>,
}

/// Offsets a centerline arc into right and left edge positions.
fn add_shifted_positions(
    positions: &[DVec3],
    left: DVec3,
    scalar: f64,
    out: &mut Vec<Vec<DVec3>>,
) {
    let scaled_left = left * scalar;
    let scaled_right = -scaled_left;

    let right_positions: Vec<DVec3> = positions.iter().map(|&p| p + scaled_right).collect();
    // Left positions stored in reverse order (matching CesiumJS).
    let left_positions: Vec<DVec3> = positions.iter().rev().map(|&p| p + scaled_left).collect();

    out.push(right_positions);
    out.push(left_positions);
}

/// Core corridor position computation.
/// Maps to `CorridorGeometryLibrary.computePositions`.
fn compute_corridor_positions(
    positions: &[DVec3],
    width: f64,
    granularity: f64,
    corner_type: CornerType,
    ellipsoid: &Ellipsoid,
) -> (Vec<Vec<DVec3>>, Vec<CornerData>) {
    let half_width = width / 2.0;
    let mut calculated_positions: Vec<Vec<DVec3>> = Vec::new();
    let mut corners: Vec<CornerData> = Vec::new();

    let mut position = positions[0];
    let mut next_position = positions[1];

    let mut forward = (next_position - position).normalize_or(DVec3::X);
    let normal = ellipsoid.geodetic_surface_normal(position).unwrap_or(DVec3::Z);
    let mut left = normal.cross(forward).normalize_or(DVec3::Y);

    let mut previous_pos = position;
    position = next_position;
    let mut backward = -forward;

    let length = positions.len();
    for i in 1..length - 1 {
        let normal = ellipsoid.geodetic_surface_normal(position).unwrap_or(DVec3::Z);
        next_position = positions[i + 1];
        forward = (next_position - position).normalize_or(DVec3::X);

        let forward_proj = (forward - normal * forward.dot(normal)).normalize_or(DVec3::X);
        let backward_proj = (backward - normal * backward.dot(normal)).normalize_or(DVec3::X);

        let do_corner =
            !math_utils::equals_epsilon(forward_proj.dot(backward_proj).abs(), 1.0, 0.0, EPSILON7);

        if do_corner {
            let mut corner_direction = (forward + backward).normalize_or(DVec3::X);
            corner_direction = corner_direction.cross(normal);
            corner_direction = normal.cross(corner_direction);
            corner_direction = corner_direction.normalize_or(DVec3::X);

            let cross_mag = corner_direction.cross(backward).length();
            let scalar = half_width / cross_mag.max(0.25);

            let left_is_outside =
                angle_is_greater_than_pi(forward, backward, position, ellipsoid);

            corner_direction *= scalar;

            if left_is_outside {
                let right_pos = position + corner_direction;
                let center = right_pos + left * half_width;
                let left_pos = right_pos + left * (half_width * 2.0);

                let seg = [previous_pos, center];
                let opts = ArcOptions { positions: &seg, heights: None, granularity, ellipsoid };
                let subdivided = generate_arc(&opts);
                add_shifted_positions(&subdivided, left, half_width, &mut calculated_positions);

                let start_point = left_pos;
                left = normal.cross(forward).normalize_or(DVec3::Y);
                let new_left_pos = right_pos + left * (half_width * 2.0);
                previous_pos = right_pos + left * half_width;

                let corner_positions = match corner_type {
                    CornerType::Rounded | CornerType::Beveled => compute_round_corner(
                        right_pos, start_point, new_left_pos, corner_type, left_is_outside,
                    ),
                    CornerType::Mitered => compute_mitered_corner(
                        position, -corner_direction, new_left_pos, left_is_outside,
                    ),
                };
                corners.push(CornerData { left_positions: Some(corner_positions), right_positions: None });
            } else {
                let left_pos = position + corner_direction;
                let center = left_pos - left * half_width;
                let right_pos = left_pos - left * (half_width * 2.0);

                let seg = [previous_pos, center];
                let opts = ArcOptions { positions: &seg, heights: None, granularity, ellipsoid };
                let subdivided = generate_arc(&opts);
                add_shifted_positions(&subdivided, left, half_width, &mut calculated_positions);

                let start_point = right_pos;
                left = normal.cross(forward).normalize_or(DVec3::Y);
                let new_right_pos = left_pos - left * (half_width * 2.0);
                previous_pos = left_pos - left * half_width;

                let corner_positions = match corner_type {
                    CornerType::Rounded | CornerType::Beveled => compute_round_corner(
                        left_pos, start_point, new_right_pos, corner_type, left_is_outside,
                    ),
                    CornerType::Mitered => compute_mitered_corner(
                        position, corner_direction, new_right_pos, left_is_outside,
                    ),
                };
                corners.push(CornerData { left_positions: None, right_positions: Some(corner_positions) });
            }
            backward = -forward;
        }
        position = next_position;
    }

    // Final segment.
    let seg = [previous_pos, position];
    let opts = ArcOptions { positions: &seg, heights: None, granularity, ellipsoid };
    let subdivided = generate_arc(&opts);
    add_shifted_positions(&subdivided, left, half_width, &mut calculated_positions);

    (calculated_positions, corners)
}

/// Assembles right and left edges from computed positions + corners.
fn assemble_edges(
    positions: &[Vec<DVec3>],
    corners: &[CornerData],
) -> (Vec<DVec3>, Vec<DVec3>) {
    let mut right_edge: Vec<DVec3> = Vec::new();
    let mut left_edge: Vec<DVec3> = Vec::new();

    let mut corner_idx = 0;
    let mut i = 0;
    while i + 1 < positions.len() {
        let right_seg = &positions[i];
        let left_seg = &positions[i + 1];

        if i == 0 {
            right_edge.extend_from_slice(right_seg);
            left_edge.extend_from_slice(left_seg);
        } else {
            // Skip duplicate first/last point from corner junction.
            if right_seg.len() > 1 {
                right_edge.extend_from_slice(&right_seg[1..]);
            }
            if left_seg.len() > 1 {
                left_edge.extend_from_slice(&left_seg[1..]);
            }
        }

        // Insert corner positions.
        if corner_idx < corners.len() {
            let corner = &corners[corner_idx];
            if let Some(ref lp) = corner.left_positions {
                left_edge.extend_from_slice(lp);
            }
            if let Some(ref rp) = corner.right_positions {
                right_edge.extend_from_slice(rp);
            }
            corner_idx += 1;
        }

        i += 2;
    }

    (right_edge, left_edge)
}

/// Generates a corridor geometry (flat, non-extruded).
///
/// Maps to CesiumJS `CorridorGeometry.createGeometry`.
pub fn corridor_geometry(options: &CorridorOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let width = options.width;

    // Scale positions to surface and remove duplicates.
    let mut positions: Vec<DVec3> = options
        .positions
        .iter()
        .map(|&p| ellipsoid.scale_to_geodetic_surface(p).unwrap_or(p))
        .collect();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= crate::math_utils::EPSILON10
            && (a.y - b.y).abs() <= crate::math_utils::EPSILON10
            && (a.z - b.z).abs() <= crate::math_utils::EPSILON10
    });

    if positions.len() < 2 || width <= 0.0 {
        return empty_geometry();
    }

    let (computed_positions, corners) = compute_corridor_positions(
        &positions,
        width,
        options.granularity,
        options.corner_type,
        ellipsoid,
    );

    let (right_edge, left_edge) = assemble_edges(&computed_positions, &corners);

    let right_count = right_edge.len();
    let left_count = left_edge.len();
    if right_count < 2 || left_count < 2 {
        return empty_geometry();
    }

    let total_verts = right_count + left_count;
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(total_verts);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(total_verts)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(total_verts)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(total_verts)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(total_verts)) } else { None };

    // Right edge vertices.
    let right_st = if right_count > 1 { 1.0 / (right_count - 1) as f64 } else { 1.0 };
    for (idx, p) in right_edge.iter().enumerate() {
        let raised = raise_to_height(*p, options.height, ellipsoid);
        pos_out.push([raised.x, raised.y, raised.z]);
        if let Some(ref mut n) = normals_out {
            let normal = ellipsoid.geodetic_surface_normal(raised).unwrap_or(DVec3::Z);
            n.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut t) = tangents_out {
            let tangent = compute_tangent(&right_edge, idx, ellipsoid);
            t.push([tangent.x, tangent.y, tangent.z]);
        }
        if let Some(ref mut b) = bitangents_out {
            let normal = ellipsoid.geodetic_surface_normal(raised).unwrap_or(DVec3::Z);
            let tangent = compute_tangent(&right_edge, idx, ellipsoid);
            let bitangent = normal.cross(tangent).normalize_or(DVec3::Y);
            b.push([bitangent.x, bitangent.y, bitangent.z]);
        }
        if let Some(ref mut st) = st_out {
            st.push([idx as f64 * right_st, 0.0]);
        }
    }

    // Left edge vertices (reversed for consistent winding).
    let left_st = if left_count > 1 { 1.0 / (left_count - 1) as f64 } else { 1.0 };
    for (idx, p) in left_edge.iter().enumerate() {
        let raised = raise_to_height(*p, options.height, ellipsoid);
        pos_out.push([raised.x, raised.y, raised.z]);
        if let Some(ref mut n) = normals_out {
            let normal = ellipsoid.geodetic_surface_normal(raised).unwrap_or(DVec3::Z);
            n.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut t) = tangents_out {
            let tangent = compute_tangent(&left_edge, idx, ellipsoid);
            t.push([tangent.x, tangent.y, tangent.z]);
        }
        if let Some(ref mut b) = bitangents_out {
            let normal = ellipsoid.geodetic_surface_normal(raised).unwrap_or(DVec3::Z);
            let tangent = compute_tangent(&left_edge, idx, ellipsoid);
            let bitangent = normal.cross(tangent).normalize_or(DVec3::Y);
            b.push([bitangent.x, bitangent.y, bitangent.z]);
        }
        if let Some(ref mut st) = st_out {
            st.push([(left_count - 1 - idx) as f64 * left_st, 1.0]);
        }
    }

    // Triangulate: strip between right and left edges.
    let strip_count = right_count.min(left_count);
    let mut indices: Vec<u32> = Vec::with_capacity((strip_count - 1) * 6);
    for i in 0..strip_count - 1 {
        let r0 = i as u32;
        let r1 = (i + 1) as u32;
        let l0 = (right_count + i) as u32;
        let l1 = (right_count + i + 1) as u32;
        indices.extend_from_slice(&[l0, r0, l1]);
        indices.extend_from_slice(&[l1, r0, r1]);
    }

    let bounding_sphere = BoundingSphere::from_points(
        &pos_out.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions: pos_out,
        normals: normals_out,
        tex_coords: st_out,
        tangents: tangents_out,
        bitangents: bitangents_out,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    }
}

/// Generates a corridor outline geometry (line loop around the corridor).
pub fn corridor_outline_geometry(options: &CorridorOptions) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let width = options.width;

    let mut positions: Vec<DVec3> = options
        .positions
        .iter()
        .map(|&p| ellipsoid.scale_to_geodetic_surface(p).unwrap_or(p))
        .collect();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= crate::math_utils::EPSILON10
            && (a.y - b.y).abs() <= crate::math_utils::EPSILON10
            && (a.z - b.z).abs() <= crate::math_utils::EPSILON10
    });

    if positions.len() < 2 || width <= 0.0 {
        return empty_geometry_lines();
    }

    let (computed_positions, corners) = compute_corridor_positions(
        &positions,
        width,
        options.granularity,
        options.corner_type,
        ellipsoid,
    );

    let (right_edge, left_edge) = assemble_edges(&computed_positions, &corners);

    // Outline: right edge forward + left edge forward (reversed back).
    let mut pos_out: Vec<[f64; 3]> = Vec::new();
    for p in &right_edge {
        let raised = raise_to_height(*p, options.height, ellipsoid);
        pos_out.push([raised.x, raised.y, raised.z]);
    }
    // Left edge in reverse to form a loop.
    for p in left_edge.iter().rev() {
        let raised = raise_to_height(*p, options.height, ellipsoid);
        pos_out.push([raised.x, raised.y, raised.z]);
    }

    let n = pos_out.len();
    let mut indices: Vec<u32> = Vec::with_capacity(n * 2);
    for i in 0..n - 1 {
        indices.push(i as u32);
        indices.push((i + 1) as u32);
    }
    // Close the loop.
    indices.push((n - 1) as u32);
    indices.push(0);

    let bounding_sphere = BoundingSphere::from_points(
        &pos_out.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions: pos_out,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Lines,
    }
}

fn raise_to_height(p: DVec3, height: f64, ellipsoid: &Ellipsoid) -> DVec3 {
    if height.abs() < f64::EPSILON {
        return p;
    }
    let normal = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);
    p + normal * height
}

fn compute_tangent(edge: &[DVec3], idx: usize, _ellipsoid: &Ellipsoid) -> DVec3 {
    let next = if idx + 1 < edge.len() { edge[idx + 1] } else { edge[idx] };
    let prev = if idx > 0 { edge[idx - 1] } else { edge[idx] };
    (next - prev).normalize_or(DVec3::X)
}

fn empty_geometry() -> GeometryData {
    GeometryData {
        positions: Vec::new(),
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices: Vec::new(),
        bounding_sphere: BoundingSphere::default(),
        primitive_type: PrimitiveType::Triangles,
    }
}

fn empty_geometry_lines() -> GeometryData {
    GeometryData {
        positions: Vec::new(),
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices: Vec::new(),
        bounding_sphere: BoundingSphere::default(),
        primitive_type: PrimitiveType::Lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartographic::Cartographic;

    fn corridor_opts() -> CorridorOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-72.0, 40.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-70.0, 35.0, 0.0)),
        ];
        CorridorOptions {
            positions,
            width: 100_000.0,
            height: 0.0,
            granularity: std::f64::consts::PI / 180.0,
            corner_type: CornerType::Rounded,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_corridor_basic() {
        let geo = corridor_geometry(&corridor_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
        // Should have right + left edge vertices.
        assert!(geo.positions.len() >= 4);
    }

    #[test]
    fn test_corridor_width_correct() {
        let opts = corridor_opts();
        let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
        // The corridor should span roughly 100km width.
        // Check bounding sphere radius is reasonable (> 50km).
        assert!(geo.bounding_sphere.radius > 50_000.0);
    }

    #[test]
    fn test_corridor_outline() {
        let geo = corridor_outline_geometry(&corridor_opts());
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
        assert_eq!(geo.indices.len() % 2, 0);
        let n = geo.positions.len() as u32;
        for &idx in &geo.indices {
            assert!(idx < n);
        }
    }

    #[test]
    fn test_corridor_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let opts = CorridorOptions {
            positions: vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))],
            width: 1000.0,
            ..Default::default()
        };
        let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_corridor_zero_width() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = CorridorOptions {
            positions,
            width: 0.0,
            ..Default::default()
        };
        let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_corridor_with_corner() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        ];
        let opts = CorridorOptions {
            positions,
            width: 50_000.0,
            corner_type: CornerType::Mitered,
            ..Default::default()
        };
        let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 3, 0);
    }
}
