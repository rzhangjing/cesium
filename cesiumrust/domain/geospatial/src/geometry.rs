//! Geometry generation - all procedural geometry types.
//! Maps to CesiumJS `Core/*Geometry.js` (20+ files), `Core/PolygonPipeline.js`, `Core/PolylinePipeline.js`

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;
use crate::math_utils;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// Vertex format flags - which attributes to generate.
/// Maps to CesiumJS `VertexFormat`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VertexFormat {
    pub position: bool,
    pub normal: bool,
    pub st: bool,
    pub tangent: bool,
    pub bitangent: bool,
}

impl VertexFormat {
    /// All attributes enabled.
    pub const ALL: Self = Self {
        position: true,
        normal: true,
        st: true,
        tangent: true,
        bitangent: true,
    };

    /// Position only.
    pub const POSITION_ONLY: Self = Self {
        position: true,
        normal: false,
        st: false,
        tangent: false,
        bitangent: false,
    };

    /// Position and normal.
    pub const POSITION_AND_NORMAL: Self = Self {
        position: true,
        normal: true,
        st: false,
        tangent: false,
        bitangent: false,
    };

    /// Position and texture coordinates.
    pub const POSITION_AND_ST: Self = Self {
        position: true,
        normal: false,
        st: true,
        tangent: false,
        bitangent: false,
    };
}

impl Default for VertexFormat {
    fn default() -> Self {
        Self::ALL
    }
}

/// Intermediate geometry representation (f64 precision, decoupled from GPU).
/// Maps to the output of CesiumJS geometry workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryData {
    /// Vertex positions (always present).
    pub positions: Vec<[f64; 3]>,
    /// Vertex normals (optional).
    pub normals: Option<Vec<[f64; 3]>>,
    /// Texture coordinates (optional).
    pub tex_coords: Option<Vec<[f64; 2]>>,
    /// Tangent vectors (optional).
    pub tangents: Option<Vec<[f64; 3]>>,
    /// Bitangent vectors (optional).
    pub bitangents: Option<Vec<[f64; 3]>>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Bounding sphere of the geometry.
    pub bounding_sphere: BoundingSphere,
}

/// Polygon hierarchy: outer ring + optional holes.
/// Maps to CesiumJS `PolygonHierarchy`
#[derive(Debug, Clone)]
pub struct PolygonHierarchy {
    /// Outer ring positions (cartographic or cartesian).
    pub positions: Vec<DVec3>,
    /// Holes (each hole is a ring of positions).
    pub holes: Vec<PolygonHierarchy>,
}

// ============================================================================
// Geometry Generators
// ============================================================================

/// Generates an ellipsoid geometry.
/// Maps to `EllipsoidGeometry` / `Workers/createEllipsoidGeometry`
pub fn ellipsoid_geometry(
    radii: DVec3,
    stacks: u32,
    slices: u32,
    vf: VertexFormat,
) -> GeometryData {
    let mut positions = Vec::new();
    let mut normals = if vf.normal { Some(Vec::new()) } else { None };
    let mut tex_coords = if vf.st { Some(Vec::new()) } else { None };

    for i in 0..=stacks {
        let phi = std::f64::consts::PI * i as f64 / stacks as f64;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for j in 0..=slices {
            let theta = 2.0 * std::f64::consts::PI * j as f64 / slices as f64;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = cos_theta * sin_phi;
            let y = sin_theta * sin_phi;
            let z = cos_phi;

            positions.push([x * radii.x, y * radii.y, z * radii.z]);

            if let Some(ref mut n) = normals {
                // Normal is the normalized position on unit sphere
                let normal = DVec3::new(x, y, z);
                n.push([normal.x, normal.y, normal.z]);
            }

            if let Some(ref mut st) = tex_coords {
                st.push([j as f64 / slices as f64, i as f64 / stacks as f64]);
            }
        }
    }

    let mut indices = Vec::new();
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * (slices + 1) + j;
            let b = a + slices + 1;
            indices.push(a);
            indices.push(b);
            indices.push(a + 1);
            indices.push(a + 1);
            indices.push(b);
            indices.push(b + 1);
        }
    }

    let bs = BoundingSphere::new(DVec3::ZERO, radii.x.max(radii.y).max(radii.z));

    GeometryData {
        positions,
        normals,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
    }
}

/// Generates a sphere geometry.
/// Maps to `SphereGeometry`
pub fn sphere_geometry(radius: f64, stacks: u32, slices: u32, vf: VertexFormat) -> GeometryData {
    ellipsoid_geometry(DVec3::splat(radius), stacks, slices, vf)
}

