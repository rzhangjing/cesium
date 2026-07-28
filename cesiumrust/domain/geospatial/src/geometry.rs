//! Geometry generation - all procedural geometry types.
//! Maps to CesiumJS `Core/*Geometry.js` (20+ files), `Core/PolygonPipeline.js`, `Core/PolylinePipeline.js`

pub mod coplanar_polygon;
pub mod corridor;
pub mod ellipse;
pub mod frustum_geo;
pub mod ground_polyline;
pub mod polyline_geo;
pub mod polyline_volume;
pub mod wall;

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;
use crate::math_utils;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

pub use coplanar_polygon::{coplanar_polygon_geometry, CoplanarPolygonOptions};
pub use corridor::{corridor_geometry, corridor_outline_geometry, CornerType, CorridorOptions};
pub use ground_polyline::{ground_polyline_geometry, GroundPolylineOptions};
pub use ellipse::{
    ellipse_geometry, ellipse_outline_geometry, EllipseOptions,
};
pub use polyline_geo::{polyline_geometry, PolylineOptions};
pub use polyline_volume::{polyline_volume_geometry, PolylineVolumeOptions};
pub use frustum_geo::{
    frustum_geometry, frustum_outline_geometry, FrustumDef,
};
pub use wall::{wall_geometry, wall_outline_geometry, WallOptions};

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

/// Primitive topology of the generated geometry.
/// Maps to CesiumJS `PrimitiveType` (TRIANGLES / LINES).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Triangle list (filled surfaces).
    #[default]
    Triangles,
    /// Line list (outlines; indices are pairs of vertices).
    Lines,
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
    /// Indices (triangles or line pairs depending on `primitive_type`).
    pub indices: Vec<u32>,
    /// Bounding sphere of the geometry.
    pub bounding_sphere: BoundingSphere,
    /// Primitive topology (triangles for fills, lines for outlines).
    #[serde(default)]
    pub primitive_type: PrimitiveType,
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
            // Counter-clockwise winding when viewed from outside (outward normals)
            indices.push(a);
            indices.push(a + 1);
            indices.push(b);
            indices.push(a + 1);
            indices.push(b + 1);
            indices.push(b);
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
        primitive_type: PrimitiveType::Triangles,
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
        primitive_type: PrimitiveType::Triangles,
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
        primitive_type: PrimitiveType::Triangles,
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
        primitive_type: PrimitiveType::Triangles,
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
        primitive_type: PrimitiveType::Triangles,
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
        primitive_type: PrimitiveType::Triangles,
    }
}

/// Generates a box outline geometry (12 edges as line segments).
/// Maps to `BoxOutlineGeometry`
pub fn box_outline_geometry(minimum: DVec3, maximum: DVec3) -> GeometryData {
    let size = maximum - minimum;
    let center = (minimum + maximum) * 0.5;
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    // 8 corners of the box.
    let corners = [
        [center.x - hx, center.y - hy, center.z - hz], // 0
        [center.x + hx, center.y - hy, center.z - hz], // 1
        [center.x + hx, center.y + hy, center.z - hz], // 2
        [center.x - hx, center.y + hy, center.z - hz], // 3
        [center.x - hx, center.y - hy, center.z + hz], // 4
        [center.x + hx, center.y - hy, center.z + hz], // 5
        [center.x + hx, center.y + hy, center.z + hz], // 6
        [center.x - hx, center.y + hy, center.z + hz], // 7
    ];

    // 12 edges: 4 bottom, 4 top, 4 vertical.
    let indices: Vec<u32> = vec![
        0, 1, 1, 2, 2, 3, 3, 0, // bottom
        4, 5, 5, 6, 6, 7, 7, 4, // top
        0, 4, 1, 5, 2, 6, 3, 7, // vertical
    ];

    let bs = BoundingSphere::new(center, size.length() * 0.5);

    GeometryData {
        positions: corners.to_vec(),
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
        primitive_type: PrimitiveType::Lines,
    }
}

