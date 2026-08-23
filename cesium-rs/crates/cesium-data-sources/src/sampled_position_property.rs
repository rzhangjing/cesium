//! Ported from `packages/engine/Source/DataSources/SampledPositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use crate::property::{Property, PropertyResult};
use crate::position_property::{PositionProperty, PositionReferenceFrame};

/// A position property whose value is interpolated from a set of time-position samples.
pub struct SampledPositionProperty {
    times: Vec<f64>,
    values: Vec<Cartesian3>,
    reference_frame: PositionReferenceFrame,
    extrapolation_type: ExtrapolationType,
}

/// The type of extrapolation to use when outside the sample range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrapolationType {
    /// No extrapolation; returns None outside the range.
    None,
    /// Hold the last known value.
    Hold,
    /// Linear extrapolation.
    Linear,
}

impl SampledPositionProperty {
    /// Creates a new sampled position property.
    pub fn new() -> Self {
        Self {
            times: Vec::new(),
            values: Vec::new(),
            reference_frame: PositionReferenceFrame::Fixed,
            extrapolation_type: ExtrapolationType::None,
        }
    }

    /// Adds a sample to the property.
    pub fn add_sample(&mut self, time: f64, position: Cartesian3) {
        self.times.push(time);
        self.values.push(position);
    }

    /// Sets the extrapolation type.
    pub fn set_extrapolation_type(&mut self, extrapolation_type: ExtrapolationType) {
        self.extrapolation_type = extrapolation_type;
    }

    /// Returns the number of samples.
    pub fn num_samples(&self) -> usize { self.times.len() }
}

impl Default for SampledPositionProperty {
    fn default() -> Self { Self::new() }
}

impl Property for SampledPositionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        let mut result = Cartesian3::new(0.0, 0.0, 0.0);
        match self.position_value(time, &mut result) {
            Some(pos) => PropertyResult::Position(pos.x, pos.y, pos.z),
            None => PropertyResult::None,
        }
    }

    fn is_constant(&self) -> bool { self.times.len() <= 1 }
    fn is_destroyed(&self) -> bool { false }
}

impl PositionProperty for SampledPositionProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        if self.times.is_empty() { return None; }
        if self.times.len() == 1 {
            result.x = self.values[0].x;
            result.y = self.values[0].y;
            result.z = self.values[0].z;
            return Some(result);
        }
        // Simple linear interpolation between samples
        if time <= self.times[0] {
            result.x = self.values[0].x;
            result.y = self.values[0].y;
            result.z = self.values[0].z;
            return Some(result);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            result.x = self.values[last].x;
            result.y = self.values[last].y;
            result.z = self.values[last].z;
            return Some(result);
        }
        // Find the interval
        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let t = (time - self.times[i]) / (self.times[i + 1] - self.times[i]);
                result.x = self.values[i].x + t * (self.values[i + 1].x - self.values[i].x);
                result.y = self.values[i].y + t * (self.values[i + 1].y - self.values[i].y);
                result.z = self.values[i].z + t * (self.values[i + 1].z - self.values[i].z);
                return Some(result);
            }
        }
        None
    }

    fn reference_frame(&self) -> PositionReferenceFrame { self.reference_frame }
}
