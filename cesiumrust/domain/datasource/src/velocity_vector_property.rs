//! VelocityVectorProperty - derives velocity direction from a position property.
//!
//! Maps to CesiumJS `DataSources/VelocityVectorProperty.js`

use crate::property_system::property::DynProperty;
use crate::property_system::value::PropertyValue;
use cesium_time::JulianDate;
use glam::DVec3;
use std::sync::Arc;

/// A property that computes the velocity vector (optionally normalized) from
/// a position property by finite differencing.
///
/// Maps to CesiumJS `DataSources/VelocityVectorProperty.js`
#[derive(Clone)]
pub struct VelocityVectorProperty {
    /// The position property to derive velocity from.
    position: Option<Arc<dyn DynProperty>>,
    /// Whether to normalize the velocity vector.
    normalize: bool,
}

impl VelocityVectorProperty {
    /// Creates a new VelocityVectorProperty with no position.
    pub fn new() -> Self {
        Self {
            position: None,
            normalize: true,
        }
    }

    /// Creates a VelocityVectorProperty with a position property.
    pub fn with_position(position: Arc<dyn DynProperty>, normalize: bool) -> Self {
        Self {
            position: Some(position),
            normalize,
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

    /// Gets whether the velocity is normalized.
    pub fn normalize(&self) -> bool {
        self.normalize
    }

    /// Sets whether to normalize the velocity.
    pub fn set_normalize(&mut self, normalize: bool) {
        self.normalize = normalize;
    }

    /// Gets the velocity vector at the given time.
    ///
    /// Computes the velocity by evaluating the position at time and time+dt,
    /// then computing the difference. If normalize is true, the result is
    /// normalized to unit length.
    ///
    /// Maps to `VelocityVectorProperty.prototype.getValue`
    pub fn get_value(&self, time: &JulianDate) -> Option<DVec3> {
        let position = self.position.as_ref()?;

        // Use a small time delta for finite differencing
        let dt = 1.0 / 60.0; // 1/60th of a second
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

        let velocity = (after - before) / dt;

        if self.normalize {
            let length = velocity.length();
            if length < 1e-15 {
                return None;
            }
            Some(velocity / length)
        } else {
            Some(velocity)
        }
    }

    /// Compares this property to another.
    pub fn equals(&self, other: &VelocityVectorProperty) -> bool {
        self.normalize == other.normalize
            && match (&self.position, &other.position) {
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            }
    }
}

impl Default for VelocityVectorProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for VelocityVectorProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelocityVectorProperty")
            .field("normalize", &self.normalize)
            .field("has_position", &self.position.is_some())
            .finish()
    }
}
