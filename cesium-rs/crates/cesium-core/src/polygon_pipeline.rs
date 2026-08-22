//! Ported from `packages/engine/Source/Core/PolygonPipeline.js`.
//!
//! Polygon pipeline utilities. Skeleton implementation.

use crate::cartesian2::Cartesian2;
use crate::winding_order::WindingOrder;

/// Computes the signed area of a 2D polygon.
pub fn compute_area_2d(positions: &[Cartesian2]) -> f64 {
    let length = positions.len();
    if length < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let mut i0 = length - 1;
    for i1 in 0..length {
        let v0 = &positions[i0];
        let v1 = &positions[i1];
        area += v0.x * v1.y - v1.x * v0.y;
        i0 = i1;
    }
    area * 0.5
}

/// Computes the winding order of a 2D polygon.
pub fn compute_winding_order_2d(positions: &[Cartesian2]) -> Option<WindingOrder> {
    let area = compute_area_2d(positions);
    if area > 0.0 {
        Some(WindingOrder::CounterClockwise)
    } else if area < 0.0 {
        Some(WindingOrder::Clockwise)
    } else {
        None
    }
}