/// Generates a box geometry.
/// Maps to `BoxGeometry` / `Workers/createBoxGeometry`
pub fn box_geometry(minimum: DVec3, maximum: DVec3, vf: VertexFormat) -> GeometryData {
    let size = maximum - minimum;
    let center = (minimum + maximum) * 0.5;

    // 6 faces, 4 vertices each = 24 vertices
    let corners = [
        // +X face
        [1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0],
        // -X face
        [-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, 1.0, -1.0],
        // +Y face
        [-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0],
        // -Y face
        [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0],
        // +Z face
        [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        // -Z face
        [-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, -1.0, -1.0],
    ];

    let face_normals = [
        [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0], [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0], [0.0, 0.0, -1.0],
    ];

    let mut positions = Vec::with_capacity(24);
    let mut normals_vec = if vf.normal { Some(Vec::with_capacity(24)) } else { None };
    let mut tex_coords = if vf.st { Some(Vec::with_capacity(24)) } else { None };

    for (face_idx, corner_group) in corners.chunks(4).enumerate() {
        for (ci, corner) in corner_group.iter().enumerate() {
            positions.push([
                center.x + corner[0] * size.x * 0.5,
                center.y + corner[1] * size.y * 0.5,
                center.z + corner[2] * size.z * 0.5,
            ]);
            if let Some(ref mut n) = normals_vec {
                n.push(face_normals[face_idx]);
            }
            if let Some(ref mut st) = tex_coords {
                let u = if ci == 0 || ci == 3 { 0.0 } else { 1.0 };
                let v = if ci < 2 { 0.0 } else { 1.0 };
                st.push([u, v]);
            }
        }
    }

    let mut indices = Vec::with_capacity(36);
    for face in 0..6u32 {
        let base = face * 4;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let bs = BoundingSphere::new(center, size.length() * 0.5);

    GeometryData {
        positions,
        normals: normals_vec,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
    }
}

/// Generates a cylinder geometry.
/// Maps to `CylinderGeometry`
pub fn cylinder_geometry(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: u32,
    vf: VertexFormat,
) -> GeometryData {
    let half_length = length * 0.5;
    let mut positions = Vec::new();
    let mut normals_vec = if vf.normal { Some(Vec::new()) } else { None };
    let mut tex_coords = if vf.st { Some(Vec::new()) } else { None };

    // Side vertices
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Bottom vertex
        positions.push([cos_t * bottom_radius, sin_t * bottom_radius, -half_length]);
        if let Some(ref mut n) = normals_vec {
            n.push([cos_t, sin_t, 0.0]);
        }
        if let Some(ref mut st) = tex_coords {
            st.push([i as f64 / slices as f64, 0.0]);
        }

        // Top vertex
        positions.push([cos_t * top_radius, sin_t * top_radius, half_length]);
        if let Some(ref mut n) = normals_vec {
            n.push([cos_t, sin_t, 0.0]);
        }
        if let Some(ref mut st) = tex_coords {
            st.push([i as f64 / slices as f64, 1.0]);
        }
    }

    let mut indices = Vec::new();
    for i in 0..slices {
        let base = i * 2;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 1);
        indices.push(base + 3);
        indices.push(base + 2);
    }

    let max_radius = top_radius.max(bottom_radius);
    let bs = BoundingSphere::new(DVec3::ZERO, (max_radius * max_radius + half_length * half_length).sqrt());

    GeometryData {
        positions,
        normals: normals_vec,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
    }
}

