//! Frustum geometry (camera frustum visualisation).
//!
//! Faithful port of CesiumJS `FrustumGeometry.js` and `FrustumOutlineGeometry.js`.
//! The frustum is built from its 8 corners (4 near + 4 far), which are found by
//! unprojecting the NDC corners through the inverse view-projection matrix, then
//! assembled into 6 quad planes (near, far, -x, -y, +x, +y).

use crate::bounding::BoundingSphere;
use crate::frustum::{OrthographicFrustum, PerspectiveFrustum};
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use glam::{DMat3, DMat4, DQuat, DVec3, DVec4};

/// A frustum definition (perspective or orthographic).
pub enum FrustumDef {
    Perspective(PerspectiveFrustum),
    Orthographic(OrthographicFrustum),
}

/// NDC corners of the far plane (x, y, z=1, w=1).
const FRUSTUM_CORNERS_NDC: [[f64; 4]; 4] = [
    [-1.0, -1.0, 1.0, 1.0],
    [1.0, -1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0, 1.0],
];

/// Builds a right-handed view matrix from camera axes.
///
/// Port of `Matrix4.computeView(position, direction, up, right)`. CesiumJS lays
/// the matrix out row-major as `[right; up; -direction]` with the translation
/// in the last column. glam is column-major, so we supply the columns directly.
fn compute_view(position: DVec3, direction: DVec3, up: DVec3, right: DVec3) -> DMat4 {
    DMat4::from_cols(
        DVec4::new(right.x, up.x, -direction.x, 0.0),
        DVec4::new(right.y, up.y, -direction.y, 0.0),
        DVec4::new(right.z, up.z, -direction.z, 0.0),
        DVec4::new(
            -right.dot(position),
            -up.dot(position),
            direction.dot(position),
            1.0,
        ),
    )
}

/// Computes the 8 corner positions of the frustum (near plane first, then far).
///
/// Port of `FrustumGeometry._computeNearFarPlanes`. Returns 8 positions laid out
/// as `[near0, near1, near2, near3, far0, far1, far2, far3]`, where the corner
/// order matches [`FRUSTUM_CORNERS_NDC`].
fn compute_near_far_planes(
    origin: DVec3,
    orientation: DQuat,
    frustum: &FrustumDef,
) -> Vec<[f64; 3]> {
    let rotation = DMat3::from_quat(orientation);
    let mut x = rotation.col(0).normalize();
    let y = rotation.col(1).normalize();
    let z = rotation.col(2).normalize();
    x = -x;

    let view = compute_view(origin, z, y, x);

    let mut positions = vec![[0.0f64; 3]; 8];

    match frustum {
        FrustumDef::Perspective(p) => {
            let projection = p.projection_matrix();
            let view_projection = projection * view;
            let inv_vp = view_projection.inverse();
            let splits = [p.near, p.far];

            for i in 0..2 {
                for j in 0..4 {
                    let c = FRUSTUM_CORNERS_NDC[j];
                    let corner = inv_vp * DVec4::new(c[0], c[1], c[2], c[3]);
                    // Reverse perspective divide.
                    let w = 1.0 / corner.w;
                    let mut corner3 = DVec3::new(corner.x, corner.y, corner.z) * w;

                    corner3 = (corner3 - origin).normalize();
                    let fac = z.dot(corner3);
                    corner3 = corner3 * (splits[i] / fac) + origin;

                    positions[4 * i + j] = [corner3.x, corner3.y, corner3.z];
                }
            }
        }
        FrustumDef::Orthographic(o) => {
            let inv_view = view.inverse();
            let right = o.width * 0.5;
            let left = -right;
            let top = o.height() * 0.5;
            let bottom = -top;
            // For orthographic the splits are [0, near, far]; iteration i uses
            // the plane at distance splits[i + 1].
            let splits = [0.0, o.near, o.far];

            for i in 0..2 {
                for j in 0..4 {
                    let c = FRUSTUM_CORNERS_NDC[j];
                    let cx = (c[0] * (right - left) + left + right) * 0.5;
                    let cy = (c[1] * (top - bottom) + bottom + top) * 0.5;
                    let cz = -splits[i + 1];
                    let corner = inv_view * DVec4::new(cx, cy, cz, 1.0);
                    positions[4 * i + j] = [corner.x, corner.y, corner.z];
                }
            }
        }
    }

    positions
}

