//! Ported from `packages/engine/Source/DataSources/PositionProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::julian_date::JulianDate;
use cesium_core::matrix3::Matrix3;
use cesium_core::transforms;

use crate::property::Property;

/// A property that defines a position in 3D space.
///
/// Position properties return `Cartesian3` values and may vary over time.
pub trait PositionProperty: Property {
    /// Returns the position value at the given time.
    fn position_value<'a>(&self, time: f64, result: &'a mut Cartesian3) -> Option<&'a Cartesian3>;

    /// Returns the reference frame in which this position is defined.
    fn reference_frame(&self) -> PositionReferenceFrame;

    /// Port of the per-implementation `getValueInReferenceFrame(time,
    /// referenceFrame, result)`: evaluates [`position_value`](Self::position_value)
    /// and converts from the property's own frame into `reference_frame`
    /// when they differ (mirroring e.g. `ConstantPositionProperty.
    /// getValueInReferenceFrame`).
    fn get_value_in_reference_frame<'a>(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
        result: &'a mut Cartesian3,
    ) -> Option<&'a Cartesian3> {
        let mut scratch = Cartesian3::ZERO;
        let value = self.position_value(time, &mut scratch)?;
        let value = *value;
        let input_frame = self.reference_frame();
        if input_frame == reference_frame {
            *result = value;
            return Some(result);
        }
        convert_to_reference_frame(time, &value, input_frame, reference_frame, result)
            .map(|r| &*r)
    }
}

/// The reference frame for a position property.
///
/// Port of `Core/ReferenceFrame.js` (the two frames used by position
/// properties).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionReferenceFrame {
    /// The position is defined in the fixed frame (Earth-centered, Earth-fixed).
    Fixed,
    /// The position is defined in the inertial frame.
    Inertial,
}

/// Julian day number of the J2000 epoch used to map the crate-wide `f64`
/// time convention (seconds) onto a [`JulianDate`] for reference-frame
/// conversions.
pub const J2000_DAY_NUMBER: i32 = 2451545;

/// Converts the crate-wide `f64` time (seconds since the J2000 epoch) to a
/// [`JulianDate`].
///
/// DEVIATION: CesiumJS position properties accept a `JulianDate` directly;
/// the Rust port uses seconds offset from the J2000 epoch
/// (`2451545.0` days, TAI components), matching the crate-wide `f64` time
/// convention, and converts to `JulianDate` only where frame conversions
/// need calendar time.
pub fn time_to_julian_date(time: f64) -> JulianDate {
    JulianDate::add_seconds_new(&JulianDate::from_tai_components(J2000_DAY_NUMBER, 0.0), time)
}

/// Port of the (private static) `PositionProperty.convertToReferenceFrame`.
///
/// Converts the provided position `value` from `input_frame` to
/// `output_frame` at the provided time, storing the result in `result`.
/// Returns `None` when the conversion data is unavailable or the input
/// frame is unknown (mirroring the JS `undefined` return).
pub fn convert_to_reference_frame<'a>(
    time: f64,
    value: &Cartesian3,
    input_frame: PositionReferenceFrame,
    output_frame: PositionReferenceFrame,
    result: &'a mut Cartesian3,
) -> Option<&'a mut Cartesian3> {
    if input_frame == output_frame {
        *result = *value;
        return Some(result);
    }

    let date = time_to_julian_date(time);
    let mut scratch = Matrix3::default();
    let icrf_to_fixed =
        transforms::compute_icrf_to_central_body_fixed_matrix(&date, &mut scratch)?;

    match input_frame {
        PositionReferenceFrame::Inertial => {
            Matrix3::multiply_by_vector(icrf_to_fixed, value, result);
            Some(result)
        }
        PositionReferenceFrame::Fixed => {
            let transposed = Matrix3::transpose_new(icrf_to_fixed);
            Matrix3::multiply_by_vector(&transposed, value, result);
            Some(result)
        }
    }
}
