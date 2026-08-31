//! Ported from `packages/engine/Source/DataSources/SampledPositionProperty.js`.
//!
//! A [`SampledProperty`] which is also a [`PositionProperty`].

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;
use cesium_core::extrapolation_type::ExtrapolationType;

use crate::position_property::{
    convert_to_reference_frame, PositionProperty, PositionReferenceFrame,
};
use crate::property::{Property, PropertyResult};
use crate::sampled_property::{InterpolationAlgorithmKind, PackableType, SampledProperty};

/// A [`SampledProperty`] which is also a [`PositionProperty`].
///
/// Port of `SampledPositionProperty`; delegates all sample storage and
/// interpolation to an inner `SampledProperty` of `Cartesian3` values,
/// mirroring the CesiumJS delegation architecture.
pub struct SampledPositionProperty {
    number_of_derivatives: usize,
    property: SampledProperty,
    definition_changed: Event<()>,
    reference_frame: PositionReferenceFrame,
}

impl SampledPositionProperty {
    /// Port of `new SampledPositionProperty(referenceFrame, numberOfDerivatives)`.
    ///
    /// `reference_frame` defaults to [`PositionReferenceFrame::Fixed`] and
    /// `number_of_derivatives` to `0` when `None` (JS `??` defaults).
    pub fn new(
        reference_frame: Option<PositionReferenceFrame>,
        number_of_derivatives: Option<usize>,
    ) -> Self {
        let number_of_derivatives = number_of_derivatives.unwrap_or(0);

        let derivative_types = if number_of_derivatives > 0 {
            Some(vec![PackableType::Cartesian3; number_of_derivatives])
        } else {
            None
        };

        Self {
            number_of_derivatives,
            property: SampledProperty::with_derivative_types(
                PackableType::Cartesian3,
                derivative_types,
            ),
            definition_changed: Event::new(),
            reference_frame: reference_frame.unwrap_or(PositionReferenceFrame::Fixed),
        }
    }

    /// Port of the `numberOfDerivatives` getter.
    pub fn number_of_derivatives(&self) -> usize {
        self.number_of_derivatives
    }

    /// Port of the `referenceFrame` getter.
    pub fn reference_frame(&self) -> PositionReferenceFrame {
        self.reference_frame
    }

    /// Port of the `definitionChanged` getter.
    pub fn definition_changed_event(&self) -> &Event<()> {
        &self.definition_changed
    }

    /// Port of the `interpolationDegree` getter.
    pub fn interpolation_degree(&self) -> u32 {
        self.property.interpolation_degree()
    }

    /// Port of the `interpolationAlgorithm` getter.
    pub fn interpolation_algorithm(&self) -> InterpolationAlgorithmKind {
        self.property.interpolation_algorithm()
    }

    /// Port of the `forwardExtrapolationType` getter.
    pub fn forward_extrapolation_type(&self) -> ExtrapolationType {
        self.property.forward_extrapolation_type()
    }

    /// Port of the `forwardExtrapolationType` setter: raises
    /// `definitionChanged` only when the value actually changes (the JS
    /// inner `SampledProperty` setter semantics).
    pub fn set_forward_extrapolation_type(&mut self, value: ExtrapolationType) {
        if self.property.forward_extrapolation_type() != value {
            self.property.set_forward_extrapolation_type(value);
            self.definition_changed.raise_event(&());
        }
    }

    /// Port of the `forwardExtrapolationDuration` getter.
    pub fn forward_extrapolation_duration(&self) -> f64 {
        self.property.forward_extrapolation_duration()
    }

    /// Port of the `forwardExtrapolationDuration` setter: raises
    /// `definitionChanged` only when the value actually changes.
    pub fn set_forward_extrapolation_duration(&mut self, value: f64) {
        if self.property.forward_extrapolation_duration() != value {
            self.property.set_forward_extrapolation_duration(value);
            self.definition_changed.raise_event(&());
        }
    }

    /// Port of the `backwardExtrapolationType` getter.
    pub fn backward_extrapolation_type(&self) -> ExtrapolationType {
        self.property.backward_extrapolation_type()
    }

    /// Port of the `backwardExtrapolationType` setter: raises
    /// `definitionChanged` only when the value actually changes.
    pub fn set_backward_extrapolation_type(&mut self, value: ExtrapolationType) {
        if self.property.backward_extrapolation_type() != value {
            self.property.set_backward_extrapolation_type(value);
            self.definition_changed.raise_event(&());
        }
    }

    /// Port of the `backwardExtrapolationDuration` getter.
    pub fn backward_extrapolation_duration(&self) -> f64 {
        self.property.backward_extrapolation_duration()
    }

    /// Port of the `backwardExtrapolationDuration` setter: raises
    /// `definitionChanged` only when the value actually changes.
    pub fn set_backward_extrapolation_duration(&mut self, value: f64) {
        if self.property.backward_extrapolation_duration() != value {
            self.property.set_backward_extrapolation_duration(value);
            self.definition_changed.raise_event(&());
        }
    }

