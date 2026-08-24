//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: projectTo2D).

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::developer_error::throw_developer_error;
use crate::geographic_projection::GeographicProjection;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;

/// Projects a geometry's 3D `position` attribute to 2D, replacing the
/// `position` attribute with separate `position3D` and `position2D` attributes.
///
/// If the geometry does not have a `position`, this function has no effect.
///
/// Port of `GeometryPipeline.projectTo2D(geometry, attributeName,
/// attributeName3D, attributeName2D, projection)`.
///
/// # Panics (debug)
/// - If the attribute matching `attribute_name` does not exist.
/// - If the attribute's component datatype is not `DOUBLE`.
/// - If a point cannot be projected to 2D.
pub fn project_to_2d(
    geometry: &mut Geometry,
    attribute_name: &str,
    attribute_name_3d: &str,
    attribute_name_2d: &str,
    projection: Option<&GeographicProjection>,
) {
    if cfg!(debug_assertions) {
        if !geometry.attributes.contains_key(attribute_name) {
            throw_developer_error(&format!(
                "geometry must have attribute matching the attributeName argument: {attribute_name}."
            ));
        }
        if geometry.attributes[attribute_name].component_datatype != ComponentDatatype::Double {
            throw_developer_error(
                "The attribute componentDatatype must be ComponentDatatype.DOUBLE.",
            );
        }
    }

    let attribute = geometry
        .attributes
        .remove(attribute_name)
        .expect("attribute checked above");
    let default_projection = GeographicProjection::new(None);
    let projection = projection.unwrap_or(&default_projection);
    let ellipsoid = projection.ellipsoid();

    // Project original values to 2D.
    let values3d = &attribute.values;
    let mut projected_values = vec![0.0f64; values3d.len()];
    let mut index = 0usize;

    let mut scratch = Cartesian3::ZERO;
    let mut scratch_cartographic = Cartographic::default();
    let mut i = 0usize;
    while i < values3d.len() {
        Cartesian3::from_array(values3d, Some(i), &mut scratch);

        let ok = ellipsoid.cartesian_to_cartographic(&scratch, &mut scratch_cartographic);
        if cfg!(debug_assertions) && !ok {
            throw_developer_error(&format!(
                "Could not project point ({}, {}, {}) to 2D.",
                scratch.x, scratch.y, scratch.z
            ));
        }

        let mut projected = Cartesian3::ZERO;
        projection.project_into(&scratch_cartographic, &mut projected);

        projected_values[index] = projected.x;
        projected_values[index + 1] = projected.y;
        projected_values[index + 2] = projected.z;
        index += 3;
        i += 3;
    }

    // Rename original cartesians to ellipsoid cartesians.
    geometry
        .attributes
        .insert(attribute_name_3d.to_string(), attribute);

    // Replace original cartesians with 2D projected cartesians.
    geometry.attributes.insert(
        attribute_name_2d.to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, projected_values),
    );
}
