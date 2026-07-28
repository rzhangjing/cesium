//! VelocityOrientationProperty - derives orientation quaternion from position velocity.
//!
//! Maps to CesiumJS `DataSources/VelocityOrientationProperty.js`

use crate::property_system::property::DynProperty;
use crate::property_system::value::PropertyValue;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::transforms::rotation_matrix_from_position_velocity;
use cesium_time::JulianDate;
use glam::DQuat;
use std::sync::Arc;

/// A property that computes an orientation quaternion from the velocity
/// of a position property. The resulting quaternion represents the rotation
/// from the ellipsoid-fixed frame to the velocity-aligned frame.
///
/// Maps to CesiumJS `DataSources/VelocityOrientationProperty.js`
#[derive(Clone)]
pub struct VelocityOrientationProperty {
    /// The position property to derive velocity from.
    position: Option<Arc<dyn DynProperty>>,
    /// The ellipsoid used to compute the rotation.
    ellipsoid: Ellipsoid,
}

impl VelocityOrientationProperty {
    /// Creates a new VelocityOrientationProperty with no position.
    pub fn new() -> Self {
        Self {
            position: None,
            ellipsoid: Ellipsoid::WGS84,
        }
    }

    /// Creates a VelocityOrientationProperty with a position property and ellipsoid.
    pub fn with_position(position: Arc<dyn DynProperty>, ellipsoid: Ellipsoid) -> Self {
        Self {
            position: Some(position),
            ellipsoid,
        }
    }

    /// Gets whether this property is constant.
    pub fn is_constant(&self) -> bool {
        match &self.position {
            None => true,
            Some(p) => p.is_constant(),
        }
    }

    /// Gets the position property.
    pub fn position(&self) -> Option<&Arc<dyn DynProperty>> {
        self.position.as_ref()
    }

    /// Sets the position property.
    pub fn set_position(&mut self, position: Option<Arc<dyn DynProperty>>) {
        self.position = position;
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Sets the ellipsoid.
    pub fn set_ellipsoid(&mut self, ellipsoid: Ellipsoid) {
        self.ellipsoid = ellipsoid;
    }

    /// Gets the orientation quaternion at the given time.
    ///
    /// Computes velocity by finite differencing the position property,
    /// then uses `rotationMatrixFromPositionVelocity` to get the rotation
    /// matrix, which is converted to a quaternion.
    ///
    /// Maps to `VelocityOrientationProperty.prototype.getValue`
    pub fn get_value(&self, time: &JulianDate) -> Option<DQuat> {
        let position = self.position.as_ref()?;

        // Use a small time delta for finite differencing
        let dt = 1.0 / 60.0;
        let time_after = time.add_seconds(dt);

        let pos_before = position.get_value(time);
        let pos_after = position.get_value(&time_after);

        let before = match pos_before {
            PropertyValue::Cartesian3(v) => v,
            _ => return None,
        };
        let after = match pos_after {
            PropertyValue::Cartesian3(v) => v,
            _ => return None,
        };

        let velocity = after - before;
        if velocity.length() < 1e-15 {
            return None;
        }
        let velocity_normalized = velocity.normalize();

        let matrix =
            rotation_matrix_from_position_velocity(before, velocity_normalized, &self.ellipsoid);

        Some(DQuat::from_mat3(&matrix))
    }

    /// Compares this property to another.
    pub fn equals(&self, other: &VelocityOrientationProperty) -> bool {
        self.ellipsoid == other.ellipsoid
            && match (&self.position, &other.position) {
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            }
    }
}

impl Default for VelocityOrientationProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for VelocityOrientationProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelocityOrientationProperty")
            .field("has_position", &self.position.is_some())
            .field("ellipsoid", &self.ellipsoid)
            .finish()
    }
}
