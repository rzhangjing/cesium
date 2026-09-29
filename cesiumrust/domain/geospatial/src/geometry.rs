//! 几何生成 - 所有程序化几何类型。
//! 映射到 CesiumJS `Core/*Geometry.js`（20+ 个文件）、`Core/PolygonPipeline.js`、`Core/PolylinePipeline.js`

// 遗留的 CesiumJS 移植风格技术债（deferred.md #18）；在 M13 lint-cleanup
// 或本文件在其里程碑被重写时重新审视
#![allow(clippy::clone_on_copy)]
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

/// 顶点格式标志 - 生成哪些属性。
/// 映射到 CesiumJS `VertexFormat`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VertexFormat {
    pub position: bool,
    pub normal: bool,
    pub st: bool,
    pub tangent: bool,
    pub bitangent: bool,
}

impl VertexFormat {
    /// 启用所有属性。
    pub const ALL: Self = Self {
        position: true,
        normal: true,
        st: true,
        tangent: true,
        bitangent: true,
    };

    /// 仅位置。
    pub const POSITION_ONLY: Self = Self {
        position: true,
        normal: false,
        st: false,
        tangent: false,
        bitangent: false,
    };

    /// 位置和法线。
    pub const POSITION_AND_NORMAL: Self = Self {
        position: true,
        normal: true,
        st: false,
        tangent: false,
        bitangent: false,
    };

    /// 位置和纹理坐标。
    pub const POSITION_AND_ST: Self = Self {
        position: true,
        normal: false,
        st: true,
        tangent: false,
        bitangent: false,
    };

    /// 打包本结构所用的元素数量。
    pub const PACKED_LENGTH: usize = 5;

    /// 将本 VertexFormat 打包进一个扁平数组。
    ///
    /// 映射到 CesiumJS `VertexFormat.pack`
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = if self.position { 1.0 } else { 0.0 };
        array[starting_index + 1] = if self.normal { 1.0 } else { 0.0 };
        array[starting_index + 2] = if self.st { 1.0 } else { 0.0 };
        array[starting_index + 3] = if self.tangent { 1.0 } else { 0.0 };
        array[starting_index + 4] = if self.bitangent { 1.0 } else { 0.0 };
    }

    /// 从一个扁平数组解包出 VertexFormat。
    ///
    /// 映射到 CesiumJS `VertexFormat.unpack`
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            position: array[starting_index] != 0.0,
            normal: array[starting_index + 1] != 0.0,
            st: array[starting_index + 2] != 0.0,
            tangent: array[starting_index + 3] != 0.0,
            bitangent: array[starting_index + 4] != 0.0,
        }
    }

    /// 将本 VertexFormat 打包进一个新的 Vec<f64>。
    pub fn pack_array(&self) -> Vec<f64> {
        let mut array = vec![0.0; Self::PACKED_LENGTH];
        self.pack(&mut array, 0);
        array
    }

    /// 从一个 Vec<f64> 解包出 VertexFormat。
    pub fn unpack_array(array: &[f64]) -> Self {
        Self::unpack(array, 0)
    }
}

impl Default for VertexFormat {
    fn default() -> Self {
        Self::ALL
    }
}

/// 所生成几何的图元拓扑。
/// 映射到 CesiumJS `PrimitiveType`（TRIANGLES / LINES）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// 三角形列表（填充表面）。
    #[default]
    Triangles,
    /// 线列表（轮廓；索引为顶点对）。
    Lines,
}

/// 中间几何表示（f64 精度，与 GPU 解耦）。
/// 映射到 CesiumJS 几何 worker 的输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryData {
    /// 顶点位置（始终存在）。
    pub positions: Vec<[f64; 3]>,
    /// 顶点法线（可选）。
    pub normals: Option<Vec<[f64; 3]>>,
    /// 纹理坐标（可选）。
    pub tex_coords: Option<Vec<[f64; 2]>>,
    /// 切线向量（可选）。
    pub tangents: Option<Vec<[f64; 3]>>,
    /// 副切线向量（可选）。
    pub bitangents: Option<Vec<[f64; 3]>>,
    /// 索引（根据 `primitive_type` 为三角形或线对）。
    pub indices: Vec<u32>,
    /// 几何的包围球。
    pub bounding_sphere: BoundingSphere,
    /// 图元拓扑（填充用三角形，轮廓用线）。
    #[serde(default)]
    pub primitive_type: PrimitiveType,
}

