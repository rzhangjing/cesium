//! Position properties: properties whose value is a world location
//! (`Cartesian3`) with an associated reference frame.
//!
//! Maps to CesiumJS `DataSources/PositionProperty.js` and the concrete
//! implementations `ConstantPositionProperty`, `SampledPositionProperty`,
//! `CompositePositionProperty`, `TimeIntervalCollectionPositionProperty` and
//! `CallbackPositionProperty`.

use crate::property_system::interpolation::{ExtrapolationType, InterpolationAlgorithmKind};
use crate::property_system::property::{CompositeProperty, DynProperty, SampledProperty};
use crate::property_system::value::{PackableType, PropertyValue, ReferenceFrame};
use cesium_geospatial::transforms::compute_icrf_to_fixed_matrix;
use cesium_time::{JulianDate, TimeInterval, TimeIntervalCollection, TimeIntervalData};
use glam::DVec3;
use std::any::Any;
use std::sync::Arc;

/// Converts a position from one reference frame to another at the given time.
///
/// Maps to `PositionProperty.convertToReferenceFrame`. When the frames match
/// the value is returned unchanged. Otherwise the ICRF-to-fixed rotation
/// matrix is computed for `time`; inertial→fixed multiplies by the matrix and
/// fixed→inertial multiplies by its transpose.
pub fn convert_to_reference_frame(
    time: &JulianDate,
    value: DVec3,
    input_frame: ReferenceFrame,
    output_frame: ReferenceFrame,
) -> Option<DVec3> {
    if input_frame == output_frame {
        return Some(value);
    }

    let julian_date_seconds = time.total_days() * 86400.0;
    let icrf_to_fixed = compute_icrf_to_fixed_matrix(julian_date_seconds)?;
    match input_frame {
        ReferenceFrame::Inertial => Some(icrf_to_fixed * value),
        ReferenceFrame::Fixed => Some(icrf_to_fixed.transpose() * value),
    }
}

/// Wraps an optional position into a `PropertyValue`.
fn position_to_value(position: Option<DVec3>) -> PropertyValue {
    match position {
        Some(p) => PropertyValue::Cartesian3(p),
        None => PropertyValue::Undefined,
    }
}

// ---------------------------------------------------------------------------
// ConstantPositionProperty
// ---------------------------------------------------------------------------

/// A position property whose value does not change with respect to the
/// reference frame in which it is defined.
///
/// Maps to CesiumJS `DataSources/ConstantPositionProperty.js`.
#[derive(Debug, Clone, Default)]
pub struct ConstantPositionProperty {
    value: Option<DVec3>,
    reference_frame: ReferenceFrame,
}

impl ConstantPositionProperty {
    /// Creates a new constant position property in the fixed frame.
    pub fn new(value: DVec3) -> Self {
        Self {
            value: Some(value),
            reference_frame: ReferenceFrame::Fixed,
        }
    }

    /// Creates a new constant position property in the given reference frame.
    /// Maps to `new ConstantPositionProperty(value, referenceFrame)`.
    pub fn with_reference_frame(value: DVec3, reference_frame: ReferenceFrame) -> Self {
        Self {
            value: Some(value),
            reference_frame,
        }
    }

    /// Creates a constant position property with no value.
    pub fn undefined() -> Self {
        Self {
            value: None,
            reference_frame: ReferenceFrame::Fixed,
        }
    }

    /// Sets the value and optionally the reference frame.
    /// Maps to `ConstantPositionProperty.prototype.setValue`.
    pub fn set_value(&mut self, value: Option<DVec3>, reference_frame: Option<ReferenceFrame>) {
        self.value = value;
        if let Some(frame) = reference_frame {
            self.reference_frame = frame;
        }
    }

    /// The stored value (in this property's reference frame).
    pub fn value(&self) -> Option<DVec3> {
        self.value
    }

    /// Gets the position at `time` in the provided reference frame.
    /// Maps to `ConstantPositionProperty.prototype.getValueInReferenceFrame`.
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let value = self.value?;
        convert_to_reference_frame(time, value, self.reference_frame, reference_frame)
    }
}

