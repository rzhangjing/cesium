//! SimplePolylineGeometry —— 一个简单的折线几何生成器。
//!
//! 映射到 CesiumJS `Core/SimplePolylineGeometry.js`

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::chord_length;
use crate::polygon_geometry_library::ArcType;
use crate::polyline_pipeline::{generate_arc, number_of_points, ArcOptions};
use glam::DVec3;

/// 使用椭球从笛卡尔位置中提取高度。
///
/// 映射到 `PolylinePipeline.extractHeights`。
pub fn extract_heights(positions: &[DVec3], ellipsoid: &Ellipsoid) -> Vec<f64> {
    positions
        .iter()
        .map(|p| {
            ellipsoid
                .cartesian_to_cartographic(*p)
                .map(|c| c.height)
                .unwrap_or(0.0)
        })
        .collect()
}

/// 以 RGBA 字节表示的颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl ColorRgba {
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self { red, green, blue, alpha }
    }

    /// 将浮点颜色分量 [0,1] 转换为字节 [0,255]。
    /// 映射到 `Color.floatToByte`。
    pub fn float_to_byte(value: f64) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// 以字节数组返回 RGBA。
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            Self::float_to_byte(self.red),
            Self::float_to_byte(self.green),
            Self::float_to_byte(self.blue),
            Self::float_to_byte(self.alpha),
        ]
    }
}

/// SimplePolylineGeometry::create_geometry 的结果。
#[derive(Debug, Clone)]
pub struct SimplePolylineResult {
    /// 顶点位置（扁平存储：x,y,z,x,y,z,...）。
    pub position_values: Vec<f64>,
    /// 逐顶点的 RGBA 颜色字节（可选）。
    pub color_values: Option<Vec<u8>>,
    /// 线条索引（成对）。
    pub indices: Vec<u32>,
    /// 恒为 PrimitiveType::Lines。
    pub is_lines: bool,
    /// 由原始位置计算出的包围球。
    pub bounding_sphere: BoundingSphere,
}

/// SimplePolylineGeometry 的描述。
///
/// 映射到 CesiumJS `Core/SimplePolylineGeometry`。
#[derive(Debug, Clone)]
pub struct SimplePolylineGeometry {
    pub positions: Vec<DVec3>,
    pub colors: Option<Vec<ColorRgba>>,
    pub colors_per_vertex: bool,
    pub arc_type: ArcType,
    pub granularity: f64,
    pub ellipsoid: Ellipsoid,
}

impl SimplePolylineGeometry {
    /// 创建一个新的 SimplePolylineGeometry。
    pub fn new(
        positions: Vec<DVec3>,
        colors: Option<Vec<ColorRgba>>,
        colors_per_vertex: bool,
        arc_type: ArcType,
        granularity: f64,
        ellipsoid: Ellipsoid,
    ) -> Self {
        Self {
            positions,
            colors,
            colors_per_vertex,
            arc_type,
            granularity,
            ellipsoid,
        }
    }

