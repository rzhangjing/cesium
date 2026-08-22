//! Ported from `packages/engine/Source/Core/Intersections2D.js`.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;

/// Contains functions for operating on 2D triangles.
pub struct Intersections2D;

impl Intersections2D {
    /// Splits a 2D triangle at given axis-aligned threshold value and returns the resulting polygon.
    ///
    /// The resulting polygon is specified as a list of vertex indices and interpolation values.
    /// Each entry is either an index (0, 1, 2) referencing the original triangle vertices,
    /// or -1 followed by three values (i, j, ratio) indicating a new vertex on the edge from
    /// vertex i to vertex j at the given interpolation ratio.
    pub fn clip_triangle_at_axis_aligned_threshold(
        threshold: f64,
        keep_above: bool,
        u0: f64,
        u1: f64,
        u2: f64,
    ) -> Vec<f64> {
        let mut result = Vec::new();

        let u0_behind = if keep_above { u0 < threshold } else { u0 > threshold };
        let u1_behind = if keep_above { u1 < threshold } else { u1 > threshold };
        let u2_behind = if keep_above { u2 < threshold } else { u2 > threshold };

        let num_behind = (u0_behind as usize) + (u1_behind as usize) + (u2_behind as usize);

        if num_behind == 1 {
            if u0_behind {
                let u01_ratio = (threshold - u0) / (u1 - u0);
                let u02_ratio = (threshold - u0) / (u2 - u0);
                result.push(1.0);
                result.push(2.0);
                if u02_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(0.0);
                    result.push(2.0);
                    result.push(u02_ratio);
                }
                if u01_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(0.0);
                    result.push(1.0);
                    result.push(u01_ratio);
                }
            } else if u1_behind {
                let u12_ratio = (threshold - u1) / (u2 - u1);
                let u10_ratio = (threshold - u1) / (u0 - u1);
                result.push(2.0);
                result.push(0.0);
                if u10_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(1.0);
                    result.push(0.0);
                    result.push(u10_ratio);
                }
                if u12_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(1.0);
                    result.push(2.0);
                    result.push(u12_ratio);
                }
            } else {
                let u20_ratio = (threshold - u2) / (u0 - u2);
                let u21_ratio = (threshold - u2) / (u1 - u2);
                result.push(0.0);
                result.push(1.0);
                if u21_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(2.0);
                    result.push(1.0);
                    result.push(u21_ratio);
                }
                if u20_ratio != 1.0 {
                    result.push(-1.0);
                    result.push(2.0);
                    result.push(0.0);
                    result.push(u20_ratio);
                }
            }
        } else if num_behind == 2 {
            if !u0_behind && u0 != threshold {
                let u10_ratio = (threshold - u1) / (u0 - u1);
                let u20_ratio = (threshold - u2) / (u0 - u2);
                result.push(0.0);
                result.push(-1.0);
                result.push(1.0);
                result.push(0.0);
                result.push(u10_ratio);
                result.push(-1.0);
                result.push(2.0);
                result.push(0.0);
                result.push(u20_ratio);
            } else if !u1_behind && u1 != threshold {
                let u21_ratio = (threshold - u2) / (u1 - u2);
                let u01_ratio = (threshold - u0) / (u1 - u0);
                result.push(1.0);
                result.push(-1.0);
                result.push(2.0);
                result.push(1.0);
                result.push(u21_ratio);
                result.push(-1.0);
                result.push(0.0);
                result.push(1.0);
                result.push(u01_ratio);
            } else if !u2_behind && u2 != threshold {
                let u02_ratio = (threshold - u0) / (u2 - u0);
                let u12_ratio = (threshold - u1) / (u2 - u1);
                result.push(2.0);
                result.push(-1.0);
                result.push(0.0);
                result.push(2.0);
                result.push(u02_ratio);
                result.push(-1.0);
                result.push(1.0);
                result.push(2.0);
                result.push(u12_ratio);
            }
        } else if num_behind != 3 {
            // Completely in front of threshold
            result.push(0.0);
            result.push(1.0);
            result.push(2.0);
        }
        // else: completely behind threshold, empty result

        result
    }

    /// Computes the barycentric coordinates of a 2D position within a 2D triangle.
    pub fn compute_barycentric_coordinates(
        x: f64,
        y: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    ) -> Cartesian3 {
        let x1mx3 = x1 - x3;
        let x3mx2 = x3 - x2;
        let y2my3 = y2 - y3;
        let y1my3 = y1 - y3;
        let inverse_determinant = 1.0 / (y2my3 * x1mx3 + x3mx2 * y1my3);
        let ymy3 = y - y3;
        let xmx3 = x - x3;
        let l1 = (y2my3 * xmx3 + x3mx2 * ymy3) * inverse_determinant;
        let l2 = (-y1my3 * xmx3 + x1mx3 * ymy3) * inverse_determinant;
        let l3 = 1.0 - l1 - l2;
        Cartesian3::new(l1, l2, l3)
    }

    /// Computes the intersection point of two line segments.
    pub fn compute_line_segment_line_segment_intersection(
        x00: f64,
        y00: f64,
        x01: f64,
        y01: f64,
        x10: f64,
        y10: f64,
        x11: f64,
        y11: f64,
    ) -> Option<Cartesian2> {
        let numerator1_a = (x11 - x10) * (y00 - y10) - (y11 - y10) * (x00 - x10);
        let numerator1_b = (x01 - x00) * (y00 - y10) - (y01 - y00) * (x00 - x10);
        let denominator1 = (y11 - y10) * (x01 - x00) - (x11 - x10) * (y01 - y00);

        if denominator1 == 0.0 {
            return None;
        }

        let ua = numerator1_a / denominator1;
        let ub = numerator1_b / denominator1;

        if ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0 {
            Some(Cartesian2::new(
                x00 + ua * (x01 - x00),
                y00 + ua * (y01 - y00),
            ))
        } else {
            None
        }
    }
}
