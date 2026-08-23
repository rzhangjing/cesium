//! Ported from `packages/engine/Source/DataSources/PositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};

/// A property that defines a position in 3D space.
///
/// Position properties return `Cartesian3` values and may vary over time.
pub trait PositionProperty: Property {
    /// Returns the position value at the given time.
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3>;

    /// Returns the reference frame in which this position is defined.
    fn reference_frame(&self) -> PositionReferenceFrame;
}

/// The reference frame for a position property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionReferenceFrame {
    /// The position is defined in the fixed frame (Earth-centered, Earth-fixed).
    Fixed,
    /// The position is defined in the inertial frame.
    Inertial,
}
