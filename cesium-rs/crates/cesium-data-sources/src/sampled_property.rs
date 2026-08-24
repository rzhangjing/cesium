//! Ported from `packages/engine/Source/DataSources/SampledProperty.js`.
//!
//! A [`Property`] whose value is interpolated for a given time from the
//! provided set of samples and specified interpolation algorithm and degree.
//!
//! DEVIATION (structural): CesiumJS stores JulianDate instances in `_times`;
//! the Rust port uses plain `f64` seconds (the crate-wide time convention),
//! so `convertDate`/epoch handling reduces to an optional epoch offset and
//! `JulianDate.secondsDifference` reduces to subtraction.
//!
//! DEVIATION (events): the `definitionChanged` event is intentionally not
//! implemented here; the event system is owned by a separate work item.
//!
//! DEVIATION (PackableForInterpolation): CesiumJS types implementing
//! `PackableForInterpolation` (Quaternion) are converted to a dedicated
//! interpolation representation (`convertPackedArrayForInterpolation` /
//! `unpackInterpolationResult`). The Rust port interpolates all types
//! directly in their packed representation, so Quaternion interpolation is
//! component-wise rather than CesiumJS's relative-quaternion form.

use cesium_core::binary_search::binary_search;
use cesium_core::extrapolation_type::ExtrapolationType;
use cesium_core::hermite_polynomial_approximation::HermitePolynomialApproximation;
use cesium_core::lagrange_polynomial_approximation::LagrangePolynomialApproximation;
use cesium_core::linear_approximation;

use crate::property::{Property, PropertyResult};

/// The packable value type of a [`SampledProperty`] (mirrors the JS
/// `type` / `derivativeTypes` constructor parameters, which accept
/// `Number` or any `Packable` constructor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PackableType {
    /// A scalar number (JS `Number`, packed length 1).
    Number,
    /// An RGBA color (packed length 4).
    Color,
    /// A 3D position (packed length 3).
    Position,
    /// A 3D Cartesian (packed length 3).
    Cartesian3,
    /// A quaternion orientation (packed length 4).
    Quaternion,
    /// A near/far scalar (packed length 4).
    NearFarScalar,
    /// A rectangle (packed length 4).
    Rectangle,
}

impl PackableType {
    /// The number of components used to pack one value of this type.
    pub fn packed_length(self) -> usize {
        match self {
            PackableType::Number => 1,
            PackableType::Color => 4,
            PackableType::Position | PackableType::Cartesian3 => 3,
            PackableType::Quaternion => 4,
            PackableType::NearFarScalar => 4,
            PackableType::Rectangle => 4,
        }
    }

    /// Appends the packed components of `value` to `out`.
    ///
    /// Mirrors `innerType.pack(value, array, startingIndex)`.
    pub fn pack(self, value: &PropertyResult, out: &mut Vec<f64>) {
        match (self, value) {
            (PackableType::Number, PropertyResult::Number(v)) => out.push(*v),
            (PackableType::Color, PropertyResult::Color(r, g, b, a)) => {
                out.extend_from_slice(&[*r, *g, *b, *a]);
            }
            (PackableType::Position, PropertyResult::Position(x, y, z))
            | (PackableType::Cartesian3, PropertyResult::Cartesian3(x, y, z)) => {
                out.extend_from_slice(&[*x, *y, *z]);
            }
            (PackableType::Quaternion, PropertyResult::Quaternion(x, y, z, w)) => {
                out.extend_from_slice(&[*x, *y, *z, *w]);
            }
            (PackableType::NearFarScalar, PropertyResult::NearFarScalar(a, b, c, d)) => {
                out.extend_from_slice(&[*a, *b, *c, *d]);
            }
            (PackableType::Rectangle, PropertyResult::Rectangle(a, b, c, d)) => {
                out.extend_from_slice(&[*a, *b, *c, *d]);
            }
            _ => debug_assert!(
                false,
                "value variant does not match the declared PackableType"
            ),
        }
    }

