//! Ported from `packages/engine/Source/DataSources/TimeIntervalCollectionPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property defined by a collection of time intervals, each with its own position property.
pub struct TimeIntervalCollectionPositionProperty {
    intervals: Vec<(f64, f64, Box<dyn PositionProperty>)>,
    reference_frame: PositionReferenceFrame,
}

impl TimeIntervalCollectionPositionProperty {
    /// Creates a new time interval collection position property.
    pub fn new(reference_frame: PositionReferenceFrame) -> Self {
        Self { intervals: Vec::new(), reference_frame }
    }

    /// Adds a position property for the given time interval.
    pub fn add_interval(&mut self, start: f64, stop: f64, property: Box<dyn PositionProperty>) {
        self.intervals.push((start, stop, property));
    }
}

impl Property for TimeIntervalCollectionPositionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        for (start, stop, prop) in &self.intervals {
            if time >= *start && time < *stop {
                return prop.get_value(time);
            }
        }
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { self.intervals.len() <= 1 }
    fn is_destroyed(&self) -> bool { false }

    fn as_position_property(
        &self,
    ) -> Option<&dyn crate::position_property::PositionProperty> {
        Some(self)
    }
}

impl PositionProperty for TimeIntervalCollectionPositionProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        for (start, stop, prop) in &self.intervals {
            if time >= *start && time < *stop {
                return prop.position_value(time, result);
            }
        }
        None
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.reference_frame }
}
