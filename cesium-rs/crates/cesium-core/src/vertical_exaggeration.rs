//! Ported from `packages/engine/Source/Core/VerticalExaggeration.js`.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;

/// Utilities for vertical exaggeration of terrain.
pub struct VerticalExaggeration;

impl VerticalExaggeration {
    /// Scales a height relative to an offset.
    pub fn get_height(height: f64, scale: f64, relative_height: f64) -> f64 {
        (height - relative_height) * scale + relative_height
    }

    /// Scales a position by exaggeration.
    pub fn get_position(
        position: &Cartesian3,
        ellipsoid: &Ellipsoid,
        vertical_exaggeration: f64,
        vertical_exaggeration_relative_height: f64,
        result: &mut Cartesian3,
    ) {
        let mut cartographic = Cartographic::default();
        if ellipsoid.cartesian_to_cartographic(position, &mut cartographic) {
            let new_height = Self::get_height(
                cartographic.height,
                vertical_exaggeration,
                vertical_exaggeration_relative_height,
            );
            let radii_squared = ellipsoid.radii_squared();
            Cartesian3::from_radians(
                cartographic.longitude,
                cartographic.latitude,
                Some(new_height),
                Some(radii_squared),
                result,
            );
        } else {
            Cartesian3::clone_into(position, result);
        }
    }
}
