//! `projectTo2D` – projects 3D positions to 2D using a projection.

use crate::geometry::Geometry;

/// Projects a geometry's 3D position attribute to 2D.
///
/// TODO: full implementation — requires GeographicProjection/WebMercatorProjection integration.
pub fn project_to_2d(
    geometry: &mut Geometry,
    attribute_name: &str,
    attribute_name_3d: &str,
    attribute_name_2d: &str,
) {
    let _ = (geometry, attribute_name, attribute_name_3d, attribute_name_2d);
}