/// Generates an ellipsoid outline geometry (3 great circles).
/// Maps to `EllipsoidOutlineGeometry`
pub fn ellipsoid_outline_geometry(radii: DVec3, stacks: u32, slices: u32) -> GeometryData {
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // XY circle (equator).
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([radii.x * theta.cos(), radii.y * theta.sin(), 0.0]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // XZ circle.
    let base = positions.len() as u32;
    for i in 0..=stacks {
        let phi = 2.0 * std::f64::consts::PI * i as f64 / stacks as f64;
        positions.push([radii.x * phi.cos(), 0.0, radii.z * phi.sin()]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // YZ circle.
    let base = positions.len() as u32;
    for i in 0..=stacks {
        let phi = 2.0 * std::f64::consts::PI * i as f64 / stacks as f64;
        positions.push([0.0, radii.y * phi.cos(), radii.z * phi.sin()]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    let max_r = radii.x.max(radii.y).max(radii.z);
    let bs = BoundingSphere::new(DVec3::ZERO, max_r);

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
        primitive_type: PrimitiveType::Lines,
    }
}

/// Generates a circle outline geometry on the ellipsoid surface.
/// Maps to `CircleOutlineGeometry`
pub fn circle_outline_geometry(
    center: DVec3,
    radius: f64,
    ellipsoid: &Ellipsoid,
    granularity: f64,
) -> GeometryData {
    let center_carto = ellipsoid.cartesian_to_cartographic(center);
    let Some(center_carto) = center_carto else {
        return empty_lines();
    };

    let num_segments = ((2.0 * std::f64::consts::PI / granularity).ceil() as u32).max(3);
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(num_segments as usize);
    let mut indices: Vec<u32> = Vec::with_capacity(num_segments as usize * 2);

    for i in 0..num_segments {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / num_segments as f64;
        let d_lat = (radius / ellipsoid.maximum_radius()) * angle.sin();
        let d_lon = (radius / (ellipsoid.maximum_radius() * center_carto.latitude.cos().max(0.01))) * angle.cos();
        let carto = crate::cartographic::Cartographic::from_radians(
            center_carto.longitude + d_lon,
            center_carto.latitude + d_lat,
            center_carto.height,
        );
        let p = ellipsoid.cartographic_to_cartesian(&carto);
        positions.push([p.x, p.y, p.z]);

        if i > 0 {
            indices.push(i - 1);
            indices.push(i);
        }
    }
    // Close the loop.
    indices.push(num_segments - 1);
    indices.push(0);

    let bs = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
        primitive_type: PrimitiveType::Lines,
    }
}

/// Generates a rectangle outline geometry on the ellipsoid surface.
/// Maps to `RectangleOutlineGeometry`
pub fn rectangle_outline_geometry(
    rect: &Rectangle,
    ellipsoid: &Ellipsoid,
    granularity: f64,
) -> GeometryData {
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let add_edge = |positions: &mut Vec<[f64; 3]>, indices: &mut Vec<u32>,
                    lon0: f64, lat0: f64, lon1: f64, lat1: f64| {
        let angular_dist = ((lat1 - lat0).powi(2) + (lon1 - lon0).powi(2)).sqrt();
        let num_seg = ((angular_dist / granularity).ceil() as usize).max(1);
        let base = positions.len() as u32;
        for i in 0..=num_seg {
            let t = i as f64 / num_seg as f64;
            let lon = math_utils::lerp(lon0, lon1, t);
            let lat = math_utils::lerp(lat0, lat1, t);
            let carto = crate::cartographic::Cartographic::from_radians(lon, lat, 0.0);
            let p = ellipsoid.cartographic_to_cartesian(&carto);
            positions.push([p.x, p.y, p.z]);
            if i > 0 {
                indices.push(base + i as u32 - 1);
                indices.push(base + i as u32);
            }
        }
    };

    // Bottom edge (west to east at south).
    add_edge(&mut positions, &mut indices, rect.west, rect.south, rect.east, rect.south);
    // Right edge (south to north at east).
    add_edge(&mut positions, &mut indices, rect.east, rect.south, rect.east, rect.north);
    // Top edge (east to west at north).
    add_edge(&mut positions, &mut indices, rect.east, rect.north, rect.west, rect.north);
    // Left edge (north to south at west).
    add_edge(&mut positions, &mut indices, rect.west, rect.north, rect.west, rect.south);

    let bs = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
        primitive_type: PrimitiveType::Lines,
    }
}

/// Generates a cylinder outline geometry.
/// Maps to `CylinderOutlineGeometry`
pub fn cylinder_outline_geometry(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: u32,
) -> GeometryData {
    let half_length = length * 0.5;
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Bottom circle.
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([bottom_radius * theta.cos(), bottom_radius * theta.sin(), -half_length]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // Top circle.
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([top_radius * theta.cos(), top_radius * theta.sin(), half_length]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // Vertical edges (connect bottom to top at intervals).
    let num_verticals = slices.min(16);
    for i in 0..num_verticals {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        let bottom_idx = positions.len() as u32;
        positions.push([bottom_radius * theta.cos(), bottom_radius * theta.sin(), -half_length]);
        let top_idx = positions.len() as u32;
        positions.push([top_radius * theta.cos(), top_radius * theta.sin(), half_length]);
        indices.push(bottom_idx);
        indices.push(top_idx);
    }

    let max_radius = top_radius.max(bottom_radius);
    let bs = BoundingSphere::new(DVec3::ZERO, (max_radius * max_radius + half_length * half_length).sqrt());

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: bs,
        primitive_type: PrimitiveType::Lines,
    }
}

/// Generates a plane outline geometry (unit quad edges).
/// Maps to `PlaneOutlineGeometry`
pub fn plane_outline_geometry() -> GeometryData {
    let positions = vec![
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    let indices = vec![0, 1, 1, 2, 2, 3, 3, 0];

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, std::f64::consts::FRAC_1_SQRT_2),
        primitive_type: PrimitiveType::Lines,
    }
}

fn empty_lines() -> GeometryData {
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

/// Triangulates a 2D polygon using the earcut algorithm.
/// `holes` is an array of starting indices of holes within `positions`.
/// Maps to `PolygonPipeline.triangulate`
pub fn triangulate_polygon(positions: &[DVec2], holes: &[u32]) -> Vec<u32> {
    let n = positions.len();
    if n < 3 {
        return Vec::new();
    }

    let data: Vec<[f64; 2]> = positions.iter().map(|p| [p.x, p.y]).collect();
    let mut earcut = earcut::Earcut::new();
    let mut triangles: Vec<u32> = Vec::new();
    earcut.earcut(data.iter().copied(), holes, &mut triangles);
    triangles
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

// ============================================================================
// GeometryPipeline functions
// ============================================================================

/// Computes per-vertex normals for a triangle geometry.
/// Maps to `GeometryPipeline.computeNormal`
pub fn compute_normal(geo: &mut GeometryData) {
    if geo.primitive_type != PrimitiveType::Triangles || geo.indices.is_empty() {
        return;
    }

    let num_vertices = geo.positions.len();
    let num_triangles = geo.indices.len() / 3;

    // Compute face normals.
    let mut face_normals: Vec<DVec3> = Vec::with_capacity(num_triangles);
    for tri in 0..num_triangles {
        let i0 = geo.indices[tri * 3] as usize;
        let i1 = geo.indices[tri * 3 + 1] as usize;
        let i2 = geo.indices[tri * 3 + 2] as usize;

        let v0 = DVec3::from(geo.positions[i0]);
        let v1 = DVec3::from(geo.positions[i1]);
        let v2 = DVec3::from(geo.positions[i2]);

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        face_normals.push(edge1.cross(edge2));
    }

    // Accumulate face normals per vertex.
    let mut vertex_normals: Vec<DVec3> = vec![DVec3::ZERO; num_vertices];
    for (tri, &fnormal) in face_normals.iter().enumerate() {
        let i0 = geo.indices[tri * 3] as usize;
        let i1 = geo.indices[tri * 3 + 1] as usize;
        let i2 = geo.indices[tri * 3 + 2] as usize;

        vertex_normals[i0] += fnormal;
        vertex_normals[i1] += fnormal;
        vertex_normals[i2] += fnormal;
    }

    // Normalize.
    let normals: Vec<[f64; 3]> = vertex_normals
        .iter()
        .map(|n| {
            let normalized = n.normalize_or(DVec3::Z);
            [normalized.x, normalized.y, normalized.z]
        })
        .collect();

    geo.normals = Some(normals);
}

/// Computes per-vertex tangents and bitangents for a triangle geometry.
/// Maps to `GeometryPipeline.computeTangentAndBitangent`
///
/// Based on "Computing Tangent Space Basis Vectors for an Arbitrary Mesh" by Eric Lengyel.
pub fn compute_tangent_and_bitangent(geo: &mut GeometryData) {
    if geo.primitive_type != PrimitiveType::Triangles || geo.indices.is_empty() {
        return;
    }

    let normals = match &geo.normals {
        Some(n) => n,
        None => {
            compute_normal(geo);
            geo.normals.as_ref().unwrap()
        }
    };
    let tex_coords = match &geo.tex_coords {
        Some(st) => st,
        None => return, // Need UVs for tangent computation.
    };

    let num_vertices = geo.positions.len();
    let num_triangles = geo.indices.len() / 3;

    let mut tan1: Vec<DVec3> = vec![DVec3::ZERO; num_vertices];
    let mut tan2: Vec<DVec3> = vec![DVec3::ZERO; num_vertices];

    for tri in 0..num_triangles {
        let i0 = geo.indices[tri * 3] as usize;
        let i1 = geo.indices[tri * 3 + 1] as usize;
        let i2 = geo.indices[tri * 3 + 2] as usize;

        let v0 = DVec3::from(geo.positions[i0]);
        let v1 = DVec3::from(geo.positions[i1]);
        let v2 = DVec3::from(geo.positions[i2]);

        let w0 = DVec2::from(tex_coords[i0]);
        let w1 = DVec2::from(tex_coords[i1]);
        let w2 = DVec2::from(tex_coords[i2]);

        let x1 = v1.x - v0.x;
        let x2 = v2.x - v0.x;
        let y1 = v1.y - v0.y;
        let y2 = v2.y - v0.y;
        let z1 = v1.z - v0.z;
        let z2 = v2.z - v0.z;

        let s1 = w1.x - w0.x;
        let s2 = w2.x - w0.x;
        let t1 = w1.y - w0.y;
        let t2 = w2.y - w0.y;

        let denom = s1 * t2 - s2 * t1;
        let r = if denom.abs() > 1e-10 { 1.0 / denom } else { 0.0 };

        let sdir = DVec3::new(
            (t2 * x1 - t1 * x2) * r,
            (t2 * y1 - t1 * y2) * r,
            (t2 * z1 - t1 * z2) * r,
        );
        let tdir = DVec3::new(
            (s1 * x2 - s2 * x1) * r,
            (s1 * y2 - s2 * y1) * r,
            (s1 * z2 - s2 * z1) * r,
        );

        tan1[i0] += sdir;
        tan1[i1] += sdir;
        tan1[i2] += sdir;

        tan2[i0] += tdir;
        tan2[i1] += tdir;
        tan2[i2] += tdir;
    }

    let mut tangents: Vec<[f64; 3]> = Vec::with_capacity(num_vertices);
    let mut bitangents: Vec<[f64; 3]> = Vec::with_capacity(num_vertices);

    for i in 0..num_vertices {
        let n = DVec3::from(normals[i]);
        let t = tan1[i];

        // Gram-Schmidt orthogonalize.
        let tangent = (t - n * n.dot(t)).normalize_or(DVec3::X);
        tangents.push([tangent.x, tangent.y, tangent.z]);

        // Calculate handedness.
        let bitangent = n.cross(tangent).normalize_or(DVec3::Y);
        bitangents.push([bitangent.x, bitangent.y, bitangent.z]);
    }

    geo.tangents = Some(tangents);
    geo.bitangents = Some(bitangents);
}

/// Converts triangle indices to line indices (wireframe).
/// Maps to `GeometryPipeline.toWireframe`
pub fn to_wireframe(geo: &mut GeometryData) {
    if geo.primitive_type != PrimitiveType::Triangles || geo.indices.is_empty() {
        return;
    }

    let num_triangles = geo.indices.len() / 3;
    let mut lines: Vec<u32> = Vec::with_capacity(num_triangles * 6);

    for tri in 0..num_triangles {
        let i0 = geo.indices[tri * 3];
        let i1 = geo.indices[tri * 3 + 1];
        let i2 = geo.indices[tri * 3 + 2];

        lines.extend_from_slice(&[i0, i1, i1, i2, i2, i0]);
    }

    geo.indices = lines;
    geo.primitive_type = PrimitiveType::Lines;
}

/// Projects 3D positions to 2D using GeographicProjection.
/// Returns (position3d, position2d) arrays.
/// Maps to `GeometryPipeline.projectTo2D`
pub fn project_to_2d(
    positions: &[[f64; 3]],
    ellipsoid: &Ellipsoid,
) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    use crate::projection::MapProjection;
    let projection = crate::projection::GeographicProjection::new(ellipsoid.clone());
    let pos3d = positions.to_vec();
    let mut pos2d: Vec<[f64; 3]> = Vec::with_capacity(positions.len());

    for p in positions {
        let cart = ellipsoid.cartesian_to_cartographic(DVec3::from(*p));
        match cart {
            Some(c) => {
                let projected = projection.project(&c);
                pos2d.push([projected.x, projected.y, projected.z]);
            }
            None => {
                pos2d.push([0.0, 0.0, 0.0]);
            }
        }
    }

    (pos3d, pos2d)
}

/// Encodes an f64 value into high and low f32 parts for GPU precision.
/// Maps to `EncodedCartesian3.encode`
pub fn encode_f64_to_f32_pair(value: f64) -> (f32, f32) {
    let high = value as f32;
    let low = (value - high as f64) as f32;
    (high, low)
}

/// Encodes a position attribute (array of [f64;3]) into high/low f32 pairs.
/// Maps to `GeometryPipeline.encodeAttribute`
pub fn encode_attribute(
    positions: &[[f64; 3]],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut high: Vec<[f32; 3]> = Vec::with_capacity(positions.len());
    let mut low: Vec<[f32; 3]> = Vec::with_capacity(positions.len());

    for p in positions {
        let (hx, lx) = encode_f64_to_f32_pair(p[0]);
        let (hy, ly) = encode_f64_to_f32_pair(p[1]);
        let (hz, lz) = encode_f64_to_f32_pair(p[2]);
        high.push([hx, hy, hz]);
        low.push([lx, ly, lz]);
    }

    (high, low)
}

/// Transforms geometry positions and normals by a model matrix.
/// Maps to `GeometryPipeline.transformToWorldCoordinates`
pub fn transform_to_world_coordinates(
    geo: &mut GeometryData,
    model_matrix: &glam::DMat4,
) {
    let normal_matrix = {
        let m3 = glam::DMat3::from_cols(
            model_matrix.x_axis.truncate(),
            model_matrix.y_axis.truncate(),
            model_matrix.z_axis.truncate(),
        );
        let det = m3.determinant();
        if det.abs() > 1e-10 {
            m3.inverse().transpose()
        } else {
            m3
        }
    };

    for p in geo.positions.iter_mut() {
        let v = model_matrix.transform_point3(DVec3::from(*p));
        *p = [v.x, v.y, v.z];
    }

    if let Some(ref mut normals) = geo.normals {
        for n in normals.iter_mut() {
            let v = normal_matrix * DVec3::from(*n);
            let normalized = v.normalize_or(DVec3::ZERO);
            *n = [normalized.x, normalized.y, normalized.z];
        }
    }

    // Update bounding sphere
    if !geo.positions.is_empty() {
        let mut center = DVec3::ZERO;
        for p in &geo.positions {
            center += DVec3::from(*p);
        }
        center /= geo.positions.len() as f64;
        let mut max_dist_sq = 0.0_f64;
        for p in &geo.positions {
            let d = (DVec3::from(*p) - center).length_squared();
            if d > max_dist_sq {
                max_dist_sq = d;
            }
        }
        geo.bounding_sphere = BoundingSphere::new(center, max_dist_sq.sqrt());
    }
}

/// Compresses vertex normals using oct encoding and packs with texture coordinates.
/// Maps to `GeometryPipeline.compressVertices`
pub fn compress_vertices(geo: &GeometryData) -> Option<Vec<u32>> {
    let normals = geo.normals.as_ref()?;
    let num_vertices = normals.len();

    // Pack compressed normals (2 u16 per vertex) + optional ST (2 u16 per vertex)
    let has_st = geo.tex_coords.is_some();
    let components_per_vertex = if has_st { 4 } else { 2 };
    let mut compressed: Vec<u32> = Vec::with_capacity(num_vertices * components_per_vertex / 2 + 1);

    let st = geo.tex_coords.as_ref();

    for i in 0..num_vertices {
        let n = DVec3::from(normals[i]);
        // Oct encode normal to 2 bytes
        let oct = crate::attribute_compression::oct_encode(n);
        let oct_x = (oct.x.round() as u32) & 0xFFFF;
        let oct_y = (oct.y.round() as u32) & 0xFFFF;

        if let Some(sts) = st {
            let s = (sts[i][0].clamp(0.0, 1.0) * 65535.0).round() as u32;
            let t = (sts[i][1].clamp(0.0, 1.0) * 65535.0).round() as u32;
            // Pack: [normal_xy(u32), st(u32)]
            let normal_packed = oct_x | (oct_y << 16);
            let st_packed = s | (t << 16);
            compressed.push(normal_packed);
            compressed.push(st_packed);
        } else {
            let normal_packed = oct_x | (oct_y << 16);
            compressed.push(normal_packed);
        }
    }

    Some(compressed)
}

/// Creates line segments for vector attributes (e.g., normals visualization).
/// Maps to `GeometryPipeline.createLineSegmentsForVectors`
pub fn create_line_segments_for_vectors(
    positions: &[[f64; 3]],
    vectors: &[[f64; 3]],
    length: f64,
) -> GeometryData {
    let mut line_positions: Vec<[f64; 3]> = Vec::with_capacity(positions.len() * 2);

    for i in 0..positions.len() {
        let p = DVec3::from(positions[i]);
        let v = DVec3::from(vectors[i]) * length;
        line_positions.push(positions[i]);
        let end = p + v;
        line_positions.push([end.x, end.y, end.z]);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(positions.len() * 2);
    for i in 0..positions.len() as u32 {
        indices.push(i * 2);
        indices.push(i * 2 + 1);
    }

    // Bounding sphere: center same as input, radius + length
    let mut center = DVec3::ZERO;
    for p in positions {
        center += DVec3::from(*p);
    }
    if !positions.is_empty() {
        center /= positions.len() as f64;
    }
    let mut max_dist_sq = 0.0_f64;
    for p in positions {
        let d = (DVec3::from(*p) - center).length_squared();
        if d > max_dist_sq {
            max_dist_sq = d;
        }
    }
    let radius = max_dist_sq.sqrt() + length;

    GeometryData {
        positions: line_positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere: BoundingSphere::new(center, radius),
        primitive_type: PrimitiveType::Lines,
    }
}

/// Reorders geometry indices and attributes for pre-vertex cache optimization.
/// Maps to `GeometryPipeline.reorderForPreVertexCache`
pub fn reorder_for_pre_vertex_cache(geo: &mut GeometryData) {
    if geo.indices.is_empty() {
        return;
    }

    let num_vertices = geo.positions.len();
    let mut used: Vec<bool> = vec![false; num_vertices];
    let mut remap: Vec<Option<u32>> = vec![None; num_vertices];
    let mut new_index: u32 = 0;

    // First pass: determine which vertices are used and assign new indices
    for &idx in &geo.indices {
        let i = idx as usize;
        if i < num_vertices && !used[i] {
            used[i] = true;
            remap[i] = Some(new_index);
            new_index += 1;
        }
    }

    // Remap indices
    let new_indices: Vec<u32> = geo.indices.iter().map(|&idx| {
        remap[idx as usize].unwrap_or(0)
    }).collect();
    geo.indices = new_indices;

    // Compact attributes
    let used_indices: Vec<usize> = (0..num_vertices)
        .filter(|&i| used[i])
        .collect();

    geo.positions = used_indices.iter().map(|&i| geo.positions[i]).collect();
    if let Some(ref mut normals) = geo.normals {
        *normals = used_indices.iter().map(|&i| normals[i]).collect();
    }
    if let Some(ref mut st) = geo.tex_coords {
        *st = used_indices.iter().map(|&i| st[i]).collect();
    }
    if let Some(ref mut tangents) = geo.tangents {
        *tangents = used_indices.iter().map(|&i| tangents[i]).collect();
    }
    if let Some(ref mut bitangents) = geo.bitangents {
        *bitangents = used_indices.iter().map(|&i| bitangents[i]).collect();
    }
}

/// Splits geometry into multiple geometries that fit in u16 indices (65536 vertices max).
/// Maps to `GeometryPipeline.fitToUnsignedShortIndices`
pub fn fit_to_unsigned_short_indices(geo: &GeometryData) -> Vec<GeometryData> {
    const MAX_VERTICES: usize = 65536;
    let num_vertices = geo.positions.len();

    if num_vertices <= MAX_VERTICES {
        return vec![geo.clone()];
    }

    let vertices_per_primitive = match geo.primitive_type {
        PrimitiveType::Triangles => 3,
        PrimitiveType::Lines => 2,
    };

    let mut result: Vec<GeometryData> = Vec::new();
    let mut current_vertices: Vec<[f64; 3]> = Vec::new();
    let mut current_normals: Vec<[f64; 3]> = Vec::new();
    let mut current_st: Vec<[f64; 2]> = Vec::new();
    let mut current_indices: Vec<u32> = Vec::new();
    let mut vertex_map: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();

    let num_primitives = geo.indices.len() / vertices_per_primitive;

    for prim in 0..num_primitives {
        let base = prim * vertices_per_primitive;

        // Check if adding this primitive would exceed limit
        let mut new_vertices_needed = 0;
        for k in 0..vertices_per_primitive {
            let old_idx = geo.indices[base + k] as usize;
            if !vertex_map.contains_key(&old_idx) {
                new_vertices_needed += 1;
            }
        }

        if current_vertices.len() + new_vertices_needed > MAX_VERTICES && !current_vertices.is_empty() {
            // Flush current batch
            result.push(GeometryData {
                positions: std::mem::take(&mut current_vertices),
                normals: if geo.normals.is_some() { Some(std::mem::take(&mut current_normals)) } else { None },
                tex_coords: if geo.tex_coords.is_some() { Some(std::mem::take(&mut current_st)) } else { None },
                tangents: None,
                bitangents: None,
                indices: std::mem::take(&mut current_indices),
                bounding_sphere: geo.bounding_sphere.clone(),
                primitive_type: geo.primitive_type,
            });
            vertex_map.clear();
        }

        // Add vertices and indices
        for k in 0..vertices_per_primitive {
            let old_idx = geo.indices[base + k] as usize;
            let new_idx = *vertex_map.entry(old_idx).or_insert_with(|| {
                let idx = current_vertices.len() as u32;
                current_vertices.push(geo.positions[old_idx]);
                if let Some(ref normals) = geo.normals {
                    current_normals.push(normals[old_idx]);
                }
                if let Some(ref st) = geo.tex_coords {
                    current_st.push(st[old_idx]);
                }
                idx
            });
            current_indices.push(new_idx);
        }
    }

    // Flush remaining
    if !current_vertices.is_empty() {
        result.push(GeometryData {
            positions: current_vertices,
            normals: if geo.normals.is_some() { Some(current_normals) } else { None },
            tex_coords: if geo.tex_coords.is_some() { Some(current_st) } else { None },
            tangents: None,
            bitangents: None,
            indices: current_indices,
            bounding_sphere: geo.bounding_sphere.clone(),
            primitive_type: geo.primitive_type,
        });
    }

    result
}

/// Splits geometry that crosses the international date line (longitude ±π).
/// Returns split geometries (west/east halves).
/// Maps to `GeometryPipeline.splitLongitude`
pub fn split_longitude(geo: &GeometryData, ellipsoid: &Ellipsoid) -> Vec<GeometryData> {
    if geo.positions.is_empty() || geo.primitive_type != PrimitiveType::Triangles {
        return vec![geo.clone()];
    }

    // Convert positions to cartographic and check if any cross IDL
    let cartos: Vec<Option<crate::cartographic::Cartographic>> = geo.positions.iter()
        .map(|p| ellipsoid.cartesian_to_cartographic(DVec3::from(*p)))
        .collect();

    // Check if geometry crosses the IDL:
    // 1. Any triangle has vertices with longitude difference > PI, OR
    // 2. The geometry has vertices on both sides of the IDL (lon > PI/2 and lon < -PI/2)
    let mut crosses_idl = false;
    let mut has_far_east = false;  // longitude > PI/2 (90°E)
    let mut has_far_west = false;  // longitude < -PI/2 (90°W)

    for c in cartos.iter().flatten() {
        if c.longitude > std::f64::consts::FRAC_PI_2 {
            has_far_east = true;
        }
        if c.longitude < -std::f64::consts::FRAC_PI_2 {
            has_far_west = true;
        }
    }
    if has_far_east && has_far_west {
        crosses_idl = true;
    }

    // Also check for large longitude jumps within triangles
    if !crosses_idl {
        for tri in 0..geo.indices.len() / 3 {
            let i0 = geo.indices[tri * 3] as usize;
            let i1 = geo.indices[tri * 3 + 1] as usize;
            let i2 = geo.indices[tri * 3 + 2] as usize;

            if let (Some(c0), Some(c1), Some(c2)) = (&cartos[i0], &cartos[i1], &cartos[i2]) {
                let lons = [c0.longitude, c1.longitude, c2.longitude];
                for a in 0..3 {
                    for b in (a+1)..3 {
                        if (lons[a] - lons[b]).abs() > std::f64::consts::PI {
                            crosses_idl = true;
                            break;
                        }
                    }
                    if crosses_idl { break; }
                }
            }
            if crosses_idl { break; }
        }
    }

    if !crosses_idl {
        return vec![geo.clone()];
    }

    // For IDL-crossing geometry, split into east (positive) and west (negative) parts
    // This is a simplified version - full CesiumJS implementation interpolates at the IDL
    let mut east_positions: Vec<[f64; 3]> = Vec::new();
    let mut west_positions: Vec<[f64; 3]> = Vec::new();
    let mut east_indices: Vec<u32> = Vec::new();
    let mut west_indices: Vec<u32> = Vec::new();
    let mut east_map: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    let mut west_map: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();

    for tri in 0..geo.indices.len() / 3 {
        let i0 = geo.indices[tri * 3] as usize;
        let i1 = geo.indices[tri * 3 + 1] as usize;
        let i2 = geo.indices[tri * 3 + 2] as usize;

        // Determine which side this triangle belongs to (majority vote)
        let mut east_count = 0;
        let mut west_count = 0;
        for &idx in &[i0, i1, i2] {
            if let Some(Some(c)) = cartos.get(idx) {
                if c.longitude >= 0.0 {
                    east_count += 1;
                } else {
                    west_count += 1;
                }
            }
        }

        if east_count >= west_count {
            // Assign to east
            for &idx in &[i0, i1, i2] {
                let new_idx = *east_map.entry(idx).or_insert_with(|| {
                    let i = east_positions.len() as u32;
                    east_positions.push(geo.positions[idx]);
                    i
                });
                east_indices.push(new_idx);
            }
        } else {
            // Assign to west
            for &idx in &[i0, i1, i2] {
                let new_idx = *west_map.entry(idx).or_insert_with(|| {
                    let i = west_positions.len() as u32;
                    west_positions.push(geo.positions[idx]);
                    i
                });
                west_indices.push(new_idx);
            }
        }
    }

    let mut result = Vec::new();
    if !east_positions.is_empty() {
        result.push(GeometryData {
            positions: east_positions,
            normals: None,
            tex_coords: None,
            tangents: None,
            bitangents: None,
            indices: east_indices,
            bounding_sphere: geo.bounding_sphere.clone(),
            primitive_type: PrimitiveType::Triangles,
        });
    }
    if !west_positions.is_empty() {
        result.push(GeometryData {
            positions: west_positions,
            normals: None,
            tex_coords: None,
            tangents: None,
            bitangents: None,
            indices: west_indices,
            bounding_sphere: geo.bounding_sphere.clone(),
            primitive_type: PrimitiveType::Triangles,
        });
    }

    if result.is_empty() {
        vec![geo.clone()]
    } else {
        result
    }
}

/// Combines several geometries into one by concatenating attributes,
/// concatenating and adjusting indices, and creating a bounding sphere
/// encompassing all inputs.
///
/// If the geometries do not all share an optional attribute (normals,
/// texture coordinates, tangents, bitangents), that attribute is dropped
/// from the result. Indices are combined only when every input geometry
/// has a non-empty index list; otherwise the result has no indices.
///
/// Maps to CesiumJS `GeometryPipeline.combineInstances` / `combineGeometries`.
pub fn combine_geometries(geometries: &[GeometryData]) -> GeometryData {
    assert!(
        !geometries.is_empty(),
        "geometries must have length greater than zero"
    );

    let primitive_type = geometries[0].primitive_type;
    let have_indices = !geometries[0].indices.is_empty();

    for geo in &geometries[1..] {
        assert_eq!(
            geo.primitive_type, primitive_type,
            "All geometries must have the same primitiveType."
        );
        assert_eq!(
            !geo.indices.is_empty(),
            have_indices,
            "All geometries must have an indices or not have one."
        );
    }

    // Combine positions.
    let mut positions: Vec<[f64; 3]> = Vec::new();
    for geo in geometries {
        positions.extend_from_slice(&geo.positions);
    }

    // Combine optional attributes only if present in ALL geometries.
    let all_have = |f: fn(&GeometryData) -> &Option<Vec<[f64; 3]>>| -> bool {
        geometries.iter().all(|g| f(g).is_some())
    };

    let normals = if all_have(|g| &g.normals) {
        let mut v = Vec::new();
        for geo in geometries {
            v.extend_from_slice(geo.normals.as_ref().unwrap());
        }
        Some(v)
    } else {
        None
    };

    let tangents = if all_have(|g| &g.tangents) {
        let mut v = Vec::new();
        for geo in geometries {
            v.extend_from_slice(geo.tangents.as_ref().unwrap());
        }
        Some(v)
    } else {
        None
    };

    let bitangents = if all_have(|g| &g.bitangents) {
        let mut v = Vec::new();
        for geo in geometries {
            v.extend_from_slice(geo.bitangents.as_ref().unwrap());
        }
        Some(v)
    } else {
        None
    };

    let tex_coords = if geometries.iter().all(|g| g.tex_coords.is_some()) {
        let mut v = Vec::new();
        for geo in geometries {
            v.extend_from_slice(geo.tex_coords.as_ref().unwrap());
        }
        Some(v)
    } else {
        None
    };

    // Combine index lists with per-geometry vertex offsets.
    let indices = if have_indices {
        let mut dest: Vec<u32> = Vec::new();
        let mut offset: u32 = 0;
        for geo in geometries {
            for &idx in &geo.indices {
                dest.push(offset + idx);
            }
            offset += geo.positions.len() as u32;
        }
        dest
    } else {
        Vec::new()
    };

    // Create a bounding sphere that includes all geometries.
    let mut bounding_sphere = geometries[0].bounding_sphere.clone();
    for geo in &geometries[1..] {
        bounding_sphere = bounding_sphere.union(&geo.bounding_sphere);
    }

    GeometryData {
        positions,
        normals,
        tex_coords,
        tangents,
        bitangents,
        indices,
        bounding_sphere,
        primitive_type,
    }
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

    #[test]
    fn test_box_outline() {
        let geo = box_outline_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0));
        assert_eq!(geo.positions.len(), 8);
        assert_eq!(geo.indices.len(), 24); // 12 edges * 2
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_ellipsoid_outline() {
        let geo = ellipsoid_outline_geometry(DVec3::new(1.0, 2.0, 3.0), 16, 32);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 2, 0);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_circle_outline() {
        let ell = Ellipsoid::WGS84;
        let center = ell.cartographic_to_cartesian(&crate::cartographic::Cartographic::from_degrees(0.0, 0.0, 0.0));
        let geo = circle_outline_geometry(center, 100_000.0, &ell, math_utils::to_radians(1.0));
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 2, 0);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_rectangle_outline() {
        let ell = Ellipsoid::WGS84;
        let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let geo = rectangle_outline_geometry(&rect, &ell, math_utils::to_radians(1.0));
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 2, 0);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_cylinder_outline() {
        let geo = cylinder_outline_geometry(2.0, 1.0, 1.0, 16);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 2, 0);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_plane_outline() {
        let geo = plane_outline_geometry();
        assert_eq!(geo.positions.len(), 4);
        assert_eq!(geo.indices.len(), 8); // 4 edges * 2
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
    }

    #[test]
    fn test_compute_normal() {
        let mut geo = box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::POSITION_ONLY);
        assert!(geo.normals.is_none());
        compute_normal(&mut geo);
        assert!(geo.normals.is_some());
        let normals = geo.normals.unwrap();
        assert_eq!(normals.len(), geo.positions.len());
        // All normals should be unit length.
        for n in &normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_compute_tangent_and_bitangent() {
        let mut geo = box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::ALL);
        // box_geometry doesn't compute tangents, so we compute them.
        assert!(geo.tangents.is_none());
        compute_tangent_and_bitangent(&mut geo);
        assert!(geo.tangents.is_some());
        assert!(geo.bitangents.is_some());
        let tangents = geo.tangents.unwrap();
        assert_eq!(tangents.len(), geo.positions.len());
    }

    #[test]
    fn test_to_wireframe() {
        let mut geo = box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::POSITION_ONLY);
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        let tri_count = geo.indices.len() / 3;
        to_wireframe(&mut geo);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
        assert_eq!(geo.indices.len(), tri_count * 6); // Each triangle -> 3 edges -> 6 indices
    }
}