    /// Unpacks a value of this type from `array` starting at `starting_index`.
    ///
    /// Mirrors `innerType.unpack(array, startingIndex, result)`.
    pub fn unpack(self, array: &[f64], starting_index: usize) -> PropertyResult {
        match self {
            PackableType::Number => PropertyResult::Number(array[starting_index]),
            PackableType::Color => PropertyResult::Color(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
                array[starting_index + 3],
            ),
            PackableType::Position => PropertyResult::Position(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
            ),
            PackableType::Cartesian3 => PropertyResult::Cartesian3(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
            ),
            PackableType::Quaternion => PropertyResult::Quaternion(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
                array[starting_index + 3],
            ),
            PackableType::NearFarScalar => PropertyResult::NearFarScalar(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
                array[starting_index + 3],
            ),
            PackableType::Rectangle => PropertyResult::Rectangle(
                array[starting_index],
                array[starting_index + 1],
                array[starting_index + 2],
                array[starting_index + 3],
            ),
        }
    }
}

/// The interpolation algorithm used by [`SampledProperty`] (mirrors the JS
/// `InterpolationAlgorithm` singleton objects passed to
/// `setInterpolationOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InterpolationAlgorithmKind {
    /// Mirrors `LinearApproximation`.
    Linear,
    /// Mirrors `LagrangePolynomialApproximation`.
    Lagrange,
    /// Mirrors `HermitePolynomialApproximation`.
    Hermite,
}

impl InterpolationAlgorithmKind {
    /// Mirrors `InterpolationAlgorithm.getRequiredDataPoints(degree, inputOrder)`.
    pub fn get_required_data_points(self, degree: u32, input_order: usize) -> usize {
        match self {
            // LinearApproximation.getRequiredDataPoints -> 2
            InterpolationAlgorithmKind::Linear => 2,
            // LagrangePolynomialApproximation.getRequiredDataPoints -> degree + 1 (min 2)
            InterpolationAlgorithmKind::Lagrange => {
                LagrangePolynomialApproximation::get_required_data_points(degree as f64) as usize
            }
            // HermitePolynomialApproximation.getRequiredDataPoints(degree, inputOrder)
            InterpolationAlgorithmKind::Hermite => HermitePolynomialApproximation::get_required_data_points(
                degree as f64,
                Some(input_order as f64),
            ) as usize,
        }
    }

    /// Whether this algorithm defines a higher-order `interpolate` method
    /// (only Hermite does in CesiumJS).
    fn has_interpolate(self) -> bool {
        matches!(self, InterpolationAlgorithmKind::Hermite)
    }
}

// We can't use Vec::splice for inserting new elements because function apply
// can't handle a huge number of arguments (mirrors the JS `arrayInsert` note).
fn array_insert(array: &mut Vec<f64>, start_index: usize, items: &[f64]) {
    let old_length = array.len();
    let new_length = old_length + items.len();
    array.resize(new_length, 0.0);
    // Shift the tail right to make room, mirroring the JS loop. In JS the
    // source index may run negative (reading `undefined`); those slots are
    // always overwritten by the copy below, so they are simply skipped.
    if old_length != start_index {
        let mut i = new_length;
        let mut q = old_length as isize;
        while i > start_index {
            i -= 1;
            q -= 1;
            if q >= 0 {
                array[i] = array[q as usize];
            }
        }
    }
    array[start_index..start_index + items.len()].copy_from_slice(items);
}

/// Mirrors JS `convertDate`: times are stored as `f64` seconds, and numeric
/// entries in packed sample arrays are offsets from `epoch` when one is
/// supplied.
fn convert_date(date: f64, epoch: Option<f64>) -> f64 {
    match epoch {
        Some(epoch) => epoch + date,
        None => date,
    }
}

/// Mirrors `SampledProperty._mergeNewSamples(epoch, times, values, newData, packedLength)`.
///
/// `new_data` is a flat array where each sample is a time followed by
/// `packed_length` value components.
fn merge_new_samples(
    epoch: Option<f64>,
    times: &mut Vec<f64>,
    values: &mut Vec<f64>,
    new_data: &[f64],
    packed_length: usize,
) {
    let mut new_data_index = 0usize;

    while new_data_index < new_data.len() {
        let current_time = convert_date(new_data[new_data_index], epoch);
        let search = binary_search(times, &current_time, |a: &f64, b: &f64| *a - *b);

        let mut times_splice_args: Vec<f64> = Vec::new();
        let mut values_splice_args: Vec<f64> = Vec::new();

        if search < 0 {
            // Doesn't exist, insert as many additional values as we can.
            let times_insertion_point = (!search) as usize;
            let values_insertion_point = times_insertion_point * packed_length;
            let mut prev_item: Option<f64> = None;
            let next_time = times.get(times_insertion_point).copied();

            let mut inner_index = new_data_index;
            while inner_index < new_data.len() {
                let current_time = convert_date(new_data[inner_index], epoch);
                if (prev_item.is_some() && prev_item.unwrap() >= current_time)
                    || (next_time.is_some() && current_time >= next_time.unwrap())
                {
                    break;
                }
                times_splice_args.push(current_time);
                inner_index += 1;
                for _ in 0..packed_length {
                    values_splice_args.push(new_data[inner_index]);
                    inner_index += 1;
                }
                prev_item = Some(current_time);
            }
            new_data_index = inner_index;

            if !times_splice_args.is_empty() {
                array_insert(values, values_insertion_point, &values_splice_args);
                array_insert(times, times_insertion_point, &times_splice_args);
            }
        } else {
            // Found an exact match
            let times_insertion_point = search as usize;
            for i in 0..packed_length {
                new_data_index += 1;
                values[times_insertion_point * packed_length + i] = new_data[new_data_index];
            }
            new_data_index += 1;
        }
    }
}

