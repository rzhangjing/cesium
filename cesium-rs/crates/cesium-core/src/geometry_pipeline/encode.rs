//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: encodeAttribute).

use crate::component_datatype::ComponentDatatype;
use crate::developer_error::throw_developer_error;
use crate::encoded_cartesian3::EncodedCartesian3;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;

/// Encodes floating-point geometry attribute values as two separate attributes
/// to improve rendering precision.
///
/// This is commonly used to create high-precision position vertex attributes.
///
/// Port of `GeometryPipeline.encodeAttribute(geometry, attributeName,
/// attributeHighName, attributeLowName)`.
///
/// # Panics (debug)
/// - If the attribute matching `attribute_name` does not exist.
/// - If the attribute's component datatype is not `DOUBLE`.
pub fn encode_attribute(
    geometry: &mut Geometry,
    attribute_name: &str,
    attribute_high_name: &str,
    attribute_low_name: &str,
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
    let values = &attribute.values;
    let length = values.len();
    let mut high_values = vec![0.0f64; length];
    let mut low_values = vec![0.0f64; length];

    for i in 0..length {
        let encoded = EncodedCartesian3::encode(values[i]);
        high_values[i] = encoded.high;
        low_values[i] = encoded.low;
    }

    let components_per_attribute = attribute.components_per_attribute;

    geometry.attributes.insert(
        attribute_high_name.to_string(),
        GeometryAttribute::new(
            ComponentDatatype::Float,
            components_per_attribute,
            false,
            high_values,
        ),
    );
    geometry.attributes.insert(
        attribute_low_name.to_string(),
        GeometryAttribute::new(
            ComponentDatatype::Float,
            components_per_attribute,
            false,
            low_values,
        ),
    );
}
