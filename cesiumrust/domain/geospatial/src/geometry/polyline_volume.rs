//! 折线体几何 - 沿折线路径拉伸一个 2D 形状。
//!
//! 对 CesiumJS `PolylineVolumeGeometry.js` 与
//! `PolylineVolumeGeometryLibrary.js` 的忠实适配。沿大地线弧拉伸
//! 一个 2D 截面形状，生成一个管状的体。

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// 描述一个折线体的选项。
#[derive(Debug, Clone)]
pub struct PolylineVolumeOptions {
    /// 折线的位置（至少 2 个）。
    pub positions: Vec<DVec3>,
    /// 2D 截面形状（在局部标架中：x=右，y=上）。
    pub shape: Vec<[f64; 2]>,
    /// 用于弧细分的角度粒度（弧度）。
    pub granularity: f64,
    /// 参考椭球。
    pub ellipsoid: Ellipsoid,
}

impl Default for PolylineVolumeOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            shape: Vec::new(),
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// 生成一个折线体几何。
///
/// 映射到 CesiumJS `PolylineVolumeGeometry.createGeometry`。
pub fn polyline_volume_geometry(options: &PolylineVolumeOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;

    // 去除重复项。
    let mut positions: Vec<DVec3> = options.positions.clone();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 2 || options.shape.len() < 3 {
        return empty_geometry();
    }

    // 细分为大地线弧。
    let opts = ArcOptions {
        positions: &positions,
        heights: None,
        granularity: options.granularity,
        ellipsoid,
    };
    let arc = generate_arc(&opts);

    let n = arc.len();
    let shape_len = options.shape.len();
    if n < 2 || shape_len < 3 {
        return empty_geometry();
    }

    // 对每个弧点，计算一个局部标架并变换形状。
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n * shape_len);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n * shape_len)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n * shape_len)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n * shape_len)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n * shape_len)) } else { None };

    let st_s = if n > 1 { 1.0 / (n - 1) as f64 } else { 1.0 };

    for i in 0..n {
        let p = arc[i];

        // 计算沿弧的切线。
        let tangent = if i == 0 {
            (arc[1] - arc[0]).normalize_or(DVec3::X)
        } else if i == n - 1 {
            (arc[n - 1] - arc[n - 2]).normalize_or(DVec3::X)
        } else {
            (arc[i + 1] - arc[i - 1]).normalize_or(DVec3::X)
        };

        // 计算局部标架：tangent（沿路径）、up（表面法线）、right。
        let up = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);
        let right = tangent.cross(up).normalize_or(DVec3::Y);
        let corrected_up = right.cross(tangent).normalize_or(up);

        // 将每个形状点变换到 3D。
        for (j, shape_pt) in options.shape.iter().enumerate() {
            let offset = right * shape_pt[0] + corrected_up * shape_pt[1];
            let world_pt = p + offset;
            pos_out.push([world_pt.x, world_pt.y, world_pt.z]);

            if let Some(ref mut norms) = normals_out {
                // 法线从形状中心向外。
                let normal = offset.normalize_or(corrected_up);
                norms.push([normal.x, normal.y, normal.z]);
            }
            if let Some(ref mut tans) = tangents_out {
                tans.push([tangent.x, tangent.y, tangent.z]);
            }
            if let Some(ref mut bits) = bitangents_out {
                let normal = offset.normalize_or(corrected_up);
                let bitangent = tangent.cross(normal).normalize_or(right);
                bits.push([bitangent.x, bitangent.y, bitangent.z]);
            }
            if let Some(ref mut st) = st_out {
                let s = i as f64 * st_s;
                let t = j as f64 / (shape_len - 1) as f64;
                st.push([s, t]);
            }
        }
    }

    // 三角剖分：连接相邻的截面。
    let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * shape_len * 6);
    for i in 0..n - 1 {
        for j in 0..shape_len {
            let j_next = (j + 1) % shape_len;
            let curr = (i * shape_len + j) as u32;
            let curr_next = (i * shape_len + j_next) as u32;
            let next = ((i + 1) * shape_len + j) as u32;
            let next_next = ((i + 1) * shape_len + j_next) as u32;

            indices.extend_from_slice(&[curr, curr_next, next]);
            indices.extend_from_slice(&[next, curr_next, next_next]);
        }
    }

    // 为起点和终点加上盖子。
    add_cap(&mut indices, 0, shape_len, false);
    add_cap(&mut indices, (n - 1) * shape_len, shape_len, true);

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

/// 在给定偏移处添加一个扇形三角剖分的盖子。
fn add_cap(indices: &mut Vec<u32>, offset: usize, shape_len: usize, reverse: bool) {
    if shape_len < 3 {
        return;
    }
    let base = offset as u32;
    for i in 1..(shape_len as u32 - 1) {
        if reverse {
            indices.extend_from_slice(&[base, base + i + 1, base + i]);
        } else {
            indices.extend_from_slice(&[base, base + i, base + i + 1]);
        }
    }
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

    fn square_shape() -> Vec<[f64; 2]> {
        vec![
            [-5000.0, -5000.0],
            [5000.0, -5000.0],
            [5000.0, 5000.0],
            [-5000.0, 5000.0],
        ]
    }

    fn polyvol_opts() -> PolylineVolumeOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(2.0, 0.0, 0.0)),
        ];
        PolylineVolumeOptions {
            positions,
            shape: square_shape(),
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_polyvol_basic() {
        let geo = polyline_volume_geometry(&polyvol_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
    }

    #[test]
    fn test_polyvol_vertex_count() {
        let geo = polyline_volume_geometry(&polyvol_opts(), VertexFormat::POSITION_ONLY);
        // n 个弧点 * 4 个形状顶点。
        let n_verts = geo.positions.len();
        assert_eq!(n_verts % 4, 0);
        let n_arc = n_verts / 4;
        assert!(n_arc >= 2);
    }

    #[test]
    fn test_polyvol_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let opts = PolylineVolumeOptions {
            positions: vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))],
            shape: square_shape(),
            ..Default::default()
        };
        let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyvol_too_few_shape() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = PolylineVolumeOptions {
            positions,
            shape: vec![[0.0, 0.0], [1.0, 0.0]], // 只有 2 个点，需要 3 个以上。
            ..Default::default()
        };
        let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyvol_with_corner() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        ];
        let opts = PolylineVolumeOptions {
            positions,
            shape: square_shape(),
            ..Default::default()
        };
        let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.indices.len() % 3, 0);
    }
}