/// Generates a filled frustum geometry (6 quad planes).
///
/// Maps to CesiumJS `FrustumGeometry`.
pub fn frustum_geometry(
    frustum: &FrustumDef,
    origin: DVec3,
    orientation: DQuat,
    vf: VertexFormat,
) -> GeometryData {
    let corners = compute_near_far_planes(origin, orientation, frustum);

    // Build 6 planes x 4 vertices. The near/far planes come directly from the
    // corners; the four side planes are assembled from corner combinations
    // (mirroring the index arithmetic in FrustumGeometry.createGeometry).
    let c = |k: usize| corners[k];
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(24);

    // Near plane (corners 0..4).
    positions.extend_from_slice(&[c(0), c(1), c(2), c(3)]);
    // Far plane (corners 4..8).
    positions.extend_from_slice(&[c(4), c(5), c(6), c(7)]);

    // The four side planes are assembled from corner combinations, mirroring
    // the flat-index arithmetic in FrustumGeometry.createGeometry. The flat
    // corner array is [near0..near3, far0..far3], so flat index k maps to
    // near[k] for k < 4 and far[k - 4] for k >= 4.
    let near = [c(0), c(1), c(2), c(3)];
    let far = [c(4), c(5), c(6), c(7)];
    // -x plane: flat [4], [0], [3], [7] => near[0], near[3], far[3], far[0]
    positions.extend_from_slice(&[near[0], near[3], far[3], far[0]]);
    // -y plane: flat [5], [1], [0], [4] => near[1], near[0], far[0], far[1]
    positions.extend_from_slice(&[near[1], near[0], far[0], far[1]]);
    // +x plane: flat [1], [5], [6], [2] => near[1], far[1], far[2], near[2]
    positions.extend_from_slice(&[near[1], far[1], far[2], near[2]]);
    // +y plane: flat [2], [6], [7], [3] => near[2], far[2], far[3], near[3]
    positions.extend_from_slice(&[near[2], far[2], far[3], near[3]]);

    let number_of_planes = 6usize;

    // Per-plane constant attributes.
    let rotation = DMat3::from_quat(orientation);
    let mut x = rotation.col(0).normalize();
    let y = rotation.col(1).normalize();
    let z = rotation.col(2).normalize();
    x = -x;
    let neg_x = -x;
    let neg_y = -y;
    let neg_z = -z;

    // (normal, tangent, bitangent) per plane, in CesiumJS order.
    let plane_attrs: [(DVec3, DVec3, DVec3); 6] = [
        (neg_z, x, y),    // near
        (z, neg_x, y),    // far
        (neg_x, neg_z, y),   // -x
        (neg_y, neg_z, neg_x), // -y
        (x, z, y),        // +x
        (y, z, neg_x),    // +y
    ];

    let mut normals: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::new()) } else { None };
    let mut tangents: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::new()) } else { None };
    let mut bitangents: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::new()) } else { None };
    let mut tex_coords: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::new()) } else { None };

    let st_quad = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (normal, tangent, bitangent) in &plane_attrs {
        for _ in 0..4 {
            if let Some(ref mut n) = normals {
                n.push([normal.x, normal.y, normal.z]);
            }
            if let Some(ref mut t) = tangents {
                t.push([tangent.x, tangent.y, tangent.z]);
            }
            if let Some(ref mut b) = bitangents {
                b.push([bitangent.x, bitangent.y, bitangent.z]);
            }
        }
        if let Some(ref mut st) = tex_coords {
            st.extend_from_slice(&st_quad);
        }
    }

    // Two triangles per plane.
    let mut indices: Vec<u32> = Vec::with_capacity(6 * number_of_planes);
    for i in 0..number_of_planes {
        let index = (i * 4) as u32;
        indices.extend_from_slice(&[index, index + 1, index + 2, index, index + 2, index + 3]);
    }

    let bounding_sphere = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals,
        tex_coords,
        tangents,
        bitangents,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    }
}

