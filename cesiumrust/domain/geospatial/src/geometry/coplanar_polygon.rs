//! 共面多边形几何 —— 由任意共面位置构成的多边形。
//!
//! 对 CesiumJS `CoplanarPolygonGeometry.js` 与
//! `CoplanarPolygonGeometryLibrary.js` 的忠实移植。将共面的 3D 位置投影到
//! 其最佳拟合平面上，在 2D 中三角剖分，并生成网格。

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{triangulate_polygon, GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use glam::DVec3;

/// 描述一个共面多边形的选项。
#[derive(Debug, Clone)]
pub struct CoplanarPolygonOptions {
    /// 多边形的位置（至少 3 个，必须共面）。
    pub positions: Vec<DVec3>,
    /// 纹理坐标旋转（弧度）。
    pub st_rotation: f64,
    /// 参考椭球。
    pub ellipsoid: Ellipsoid,
}

impl Default for CoplanarPolygonOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            st_rotation: 0.0,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// 使用 Newell 法计算平面法线（对任意多边形都稳健）。
fn compute_normal(positions: &[DVec3]) -> DVec3 {
    let n = positions.len();
    let mut normal = DVec3::ZERO;
    for i in 0..n {
        let current = positions[i];
        let next = positions[(i + 1) % n];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.normalize_or(DVec3::Z)
}

/// 计算位置的形心。
fn compute_center(positions: &[DVec3]) -> DVec3 {
    let sum: DVec3 = positions.iter().sum();
    sum / positions.len() as f64
}

/// 生成一个共面多边形几何。
///
/// 映射到 CesiumJS `CoplanarPolygonGeometry.createGeometry`。
pub fn coplanar_polygon_geometry(options: &CoplanarPolygonOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;

    // 去除重复项。
    let mut positions: Vec<DVec3> = options.positions.clone();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 3 {
        return empty_geometry();
    }

    // 计算平面法线和坐标轴。
    let normal = compute_normal(&positions);

    // 确保法线朝外（远离椭球中心）。
    let center = compute_center(&positions);
    if center.length_squared() > 1e-12 {
        let surface_normal = ellipsoid.geodetic_surface_normal(center).unwrap_or(DVec3::Z);
        if normal.dot(surface_normal) < 0.0 {
            // 翻转法线和 axis1 以保持一致的绕序。
            let normal = -normal;
            return build_geometry(&positions, normal, options.st_rotation, &vf);
        }
    }

    build_geometry(&positions, normal, options.st_rotation, &vf)
}

fn build_geometry(
    positions: &[DVec3],
    normal: DVec3,
    st_rotation: f64,
    vf: &VertexFormat,
) -> GeometryData {
    let n = positions.len();

    // 计算平面坐标轴。
    let axis1 = compute_axis1(normal);
    let axis2 = normal.cross(axis1).normalize_or(DVec3::Y);

    // 将位置投影到 2D。
    let center = compute_center(positions);
    let positions_2d: Vec<glam::DVec2> = positions
        .iter()
        .map(|&p| {
            let v = p - center;
            glam::DVec2::new(v.dot(axis1), v.dot(axis2))
        })
        .collect();

    // 三角剖分。
    let indices = triangulate_polygon(&positions_2d, &[]);
    if indices.is_empty() {
        return empty_geometry();
    }

    // 计算用于 ST 的包围矩形。
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for p in &positions_2d {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let width = (max_x - min_x).max(1e-10);
    let height = (max_y - min_y).max(1e-10);

    // 若需要则应用 ST 旋转。
    let (cos_r, sin_r) = if st_rotation.abs() > 1e-15 {
        (st_rotation.cos(), st_rotation.sin())
    } else {
        (1.0, 0.0)
    };

    // 生成顶点属性。
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n)) } else { None };

    for (i, &p) in positions.iter().enumerate() {
        pos_out.push([p.x, p.y, p.z]);

        if let Some(ref mut norms) = normals_out {
            norms.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut tans) = tangents_out {
            tans.push([axis1.x, axis1.y, axis1.z]);
        }
        if let Some(ref mut bits) = bitangents_out {
            bits.push([axis2.x, axis2.y, axis2.z]);
        }
        if let Some(ref mut st) = st_out {
            let p2d = positions_2d[i];
            // 绕中心应用旋转。
            let rx = p2d.x * cos_r - p2d.y * sin_r;
            let ry = p2d.x * sin_r + p2d.y * cos_r;
            let stx = ((rx - min_x) / width).clamp(0.0, 1.0);
            let sty = ((ry - min_y) / height).clamp(0.0, 1.0);
            st.push([stx, sty]);
        }
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

/// 计算一个垂直于法线的向量（平面的 axis1）。
fn compute_axis1(normal: DVec3) -> DVec3 {
    // 选择与法线对齐程度最低的世界轴进行叉乘。
    let candidate = if normal.x.abs() <= normal.y.abs() && normal.x.abs() <= normal.z.abs() {
        DVec3::X
    } else if normal.y.abs() <= normal.z.abs() {
        DVec3::Y
    } else {
        DVec3::Z
    };
    normal.cross(candidate).normalize_or(DVec3::X)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartographic::Cartographic;

    fn coplanar_opts() -> CoplanarPolygonOptions {
        let ell = Ellipsoid::WGS84;
        // 表面上的一个四边形（在小区域内大致共面）。
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-72.0, 40.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-70.0, 40.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-70.0, 38.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-72.0, 38.0, 0.0)),
        ];
        CoplanarPolygonOptions {
            positions,
            st_rotation: 0.0,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_coplanar_basic() {
        let geo = coplanar_polygon_geometry(&coplanar_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        // 4 个顶点，扇形三角剖分 = 2 个三角形 = 6 个索引。
        assert_eq!(geo.positions.len(), 4);
        assert_eq!(geo.indices.len(), 6);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
    }

    #[test]
    fn test_coplanar_triangle() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 3);
        assert_eq!(geo.indices.len(), 3);
    }

    #[test]
    fn test_coplanar_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_coplanar_normals_consistent() {
        let geo = coplanar_polygon_geometry(&coplanar_opts(), VertexFormat::ALL);
        let normals = geo.normals.unwrap();
        // 所有法线应相同（共面多边形）。
        let n0 = DVec3::new(normals[0][0], normals[0][1], normals[0][2]);
        for n in &normals {
            let ni = DVec3::new(n[0], n[1], n[2]);
            assert!((n0 - ni).length() < 1e-10);
        }
        // 法线应朝外。
        let center = compute_center(
            &geo.positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
        );
        assert!(center.dot(n0) > 0.0);
    }

    #[test]
    fn test_coplanar_pentagon() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.5, 0.5, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-0.5, 0.5, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 5);
        // 扇形三角剖分：5-2 = 3 个三角形 = 9 个索引。
        assert_eq!(geo.indices.len(), 9);
    }
}
