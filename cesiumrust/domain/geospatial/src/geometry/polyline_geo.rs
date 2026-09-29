//! 折线几何 - 沿大地线弧宽度恒定的带状体。
//!
//! 对 CesiumJS `PolylineGeometry.js` 的忠实适配。CesiumJS 使用
//! GPU 端展开（prevPosition/nextPosition/expandAndWidth 属性）；
//! 这里我们在世界空间中生成一个由 CPU 展开的三角带条状体，
//! 可直接用 Bevy 的标准网格管线渲染。

// 遗留的 CesiumJS 移植风格技术债（deferred.md #18）；在 M13 lint-cleanup
// 或本文件在其里程碑被重写时重新审视
#![allow(unused_variables)]
use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// 描述一条折线的选项。
#[derive(Debug, Clone)]
pub struct PolylineOptions {
    /// 折线的位置（至少 2 个）。
    pub positions: Vec<DVec3>,
    /// 宽度（米）。
    pub width: f64,
    /// 用于弧细分的角度粒度（弧度）。
    pub granularity: f64,
    /// 参考椭球。
    pub ellipsoid: Ellipsoid,
}

impl Default for PolylineOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// 将一条折线生成为扁平带状体（三角带）。
///
/// 带状体位于椭球表面上，以大地线弧为中心，
/// 宽度为指定值。法线从椭球向外。
pub fn polyline_geometry(options: &PolylineOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let width = options.width;

    // 去除重复项。
    let mut positions: Vec<DVec3> = options.positions.clone();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 2 || width <= 0.0 {
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
    if n < 2 {
        return empty_geometry();
    }

    let half_width = width / 2.0;

    // 对每个弧点，计算垂直（左）方向并
    // 偏移以得到左/右边缘顶点。
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n * 2);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n * 2)) } else { None };

    let st_s = if n > 1 { 1.0 / (n - 1) as f64 } else { 1.0 };

    for i in 0..n {
        let p = arc[i];
        let normal = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);

        // 沿弧的切线方向。
        let tangent = if i == 0 {
            (arc[1] - arc[0]).normalize_or(DVec3::X)
        } else if i == n - 1 {
            (arc[n - 1] - arc[n - 2]).normalize_or(DVec3::X)
        } else {
            (arc[i + 1] - arc[i - 1]).normalize_or(DVec3::X)
        };

        // 左方向：cross(normal, tangent) 给出切平面内的垂直方向。
        let left = normal.cross(tangent).normalize_or(DVec3::Y);

        let right_pt = p - left * half_width;
        let left_pt = p + left * half_width;

        // 先推入右侧顶点，再推入左侧顶点。
        pos_out.push([right_pt.x, right_pt.y, right_pt.z]);
        pos_out.push([left_pt.x, left_pt.y, left_pt.z]);

        if let Some(ref mut norms) = normals_out {
            norms.push([normal.x, normal.y, normal.z]);
            norms.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut tans) = tangents_out {
            tans.push([tangent.x, tangent.y, tangent.z]);
            tans.push([tangent.x, tangent.y, tangent.z]);
        }
        if let Some(ref mut bits) = bitangents_out {
            let bitangent = normal.cross(tangent).normalize_or(DVec3::Y);
            bits.push([bitangent.x, bitangent.y, bitangent.z]);
            bits.push([bitangent.x, bitangent.y, bitangent.z]);
        }
        if let Some(ref mut st) = st_out {
            let s = i as f64 * st_s;
            st.push([s, 0.0]); // 右
            st.push([s, 1.0]); // 左
        }
    }

    // 三角剖分：相邻顶点对之间的每个四边形。
    let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * 6);
    for i in 0..n - 1 {
        let r0 = (i * 2) as u32;
        let l0 = (i * 2 + 1) as u32;
        let r1 = ((i + 1) * 2) as u32;
        let l1 = ((i + 1) * 2 + 1) as u32;

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

    fn polyline_opts() -> PolylineOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(5.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(5.0, 5.0, 0.0)),
        ];
        PolylineOptions {
            positions,
            width: 10_000.0,
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_polyline_basic() {
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        // 每个弧点生成 2 个顶点。
        assert_eq!(geo.positions.len() % 2, 0);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
        assert!(geo.tangents.is_some());
        assert!(geo.bitangents.is_some());
    }

    #[test]
    fn test_polyline_vertex_count() {
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::POSITION_ONLY);
        let n_verts = geo.positions.len();
        // n_verts = 2 * arc_points，indices = 6 * (arc_points - 1)
        let arc_points = n_verts / 2;
        assert_eq!(geo.indices.len(), (arc_points - 1) * 6);
    }

    #[test]
    fn test_polyline_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let opts = PolylineOptions {
            positions: vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))],
            width: 100.0,
            ..Default::default()
        };
        let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyline_zero_width() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = PolylineOptions {
            positions,
            width: 0.0,
            ..Default::default()
        };
        let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyline_normals_outward() {
        let ell = Ellipsoid::WGS84;
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::ALL);
        let normals = geo.normals.unwrap();
        // 所有法线应大致指向外侧（与位置的点积为正）。
        for (i, p) in geo.positions.iter().enumerate() {
            let pos = DVec3::new(p[0], p[1], p[2]);
            let n = DVec3::new(normals[i][0], normals[i][1], normals[i][2]);
            assert!(pos.dot(n) > 0.0, "normal not outward at {}", i);
        }
    }
}