/// Generates a frustum outline geometry (12 edges as line segments).
///
/// Maps to CesiumJS `FrustumOutlineGeometry`. The outline consists of the 4 near
/// edges, 4 far edges and 4 connecting edges.
pub fn frustum_outline_geometry(
    frustum: &FrustumDef,
    origin: DVec3,
    orientation: DQuat,
) -> GeometryData {
    let corners = compute_near_far_planes(origin, orientation, frustum);
    let positions = corners;

    // Edges: near ring, far ring, and 4 connectors.
    let mut indices: Vec<u32> = Vec::with_capacity(24);
    // Near ring (0-1-2-3).
    for i in 0..4u32 {
        indices.push(i);
        indices.push((i + 1) % 4);
    }
    // Far ring (4-5-6-7).
    for i in 0..4u32 {
        indices.push(4 + i);
        indices.push(4 + (i + 1) % 4);
    }
    // Connectors.
    for i in 0..4u32 {
        indices.push(i);
        indices.push(4 + i);
    }

    let bounding_sphere = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perspective() -> FrustumDef {
        FrustumDef::Perspective(PerspectiveFrustum::new(
            std::f64::consts::FRAC_PI_3,
            16.0 / 9.0,
            1.0,
            100.0,
        ))
    }

    #[test]
    fn test_frustum_geometry_counts() {
        let geo = frustum_geometry(&perspective(), DVec3::ZERO, DQuat::IDENTITY, VertexFormat::ALL);
        assert_eq!(geo.positions.len(), 24); // 6 planes x 4 vertices
        assert_eq!(geo.indices.len(), 36); // 6 planes x 2 triangles x 3
        assert_eq!(geo.normals.as_ref().unwrap().len(), 24);
        assert_eq!(geo.tex_coords.as_ref().unwrap().len(), 24);
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
    }

    #[test]
    fn test_frustum_corners_at_correct_depth() {
        // With identity orientation the frustum looks down -Z... the near-plane
        // corners should all lie at distance `near` along the frustum axis.
        let corners = compute_near_far_planes(DVec3::ZERO, DQuat::IDENTITY, &perspective());
        // The frustum axis is the orientation's Z column (identity => +Z).
        let axis = DVec3::Z;
        for corner in &corners[0..4] {
            let d = DVec3::new(corner[0], corner[1], corner[2]).dot(axis);
            assert!((d - 1.0).abs() < 1e-6, "near corner depth {}", d);
        }
        for corner in &corners[4..8] {
            let d = DVec3::new(corner[0], corner[1], corner[2]).dot(axis);
            assert!((d - 100.0).abs() < 1e-6, "far corner depth {}", d);
        }
    }

    #[test]
    fn test_frustum_outline_counts() {
        let geo = frustum_outline_geometry(&perspective(), DVec3::ZERO, DQuat::IDENTITY);
        assert_eq!(geo.positions.len(), 8);
        assert_eq!(geo.indices.len(), 24); // 12 edges x 2
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_orthographic_frustum() {
        let frustum = FrustumDef::Orthographic(OrthographicFrustum::new(10.0, 1.0, 1.0, 50.0));
        let geo = frustum_geometry(&frustum, DVec3::ZERO, DQuat::IDENTITY, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 24);
        assert_eq!(geo.indices.len(), 36);
        // Orthographic near-plane corners should form a width x height rectangle.
        let corners = compute_near_far_planes(DVec3::ZERO, DQuat::IDENTITY, &frustum);
        let xs: Vec<f64> = corners[0..4].iter().map(|c| c[0]).collect();
        let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!((max_x - 5.0).abs() < 1e-6, "half-width {}", max_x);
    }
}
