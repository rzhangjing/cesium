//! `fitToUnsignedShortIndices` – splits geometry if indices exceed u16 range.

use crate::geometry::Geometry;

/// Splits a geometry into multiple geometries so indices fit in unsigned shorts.
///
/// TODO: full implementation.
pub fn fit_to_unsigned_short_indices(geometry: &Geometry) -> Vec<Geometry> {
    // TODO: implement
    vec![geometry.clone()]
}
