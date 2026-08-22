//! `transformToWorldCoordinates` – transforms geometry to world coordinates.

use crate::geometry::Geometry;
use crate::matrix4::Matrix4;

/// Transforms a geometry instance to world coordinates using its model matrix.
///
/// TODO: full implementation.
pub fn transform_to_world_coordinates(geometry: &mut Geometry, model_matrix: &Matrix4) {
    let _ = (geometry, model_matrix);
}