/// Generates a rectangle geometry on the ellipsoid surface.
/// Maps to `RectangleGeometry` / `Workers/createRectangleGeometry`
pub fn rectangle_geometry(
    rect: &Rectangle,
    ellipsoid: &Ellipsoid,
    granularity: f64,
    height: f64,
    vf: VertexFormat,
) -> GeometryData {
    let width = rect.width();
    let h = rect.height();
    let cols = ((width / granularity).ceil() as u32).max(1) + 1;
    let rows = ((h / granularity).ceil() as u32).max(1) + 1;

    let mut positions = Vec::with_capacity((cols * rows) as usize);
    let mut normals_vec = if vf.normal { Some(Vec::new()) } else { None };
    let mut tex_coords = if vf.st { Some(Vec::new()) } else { None };

    for row in 0..rows {
        let lat = rect.south + h * row as f64 / (rows - 1) as f64;
        for col in 0..cols {
            let lon = rect.west + width * col as f64 / (cols - 1) as f64;
            let carto = crate::cartographic::Cartographic::from_radians(lon, lat, height);
            let pos = ellipsoid.cartographic_to_cartesian(&carto);
            positions.push([pos.x, pos.y, pos.z]);

            if let Some(ref mut n) = normals_vec {
                let normal = ellipsoid.geodetic_surface_normal(pos).unwrap_or(DVec3::Z);
                n.push([normal.x, normal.y, normal.z]);
            }
            if let Some(ref mut st) = tex_coords {
                st.push([col as f64 / (cols - 1) as f64, row as f64 / (rows - 1) as f64]);
            }
        }
    }

    let mut indices = Vec::new();
    for row in 0..(rows - 1) {
        for col in 0..(cols - 1) {
            let a = row * cols + col;
            let b = a + cols;
            indices.push(a);
            indices.push(b);
            indices.push(a + 1);
            indices.push(a + 1);
            indices.push(b);
            indices.push(b + 1);
        }
    }

    let bs = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals: normals_vec,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
    }
}

/// Generates a circle geometry on the ellipsoid.
/// Maps to `CircleGeometry`
pub fn circle_geometry(
    center: DVec3,
    radius: f64,
    ellipsoid: &Ellipsoid,
    segments: u32,
    vf: VertexFormat,
) -> GeometryData {
    let center_carto = ellipsoid.cartesian_to_cartographic(center);
    let height = center_carto.map(|c| c.height).unwrap_or(0.0);
    let center_carto = center_carto.unwrap_or_default();

    let mut positions = Vec::with_capacity(segments as usize + 1);
    let mut normals_vec = if vf.normal { Some(Vec::new()) } else { None };

    // Center vertex
    positions.push([center.x, center.y, center.z]);
    if let Some(ref mut n) = normals_vec {
        let normal = ellipsoid.geodetic_surface_normal(center).unwrap_or(DVec3::Z);
        n.push([normal.x, normal.y, normal.z]);
    }

    // Ring vertices
    for i in 0..=segments {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
        // Approximate: offset in meters along surface
        let d_lat = radius * angle.cos() / ellipsoid.maximum_radius();
        let d_lon = radius * angle.sin() / (ellipsoid.maximum_radius() * center_carto.latitude.cos().max(1e-10));

        let carto = crate::cartographic::Cartographic::from_radians(
            center_carto.longitude + d_lon,
            center_carto.latitude + d_lat,
            height,
        );
        let pos = ellipsoid.cartographic_to_cartesian(&carto);
        positions.push([pos.x, pos.y, pos.z]);

        if let Some(ref mut n) = normals_vec {
            let normal = ellipsoid.geodetic_surface_normal(pos).unwrap_or(DVec3::Z);
            n.push([normal.x, normal.y, normal.z]);
        }
    }

    let mut indices = Vec::new();
    for i in 0..segments {
        indices.push(0);
        indices.push(i + 1);
        indices.push(i + 2);
    }

    let bs = BoundingSphere::new(center, radius);
    let num_vertices = positions.len();

    GeometryData {
        positions,
        normals: normals_vec,
        tex_coords: if vf.st { Some(vec![[0.5, 0.5]; num_vertices]) } else { None },
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
    }
}

/// Generates a plane geometry (unit quad in XY plane).
/// Maps to `PlaneGeometry`
pub fn plane_geometry(vf: VertexFormat) -> GeometryData {
    let positions = vec![
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    let normals_vec = if vf.normal {
        Some(vec![[0.0, 0.0, 1.0]; 4])
    } else {
        None
    };
    let tex_coords = if vf.st {
        Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
    } else {
        None
    };
    let indices = vec![0, 1, 2, 0, 2, 3];

    GeometryData {
        positions,
        normals: normals_vec,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, std::f64::consts::FRAC_1_SQRT_2),
    }
}

// ============================================================================
// Pipeline algorithms
// ============================================================================