    /// Port of `getValueInReferenceFrame(time, referenceFrame, result)`.
    ///
    /// Returns the position at `time` expressed in `reference_frame`, or
    /// `None` when the property is undefined at the time or the frame
    /// conversion data is unavailable.
    pub fn get_value_in_reference_frame<'a>(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        let value = self.property.get_value_option(time)?;
        let (x, y, z) = value.as_position()?;
        let value = Cartesian3::new(x, y, z);
        convert_to_reference_frame(time, &value, self.reference_frame, reference_frame, result)
            .map(|r| r as &Cartesian3)
    }

    /// Port of `setInterpolationOptions(options)`.
    pub fn set_interpolation_options(
        &mut self,
        interpolation_algorithm: Option<InterpolationAlgorithmKind>,
        interpolation_degree: Option<u32>,
    ) {
        self.property
            .set_interpolation_options(interpolation_algorithm, interpolation_degree);
        // The JS inner SampledProperty raises `definitionChanged` here; the
        // wrapper forwards it (see DEVIATION note on `definition_changed`).
        self.definition_changed.raise_event(&());
    }

    /// Port of `addSample(time, position, derivatives)`.
    ///
    /// # Panics
    ///
    /// Debug builds panic with a `DeveloperError` when this property has
    /// derivatives and `derivatives` is missing or of the wrong length.
    pub fn add_sample(&mut self, time: f64, position: &Cartesian3, derivatives: &[Cartesian3]) {
        let number_of_derivatives = self.number_of_derivatives;
        if cfg!(debug_assertions) {
            if number_of_derivatives > 0 && derivatives.len() != number_of_derivatives {
                panic!("DeveloperError: derivatives length must be equal to the number of derivatives.");
            }
        }
        let value = PropertyResult::Cartesian3(position.x, position.y, position.z);
        if number_of_derivatives > 0 {
            let derivative_values: Vec<PropertyResult> = derivatives
                .iter()
                .map(|d| PropertyResult::Cartesian3(d.x, d.y, d.z))
                .collect();
            self.property
                .add_sample_with_derivatives(time, &value, &derivative_values);
        } else {
            self.property.add_sample(time, &value);
        }
        // See DEVIATION note on `definition_changed`.
        self.definition_changed.raise_event(&());
    }

    /// Port of `addSamples(times, positions, derivatives)`.
    ///
    /// `derivatives` is optional (one derivative list per time index).
    pub fn add_samples(
        &mut self,
        times: &[f64],
        positions: &[Cartesian3],
        derivatives: Option<&[Vec<Cartesian3>]>,
    ) {
        let values: Vec<PropertyResult> = positions
            .iter()
            .map(|p| PropertyResult::Cartesian3(p.x, p.y, p.z))
            .collect();
        match derivatives {
            Some(derivative_values) => {
                let mapped: Vec<Vec<PropertyResult>> = derivative_values
                    .iter()
                    .map(|ds| {
                        ds.iter()
                            .map(|d| PropertyResult::Cartesian3(d.x, d.y, d.z))
                            .collect()
                    })
                    .collect();
                self.property
                    .add_samples_with_derivatives(times, &values, &mapped);
            }
            None => self.property.add_samples(times, &values),
        }
        // See DEVIATION note on `definition_changed`.
        self.definition_changed.raise_event(&());
    }

    /// Port of `addSamplesPackedArray(packedSamples, epoch)`.
    pub fn add_samples_packed_array(&mut self, packed_samples: &[f64], epoch: Option<f64>) {
        self.property.add_samples_packed_array(packed_samples, epoch);
        // See DEVIATION note on `definition_changed`.
        self.definition_changed.raise_event(&());
    }

    /// Port of `removeSample(time)`. Returns `true` when a sample was removed.
    pub fn remove_sample(&mut self, time: f64) -> bool {
        let removed = self.property.remove_sample(time);
        if removed {
            // See DEVIATION note on `definition_changed`.
            self.definition_changed.raise_event(&());
        }
        removed
    }

    /// Port of `removeSamples(timeInterval)`.
    ///
    /// DEVIATION: CesiumJS takes a `TimeInterval` (with `JulianDate`
    /// endpoints); the Rust port takes the interval components directly in
    /// the crate-wide `f64` seconds convention.
    pub fn remove_samples(
        &mut self,
        start: f64,
        stop: f64,
        is_start_included: bool,
        is_stop_included: bool,
    ) {
        self.property.remove_samples_interval(
            start,
            stop,
            is_start_included,
            is_stop_included,
        );
        // See DEVIATION note on `definition_changed`.
        self.definition_changed.raise_event(&());
    }

    /// Port of `equals(other)`: compares the inner sampled property and the
    /// reference frame.
    pub fn equals_sampled(&self, other: &SampledPositionProperty) -> bool {
        self.reference_frame == other.reference_frame && self.property.equals(&other.property)
    }

    /// Read access to the inner [`SampledProperty`] (testing/interop).
    pub fn inner_property(&self) -> &SampledProperty {
        &self.property
    }
}

impl Default for SampledPositionProperty {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl Property for SampledPositionProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        // JS `getValue` evaluates in the FIXED frame.
        let mut result = Cartesian3::default();
        match self.get_value_in_reference_frame(time, PositionReferenceFrame::Fixed, &mut result) {
            Some(p) => PropertyResult::Position(p.x, p.y, p.z),
            None => PropertyResult::None,
        }
    }

    fn is_constant(&self) -> bool {
        self.property.is_constant()
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<SampledPositionProperty>())
            .map(|other| self.equals_sampled(other))
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_position_property(
        &self,
    ) -> Option<&dyn crate::position_property::PositionProperty> {
        Some(self)
    }

    /// DEVIATION: CesiumJS forwards the inner `SampledProperty`'s
    /// `definitionChanged` event; the Rust `SampledProperty` port does not
    /// expose that event, so this wrapper raises its own event directly at
    /// every mutation point. The observable event behavior is identical.
    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}

impl PositionProperty for SampledPositionProperty {
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3> {
        // JS `getValue` returns the value in the FIXED frame.
        self.get_value_in_reference_frame(time, PositionReferenceFrame::Fixed, result)
    }

    fn reference_frame(&self) -> PositionReferenceFrame {
        self.reference_frame
    }

    fn get_value_in_reference_frame<'a>(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        // Delegate to the inherent port of JS `getValueInReferenceFrame`
        // (converts from the property's own frame, not from FIXED).
        self.get_value_in_reference_frame(time, reference_frame, result)
    }
}