    /// 计算一条简单折线的几何表示。
    ///
    /// 映射到 `SimplePolylineGeometry.createGeometry`。
    pub fn create_geometry(&self) -> SimplePolylineResult {
        let positions = &self.positions;
        let colors = &self.colors;
        let colors_per_vertex = self.colors_per_vertex;
        let arc_type = self.arc_type;
        let granularity = self.granularity;
        let ellipsoid = &self.ellipsoid;

        let per_segment_colors = colors.is_some() && !colors_per_vertex;
        let length = positions.len();

        let position_values: Vec<f64>;
        let mut color_values: Option<Vec<u8>> = None;

        if arc_type == ArcType::Geodesic || arc_type == ArcType::Rhumb {
            let heights = extract_heights(positions, ellipsoid);

            if per_segment_colors {
                // 逐段颜色：为每一段生成弧
                let colors_arr = colors.as_ref().unwrap();
                let min_distance = chord_length(granularity, ellipsoid.maximum_radius());

                let mut position_count = 0usize;
                for i in 0..length - 1 {
                    position_count += number_of_points(positions[i], positions[i + 1], min_distance) + 1;
                }

                let mut pos_vals: Vec<f64> = Vec::with_capacity(position_count * 3);
                let mut col_vals: Vec<u8> = Vec::with_capacity(position_count * 4);

                for i in 0..length - 1 {
                    let arc_positions = generate_arc(&ArcOptions {
                        positions: &[positions[i], positions[i + 1]],
                        heights: Some(&[heights[i], heights[i + 1]]),
                        granularity,
                        ellipsoid,
                    });

                    let seg_len = arc_positions.len();
                    let color = colors_arr[i];
                    let bytes = color.to_bytes();
                    for _ in 0..seg_len {
                        col_vals.extend_from_slice(&bytes);
                    }

                    for p in &arc_positions {
                        pos_vals.push(p.x);
                        pos_vals.push(p.y);
                        pos_vals.push(p.z);
                    }
                }

                position_values = pos_vals;
                color_values = Some(col_vals);
            } else {
                // 逐顶点颜色或无颜色：生成完整的弧
                let arc_positions = generate_arc(&ArcOptions {
                    positions,
                    heights: Some(&heights),
                    granularity,
                    ellipsoid,
                });

                let mut pos_vals: Vec<f64> = Vec::with_capacity(arc_positions.len() * 3);
                for p in &arc_positions {
                    pos_vals.push(p.x);
                    pos_vals.push(p.y);
                    pos_vals.push(p.z);
                }
                position_values = pos_vals;

                if let Some(colors_arr) = colors {
                    // 沿弧插值逐顶点颜色
                    let num_positions = arc_positions.len();
                    let mut col_vals: Vec<u8> = Vec::with_capacity(num_positions * 4);

                    let min_distance = chord_length(granularity, ellipsoid.maximum_radius());

                    for i in 0..length - 1 {
                        let p0 = positions[i];
                        let p1 = positions[i + 1];
                        let c0 = colors_arr[i];
                        let c1 = colors_arr[i + 1];

                        let num_pts = number_of_points(p0, p1, min_distance);
                        for j in 0..num_pts {
                            let t = j as f64 / num_pts as f64;
                            let r = c0.red + (c1.red - c0.red) * t;
                            let g = c0.green + (c1.green - c0.green) * t;
                            let b = c0.blue + (c1.blue - c0.blue) * t;
                            let a = c0.alpha + (c1.alpha - c0.alpha) * t;
                            col_vals.push(ColorRgba::float_to_byte(r));
                            col_vals.push(ColorRgba::float_to_byte(g));
                            col_vals.push(ColorRgba::float_to_byte(b));
                            col_vals.push(ColorRgba::float_to_byte(a));
                        }
                    }

                    // 最后一个颜色
                    let last_color = colors_arr[length - 1];
                    col_vals.extend_from_slice(&last_color.to_bytes());

                    color_values = Some(col_vals);
                }
            }
        } else {
            // ArcType::None —— 不进行细分
            let number_of_positions = if per_segment_colors {
                length * 2 - 2
            } else {
                length
            };

            let mut pos_vals: Vec<f64> = Vec::with_capacity(number_of_positions * 3);
            let mut col_vals: Vec<u8> = if colors.is_some() {
                Vec::with_capacity(number_of_positions * 4)
            } else {
                Vec::new()
            };

            let colors_arr = colors.as_deref();

            for i in 0..length {
                let p = positions[i];

                if per_segment_colors && i > 0 {
                    pos_vals.push(p.x);
                    pos_vals.push(p.y);
                    pos_vals.push(p.z);

                    if let Some(cols) = colors_arr {
                        let color = cols[i - 1];
                        col_vals.extend_from_slice(&color.to_bytes());
                    }
                }

                if per_segment_colors && i == length - 1 {
                    break;
                }

                pos_vals.push(p.x);
                pos_vals.push(p.y);
                pos_vals.push(p.z);

                if let Some(cols) = colors_arr {
                    let color = cols[i];
                    col_vals.extend_from_slice(&color.to_bytes());
                }
            }

            position_values = pos_vals;
            if colors.is_some() {
                color_values = Some(col_vals);
            }
        }

        // 生成线条索引
        let number_of_positions = position_values.len() / 3;
        let number_of_indices = (number_of_positions - 1) * 2;
        let mut indices: Vec<u32> = Vec::with_capacity(number_of_indices);
        for i in 0..(number_of_positions - 1) as u32 {
            indices.push(i);
            indices.push(i + 1);
        }

        // 由原始位置计算包围球
        let bounding_sphere = BoundingSphere::from_points(positions);

        SimplePolylineResult {
            position_values,
            color_values,
            indices,
            is_lines: true,
            bounding_sphere,
        }
    }
}