impl DynProperty for ConstantPositionProperty {
    fn is_constant(&self) -> bool {
        // An inertial-frame position varies with time when expressed in the
        // fixed frame, so it is only constant when undefined or fixed-frame.
        self.value.is_none() || self.reference_frame == ReferenceFrame::Fixed
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "ConstantPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<ConstantPositionProperty>() {
            Some(o) => self.value == o.value && self.reference_frame == o.reference_frame,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// SampledPositionProperty
// ---------------------------------------------------------------------------

/// A `SampledProperty` which is also a position property.
///
/// Maps to CesiumJS `DataSources/SampledPositionProperty.js`.
#[derive(Debug, Clone)]
pub struct SampledPositionProperty {
    property: SampledProperty,
    reference_frame: ReferenceFrame,
    number_of_derivatives: usize,
}

impl SampledPositionProperty {
    /// Creates a new sampled position property.
    ///
    /// Maps to `new SampledPositionProperty(referenceFrame, numberOfDerivatives)`.
    pub fn new(reference_frame: ReferenceFrame, number_of_derivatives: usize) -> Self {
        let derivative_types = if number_of_derivatives > 0 {
            Some(vec![PackableType::Cartesian3; number_of_derivatives])
        } else {
            None
        };
        Self {
            property: SampledProperty::with_derivative_types(
                PackableType::Cartesian3,
                derivative_types,
            ),
            reference_frame,
            number_of_derivatives,
        }
    }

    /// Creates a new sampled position property in the fixed frame with no
    /// derivatives.
    pub fn fixed() -> Self {
        Self::new(ReferenceFrame::Fixed, 0)
    }

    /// The number of derivatives that accompany each position.
    /// Maps to `numberOfDerivatives`.
    pub fn number_of_derivatives(&self) -> usize {
        self.number_of_derivatives
    }

    /// The interpolation degree. Maps to `interpolationDegree`.
    pub fn interpolation_degree(&self) -> usize {
        self.property.interpolation_degree()
    }

    /// The interpolation algorithm. Maps to `interpolationAlgorithm`.
    pub fn interpolation_algorithm(&self) -> InterpolationAlgorithmKind {
        self.property.interpolation_algorithm()
    }

    /// The number of samples currently stored.
    pub fn sample_count(&self) -> usize {
        self.property.sample_count()
    }

    /// Sets the algorithm and degree to use when interpolating a position.
    /// Maps to `SampledPositionProperty.prototype.setInterpolationOptions`.
    pub fn set_interpolation_options(
        &mut self,
        algorithm: Option<InterpolationAlgorithmKind>,
        degree: Option<usize>,
    ) {
        self.property.set_interpolation_options(algorithm, degree);
    }

    /// Sets the forward extrapolation type. Maps to `forwardExtrapolationType`.
    pub fn set_forward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.property.set_forward_extrapolation_type(value);
    }

    /// Sets the forward extrapolation duration.
    /// Maps to `forwardExtrapolationDuration`.
    pub fn set_forward_extrapolation_duration(&mut self, value: f64) {
        self.property.set_forward_extrapolation_duration(value);
    }

    /// Sets the backward extrapolation type.
    /// Maps to `backwardExtrapolationType`.
    pub fn set_backward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.property.set_backward_extrapolation_type(value);
    }

    /// Sets the backward extrapolation duration.
    /// Maps to `backwardExtrapolationDuration`.
    pub fn set_backward_extrapolation_duration(&mut self, value: f64) {
        self.property.set_backward_extrapolation_duration(value);
    }

    /// Adds a new sample. Maps to `SampledPositionProperty.prototype.addSample`.
    pub fn add_sample(&mut self, time: JulianDate, position: DVec3, derivatives: &[DVec3]) {
        let value = PropertyValue::Cartesian3(position);
        let deriv_values: Vec<PropertyValue> = derivatives
            .iter()
            .map(|d| PropertyValue::Cartesian3(*d))
            .collect();
        self.property.add_sample(time, &value, &deriv_values);
    }