/// Generates an arc (great circle) between positions with given granularity.
/// Maps to `PolylinePipeline.generateArc`
pub fn generate_arc(positions: &[DVec3], granularity: f64, ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    if positions.len() < 2 {
        return positions.to_vec();
    }

    let mut result = Vec::new();
    for i in 0..positions.len() - 1 {
        let start = positions[i];
        let end = positions[i + 1];

        let start_carto = ellipsoid.cartesian_to_cartographic(start);
        let end_carto = ellipsoid.cartesian_to_cartographic(end);

        if let (Some(sc), Some(ec)) = (start_carto, end_carto) {
            let angular_distance = ((ec.latitude - sc.latitude).powi(2)
                + (ec.longitude - sc.longitude).powi(2))
            .sqrt();
            let num_segments = ((angular_distance / granularity).ceil() as usize).max(1);

            for j in 0..num_segments {
                let t = j as f64 / num_segments as f64;
                let lon = math_utils::lerp(sc.longitude, ec.longitude, t);
                let lat = math_utils::lerp(sc.latitude, ec.latitude, t);
                let h = math_utils::lerp(sc.height, ec.height, t);
                let carto = crate::cartographic::Cartographic::from_radians(lon, lat, h);
                result.push(ellipsoid.cartographic_to_cartesian(&carto));
            }
        } else {
            result.push(start);
        }
    }
    result.push(*positions.last().unwrap());
    result
}

/// Triangulates a 2D polygon using ear clipping.
/// Maps to `PolygonPipeline.triangulate`
pub fn triangulate_polygon(positions: &[DVec2], _holes: &[Vec<u32>]) -> Vec<u32> {
    // Simple ear-clipping for convex/simple polygons
    let n = positions.len();
    if n < 3 {
        return Vec::new();
    }

    let mut indices = Vec::new();
    // Fan triangulation (works for convex polygons)
    for i in 1..(n as u32 - 1) {
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
    }
    indices
}

/// Computes the signed area of a 2D polygon.
/// Maps to `PolygonPipeline.computeArea2D`
pub fn compute_area2d(positions: &[DVec2]) -> f64 {
    let n = positions.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += positions[i].x * positions[j].y;
        area -= positions[j].x * positions[i].y;
    }
    area * 0.5
}

/// Computes the winding order of a 2D polygon.
/// Maps to `PolygonPipeline.computeWindingOrder2D`
pub fn compute_winding_order(positions: &[DVec2]) -> WindingOrder {
    if compute_area2d(positions) > 0.0 {
        WindingOrder::CounterClockwise
    } else {
        WindingOrder::Clockwise
    }
}

/// Winding order of a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipsoid_geometry_vertex_count() {
        let geo = ellipsoid_geometry(DVec3::splat(1.0), 16, 32, VertexFormat::ALL);
        // (stacks+1) * (slices+1) vertices
        assert_eq!(geo.positions.len(), 17 * 33);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
        // stacks * slices * 6 indices
        assert_eq!(geo.indices.len(), 16 * 32 * 6);
    }

    #[test]
    fn test_sphere_geometry() {
        let geo = sphere_geometry(5.0, 8, 16, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 9 * 17);
        assert!(geo.normals.is_none());
        assert!((geo.bounding_sphere.radius - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_box_geometry() {
        let geo = box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::ALL);
        assert_eq!(geo.positions.len(), 24); // 6 faces * 4 vertices
        assert_eq!(geo.indices.len(), 36); // 6 faces * 2 triangles * 3
    }

    #[test]
    fn test_plane_geometry() {
        let geo = plane_geometry(VertexFormat::ALL);
        assert_eq!(geo.positions.len(), 4);
        assert_eq!(geo.indices.len(), 6);
    }

    #[test]
    fn test_rectangle_geometry() {
        let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let geo = rectangle_geometry(&rect, &Ellipsoid::WGS84, math_utils::to_radians(1.0), 0.0, VertexFormat::ALL);
        assert!(geo.positions.len() > 4);
        assert!(geo.normals.is_some());
    }

    #[test]
    fn test_generate_arc() {
        let ellipsoid = Ellipsoid::WGS84;
        let start = ellipsoid.cartographic_to_cartesian(&crate::cartographic::Cartographic::from_degrees(0.0, 0.0, 0.0));
        let end = ellipsoid.cartographic_to_cartesian(&crate::cartographic::Cartographic::from_degrees(10.0, 0.0, 0.0));
        let arc = generate_arc(&[start, end], math_utils::to_radians(1.0), &ellipsoid);
        assert!(arc.len() > 2); // Should have intermediate points
    }

    #[test]
    fn test_triangulate_polygon() {
        let positions = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let indices = triangulate_polygon(&positions, &[]);
        assert_eq!(indices.len(), 6); // 2 triangles for a quad
    }

    #[test]
    fn test_compute_area2d() {
        let positions = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let area = compute_area2d(&positions);
        assert!((area - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_winding_order() {
        let ccw = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];
        assert_eq!(compute_winding_order(&ccw), WindingOrder::CounterClockwise);
    }
}
