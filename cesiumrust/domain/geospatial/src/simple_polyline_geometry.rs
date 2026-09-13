//! SimplePolylineGeometry - a simple polyline geometry generator.
//!
//! Maps to CesiumJS `Core/SimplePolylineGeometry.js`

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::chord_length;
use crate::polygon_geometry_library::ArcType;
use crate::polyline_pipeline::{generate_arc, number_of_points, ArcOptions};
use glam::DVec3;

/// Extracts heights from cartesian positions using the ellipsoid.
///
/// Maps to `PolylinePipeline.extractHeights`.
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

/// Color as RGBA bytes.
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

    /// Converts a float color component [0,1] to byte [0,255].
    /// Maps to `Color.floatToByte`.
    pub fn float_to_byte(value: f64) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Returns RGBA as byte array.
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            Self::float_to_byte(self.red),
            Self::float_to_byte(self.green),
            Self::float_to_byte(self.blue),
            Self::float_to_byte(self.alpha),
        ]
    }
}

/// Result of SimplePolylineGeometry::create_geometry.
#[derive(Debug, Clone)]
pub struct SimplePolylineResult {
    /// Vertex positions (flat: x,y,z,x,y,z,...).
    pub position_values: Vec<f64>,
    /// Per-vertex RGBA color bytes (optional).
    pub color_values: Option<Vec<u8>>,
    /// Line indices (pairs).
    pub indices: Vec<u32>,
    /// Always PrimitiveType::Lines.
    pub is_lines: bool,
    /// Bounding sphere from original positions.
    pub bounding_sphere: BoundingSphere,
}

/// SimplePolylineGeometry description.
///
/// Maps to CesiumJS `Core/SimplePolylineGeometry`.
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
    /// Creates a new SimplePolylineGeometry.
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

    /// Computes the geometric representation of a simple polyline.
    ///
    /// Maps to `SimplePolylineGeometry.createGeometry`.
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
                // Per-segment colors: generate arc per segment
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
                // Per-vertex colors or no colors: generate full arc
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
                    // Interpolate per-vertex colors along the arc
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

                    // Last color
                    let last_color = colors_arr[length - 1];
                    col_vals.extend_from_slice(&last_color.to_bytes());

                    color_values = Some(col_vals);
                }
            }
        } else {
            // ArcType::None - no subdivision
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

        // Generate line indices
        let number_of_positions = position_values.len() / 3;
        let number_of_indices = (number_of_positions - 1) * 2;
        let mut indices: Vec<u32> = Vec::with_capacity(number_of_indices);
        for i in 0..(number_of_positions - 1) as u32 {
            indices.push(i);
            indices.push(i + 1);
        }

        // Bounding sphere from original positions
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
