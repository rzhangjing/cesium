//! PolygonPipeline —— 椭球上多边形网格的细分算法。
//! 对 CesiumJS `Source/Core/PolygonPipeline.js`
//! （computeSubdivision + computeRhumbLineSubdivision）的忠实移植

// 遗留的 CesiumJS 移植风格技术债（deferred.md #18）；在 M13 lint-cleanup
// 或本文件在其里程碑被重写时重新审视
#![allow(clippy::manual_is_multiple_of, clippy::len_zero)]
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_rhumb_line::EllipsoidRhumbLine;
use crate::math_utils::chord_length;
use glam::{DVec2, DVec3};
use std::collections::HashMap;
use std::f64::consts::PI;

/// 一次细分操作的结果。
/// 映射到 PolygonPipeline.computeSubdivision 返回的 Geometry 对象
#[derive(Debug, Clone)]
pub struct SubdivisionResult {
    /// 扁平的位置值 [x0, y0, z0, x1, y1, z1, ...]
    pub positions: Vec<f64>,
    /// 三角形索引
    pub indices: Vec<u32>,
    /// 可选的扁平纹理坐标 [u0, v0, u1, v1, ...]
    pub texcoords: Option<Vec<f64>>,
}

const RADIANS_PER_DEGREE: f64 = PI / 180.0;

/// 细分位置并将点抬升到椭球表面。
/// 映射到 `PolygonPipeline.computeSubdivision`
///
/// # 参数
/// * `ellipsoid` - 多边形所在的椭球
/// * `positions` - Cartesian3 位置数组
/// * `indices` - 三角形索引
/// * `texcoords` - 可选的纹理坐标
/// * `granularity` - 相邻细分之间的距离（弧度）（默认：RADIANS_PER_DEGREE）
pub fn compute_subdivision(
    ellipsoid: &Ellipsoid,
    positions: &[DVec3],
    indices: &[u32],
    texcoords: Option<&[DVec2]>,
    granularity: Option<f64>,
) -> SubdivisionResult {
    let granularity = granularity.unwrap_or(RADIANS_PER_DEGREE);
    let has_texcoords = texcoords.is_some();

    debug_assert!(indices.len() >= 3, "At least three indices are required");
    debug_assert!(indices.len() % 3 == 0, "Number of indices must be divisible by three");
    debug_assert!(granularity > 0.0, "Granularity must be greater than zero");

    // 需要（或可能需要）细分的三角形
    let mut triangles: Vec<u32> = indices.to_vec();

    // 由边拆分产生的新位置会被追加到位置列表末尾。
    let mut subdivided_positions: Vec<f64> = Vec::with_capacity(positions.len() * 3);
    let mut subdivided_texcoords: Vec<f64> = if has_texcoords {
        Vec::with_capacity(positions.len() * 2)
    } else {
        Vec::new()
    };

    for item in positions {
        subdivided_positions.push(item.x);
        subdivided_positions.push(item.y);
        subdivided_positions.push(item.z);
    }
    if let Some(tcs) = texcoords {
        for tc in tcs {
            subdivided_texcoords.push(tc.x);
            subdivided_texcoords.push(tc.y);
        }
    }

    let mut subdivided_indices: Vec<u32> = Vec::new();

    // 用于确保共享的边不会被拆分多次。
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();

    let radius = ellipsoid.maximum_radius();
    let min_distance = chord_length(granularity, radius);
    let min_distance_sqrd = min_distance * min_distance;

    while triangles.len() > 0 {
        let i2 = triangles.pop().unwrap();
        let i1 = triangles.pop().unwrap();
        let i0 = triangles.pop().unwrap();

        let v0 = DVec3::new(
            subdivided_positions[(i0 * 3) as usize],
            subdivided_positions[(i0 * 3 + 1) as usize],
            subdivided_positions[(i0 * 3 + 2) as usize],
        );
        let v1 = DVec3::new(
            subdivided_positions[(i1 * 3) as usize],
            subdivided_positions[(i1 * 3 + 1) as usize],
            subdivided_positions[(i1 * 3 + 2) as usize],
        );
        let v2 = DVec3::new(
            subdivided_positions[(i2 * 3) as usize],
            subdivided_positions[(i2 * 3 + 1) as usize],
            subdivided_positions[(i2 * 3 + 2) as usize],
        );

        let s0 = v0.normalize() * radius;
        let s1 = v1.normalize() * radius;
        let s2 = v2.normalize() * radius;

        let g0 = (s0 - s1).length_squared();
        let g1 = (s1 - s2).length_squared();
        let g2 = (s2 - s0).length_squared();

        let max = g0.max(g1).max(g2);

        if max > min_distance_sqrd {
            if g0 == max {
                let edge = (i0.min(i1), i0.max(i1));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = (v0 + v1) * 0.5;
                    subdivided_positions.push(mid.x);
                    subdivided_positions.push(mid.y);
                    subdivided_positions.push(mid.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t0 = DVec2::new(
                            subdivided_texcoords[(i0 * 2) as usize],
                            subdivided_texcoords[(i0 * 2 + 1) as usize],
                        );
                        let t1 = DVec2::new(
                            subdivided_texcoords[(i1 * 2) as usize],
                            subdivided_texcoords[(i1 * 2 + 1) as usize],
                        );
                        let mid_tc = (t0 + t1) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i0);
                triangles.push(mid_idx);
                triangles.push(i2);
                triangles.push(mid_idx);
                triangles.push(i1);
                triangles.push(i2);
            } else if g1 == max {
                let edge = (i1.min(i2), i1.max(i2));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = (v1 + v2) * 0.5;
                    subdivided_positions.push(mid.x);
                    subdivided_positions.push(mid.y);
                    subdivided_positions.push(mid.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t1 = DVec2::new(
                            subdivided_texcoords[(i1 * 2) as usize],
                            subdivided_texcoords[(i1 * 2 + 1) as usize],
                        );
                        let t2 = DVec2::new(
                            subdivided_texcoords[(i2 * 2) as usize],
                            subdivided_texcoords[(i2 * 2 + 1) as usize],
                        );
                        let mid_tc = (t1 + t2) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i1);
                triangles.push(mid_idx);
                triangles.push(i0);
                triangles.push(mid_idx);
                triangles.push(i2);
                triangles.push(i0);
            } else {
                // g2 == max
                let edge = (i2.min(i0), i2.max(i0));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = (v2 + v0) * 0.5;
                    subdivided_positions.push(mid.x);
                    subdivided_positions.push(mid.y);
                    subdivided_positions.push(mid.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t2 = DVec2::new(
                            subdivided_texcoords[(i2 * 2) as usize],
                            subdivided_texcoords[(i2 * 2 + 1) as usize],
                        );
                        let t0 = DVec2::new(
                            subdivided_texcoords[(i0 * 2) as usize],
                            subdivided_texcoords[(i0 * 2 + 1) as usize],
                        );
                        let mid_tc = (t2 + t0) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i2);
                triangles.push(mid_idx);
                triangles.push(i1);
                triangles.push(mid_idx);
                triangles.push(i0);
                triangles.push(i1);
            }
        } else {
            subdivided_indices.push(i0);
            subdivided_indices.push(i1);
            subdivided_indices.push(i2);
        }
    }

    SubdivisionResult {
        positions: subdivided_positions,
        indices: subdivided_indices,
        texcoords: if has_texcoords {
            Some(subdivided_texcoords)
        } else {
            None
        },
    }
}