    /// Adds multiple samples via parallel arrays.
    /// Maps to `SampledPositionProperty.prototype.addSamples`.
    pub fn add_samples(
        &mut self,
        times: &[JulianDate],
        positions: &[DVec3],
        derivatives: Option<&[Vec<DVec3>]>,
    ) {
        let values: Vec<PropertyValue> = positions
            .iter()
            .map(|p| PropertyValue::Cartesian3(*p))
            .collect();
        let deriv_values: Option<Vec<Vec<PropertyValue>>> = derivatives.map(|ds| {
            ds.iter()
                .map(|dv| dv.iter().map(|d| PropertyValue::Cartesian3(*d)).collect())
                .collect()
        });
        self.property
            .add_samples(times, &values, deriv_values.as_deref());
    }

    /// Adds samples as a single packed array where each sample is a time
    /// offset (seconds from `epoch`) followed by the packed position and
    /// derivatives.
    /// Maps to `SampledPositionProperty.prototype.addSamplesPackedArray`.
    pub fn add_samples_packed_array(&mut self, packed_samples: &[f64], epoch: &JulianDate) {
        self.property
            .add_samples_packed_array(packed_samples, epoch);
    }

    /// Removes the sample at the given time, if present.
    /// Maps to `SampledPositionProperty.prototype.removeSample`.
    pub fn remove_sample(&mut self, time: &JulianDate) -> bool {
        self.property.remove_sample(time)
    }

    /// Removes all samples within the given time interval.
    /// Maps to `SampledPositionProperty.prototype.removeSamples`.
    pub fn remove_samples_interval(&mut self, time_interval: &TimeInterval) {
        self.property.remove_samples_interval(time_interval);
    }

    /// Gets the position at `time` in the provided reference frame.
    /// Maps to `SampledPositionProperty.prototype.getValueInReferenceFrame`.
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        match self.property.get_value(time) {
            PropertyValue::Cartesian3(p) => {
                convert_to_reference_frame(time, p, self.reference_frame, reference_frame)
            }
            _ => None,
        }
    }
}

impl DynProperty for SampledPositionProperty {
    fn is_constant(&self) -> bool {
        self.property.is_constant()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "SampledPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<SampledPositionProperty>() {
            Some(o) => {
                self.property.equals(&o.property) && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// CompositePositionProperty
// ---------------------------------------------------------------------------

/// A `CompositeProperty` which is also a position property.
///
/// Each interval's data is itself a position property; evaluation delegates
/// to the inner property's `getValueInReferenceFrame`.
///
/// Maps to CesiumJS `DataSources/CompositePositionProperty.js`.
#[derive(Clone)]
pub struct CompositePositionProperty {
    composite: CompositeProperty,
    reference_frame: ReferenceFrame,
}

impl CompositePositionProperty {
    /// Creates a new composite position property.
    /// Maps to `new CompositePositionProperty(referenceFrame)`.
    pub fn new(reference_frame: ReferenceFrame) -> Self {
        Self {
            composite: CompositeProperty::new(),
            reference_frame,
        }
    }

    /// The underlying interval collection. Maps to `intervals`.
    pub fn intervals(&self) -> &TimeIntervalCollection<Arc<dyn DynProperty>> {
        self.composite.intervals()
    }

    /// Adds an interval whose data is another (position) property.
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<Arc<dyn DynProperty>>) {
        self.composite.add_interval(interval, data);
    }

    /// Sets the "preferred" reference frame this position presents itself as.
    /// Maps to the `referenceFrame` setter.
    pub fn set_reference_frame(&mut self, frame: ReferenceFrame) {
        self.reference_frame = frame;
    }

    /// Gets the position at `time` in the provided reference frame.
    /// Maps to `CompositePositionProperty.prototype.getValueInReferenceFrame`.
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let inner = self
            .composite
            .intervals()
            .find_data_for_interval_containing_date(time)?;
        match inner.get_value_in_reference_frame(time, reference_frame)? {
            PropertyValue::Cartesian3(p) => Some(p),
            _ => None,
        }
    }
}

impl DynProperty for CompositePositionProperty {
    fn is_constant(&self) -> bool {
        self.composite.is_constant()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "CompositePositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CompositePositionProperty>() {
            Some(o) => {
                self.reference_frame == o.reference_frame
                    && self.composite.equals(&o.composite)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// TimeIntervalCollectionPositionProperty
// ---------------------------------------------------------------------------

fn position_same_data(left: &DVec3, right: &DVec3) -> bool {
    *left == *right
}

/// A `TimeIntervalCollectionProperty` which is also a position property.
///
/// Maps to CesiumJS `DataSources/TimeIntervalCollectionPositionProperty.js`.
#[derive(Debug, Clone)]
pub struct TimeIntervalCollectionPositionProperty {
    intervals: TimeIntervalCollection<DVec3>,
    reference_frame: ReferenceFrame,
}

impl TimeIntervalCollectionPositionProperty {
    /// Creates a new time interval collection position property.
    /// Maps to `new TimeIntervalCollectionPositionProperty(referenceFrame)`.
    pub fn new(reference_frame: ReferenceFrame) -> Self {
        Self {
            intervals: TimeIntervalCollection::new(),
            reference_frame,
        }
    }

    /// The underlying interval collection. Maps to `intervals`.
    pub fn intervals(&self) -> &TimeIntervalCollection<DVec3> {
        &self.intervals
    }

    /// Adds an interval with the given position data.
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<DVec3>) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &position_same_data);
    }