/// 多边形层级：外环 + 可选的洞。
/// 映射到 CesiumJS `PolygonHierarchy`
#[derive(Debug, Clone)]
pub struct PolygonHierarchy {
    /// 外环位置（测绘坐标或笛卡尔坐标）。
    pub positions: Vec<DVec3>,
    /// 洞（每个洞都是一圈位置）。
    pub holes: Vec<PolygonHierarchy>,
}

// ============================================================================
// 几何生成器
// ============================================================================

/// 生成一个椭球几何。
/// 映射到 `EllipsoidGeometry` / `Workers/createEllipsoidGeometry`
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
                // 法线是单位球上归一化后的位置
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
            // 从外侧观察时为逆时针绕序（法线朝外）
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

/// 生成一个球体几何。
/// 映射到 `SphereGeometry`
pub fn sphere_geometry(radius: f64, stacks: u32, slices: u32, vf: VertexFormat) -> GeometryData {
    ellipsoid_geometry(DVec3::splat(radius), stacks, slices, vf)
}

/// 生成一个盒子几何。
/// 映射到 `BoxGeometry` / `Workers/createBoxGeometry`
pub fn box_geometry(minimum: DVec3, maximum: DVec3, vf: VertexFormat) -> GeometryData {
    let size = maximum - minimum;
    let center = (minimum + maximum) * 0.5;

    // 6 个面，每个面 4 个顶点 = 24 个顶点
    let corners = [
        // +X 面
        [1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0],
        // -X 面
        [-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, 1.0, -1.0],
        // +Y 面
        [-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0],
        // -Y 面
        [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0],
        // +Z 面
        [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        // -Z 面
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

/// 生成一个圆柱几何。
/// 映射到 `CylinderGeometry`
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

    // 侧面顶点
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // 底部顶点
        positions.push([cos_t * bottom_radius, sin_t * bottom_radius, -half_length]);
        if let Some(ref mut n) = normals_vec {
            n.push([cos_t, sin_t, 0.0]);
        }
        if let Some(ref mut st) = tex_coords {
            st.push([i as f64 / slices as f64, 0.0]);
        }

        // 顶部顶点
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

/// 在椭球表面上生成一个矩形几何。
/// 映射到 `RectangleGeometry` / `Workers/createRectangleGeometry`
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

/// 在椭球上生成一个圆形几何。
/// 映射到 `CircleGeometry`
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

    // 中心顶点
    positions.push([center.x, center.y, center.z]);
    if let Some(ref mut n) = normals_vec {
        let normal = ellipsoid.geodetic_surface_normal(center).unwrap_or(DVec3::Z);
        n.push([normal.x, normal.y, normal.z]);
    }

    // 环绕顶点
    for i in 0..=segments {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
        // 近似：沿表面以米为单位的偏移
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

/// 生成一个平面几何（XY 平面中的单位四边形）。
/// 映射到 `PlaneGeometry`
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

/// 生成一个盒子轮廓几何（12 条边作为线段）。
/// 映射到 `BoxOutlineGeometry`
pub fn box_outline_geometry(minimum: DVec3, maximum: DVec3) -> GeometryData {
    let size = maximum - minimum;
    let center = (minimum + maximum) * 0.5;
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    // 盒子的 8 个角点。
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

    // 12 条边：4 条底、4 条顶、4 条垂直。
    let indices: Vec<u32> = vec![
        0, 1, 1, 2, 2, 3, 3, 0, // 底部
        4, 5, 5, 6, 6, 7, 7, 4, // 顶部
        0, 4, 1, 5, 2, 6, 3, 7, // 垂直
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

/// 在椭球上生成一个椭球轮廓几何（3 个大圆）。
/// 映射到 `EllipsoidOutlineGeometry`
pub fn ellipsoid_outline_geometry(radii: DVec3, stacks: u32, slices: u32) -> GeometryData {
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // XY 圆（赤道）。
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([radii.x * theta.cos(), radii.y * theta.sin(), 0.0]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // XZ 圆。
    let base = positions.len() as u32;
    for i in 0..=stacks {
        let phi = 2.0 * std::f64::consts::PI * i as f64 / stacks as f64;
        positions.push([radii.x * phi.cos(), 0.0, radii.z * phi.sin()]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // YZ 圆。
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

/// 在椭球表面上生成一个圆形轮廓几何。
/// 映射到 `CircleOutlineGeometry`
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
    // 闭合环路。
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

/// 在椭球表面上生成一个矩形轮廓几何。
/// 映射到 `RectangleOutlineGeometry`
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

    // 底边（沿南边从西到东）。
    add_edge(&mut positions, &mut indices, rect.west, rect.south, rect.east, rect.south);
    // 右边（沿东边从南到北）。
    add_edge(&mut positions, &mut indices, rect.east, rect.south, rect.east, rect.north);
    // 顶边（沿北边从东到西）。
    add_edge(&mut positions, &mut indices, rect.east, rect.north, rect.west, rect.north);
    // 左边（沿西边从北到南）。
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

/// 生成一个圆柱轮廓几何。
/// 映射到 `CylinderOutlineGeometry`
pub fn cylinder_outline_geometry(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: u32,
) -> GeometryData {
    let half_length = length * 0.5;
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // 底部圆。
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([bottom_radius * theta.cos(), bottom_radius * theta.sin(), -half_length]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // 顶部圆。
    let base = positions.len() as u32;
    for i in 0..=slices {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / slices as f64;
        positions.push([top_radius * theta.cos(), top_radius * theta.sin(), half_length]);
        if i > 0 {
            indices.push(base + i - 1);
            indices.push(base + i);
        }
    }

    // 垂直边（按间隔连接底部与顶部）。
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

/// 生成一个平面轮廓几何（单位四边形各边）。
/// 映射到 `PlaneOutlineGeometry`
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
// 管道算法
// ============================================================================

/// 以给定的细分粒度在两个位置之间生成一段弧（大圆）。
/// 映射到 `PolylinePipeline.generateArc`
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

/// 使用 earcut 算法对一个 2D 多边形进行三角剖分。
/// `holes` 是 `positions` 中各洞起始索引的数组。
/// 映射到 `PolygonPipeline.triangulate`
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

/// 计算一个 2D 多边形的带符号面积。
/// 映射到 `PolygonPipeline.computeArea2D`
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

/// 计算一个 2D 多边形的绕序。
/// 映射到 `PolygonPipeline.computeWindingOrder2D`
pub fn compute_winding_order(positions: &[DVec2]) -> WindingOrder {
    if compute_area2d(positions) > 0.0 {
        WindingOrder::CounterClockwise
    } else {
        WindingOrder::Clockwise
    }
}

/// 多边形的绕序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
}

// ============================================================================
// GeometryPipeline 函数
// ============================================================================

/// 为三角形几何计算逐顶点法线。
/// 映射到 `GeometryPipeline.computeNormal`
pub fn compute_normal(geo: &mut GeometryData) {
    if geo.primitive_type != PrimitiveType::Triangles || geo.indices.is_empty() {
        return;
    }

    let num_vertices = geo.positions.len();
    let num_triangles = geo.indices.len() / 3;

    // 计算面法线。
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

    // 逐顶点累加面法线。
    let mut vertex_normals: Vec<DVec3> = vec![DVec3::ZERO; num_vertices];
    for (tri, &fnormal) in face_normals.iter().enumerate() {
        let i0 = geo.indices[tri * 3] as usize;
        let i1 = geo.indices[tri * 3 + 1] as usize;
        let i2 = geo.indices[tri * 3 + 2] as usize;

        vertex_normals[i0] += fnormal;
        vertex_normals[i1] += fnormal;
        vertex_normals[i2] += fnormal;
    }

    // 归一化。
    let normals: Vec<[f64; 3]> = vertex_normals
        .iter()
        .map(|n| {
            let normalized = n.normalize_or(DVec3::Z);
            [normalized.x, normalized.y, normalized.z]
        })
        .collect();

    geo.normals = Some(normals);
}

/// 为三角形几何计算逐顶点切线和副切线。
/// 映射到 `GeometryPipeline.computeTangentAndBitangent`
///
/// 基于 Eric Lengyel 的《Computing Tangent Space Basis Vectors for an Arbitrary Mesh》。
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
        None => return, // 计算切线需要 UV。
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

        // Gram-Schmidt 正交化。
        let tangent = (t - n * n.dot(t)).normalize_or(DVec3::X);
        tangents.push([tangent.x, tangent.y, tangent.z]);

        // 计算手性。
        let bitangent = n.cross(tangent).normalize_or(DVec3::Y);
        bitangents.push([bitangent.x, bitangent.y, bitangent.z]);
    }

    geo.tangents = Some(tangents);
    geo.bitangents = Some(bitangents);
}

/// 将三角形索引转换为线索引（线框）。
/// 映射到 `GeometryPipeline.toWireframe`
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

/// 使用 GeographicProjection 将 3D 位置投影到 2D。
/// 返回 (position3d, position2d) 数组。
/// 映射到 `GeometryPipeline.projectTo2D`
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

/// 将一个 f64 值编码为高/低两个 f32 部分，用于 GPU 精度。
/// 映射到 `EncodedCartesian3.encode`
pub fn encode_f64_to_f32_pair(value: f64) -> (f32, f32) {
    let high = value as f32;
    let low = (value - high as f64) as f32;
    (high, low)
}

/// 将一个位置属性（[f64;3] 数组）编码为高/低 f32 对。
/// 映射到 `GeometryPipeline.encodeAttribute`
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

/// 用一个模型矩阵变换几何的位置和法线。
/// 映射到 `GeometryPipeline.transformToWorldCoordinates`
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

    // 更新包围球
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

/// 使用八面体编码压缩顶点法线，并与纹理坐标一起打包。
/// 映射到 `GeometryPipeline.compressVertices`
pub fn compress_vertices(geo: &GeometryData) -> Option<Vec<u32>> {
    let normals = geo.normals.as_ref()?;
    let num_vertices = normals.len();

    // 打包压缩后的法线（每顶点 2 个 u16）+ 可选的 ST（每顶点 2 个 u16）
    let has_st = geo.tex_coords.is_some();
    let components_per_vertex = if has_st { 4 } else { 2 };
    let mut compressed: Vec<u32> = Vec::with_capacity(num_vertices * components_per_vertex / 2 + 1);

    let st = geo.tex_coords.as_ref();

    for i in 0..num_vertices {
        let n = DVec3::from(normals[i]);
        // 将法线八面体编码为 2 字节
        let oct = crate::attribute_compression::oct_encode(n);
        let oct_x = (oct.x.round() as u32) & 0xFFFF;
        let oct_y = (oct.y.round() as u32) & 0xFFFF;

        if let Some(sts) = st {
            let s = (sts[i][0].clamp(0.0, 1.0) * 65535.0).round() as u32;
            let t = (sts[i][1].clamp(0.0, 1.0) * 65535.0).round() as u32;
            // 打包：[normal_xy(u32), st(u32)]
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

/// 为向量属性（例如法线可视化）创建线段。
/// 映射到 `GeometryPipeline.createLineSegmentsForVectors`
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

    // 包围球：中心与输入相同，半径 + length
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

/// 重新排序几何的索引和属性，以优化顶点前置缓存。
/// 映射到 `GeometryPipeline.reorderForPreVertexCache`
pub fn reorder_for_pre_vertex_cache(geo: &mut GeometryData) {
    if geo.indices.is_empty() {
        return;
    }

    let num_vertices = geo.positions.len();
    let mut used: Vec<bool> = vec![false; num_vertices];
    let mut remap: Vec<Option<u32>> = vec![None; num_vertices];
    let mut new_index: u32 = 0;

    // 第一遍：确定哪些顶点被使用，并分配新索引
    for &idx in &geo.indices {
        let i = idx as usize;
        if i < num_vertices && !used[i] {
            used[i] = true;
            remap[i] = Some(new_index);
            new_index += 1;
        }
    }

    // 重映射索引
    let new_indices: Vec<u32> = geo.indices.iter().map(|&idx| {
        remap[idx as usize].unwrap_or(0)
    }).collect();
    geo.indices = new_indices;

    // 压缩属性
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

/// 将几何拆分为多个能容纳在 u16 索引（最多 65536 个顶点）内的几何。
/// 映射到 `GeometryPipeline.fitToUnsignedShortIndices`
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

        // 检查添加此图元是否会超出限制
        let mut new_vertices_needed = 0;
        for k in 0..vertices_per_primitive {
            let old_idx = geo.indices[base + k] as usize;
            if !vertex_map.contains_key(&old_idx) {
                new_vertices_needed += 1;
            }
        }

        if current_vertices.len() + new_vertices_needed > MAX_VERTICES && !current_vertices.is_empty() {
            // 刷写当前批次
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

        // 添加顶点和索引
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

    // 刷写剩余部分
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

/// 拆分穿越国际日期变更线（经度 ±π）的几何。
/// 返回拆分后的几何（西/东两半）。
/// 映射到 `GeometryPipeline.splitLongitude`
pub fn split_longitude(geo: &GeometryData, ellipsoid: &Ellipsoid) -> Vec<GeometryData> {
    if geo.positions.is_empty() || geo.primitive_type != PrimitiveType::Triangles {
        return vec![geo.clone()];
    }

    // 将位置转换为测绘坐标，并检查是否有穿越 IDL 的
    let cartos: Vec<Option<crate::cartographic::Cartographic>> = geo.positions.iter()
        .map(|p| ellipsoid.cartesian_to_cartographic(DVec3::from(*p)))
        .collect();

    // 检查几何是否穿越 IDL：
    // 1. 某个三角形的顶点经度差 > PI，或
    // 2. 几何在 IDL 两侧都有顶点（lon > PI/2 且 lon < -PI/2）
    let mut crosses_idl = false;
    let mut has_far_east = false;  // 经度 > PI/2（90°E）
    let mut has_far_west = false;  // 经度 < -PI/2（90°W）

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

    // 还检查三角形内部的大幅度经度跳变
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

    // 对于穿越 IDL 的几何，拆分为东（止）和西（负）两部分
    // 这是一个简化版本 - 完整的 CesiumJS 实现会在 IDL 处进行插值
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

        // 确定此三角形属于哪一侧（多数投票）
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
            // 归入东侧
            for &idx in &[i0, i1, i2] {
                let new_idx = *east_map.entry(idx).or_insert_with(|| {
                    let i = east_positions.len() as u32;
                    east_positions.push(geo.positions[idx]);
                    i
                });
                east_indices.push(new_idx);
            }
        } else {
            // 归入西侧
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

/// 通过拼接属性、拼接并调整索引、以及创建一个
/// 包含所有输入的统一包围球，将多个几何合并为一个。
///
/// 若这些几何并非都共享某个可选属性（法线、
/// 纹理坐标、切线、副切线），则该属性会从结果中丢弃。
/// 仅当每个输入几何都有非空索引列表时才合并索引；否则结果没有索引。
///
/// 映射到 CesiumJS `GeometryPipeline.combineInstances` / `combineGeometries`。
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

    // 合并位置。
    let mut positions: Vec<[f64; 3]> = Vec::new();
    for geo in geometries {
        positions.extend_from_slice(&geo.positions);
    }

    // 仅当某个可选属性存在于所有几何中时才合并它。
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

    // 按每个几何的顶点偏移合并索引列表。
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

    // 创建一个包含所有几何的包围球。
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
        // (stacks+1) * (slices+1) 个顶点
        assert_eq!(geo.positions.len(), 17 * 33);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
        // stacks * slices * 6 个索引
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
        assert_eq!(geo.positions.len(), 24); // 6 个面 * 4 个顶点
        assert_eq!(geo.indices.len(), 36); // 6 个面 * 2 个三角形 * 3
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
        assert!(arc.len() > 2); // 应包含中间点
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
        assert_eq!(indices.len(), 6); // 一个四边形对应 2 个三角形
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
        assert_eq!(geo.indices.len(), 24); // 12 条边 * 2
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
        assert_eq!(geo.indices.len(), 8); // 4 条边 * 2
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
        // 所有法线都应为单位长度。
        for n in &normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_compute_tangent_and_bitangent() {
        let mut geo = box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::ALL);
        // box_geometry 不计算切线，因此由我们来计算。
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
        assert_eq!(geo.indices.len(), tri_count * 6); // 每个三角形 -> 3 条边 -> 6 个索引
    }
}