/// 在恒向线上细分位置并将点抬升到椭球表面。
/// 映射到 `PolygonPipeline.computeRhumbLineSubdivision`
pub fn compute_rhumb_line_subdivision(
    ellipsoid: &Ellipsoid,
    positions: &[DVec3],
    indices: &[u32],
    texcoords: Option<&[DVec2]>,
    granularity: Option<f64>,
) -> SubdivisionResult {
    let granularity = granularity.unwrap_or(RADIANS_PER_DEGREE);
    let has_texcoords = texcoords.is_some();

    debug_assert!(indices.len() >= 3, "At least three indices are required");
    debug_assert!(indices.len() % 3 == 0, "Number of indices must be divisible by three");
    debug_assert!(granularity > 0.0, "Granularity must be greater than zero");

    let mut triangles: Vec<u32> = indices.to_vec();

    let mut subdivided_positions: Vec<f64> = Vec::with_capacity(positions.len() * 3);
    let mut subdivided_texcoords: Vec<f64> = if has_texcoords {
        Vec::with_capacity(positions.len() * 2)
    } else {
        Vec::new()
    };

    for item in positions {
        subdivided_positions.push(item.x);
        subdivided_positions.push(item.y);
        subdivided_positions.push(item.z);
    }
    if let Some(tcs) = texcoords {
        for tc in tcs {
            subdivided_texcoords.push(tc.x);
            subdivided_texcoords.push(tc.y);
        }
    }

    let mut subdivided_indices: Vec<u32> = Vec::new();
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();

    let radius = ellipsoid.maximum_radius();
    let min_distance = chord_length(granularity, radius);

    // 用于计算的占位恒向线（会逐边重置）
    let dummy_start = Cartographic::from_radians(0.0, 0.0, 0.0);
    let dummy_end = Cartographic::from_radians(0.0, 0.1, 0.0);
    let mut rhumb0 = EllipsoidRhumbLine::new(&dummy_start, &dummy_end, ellipsoid);
    let mut rhumb1 = EllipsoidRhumbLine::new(&dummy_start, &dummy_end, ellipsoid);
    let mut rhumb2 = EllipsoidRhumbLine::new(&dummy_start, &dummy_end, ellipsoid);

    while triangles.len() > 0 {
        let i2 = triangles.pop().unwrap();
        let i1 = triangles.pop().unwrap();
        let i0 = triangles.pop().unwrap();

        let v0 = DVec3::new(
            subdivided_positions[(i0 * 3) as usize],
            subdivided_positions[(i0 * 3 + 1) as usize],
            subdivided_positions[(i0 * 3 + 2) as usize],
        );
        let v1 = DVec3::new(
            subdivided_positions[(i1 * 3) as usize],
            subdivided_positions[(i1 * 3 + 1) as usize],
            subdivided_positions[(i1 * 3 + 2) as usize],
        );
        let v2 = DVec3::new(
            subdivided_positions[(i2 * 3) as usize],
            subdivided_positions[(i2 * 3 + 1) as usize],
            subdivided_positions[(i2 * 3 + 2) as usize],
        );

        let c0 = ellipsoid.cartesian_to_cartographic(v0).unwrap();
        let c1 = ellipsoid.cartesian_to_cartographic(v1).unwrap();
        let c2 = ellipsoid.cartesian_to_cartographic(v2).unwrap();

        rhumb0.set_end_points(&c0, &c1);
        let g0 = rhumb0.surface_distance();
        rhumb1.set_end_points(&c1, &c2);
        let g1 = rhumb1.surface_distance();
        rhumb2.set_end_points(&c2, &c0);
        let g2 = rhumb2.surface_distance();

        let max = g0.max(g1).max(g2);

        if max > min_distance {
            if g0 == max {
                let edge = (i0.min(i1), i0.max(i1));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = rhumb0.interpolate_using_fraction(0.5);
                    let mid_height = (c0.height + c1.height) * 0.5;
                    let mid_cartesian = ellipsoid.cartographic_to_cartesian(
                        &Cartographic::from_radians(mid.longitude, mid.latitude, mid_height),
                    );
                    subdivided_positions.push(mid_cartesian.x);
                    subdivided_positions.push(mid_cartesian.y);
                    subdivided_positions.push(mid_cartesian.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t0 = DVec2::new(
                            subdivided_texcoords[(i0 * 2) as usize],
                            subdivided_texcoords[(i0 * 2 + 1) as usize],
                        );
                        let t1 = DVec2::new(
                            subdivided_texcoords[(i1 * 2) as usize],
                            subdivided_texcoords[(i1 * 2 + 1) as usize],
                        );
                        let mid_tc = (t0 + t1) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i0);
                triangles.push(mid_idx);
                triangles.push(i2);
                triangles.push(mid_idx);
                triangles.push(i1);
                triangles.push(i2);
            } else if g1 == max {
                let edge = (i1.min(i2), i1.max(i2));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = rhumb1.interpolate_using_fraction(0.5);
                    let mid_height = (c1.height + c2.height) * 0.5;
                    let mid_cartesian = ellipsoid.cartographic_to_cartesian(
                        &Cartographic::from_radians(mid.longitude, mid.latitude, mid_height),
                    );
                    subdivided_positions.push(mid_cartesian.x);
                    subdivided_positions.push(mid_cartesian.y);
                    subdivided_positions.push(mid_cartesian.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t1 = DVec2::new(
                            subdivided_texcoords[(i1 * 2) as usize],
                            subdivided_texcoords[(i1 * 2 + 1) as usize],
                        );
                        let t2 = DVec2::new(
                            subdivided_texcoords[(i2 * 2) as usize],
                            subdivided_texcoords[(i2 * 2 + 1) as usize],
                        );
                        let mid_tc = (t1 + t2) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i1);
                triangles.push(mid_idx);
                triangles.push(i0);
                triangles.push(mid_idx);
                triangles.push(i2);
                triangles.push(i0);
            } else {
                // g2 == max
                let edge = (i2.min(i0), i2.max(i0));
                let mid_idx = if let Some(&idx) = edges.get(&edge) {
                    idx
                } else {
                    let mid = rhumb2.interpolate_using_fraction(0.5);
                    let mid_height = (c2.height + c0.height) * 0.5;
                    let mid_cartesian = ellipsoid.cartographic_to_cartesian(
                        &Cartographic::from_radians(mid.longitude, mid.latitude, mid_height),
                    );
                    subdivided_positions.push(mid_cartesian.x);
                    subdivided_positions.push(mid_cartesian.y);
                    subdivided_positions.push(mid_cartesian.z);
                    let idx = (subdivided_positions.len() / 3 - 1) as u32;
                    edges.insert(edge, idx);

                    if has_texcoords {
                        let t2 = DVec2::new(
                            subdivided_texcoords[(i2 * 2) as usize],
                            subdivided_texcoords[(i2 * 2 + 1) as usize],
                        );
                        let t0 = DVec2::new(
                            subdivided_texcoords[(i0 * 2) as usize],
                            subdivided_texcoords[(i0 * 2 + 1) as usize],
                        );
                        let mid_tc = (t2 + t0) * 0.5;
                        subdivided_texcoords.push(mid_tc.x);
                        subdivided_texcoords.push(mid_tc.y);
                    }
                    idx
                };

                triangles.push(i2);
                triangles.push(mid_idx);
                triangles.push(i1);
                triangles.push(mid_idx);
                triangles.push(i0);
                triangles.push(i1);
            }
        } else {
            subdivided_indices.push(i0);
            subdivided_indices.push(i1);
            subdivided_indices.push(i2);
        }
    }

    SubdivisionResult {
        positions: subdivided_positions,
        indices: subdivided_indices,
        texcoords: if has_texcoords {
            Some(subdivided_texcoords)
        } else {
            None
        },
    }
}