/// A [`Property`] whose value is interpolated for a given time from the
/// provided set of samples and specified interpolation algorithm and degree.
pub struct SampledProperty {
    r#type: PackableType,
    derivative_types: Option<Vec<PackableType>>,
    input_order: usize,
    packed_length: usize,
    interpolation_degree: u32,
    interpolation_algorithm: InterpolationAlgorithmKind,
    times: Vec<f64>,
    values: Vec<f64>,
    forward_extrapolation_type: ExtrapolationType,
    forward_extrapolation_duration: f64,
    backward_extrapolation_type: ExtrapolationType,
    backward_extrapolation_duration: f64,
}

impl SampledProperty {
    /// Port of `new SampledProperty(type)`.
    pub fn new(r#type: PackableType) -> Self {
        Self::with_derivative_types(r#type, None)
    }

    /// Port of `new SampledProperty(type, derivativeTypes)`. When
    /// `derivative_types` is supplied, samples must contain derivative
    /// information of the specified types.
    pub fn with_derivative_types(
        r#type: PackableType,
        derivative_types: Option<Vec<PackableType>>,
    ) -> Self {
        let mut packed_length = r#type.packed_length();
        let input_order = match &derivative_types {
            Some(derivative_types) => {
                for derivative_type in derivative_types {
                    packed_length += derivative_type.packed_length();
                }
                derivative_types.len()
            }
            None => 0,
        };

        Self {
            r#type,
            derivative_types,
            input_order,
            packed_length,
            interpolation_degree: 1,
            interpolation_algorithm: InterpolationAlgorithmKind::Linear,
            times: Vec::new(),
            values: Vec::new(),
            forward_extrapolation_type: ExtrapolationType::None,
            forward_extrapolation_duration: 0.0,
            backward_extrapolation_type: ExtrapolationType::None,
            backward_extrapolation_duration: 0.0,
        }
    }

    /// Gets the type of property (JS `type` getter).
    pub fn property_type(&self) -> PackableType {
        self.r#type
    }

    /// Gets the derivative types used by this property (JS `derivativeTypes`
    /// getter).
    pub fn derivative_types(&self) -> Option<&[PackableType]> {
        self.derivative_types.as_deref()
    }

    /// Gets the degree of interpolation (JS `interpolationDegree` getter).
    pub fn interpolation_degree(&self) -> u32 {
        self.interpolation_degree
    }

    /// Gets the interpolation algorithm (JS `interpolationAlgorithm` getter).
    pub fn interpolation_algorithm(&self) -> InterpolationAlgorithmKind {
        self.interpolation_algorithm
    }

    /// JS `forwardExtrapolationType` getter.
    pub fn forward_extrapolation_type(&self) -> ExtrapolationType {
        self.forward_extrapolation_type
    }

    /// JS `forwardExtrapolationType` setter.
    pub fn set_forward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.forward_extrapolation_type = value;
    }

    /// JS `forwardExtrapolationDuration` getter.
    pub fn forward_extrapolation_duration(&self) -> f64 {
        self.forward_extrapolation_duration
    }

    /// JS `forwardExtrapolationDuration` setter.
    pub fn set_forward_extrapolation_duration(&mut self, value: f64) {
        self.forward_extrapolation_duration = value;
    }

    /// JS `backwardExtrapolationType` getter.
    pub fn backward_extrapolation_type(&self) -> ExtrapolationType {
        self.backward_extrapolation_type
    }