    /// Gets the position at `time` in the provided reference frame.
    ///
    /// Maps to
    /// `TimeIntervalCollectionPositionProperty.prototype.getValueInReferenceFrame`.
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let position = self.intervals.find_data_for_interval_containing_date(time)?;
        convert_to_reference_frame(time, *position, self.reference_frame, reference_frame)
    }
}

impl DynProperty for TimeIntervalCollectionPositionProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "TimeIntervalCollectionPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other
            .as_any()
            .downcast_ref::<TimeIntervalCollectionPositionProperty>()
        {
            Some(o) => {
                self.intervals.equals(&o.intervals, &position_same_data)
                    && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// CallbackPositionProperty
// ---------------------------------------------------------------------------

/// The callback signature used by `CallbackPositionProperty`: given a time,
/// returns the position in the property's reference frame (or `None`).
pub type PositionCallbackFn = Arc<dyn Fn(&JulianDate) -> Option<DVec3> + Send + Sync>;

/// A position property whose value is lazily evaluated by a callback function.
///
/// Maps to CesiumJS `DataSources/CallbackPositionProperty.js`.
pub struct CallbackPositionProperty {
    callback: PositionCallbackFn,
    is_constant: bool,
    reference_frame: ReferenceFrame,
}

impl CallbackPositionProperty {
    /// Creates a new callback position property.
    ///
    /// Maps to `new CallbackPositionProperty(callback, isConstant, referenceFrame)`.
    pub fn new(
        callback: PositionCallbackFn,
        is_constant: bool,
        reference_frame: ReferenceFrame,
    ) -> Self {
        Self {
            callback,
            is_constant,
            reference_frame,
        }
    }

    /// Replaces the callback and constancy flag.
    /// Maps to `CallbackPositionProperty.prototype.setCallback`.
    pub fn set_callback(&mut self, callback: PositionCallbackFn, is_constant: bool) {
        self.callback = callback;
        self.is_constant = is_constant;
    }

    /// Gets the position at `time` in the provided reference frame.
    ///
    /// Maps to `CallbackPositionProperty.prototype.getValueInReferenceFrame`.
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let value = (self.callback)(time)?;
        convert_to_reference_frame(time, value, self.reference_frame, reference_frame)
    }
}

impl DynProperty for CallbackPositionProperty {
    fn is_constant(&self) -> bool {
        self.is_constant
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "CallbackPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CallbackPositionProperty>() {
            Some(o) => {
                Arc::ptr_eq(&self.callback, &o.callback)
                    && self.is_constant == o.is_constant
                    && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_time::TimeInterval;
    use glam::DVec3;

    fn t(seconds: f64) -> JulianDate {
        JulianDate::new(2451545.0, seconds)
    }

    #[test]
    fn test_convert_same_frame_returns_unchanged() {
        let v = DVec3::new(1.0, 2.0, 3.0);
        let out = convert_to_reference_frame(&t(0.0), v, ReferenceFrame::Fixed, ReferenceFrame::Fixed);
        assert_eq!(out, Some(v));
        let out = convert_to_reference_frame(
            &t(0.0),
            v,
            ReferenceFrame::Inertial,
            ReferenceFrame::Inertial,
        );
        assert_eq!(out, Some(v));
    }

    #[test]
    fn test_convert_roundtrip_inertial_fixed() {
        let v = DVec3::new(1_000_000.0, 2_000_000.0, 3_000_000.0);
        let time = t(43200.0);
        let fixed = convert_to_reference_frame(&time, v, ReferenceFrame::Inertial, ReferenceFrame::Fixed)
            .unwrap();
        // Rotation preserves length.
        assert!((fixed.length() - v.length()).abs() < 1e-6);
        let back =
            convert_to_reference_frame(&time, fixed, ReferenceFrame::Fixed, ReferenceFrame::Inertial)
                .unwrap();
        assert!(back.abs_diff_eq(v, 1e-6));
    }

    #[test]
    fn test_convert_changes_with_time() {
        // The fixed-frame expression of an inertial position rotates over time.
        let v = DVec3::new(1_000_000.0, 0.0, 0.0);
        let f1 = convert_to_reference_frame(&t(0.0), v, ReferenceFrame::Inertial, ReferenceFrame::Fixed)
            .unwrap();
        let f2 = convert_to_reference_frame(
            &t(21600.0),
            v,
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(!f1.abs_diff_eq(f2, 1.0));
    }

    #[test]
    fn test_constant_position_fixed_frame() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let prop = ConstantPositionProperty::new(p);
        assert!(prop.is_constant());
        assert_eq!(prop.reference_frame(), Some(ReferenceFrame::Fixed));
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Cartesian3(p));
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Fixed),
            Some(PropertyValue::Cartesian3(p))
        );
    }

    #[test]
    fn test_constant_position_inertial_frame_not_constant() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let prop = ConstantPositionProperty::with_reference_frame(p, ReferenceFrame::Inertial);
        // Inertial-frame positions are NOT constant (they rotate in fixed frame).
        assert!(!prop.is_constant());

        // Value in its own frame is the stored value at any time.
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Inertial),
            Some(PropertyValue::Cartesian3(p))
        );
        assert_eq!(
            prop.get_value_in_reference_frame(&t(1000.0), ReferenceFrame::Inertial),
            Some(PropertyValue::Cartesian3(p))
        );

        // Fixed-frame value differs from the stored inertial value...
        let fixed = prop.get_value(&t(0.0));
        assert_ne!(fixed, PropertyValue::Cartesian3(p));
        // ...and round-trips back to the stored value.
        match fixed {
            PropertyValue::Cartesian3(fp) => {
                let back = convert_to_reference_frame(
                    &t(0.0),
                    fp,
                    ReferenceFrame::Fixed,
                    ReferenceFrame::Inertial,
                )
                .unwrap();
                assert!(back.abs_diff_eq(p, 1e-12));
            }
            _ => panic!("expected Cartesian3"),
        }
    }

    #[test]
    fn test_constant_position_undefined() {
        let prop = ConstantPositionProperty::undefined();
        assert!(prop.is_constant());
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Undefined);
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Fixed),
            None
        );
    }

    #[test]
    fn test_constant_position_equals() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let a = ConstantPositionProperty::new(p);
        let b = ConstantPositionProperty::new(p);
        let c = ConstantPositionProperty::with_reference_frame(p, ReferenceFrame::Inertial);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_sampled_position_linear_interpolation() {
        let mut prop = SampledPositionProperty::fixed();
        prop.add_sample(t(0.0), DVec3::new(0.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(10.0, 20.0, 30.0), &[]);
        assert!(!prop.is_constant());

        let mid = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        assert!(mid.abs_diff_eq(DVec3::new(5.0, 10.0, 15.0), 1e-12));

        // Exact sample times return exact values.
        let at0 = prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Fixed).unwrap();
        assert!(at0.abs_diff_eq(DVec3::ZERO, 1e-12));
    }

    #[test]
    fn test_sampled_position_inertial_frame() {
        let mut prop = SampledPositionProperty::new(ReferenceFrame::Inertial, 0);
        prop.add_sample(t(0.0), DVec3::new(1.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(2.0, 0.0, 0.0), &[]);

        // In its own (inertial) frame, interpolation is direct.
        let inertial = prop
            .position_in_reference_frame(&t(5.0), ReferenceFrame::Inertial)
            .unwrap();
        assert!(inertial.abs_diff_eq(DVec3::new(1.5, 0.0, 0.0), 1e-12));

        // Fixed-frame value is the rotated interpolated value.
        let fixed = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(5.0),
            DVec3::new(1.5, 0.0, 0.0),
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_sampled_position_with_derivatives() {
        // Hermite interpolation using velocity derivatives reconstructs a cubic.
        let mut prop = SampledPositionProperty::new(ReferenceFrame::Fixed, 1);
        prop.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));
        // p(t) = t^3 along x; p'(t) = 3t^2.
        for i in 0..=2 {
            let tf = i as f64;
            prop.add_sample(
                t(tf),
                DVec3::new(tf * tf * tf, 0.0, 0.0),
                &[DVec3::new(3.0 * tf * tf, 0.0, 0.0)],
            );
        }
        let mid = prop.position_in_reference_frame(&t(1.5), ReferenceFrame::Fixed).unwrap();
        assert!((mid.x - 3.375).abs() < 1e-9);
    }

    #[test]
    fn test_sampled_position_extrapolation_none() {
        let mut prop = SampledPositionProperty::fixed();
        prop.add_sample(t(0.0), DVec3::new(0.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(10.0, 0.0, 0.0), &[]);
        // Default extrapolation is NONE: outside the samples → undefined.
        assert_eq!(prop.get_value(&t(20.0)), PropertyValue::Undefined);
        assert_eq!(
            prop.position_in_reference_frame(&t(20.0), ReferenceFrame::Fixed),
            None
        );

        prop.set_forward_extrapolation_type(ExtrapolationType::Hold);
        let held = prop.position_in_reference_frame(&t(20.0), ReferenceFrame::Fixed).unwrap();
        assert!(held.abs_diff_eq(DVec3::new(10.0, 0.0, 0.0), 1e-12));
    }

    #[test]
    fn test_sampled_position_equals() {
        let mut a = SampledPositionProperty::fixed();
        a.add_sample(t(0.0), DVec3::new(1.0, 2.0, 3.0), &[]);
        let mut b = SampledPositionProperty::fixed();
        b.add_sample(t(0.0), DVec3::new(1.0, 2.0, 3.0), &[]);
        let c = SampledPositionProperty::new(ReferenceFrame::Inertial, 0);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_composite_position_delegates_to_inner() {
        let mut prop = CompositePositionProperty::new(ReferenceFrame::Fixed);
        let p1 = DVec3::new(1.0, 0.0, 0.0);
        let p2 = DVec3::new(2.0, 0.0, 0.0);
        prop.add_interval(
            TimeInterval::new(t(0.0), t(10.0), true, false),
            Some(Arc::new(ConstantPositionProperty::new(p1)) as Arc<dyn DynProperty>),
        );
        prop.add_interval(
            TimeInterval::new(t(10.0), t(20.0), true, true),
            Some(Arc::new(ConstantPositionProperty::new(p2)) as Arc<dyn DynProperty>),
        );
        assert!(!prop.is_constant());

        assert_eq!(prop.get_value(&t(5.0)), PropertyValue::Cartesian3(p1));
        assert_eq!(prop.get_value(&t(15.0)), PropertyValue::Cartesian3(p2));
        // Outside all intervals → undefined.
        assert_eq!(prop.get_value(&t(30.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_composite_position_inner_inertial() {
        // Inner properties handle their own frame conversion: an inertial
        // inner property queried for INERTIAL returns its stored value.
        let mut prop = CompositePositionProperty::new(ReferenceFrame::Fixed);
        let p = DVec3::new(5.0, 6.0, 7.0);
        prop.add_interval(
            TimeInterval::new(t(0.0), t(100.0), true, true),
            Some(
                Arc::new(ConstantPositionProperty::with_reference_frame(
                    p,
                    ReferenceFrame::Inertial,
                )) as Arc<dyn DynProperty>,
            ),
        );
        assert_eq!(
            prop.position_in_reference_frame(&t(50.0), ReferenceFrame::Inertial),
            Some(p)
        );
    }

    #[test]
    fn test_tic_position_property() {
        let mut prop = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        assert!(prop.is_constant()); // empty → constant

        let p = DVec3::new(100.0, 200.0, 300.0);
        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        assert!(!prop.is_constant());

        assert_eq!(prop.get_value(&t(5.0)), PropertyValue::Cartesian3(p));
        assert_eq!(prop.get_value(&t(11.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_tic_position_inertial_frame() {
        let mut prop = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Inertial);
        let p = DVec3::new(100.0, 200.0, 300.0);
        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));

        // Own frame: stored value.
        assert_eq!(
            prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Inertial),
            Some(p)
        );
        // Fixed frame: rotated.
        let fixed = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(5.0),
            p,
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_tic_position_equals() {
        let mut a = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        let mut b = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        let p = DVec3::new(1.0, 2.0, 3.0);
        a.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        b.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        assert!(a.equals(&b));

        let c = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Inertial);
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_callback_position_property() {
        let callback: PositionCallbackFn = Arc::new(|time: &JulianDate| {
            let s = time.seconds_of_day;
            Some(DVec3::new(s, 0.0, 0.0))
        });
        let prop = CallbackPositionProperty::new(callback, false, ReferenceFrame::Fixed);
        assert!(!prop.is_constant());
        assert_eq!(
            prop.get_value(&t(7.0)),
            PropertyValue::Cartesian3(DVec3::new(7.0, 0.0, 0.0))
        );
    }

    #[test]
    fn test_callback_position_none_value() {
        let callback: PositionCallbackFn = Arc::new(|_time: &JulianDate| None);
        let prop = CallbackPositionProperty::new(callback, true, ReferenceFrame::Fixed);
        assert!(prop.is_constant());
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_callback_position_inertial() {
        let callback: PositionCallbackFn =
            Arc::new(|_time: &JulianDate| Some(DVec3::new(1.0, 2.0, 3.0)));
        let prop = CallbackPositionProperty::new(callback, true, ReferenceFrame::Inertial);

        assert_eq!(
            prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Inertial),
            Some(DVec3::new(1.0, 2.0, 3.0))
        );
        let fixed = prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(0.0),
            DVec3::new(1.0, 2.0, 3.0),
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_callback_position_equals() {
        let cb: PositionCallbackFn = Arc::new(|_time: &JulianDate| Some(DVec3::ONE));
        let a = CallbackPositionProperty::new(Arc::clone(&cb), true, ReferenceFrame::Fixed);
        let b = CallbackPositionProperty::new(Arc::clone(&cb), true, ReferenceFrame::Fixed);
        let c = CallbackPositionProperty::new(Arc::clone(&cb), false, ReferenceFrame::Fixed);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }
}
