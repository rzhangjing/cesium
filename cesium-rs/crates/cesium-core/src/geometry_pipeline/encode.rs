//! `encodeAttribute` – encodes a DOUBLE attribute into high/low float pairs.

use crate::geometry::Geometry;

/// Encodes a DOUBLE attribute into high/low precision float pairs for GPU.
///
/// TODO: full implementation — requires EncodedCartesian3 port.
pub fn encode_attribute(
    geometry: &mut Geometry,
    attribute_name: &str,
    attribute_name_high: &str,
    attribute_name_low: &str,
) {
    let _ = (geometry, attribute_name, attribute_name_high, attribute_name_low);
}