    /// JS `backwardExtrapolationType` setter.
    pub fn set_backward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.backward_extrapolation_type = value;
    }

    /// JS `backwardExtrapolationDuration` getter.
    pub fn backward_extrapolation_duration(&self) -> f64 {
        self.backward_extrapolation_duration
    }

    /// JS `backwardExtrapolationDuration` setter.
    pub fn set_backward_extrapolation_duration(&mut self, value: f64) {
        self.backward_extrapolation_duration = value;
    }

    /// The number of samples currently stored.
    pub fn sample_count(&self) -> usize {
        self.times.len()
    }

    /// Port of `getValue(time)`. Returns `None` where CesiumJS returns
    /// `undefined` (empty samples, out of range without extrapolation, or
    /// not enough samples to interpolate).
    pub fn get_value_option(&self, time: f64) -> Option<PropertyResult> {
        let times = &self.times;
        let times_length = times.len();
        if times_length == 0 {
            return None;
        }

        let values = &self.values;
        let search = binary_search(times, &time, |a: &f64, b: &f64| *a - *b);

        if search >= 0 {
            return Some(self.r#type.unpack(values, search as usize * self.packed_length));
        }

        let mut index = (!search) as usize;

        if index == 0 {
            let start_time = times[index];
            let timeout = self.backward_extrapolation_duration;
            if self.backward_extrapolation_type == ExtrapolationType::None
                || (timeout != 0.0 && start_time - time > timeout)
            {
                return None;
            }
            if self.backward_extrapolation_type == ExtrapolationType::Hold {
                return Some(self.r#type.unpack(values, 0));
            }
        }

        if index >= times_length {
            index = times_length - 1;
            let end_time = times[index];
            let timeout = self.forward_extrapolation_duration;
            if self.forward_extrapolation_type == ExtrapolationType::None
                || (timeout != 0.0 && time - end_time > timeout)
            {
                return None;
            }
            if self.forward_extrapolation_type == ExtrapolationType::Hold {
                return Some(self.r#type.unpack(values, index * self.packed_length));
            }
        }

        let interpolation_algorithm = self.interpolation_algorithm;
        let packed_interpolation_length = self.packed_length;
        let input_order = self.input_order;

        // JS lazily recomputes `_numberOfPoints` when `_updateTableLength`
        // is set; recomputing on every call is observationally identical.
        let number_of_points = interpolation_algorithm
            .get_required_data_points(self.interpolation_degree, input_order)
            .min(times_length);

        let degree = number_of_points as isize - 1;
        if degree < 1 {
            return None;
        }

        let mut first_index: isize = 0;
        let mut last_index: isize = times_length as isize - 1;
        let points_in_collection = last_index - first_index + 1;

        if points_in_collection >= degree + 1 {
            let mut computed_first_index = index as isize - (degree / 2) - 1;
            if computed_first_index < first_index {
                computed_first_index = first_index;
            }
            let mut computed_last_index = computed_first_index + degree;
            if computed_last_index > last_index {
                computed_last_index = last_index;
                computed_first_index = computed_last_index - degree;
                if computed_first_index < first_index {
                    computed_first_index = first_index;
                }
            }

            first_index = computed_first_index;
            last_index = computed_last_index;
        }
        let length = (last_index - first_index + 1) as usize;
        let first_index = first_index as usize;
        let last_index = last_index as usize;

        // Build the tables
        let mut x_table = vec![0.0; length];
        for i in 0..length {
            x_table[i] = times[first_index + i] - times[last_index];
        }

        // DEVIATION: no convertPackedArrayForInterpolation support; the
        // packed values (including derivative components) are used directly.
        let y_table = values[first_index * self.packed_length..(last_index + 1) * self.packed_length]
            .to_vec();

        // Interpolate!
        let x = time - times[last_index];
        let interpolation_result: Vec<f64>;
        if input_order == 0 || !interpolation_algorithm.has_interpolate() {
            interpolation_result = match interpolation_algorithm {
                InterpolationAlgorithmKind::Linear => {
                    let mut result = vec![0.0; packed_interpolation_length];
                    linear_approximation::interpolate_order_zero(
                        x,
                        &x_table,
                        &y_table,
                        packed_interpolation_length,
                        &mut result,
                    );
                    result
                }
                InterpolationAlgorithmKind::Lagrange => {
                    LagrangePolynomialApproximation::interpolate_order_zero(
                        x,
                        &x_table,
                        &y_table,
                        packed_interpolation_length,
                        None,
                    )
                }
                InterpolationAlgorithmKind::Hermite => {
                    HermitePolynomialApproximation::interpolate_order_zero(
                        x,
                        &x_table,
                        &y_table,
                        packed_interpolation_length,
                        None,
                    )
                }
            };
        } else {
            let y_stride = packed_interpolation_length / (input_order + 1);
            interpolation_result = HermitePolynomialApproximation::interpolate(
                x,
                &x_table,
                &y_table,
                y_stride,
                input_order,
                input_order,
                None,
            );
        }

        Some(self.r#type.unpack(&interpolation_result, 0))
    }

    /// Port of `setInterpolationOptions(options)`. Unsupplied options leave
    /// the existing property unchanged.
    pub fn set_interpolation_options(
        &mut self,
        interpolation_algorithm: Option<InterpolationAlgorithmKind>,
        interpolation_degree: Option<u32>,
    ) {
        if let Some(algorithm) = interpolation_algorithm {
            if self.interpolation_algorithm != algorithm {
                self.interpolation_algorithm = algorithm;
            }
        }
        if let Some(degree) = interpolation_degree {
            if self.interpolation_degree != degree {
                self.interpolation_degree = degree;
            }
        }
    }

    /// Port of `addSample(time, value)` for properties without derivative
    /// types.
    pub fn add_sample(&mut self, time: f64, value: &PropertyResult) {
        debug_assert!(
            self.derivative_types.is_none(),
            "derivatives is required when the property was created with derivative types"
        );

        let mut data: Vec<f64> = Vec::with_capacity(1 + self.r#type.packed_length());
        data.push(time);
        self.r#type.pack(value, &mut data);

        merge_new_samples(None, &mut self.times, &mut self.values, &data, self.packed_length);
    }

    /// Port of `addSample(time, value, derivatives)` for properties created
    /// with derivative types.
    pub fn add_sample_with_derivatives(
        &mut self,
        time: f64,
        value: &PropertyResult,
        derivatives: &[PropertyResult],
    ) {
        let derivative_types = self
            .derivative_types
            .as_ref()
            .expect("derivative types are required to be defined");
        debug_assert_eq!(
            derivatives.len(),
            derivative_types.len(),
            "derivatives must have one entry per derivative type"
        );

        let mut data: Vec<f64> = Vec::with_capacity(1 + self.packed_length);
        data.push(time);
        self.r#type.pack(value, &mut data);
        for (derivative_type, derivative) in derivative_types.iter().zip(derivatives.iter()) {
            derivative_type.pack(derivative, &mut data);
        }

        merge_new_samples(None, &mut self.times, &mut self.values, &data, self.packed_length);
    }

    /// Port of `addSamples(times, values)` (no derivatives).
    pub fn add_samples(&mut self, times: &[f64], values: &[PropertyResult]) {
        debug_assert!(
            self.derivative_types.is_none(),
            "derivativeValues is required when the property was created with derivative types"
        );
        debug_assert_eq!(
            times.len(),
            values.len(),
            "times and values must be the same length."
        );

        let mut data: Vec<f64> = Vec::with_capacity(times.len() * (1 + self.r#type.packed_length()));
        for (time, value) in times.iter().zip(values.iter()) {
            data.push(*time);
            self.r#type.pack(value, &mut data);
        }

        merge_new_samples(None, &mut self.times, &mut self.values, &data, self.packed_length);
    }

    /// Port of `addSamples(times, values, derivativeValues)`.
    pub fn add_samples_with_derivatives(
        &mut self,
        times: &[f64],
        values: &[PropertyResult],
        derivative_values: &[Vec<PropertyResult>],
    ) {
        let derivative_types = self
            .derivative_types
            .as_ref()
            .expect("derivative types are required to be defined");
        debug_assert_eq!(
            times.len(),
            values.len(),
            "times and values must be the same length."
        );
        debug_assert_eq!(
            derivative_values.len(),
            times.len(),
            "times and derivativeValues must be the same length."
        );

        let mut data: Vec<f64> = Vec::with_capacity(times.len() * (1 + self.packed_length));
        for ((time, value), derivatives) in times
            .iter()
            .zip(values.iter())
            .zip(derivative_values.iter())
        {
            data.push(*time);
            self.r#type.pack(value, &mut data);
            for (derivative_type, derivative) in derivative_types.iter().zip(derivatives.iter()) {
                derivative_type.pack(derivative, &mut data);
            }
        }

        merge_new_samples(None, &mut self.times, &mut self.values, &data, self.packed_length);
    }

    /// Port of `addSamplesPackedArray(packedSamples, epoch)`. Each sample is
    /// a time (an offset in seconds from `epoch` when one is supplied)
    /// followed by the packed value and derivative components.
    pub fn add_samples_packed_array(&mut self, packed_samples: &[f64], epoch: Option<f64>) {
        merge_new_samples(
            epoch,
            &mut self.times,
            &mut self.values,
            packed_samples,
            self.packed_length,
        );
    }

    /// Port of `getSample(index)`. A negative index accesses the list of
    /// samples in reverse order.
    pub fn get_sample(&self, mut index: i64) -> Option<f64> {
        let len = self.times.len() as i64;
        if index < 0 {
            index += len;
        }
        if index < 0 || index >= len {
            return None;
        }
        Some(self.times[index as usize])
    }

    /// Port of `removeSample(time)`. Returns `true` if a sample at `time`
    /// was removed.
    pub fn remove_sample(&mut self, time: f64) -> bool {
        let index = binary_search(&self.times, &time, |a: &f64, b: &f64| *a - *b);
        if index < 0 {
            return false;
        }
        self.remove_samples_internal(index as usize, 1);
        true
    }

    /// Port of `removeSamples(timeInterval)`. Removes all samples inside the
    /// interval `[start, stop]` honoring the endpoint inclusion flags.
    pub fn remove_samples_interval(
        &mut self,
        start: f64,
        stop: f64,
        is_start_included: bool,
        is_stop_included: bool,
    ) {
        let times = &self.times;
        let mut start_index = binary_search(times, &start, |a: &f64, b: &f64| *a - *b);
        if start_index < 0 {
            start_index = !start_index;
        } else if !is_start_included {
            start_index += 1;
        }
        let mut stop_index = binary_search(times, &stop, |a: &f64, b: &f64| *a - *b);
        if stop_index < 0 {
            stop_index = !stop_index;
        } else if is_stop_included {
            stop_index += 1;
        }

        let start_index = start_index as usize;
        let stop_index = stop_index as usize;
        self.remove_samples_internal(start_index, stop_index - start_index);
    }

    /// Mirrors the private `removeSamples(property, startIndex, numberToRemove)`.
    fn remove_samples_internal(&mut self, start_index: usize, number_to_remove: usize) {
        if number_to_remove == 0 {
            return;
        }
        let packed_length = self.packed_length;
        self.times.drain(start_index..start_index + number_to_remove);
        self.values.drain(
            start_index * packed_length..(start_index + number_to_remove) * packed_length,
        );
    }

    /// Port of `equals(other)` for two [`SampledProperty`] instances.
    pub fn equals(&self, other: &SampledProperty) -> bool {
        if self.r#type != other.r#type
            || self.interpolation_degree != other.interpolation_degree
            || self.interpolation_algorithm != other.interpolation_algorithm
        {
            return false;
        }

        match (&self.derivative_types, &other.derivative_types) {
            (Some(derivative_types), Some(other_derivative_types)) => {
                if derivative_types.len() != other_derivative_types.len() {
                    return false;
                }
                for (derivative_type, other_derivative_type) in derivative_types
                    .iter()
                    .zip(other_derivative_types.iter())
                {
                    if derivative_type != other_derivative_type {
                        return false;
                    }
                }
            }
            (None, None) => {}
            _ => return false,
        }

        if self.times != other.times {
            return false;
        }

        // Since time lengths are equal, values length and other length are
        // guaranteed to be equal.
        self.values == other.values
    }

    /// Exposed for testing. Mirrors `SampledProperty._mergeNewSamples`.
    #[doc(hidden)]
    pub fn merge_new_samples_for_testing(
        epoch: Option<f64>,
        times: &mut Vec<f64>,
        values: &mut Vec<f64>,
        new_data: &[f64],
        packed_length: usize,
    ) {
        merge_new_samples(epoch, times, values, new_data, packed_length);
    }

    /// Read access to the stored sample times (exposed for testing;
    /// mirrors `property._times`).
    #[doc(hidden)]
    pub fn times_for_testing(&self) -> &[f64] {
        &self.times
    }

    /// Read access to the stored packed values (exposed for testing;
    /// mirrors `property._values`).
    #[doc(hidden)]
    pub fn values_for_testing(&self) -> &[f64] {
        &self.values
    }
}

impl Property for SampledProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        self.get_value_option(time).unwrap_or(PropertyResult::None)
    }

    fn is_constant(&self) -> bool {
        self.values.is_empty()
    }

    fn is_destroyed(&self) -> bool {
        false
    }
}
