//! Ported from `packages/engine/Source/Scene/Model/ModelImageryMapping.js`.
//!
//! Static utility functions for computing imagery texture coordinates
//! from mapped positions.

use cesium_core::cartesian2::Cartesian2;

use super::cartesian_rectangle::CartesianRectangle;
use super::mapped_positions::MappedPositions;

/// Computes texture coordinates for mapped positions relative to a bounding rectangle.
///
/// Maps each position to a [0,1] UV coordinate within the bounding rectangle,
/// clamping to the rectangle bounds.
/// Mirrors CesiumJS `ModelImageryMapping.computeTexCoords`.
pub fn compute_tex_coords(
    positions: &[(f64, f64)],
    bounding_rect: &CartesianRectangle,
) -> Vec<Cartesian2> {
    let width = bounding_rect.max_x - bounding_rect.min_x;
    let height = bounding_rect.max_y - bounding_rect.min_y;

    if width.abs() < f64::EPSILON || height.abs() < f64::EPSILON {
        return positions.iter().map(|_| Cartesian2::new(0.0, 0.0)).collect();
    }

    positions
        .iter()
        .map(|&(x, y)| {
            let u = ((x - bounding_rect.min_x) / width).clamp(0.0, 1.0);
            let v = ((y - bounding_rect.min_y) / height).clamp(0.0, 1.0);
            Cartesian2::new(u, v)
        })
        .collect()
}

/// Computes the bounding rectangle of a set of positions.
pub fn compute_bounding_rectangle(positions: &[(f64, f64)]) -> CartesianRectangle {
    if positions.is_empty() {
        return CartesianRectangle::default();
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for &(x, y) in positions {
        if x < min_x { min_x = x; }
        if y < min_y { min_y = y; }
        if x > max_x { max_x = x; }
        if y > max_y { max_y = y; }
    }

    CartesianRectangle::new(min_x, min_y, max_x, max_y)
}

/// Creates texture coordinates from [`MappedPositions`] and a projection rectangle.
///
/// Extracts the 2D positions from the mapped positions, computes the bounding
/// rectangle, and maps to UV coordinates.
pub fn create_texture_coordinates(
    mapped_positions: &MappedPositions,
) -> Vec<Cartesian2> {
    // Extract positions from the JSON value.
    let positions = extract_positions(&mapped_positions.cartographic_positions);
    if positions.is_empty() {
        return Vec::new();
    }

    let bounding_rect = compute_bounding_rectangle(&positions);
    compute_tex_coords(&positions, &bounding_rect)
}

/// Extracts (x, y) positions from a JSON value.
fn extract_positions(value: &serde_json::Value) -> Vec<(f64, f64)> {
    let mut result = Vec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(pair) = item.as_array() {
                if pair.len() >= 2 {
                    let x = pair[0].as_f64().unwrap_or(0.0);
                    let y = pair[1].as_f64().unwrap_or(0.0);
                    result.push((x, y));
                }
            }
        }
    }
    result
}
