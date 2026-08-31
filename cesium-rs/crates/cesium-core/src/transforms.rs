//! Ported from packages/engine/Source/Core/Transforms.js
//!
//! Contains functions for transforming positions to various reference frames.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::cartographic::Cartographic;
use crate::earth_orientation_parameters::EarthOrientationParameters;
use crate::earth_orientation_parameters_sample::EarthOrientationParametersSample;
use crate::ellipsoid::Ellipsoid;
use crate::heading_pitch_roll::HeadingPitchRoll;
use crate::iau2006_xys_data::Iau2006XysData;
use crate::iau2006_xys_sample::Iau2006XysSample;
use crate::julian_date::JulianDate;
use crate::map_projection::MapProjection;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;
use crate::quaternion::Quaternion;
use crate::time_constants::{DAYS_PER_JULIAN_CENTURY, SECONDS_PER_DAY};
use crate::time_interval::TimeInterval;
use std::sync::{Mutex, OnceLock};

/// Axis direction names for local reference frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    East,
    North,
    Up,
    West,
    South,
    Down,
}

/// Cross-product table: given first × second axis, returns the third axis.
fn vector_product_local_frame(first: AxisDirection, second: AxisDirection) -> Option<AxisDirection> {
    use AxisDirection::*;
    match (first, second) {
        // up × {south,north,west,east}
        (Up, South) => Some(East),
        (Up, North) => Some(West),
        (Up, West) => Some(South),
        (Up, East) => Some(North),
        // down × {south,north,west,east}
        (Down, South) => Some(West),
        (Down, North) => Some(East),
        (Down, West) => Some(South),
        (Down, East) => Some(North),
        // south × {up,down,west,east}
        (South, Up) => Some(West),
        (South, Down) => Some(East),
        (South, West) => Some(Down),
        (South, East) => Some(Up),
        // north × {up,down,west,east}
        (North, Up) => Some(East),
        (North, Down) => Some(West),
        (North, West) => Some(Up),
        (North, East) => Some(Down),
        // west × {up,down,north,south}
        (West, Up) => Some(North),
        (West, Down) => Some(South),
        (West, North) => Some(Down),
        (West, South) => Some(Up),
        // east × {up,down,north,south}
        (East, Up) => Some(South),
        (East, Down) => Some(North),
        (East, North) => Some(Up),
        (East, South) => Some(Down),
        // Degenerate / invalid combinations
        _ => None,
    }
}

/// Degenerate local frame vectors at the origin (position = 0,0,0).
fn degenerate_direction(dir: AxisDirection) -> Cartesian3 {
    use AxisDirection::*;
    match dir {
        North => Cartesian3::new(-1.0, 0.0, 0.0),
        East => Cartesian3::new(0.0, 1.0, 0.0),
        Up => Cartesian3::new(0.0, 0.0, 1.0),
        South => Cartesian3::new(1.0, 0.0, 0.0),
        West => Cartesian3::new(0.0, -1.0, 0.0),
        Down => Cartesian3::new(0.0, 0.0, -1.0),
    }
}

/// Returns true if the axis is East or West (not affected by pole sign).
fn is_east_west(dir: AxisDirection) -> bool {
    matches!(dir, AxisDirection::East | AxisDirection::West)
}

/// Port of `Transforms.localFrameToFixedFrameGenerator`.
///
/// Computes a 4×4 transformation matrix from a local reference frame
/// (defined by first and second axes) to the ellipsoid's fixed frame.
pub fn local_frame_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    first_axis: AxisDirection,
    second_axis: AxisDirection,
    result: &mut Matrix4,
) -> bool {
    let third_axis = match vector_product_local_frame(first_axis, second_axis) {
        Some(a) => a,
        None => return false,
    };

    let mut first;
    let mut second;
    let mut third;

    if Cartesian3::equals_epsilon(Some(origin), Some(&Cartesian3::ZERO), Some(CesiumMath::EPSILON14), None) {
        // Origin at center — use degenerate local frame
        first = degenerate_direction(first_axis);
        second = degenerate_direction(second_axis);
        third = degenerate_direction(third_axis);
    } else if CesiumMath::equals_epsilon(origin.x, 0.0, Some(CesiumMath::EPSILON14), None)
        && CesiumMath::equals_epsilon(origin.y, 0.0, Some(CesiumMath::EPSILON14), None)
    {
        // At a pole — special case
        let sign = CesiumMath::sign(origin.z);

        first = degenerate_direction(first_axis);
        if !is_east_west(first_axis) {
            first = Cartesian3::multiply_by_scalar_new(&first, sign);
        }

        second = degenerate_direction(second_axis);
        if !is_east_west(second_axis) {
            second = Cartesian3::multiply_by_scalar_new(&second, sign);
        }

        third = degenerate_direction(third_axis);
        if !is_east_west(third_axis) {
            third = Cartesian3::multiply_by_scalar_new(&third, sign);
        }
    } else {
        // Normal case — compute from geodetic surface normal
        let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let mut up = Cartesian3::default();
        ell.geodetic_surface_normal(origin, &mut up);

        let mut east = Cartesian3::new(-origin.y, origin.x, 0.0);
        east = Cartesian3::normalize_new(&east);
        let north = Cartesian3::cross_new(&up, &east);

        let down = Cartesian3::multiply_by_scalar_new(&up, -1.0);
        let west = Cartesian3::multiply_by_scalar_new(&east, -1.0);
        let south = Cartesian3::multiply_by_scalar_new(&north, -1.0);

        first = direction_vector(first_axis, &east, &north, &up, &west, &south, &down);
        second = direction_vector(second_axis, &east, &north, &up, &west, &south, &down);
        third = direction_vector(third_axis, &east, &north, &up, &west, &south, &down);
    }

    // Column-major storage: col0 = first, col1 = second, col2 = third, col3 = translation
    result.elements[0] = first.x;
    result.elements[1] = first.y;
    result.elements[2] = first.z;
    result.elements[3] = 0.0;
    result.elements[4] = second.x;
    result.elements[5] = second.y;
    result.elements[6] = second.z;
    result.elements[7] = 0.0;
    result.elements[8] = third.x;
    result.elements[9] = third.y;
    result.elements[10] = third.z;
    result.elements[11] = 0.0;
    result.elements[12] = origin.x;
    result.elements[13] = origin.y;
    result.elements[14] = origin.z;
    result.elements[15] = 1.0;
    true
}

fn direction_vector(
    dir: AxisDirection,
    east: &Cartesian3,
    north: &Cartesian3,
    up: &Cartesian3,
    west: &Cartesian3,
    south: &Cartesian3,
    down: &Cartesian3,
) -> Cartesian3 {
    match dir {
        AxisDirection::East => *east,
        AxisDirection::North => *north,
        AxisDirection::Up => *up,
        AxisDirection::West => *west,
        AxisDirection::South => *south,
        AxisDirection::Down => *down,
    }
}

/// Port of `Transforms.eastNorthUpToFixedFrame`.
pub fn east_north_up_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::East, AxisDirection::North, result)
}

pub fn east_north_up_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    east_north_up_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northEastDownToFixedFrame`.
pub fn north_east_down_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::East, result)
}

pub fn north_east_down_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_east_down_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northUpEastToFixedFrame`.
pub fn north_up_east_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::Up, result)
}

pub fn north_up_east_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_up_east_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.northWestUpToFixedFrame`.
pub fn north_west_up_to_fixed_frame(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    local_frame_to_fixed_frame(origin, ellipsoid, AxisDirection::North, AxisDirection::West, result)
}

pub fn north_west_up_to_fixed_frame_new(
    origin: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    north_west_up_to_fixed_frame(origin, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.headingPitchRollToFixedFrame`.
pub fn heading_pitch_roll_to_fixed_frame(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Matrix4,
) -> bool {
    let hpr_quaternion = Quaternion::from_heading_pitch_roll_new(hpr);
    let scale = Cartesian3::new(1.0, 1.0, 1.0);
    let hpr_matrix = Matrix4::from_translation_quaternion_rotation_scale_new(
        &Cartesian3::ZERO,
        &hpr_quaternion,
        &scale,
    );
    if !east_north_up_to_fixed_frame(origin, ellipsoid, result) {
        return false;
    }
    let tmp = Matrix4::multiply_new(result, &hpr_matrix);
    *result = tmp;
    true
}

pub fn heading_pitch_roll_to_fixed_frame_new(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix4 {
    let mut result = Matrix4::default();
    heading_pitch_roll_to_fixed_frame(origin, hpr, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.headingPitchRollQuaternion`.
pub fn heading_pitch_roll_quaternion(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut Quaternion,
) -> bool {
    let mut transform = Matrix4::default();
    if !heading_pitch_roll_to_fixed_frame(origin, hpr, ellipsoid, &mut transform) {
        return false;
    }
    let rotation = Matrix4::get_matrix3_new(&transform);
    Quaternion::from_rotation_matrix(&rotation, result);
    true
}

pub fn heading_pitch_roll_quaternion_new(
    origin: &Cartesian3,
    hpr: &HeadingPitchRoll,
    ellipsoid: Option<&Ellipsoid>,
) -> Quaternion {
    let mut result = Quaternion::default();
    heading_pitch_roll_quaternion(origin, hpr, ellipsoid, &mut result);
    result
}

/// Port of `Transforms.fixedFrameToHeadingPitchRoll`.
pub fn fixed_frame_to_heading_pitch_roll(
    transform: &Matrix4,
    ellipsoid: Option<&Ellipsoid>,
    result: &mut HeadingPitchRoll,
) -> bool {
    let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
    let center = Matrix4::get_translation_new(transform);

    if Cartesian3::equals(Some(&center), Some(&Cartesian3::ZERO)) {
        result.heading = 0.0;
        result.pitch = 0.0;
        result.roll = 0.0;
        return true;
    }

    let mut ff = Matrix4::default();
    east_north_up_to_fixed_frame(&center, Some(ell), &mut ff);
    let mut to_fixed = Matrix4::default();
    Matrix4::inverse_transformation(&ff, &mut to_fixed);

    let no_scale = Cartesian3::new(1.0, 1.0, 1.0);
    let mut transform_copy = Matrix4::default();
    Matrix4::set_scale(transform, &no_scale, &mut transform_copy);
    let mut transform_copy2 = Matrix4::default();
    Matrix4::set_translation(&transform_copy, &Cartesian3::ZERO, &mut transform_copy2);

    let mut to_fixed_result = Matrix4::default();
    Matrix4::multiply(&to_fixed, &transform_copy2, &mut to_fixed_result);
    to_fixed = to_fixed_result;

    let rotation = Matrix4::get_matrix3_new(&to_fixed);
    let mut quat = Quaternion::default();
    Quaternion::from_rotation_matrix(&rotation, &mut quat);
    quat = Quaternion::normalize_new(&quat);

    HeadingPitchRoll::from_quaternion(&quat, result);
    true
}

pub fn fixed_frame_to_heading_pitch_roll_new(
    transform: &Matrix4,
    ellipsoid: Option<&Ellipsoid>,
) -> HeadingPitchRoll {
    let mut result = HeadingPitchRoll::default();
    fixed_frame_to_heading_pitch_roll(transform, ellipsoid, &mut result);
    result
}

// ---------------------------------------------------------------------------
// ICRF / TEME reference-frame transforms
// ---------------------------------------------------------------------------

const GMST_CONSTANT0: f64 = 6.0 * 3600.0 + 41.0 * 60.0 + 50.54841;
const GMST_CONSTANT1: f64 = 8640184.812866;
const GMST_CONSTANT2: f64 = 0.093104;
const GMST_CONSTANT3: f64 = -6.2e-6;
const RATE_COEF: f64 = 1.1772758384668e-19;
const WGS84_W_R_PRECESSION: f64 = 7.2921158553e-5;
const TWO_PI_OVER_SECONDS_IN_DAY: f64 = CesiumMath::TWO_PI / 86400.0;

/// Port of `Transforms.computeTemeToPseudoFixedMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from True Equator
/// Mean Equinox (TEME) axes to the pseudo-fixed axes at a given time. This
/// method treats the UT1 time standard as equivalent to UTC.
pub fn compute_teme_to_pseudo_fixed_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> &'a mut Matrix3 {
    // GMST is actually computed using UT1.  We're using UTC as an approximation of UT1.
    // We do not want to use the function like convertTaiToUtc in JulianDate because
    // we explicitly do not want to fail when inside the leap second.
    let date_in_utc = JulianDate::add_seconds_new(date, -JulianDate::compute_tai_minus_utc(date));
    let utc_day_number = date_in_utc.day_number;
    let utc_seconds_into_day = date_in_utc.seconds_of_day;

    let diff_days = utc_day_number as f64 - 2451545.0;
    let t = if utc_seconds_into_day >= 43200.0 {
        (diff_days + 0.5) / DAYS_PER_JULIAN_CENTURY
    } else {
        (diff_days - 0.5) / DAYS_PER_JULIAN_CENTURY
    };

    let gmst0 =
        GMST_CONSTANT0 + t * (GMST_CONSTANT1 + t * (GMST_CONSTANT2 + t * GMST_CONSTANT3));
    let angle = (gmst0 * TWO_PI_OVER_SECONDS_IN_DAY) % CesiumMath::TWO_PI;
    let ratio = WGS84_W_R_PRECESSION + RATE_COEF * (utc_day_number as f64 - 2451545.5);
    let seconds_since_midnight = (utc_seconds_into_day + SECONDS_PER_DAY * 0.5) % SECONDS_PER_DAY;
    let gha = angle + ratio * seconds_since_midnight;
    let cos_gha = gha.cos();
    let sin_gha = gha.sin();

    // DEVIATION (upstream quirk): the CesiumJS `new Matrix3(...)` branch
    // (no `result` argument) stores the transposed layout; the `result`
    // branch stores `[cos, -sin, 0, sin, cos, 0, ...]`. The Rust port
    // always takes `&mut result` and mirrors the `result` branch, matching
    // golden vectors generated through the two-argument call.
    result.elements = [
        cos_gha, -sin_gha, 0.0, //
        sin_gha, cos_gha, 0.0, //
        0.0, 0.0, 1.0,
    ];
    result
}

/// Allocating variant of [`compute_teme_to_pseudo_fixed_matrix`].
pub fn compute_teme_to_pseudo_fixed_matrix_new(date: &JulianDate) -> Matrix3 {
    let mut result = Matrix3::default();
    compute_teme_to_pseudo_fixed_matrix(date, &mut result);
    result
}

/// Port of `Transforms.computeIcrfToCentralBodyFixedMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from the
/// International Celestial Reference Frame (GCRF/ICRF) inertial frame axes to
/// the central body fixed frame axes at a given time. Returns `None` when the
/// data necessary for the transformation is not available.
///
/// Mirrors the JS fallback: when `computeIcrfToFixedMatrix` returns
/// `undefined` (the IAU 2006 XYS samples have not been downloaded), the
/// lower-precision `computeTemeToPseudoFixedMatrix` is used instead.
pub fn compute_icrf_to_central_body_fixed_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> Option<&'a mut Matrix3> {
    if compute_icrf_to_fixed_matrix(date, result).is_none() {
        compute_teme_to_pseudo_fixed_matrix(date, result);
    }
    Some(result)
}

/// Port of `Transforms.rotationMatrixFromPositionVelocity`.
///
/// Computes a rotation matrix from a position and velocity, where the
/// velocity direction is the local x axis of the resulting frame.
pub fn rotation_matrix_from_position_velocity<'a>(
    position: &Cartesian3,
    velocity: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
    result: &'a mut Matrix3,
) -> &'a mut Matrix3 {
    let ellipsoid = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
    let mut normal = Cartesian3::default();
    ellipsoid.geodetic_surface_normal(position, &mut normal);
    let mut right = Cartesian3::cross_new(velocity, &normal);

    if Cartesian3::equals_epsilon(
        Some(&right),
        Some(&Cartesian3::ZERO),
        Some(CesiumMath::EPSILON6),
        None,
    ) {
        right = Cartesian3::UNIT_X;
    }

    let mut up = Cartesian3::cross_new(&right, velocity);
    up = Cartesian3::normalize_new(&up);
    right = Cartesian3::cross_new(velocity, &up);
    right = Cartesian3::negate_new(&right);
    right = Cartesian3::normalize_new(&right);

    result.elements = [
        velocity.x, velocity.y, velocity.z, //
        right.x, right.y, right.z, //
        up.x, up.y, up.z,
    ];
    result
}

/// Allocating variant of [`rotation_matrix_from_position_velocity`].
pub fn rotation_matrix_from_position_velocity_new(
    position: &Cartesian3,
    velocity: &Cartesian3,
    ellipsoid: Option<&Ellipsoid>,
) -> Matrix3 {
    let mut result = Matrix3::default();
    rotation_matrix_from_position_velocity(position, velocity, ellipsoid, &mut result);
    result
}

// ---------------------------------------------------------------------------
// ICRF / EOP data plumbing and high-precision frame transforms
// ---------------------------------------------------------------------------

/// Mirrors the JS module-level `ttMinusTai` constant.
const TT_MINUS_TAI: f64 = 32.184;
/// Mirrors the JS module-level `j2000ttDays` constant.
const J2000_TT_DAYS: f64 = 2451545.0;
/// Mirrors the JS module-level `TdtMinusTai` constant.
const TDT_MINUS_TAI: f64 = 32.184;
/// Mirrors the JS module-level `J2000d` constant.
const J2000_D: f64 = 2451545.0;

/// Mirrors `Transforms.earthOrientationParameters`, which defaults to
/// `EarthOrientationParameters.NONE` (an object whose `compute` always yields
/// a zero-valued sample and never returns `undefined`).
fn earth_orientation_parameters() -> &'static EarthOrientationParameters {
    static EOP: OnceLock<EarthOrientationParameters> = OnceLock::new();
    EOP.get_or_init(|| EarthOrientationParameters::new(None, None))
}

/// Mirrors `Transforms.iau2006XysData` (`new Iau2006XysData()`); the sample
/// table starts unloaded, so `computeXysRadians` returns `undefined` (Rust
/// `None`) until samples are provided.
fn iau2006_xys_data() -> &'static Mutex<Iau2006XysData> {
    static XYS: OnceLock<Mutex<Iau2006XysData>> = OnceLock::new();
    XYS.get_or_init(|| Mutex::new(Iau2006XysData::new(None)))
}

/// Port of `Transforms.preloadIcrfFixed`.
///
/// DEVIATION: the JS implementation asynchronously downloads IAU 2006 XYS
/// pages over the network and returns a Promise. The Rust port has no network
/// resource pipeline in this module, so this function mirrors the TT
/// day/second derivation but performs no download; the ICRF samples therefore
/// remain unloaded, matching the JS "not yet loaded" state in which
/// `computeFixedToIcrfMatrix`/`computeIcrfToFixedMatrix` return `undefined`.
pub fn preload_icrf_fixed(time_interval: &TimeInterval) {
    let start_day_tt = time_interval.start.day_number;
    let start_second_tt = time_interval.start.seconds_of_day + TT_MINUS_TAI;
    let stop_day_tt = time_interval.stop.day_number;
    let stop_second_tt = time_interval.stop.seconds_of_day + TT_MINUS_TAI;
    // JS: `return Transforms.iau2006XysData.preload(startDayTT, startSecondTT, stopDayTT, stopSecondTT)`.
    let _ = (start_day_tt, start_second_tt, stop_day_tt, stop_second_tt);
}

/// Port of `Transforms.computeFixedToIcrfMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from the
/// Earth-Fixed frame axes (ITRF) to the International Celestial Reference
/// Frame (GCRF/ICRF) inertial frame axes at a given time. Returns `None`
/// (the JS `undefined`) if the data necessary to do the transformation is not
/// yet loaded.
pub fn compute_fixed_to_icrf_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> Option<&'a mut Matrix3> {
    // Compute pole wander.
    // DEVIATION: the JS `EarthOrientationParameters#compute` returns
    // `undefined` while EOP data downloaded from a URL is unavailable; the
    // JS `Transforms` module however defaults to `EarthOrientationParameters.NONE`,
    // which always yields a zero-valued sample, and the Rust port mirrors that
    // exact source (zero-filled samples when no data is loaded), so the
    // `undefined` EOP branch is unreachable here by design.
    let mut eop_scratch = EarthOrientationParametersSample::new(0.0, 0.0, 0.0, 0.0, 0.0);
    earth_orientation_parameters().compute(date, &mut eop_scratch);

    // There is no external conversion to Terrestrial Time (TT).
    // So use International Atomic Time (TAI) and convert using offsets.
    // Here we are assuming that dayTT and secondTT are positive.
    let day_tt = date.day_number as i64;
    // It's possible here that secondTT could roll over 86400.
    // This does not seem to affect the precision (unit tests check for this).
    let second_tt = date.seconds_of_day + TT_MINUS_TAI;

    let mut xys_scratch: Option<Iau2006XysSample> = None;
    let xys = {
        let mut xys_data = iau2006_xys_data().lock().unwrap();
        xys_data.compute_xys_radians(day_tt, second_tt, &mut xys_scratch)
    };
    let Some(xys) = xys else {
        return None;
    };

    let x = xys.x + eop_scratch.x_pole_offset;
    let y = xys.y + eop_scratch.y_pole_offset;

    // Compute XYS rotation.
    let a = 1.0 / (1.0 + (1.0 - x * x - y * y).sqrt());

    let mut rotation1 = Matrix3::default();
    rotation1.elements[0] = 1.0 - a * x * x;
    rotation1.elements[3] = -a * x * y;
    rotation1.elements[6] = x;
    rotation1.elements[1] = -a * x * y;
    rotation1.elements[4] = 1.0 - a * y * y;
    rotation1.elements[7] = y;
    rotation1.elements[2] = -x;
    rotation1.elements[5] = -y;
    rotation1.elements[8] = 1.0 - a * (x * x + y * y);

    let rotation2 = Matrix3::from_rotation_z_new(-xys.s);
    let matrix_q = Matrix3::multiply_new(&rotation1, &rotation2);

    // Similar to TT conversions above.
    // It's possible here that secondTT could roll over 86400.
    // This does not seem to affect the precision (unit tests check for this).
    let date_ut1_day = date.day_number;
    let date_ut1_sec = date.seconds_of_day - JulianDate::compute_tai_minus_utc(date)
        + eop_scratch.ut1_minus_utc;

    // Compute Earth rotation angle.
    // The IERS standard for era is
    //    era = 0.7790572732640 + 1.00273781191135448 * Tu
    // where
    //    Tu = JulianDateInUt1 - 2451545.0
    // However, you get much more precision with the following simplification.
    let days_since_j2000 = date_ut1_day - 2451545;
    let fraction_of_day = date_ut1_sec / SECONDS_PER_DAY;
    let mut era = 0.779057273264
        + fraction_of_day
        + 0.00273781191135448 * (days_since_j2000 as f64 + fraction_of_day);
    era = (era % 1.0) * CesiumMath::TWO_PI;

    let earth_rotation = Matrix3::from_rotation_z_new(era);

    // pseudoFixed to ICRF
    let pf_to_icrf = Matrix3::multiply_new(&matrix_q, &earth_rotation);

    // Compute pole wander matrix.
    let cosxp = eop_scratch.x_pole_wander.cos();
    let cosyp = eop_scratch.y_pole_wander.cos();
    let sinxp = eop_scratch.x_pole_wander.sin();
    let sinyp = eop_scratch.y_pole_wander.sin();

    let mut ttt = day_tt as f64 - J2000_TT_DAYS + second_tt / SECONDS_PER_DAY;
    ttt /= 36525.0;

    // Approximate sp value in radians.
    let sp = (-47.0e-6 * ttt * CesiumMath::RADIANS_PER_DEGREE) / 3600.0;
    let cossp = sp.cos();
    let sinsp = sp.sin();

    let mut f_to_pf_mtx = Matrix3::default();
    f_to_pf_mtx.elements[0] = cosxp * cossp;
    f_to_pf_mtx.elements[1] = cosxp * sinsp;
    f_to_pf_mtx.elements[2] = sinxp;
    f_to_pf_mtx.elements[3] = -cosyp * sinsp + sinyp * sinxp * cossp;
    f_to_pf_mtx.elements[4] = cosyp * cossp + sinyp * sinxp * sinsp;
    f_to_pf_mtx.elements[5] = -sinyp * cosxp;
    f_to_pf_mtx.elements[6] = -sinyp * sinsp - cosyp * sinxp * cossp;
    f_to_pf_mtx.elements[7] = sinyp * cossp - cosyp * sinxp * sinsp;
    f_to_pf_mtx.elements[8] = cosyp * cosxp;

    Matrix3::multiply(&pf_to_icrf, &f_to_pf_mtx, result);
    Some(result)
}

/// Allocating variant of [`compute_fixed_to_icrf_matrix`].
pub fn compute_fixed_to_icrf_matrix_new(date: &JulianDate) -> Option<Matrix3> {
    let mut result = Matrix3::default();
    compute_fixed_to_icrf_matrix(date, &mut result)?;
    Some(result)
}

/// Port of `Transforms.computeIcrfToFixedMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from the
/// International Celestial Reference Frame (GCRF/ICRF) inertial frame axes to
/// the Earth-Fixed frame axes (ITRF) at a given time. Returns `None` (the JS
/// `undefined`) if the data necessary to do the transformation is not yet
/// loaded.
pub fn compute_icrf_to_fixed_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> Option<&'a mut Matrix3> {
    if compute_fixed_to_icrf_matrix(date, result).is_none() {
        return None;
    }

    let transposed = Matrix3::transpose_new(result);
    *result = transposed;
    Some(result)
}

/// Allocating variant of [`compute_icrf_to_fixed_matrix`].
pub fn compute_icrf_to_fixed_matrix_new(date: &JulianDate) -> Option<Matrix3> {
    let mut result = Matrix3::default();
    compute_icrf_to_fixed_matrix(date, &mut result)?;
    Some(result)
}

/// Port of `Transforms.computeMoonFixedToIcrfMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from the
/// Moon-Fixed frame axes to the International Celestial Reference Frame
/// (GCRF/ICRF) inertial frame axes at a given time.
pub fn compute_moon_fixed_to_icrf_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> &'a mut Matrix3 {
    // Converts TAI to TT.
    let seconds_tt = JulianDate::add_seconds_new(date, TDT_MINUS_TAI);

    // Converts TT to TDB, interval in days since the standard epoch.
    let d = JulianDate::total_days(&seconds_tt) - J2000_D;

    // Compute the approximate rotation, using
    // https://articles.adsabs.harvard.edu//full/1980CeMec..22..205D/0000209.000.html
    let e1 = CesiumMath::to_radians(12.112) - CesiumMath::to_radians(0.052992) * d;
    let e2 = CesiumMath::to_radians(24.224) - CesiumMath::to_radians(0.105984) * d;
    let e3 = CesiumMath::to_radians(227.645) + CesiumMath::to_radians(13.012) * d;
    let e4 = CesiumMath::to_radians(261.105) + CesiumMath::to_radians(13.340716) * d;
    let e5 = CesiumMath::to_radians(358.0) + CesiumMath::to_radians(0.9856) * d;

    let mut hpr = HeadingPitchRoll::default();
    hpr.pitch = CesiumMath::to_radians(270.0 - 90.0)
        - CesiumMath::to_radians(3.878) * e1.sin()
        - CesiumMath::to_radians(0.12) * e2.sin()
        + CesiumMath::to_radians(0.07) * e3.sin()
        - CesiumMath::to_radians(0.017) * e4.sin();
    hpr.roll = CesiumMath::to_radians(66.53 - 90.0)
        + CesiumMath::to_radians(1.543) * e1.cos()
        + CesiumMath::to_radians(0.24) * e2.cos()
        - CesiumMath::to_radians(0.028) * e3.cos()
        + CesiumMath::to_radians(0.007) * e4.cos();
    hpr.heading = CesiumMath::to_radians(244.375 - 90.0)
        + CesiumMath::to_radians(13.17635831) * d
        + CesiumMath::to_radians(3.558) * e1.sin()
        + CesiumMath::to_radians(0.121) * e2.sin()
        - CesiumMath::to_radians(0.064) * e3.sin()
        + CesiumMath::to_radians(0.016) * e4.sin()
        + CesiumMath::to_radians(0.025) * e5.sin();
    Matrix3::from_heading_pitch_roll(&hpr, result);
    result
}

/// Allocating variant of [`compute_moon_fixed_to_icrf_matrix`].
pub fn compute_moon_fixed_to_icrf_matrix_new(date: &JulianDate) -> Matrix3 {
    let mut result = Matrix3::default();
    compute_moon_fixed_to_icrf_matrix(date, &mut result);
    result
}

/// Port of `Transforms.computeIcrfToMoonFixedMatrix`.
///
/// Computes a rotation matrix to transform a point or vector from the
/// International Celestial Reference Frame (GCRF/ICRF) inertial frame axes to
/// the Moon-Fixed frame axes at a given time.
pub fn compute_icrf_to_moon_fixed_matrix<'a>(
    date: &JulianDate,
    result: &'a mut Matrix3,
) -> &'a mut Matrix3 {
    compute_moon_fixed_to_icrf_matrix(date, result);
    let transposed = Matrix3::transpose_new(result);
    *result = transposed;
    result
}

/// Allocating variant of [`compute_icrf_to_moon_fixed_matrix`].
pub fn compute_icrf_to_moon_fixed_matrix_new(date: &JulianDate) -> Matrix3 {
    let mut result = Matrix3::default();
    compute_icrf_to_moon_fixed_matrix(date, &mut result);
    result
}

/// Port of `Transforms.pointToGLWindowCoordinates` (`@private` in JS).
pub fn point_to_gl_window_coordinates<'a>(
    model_view_projection_matrix: &Matrix4,
    viewport_transformation: &Matrix4,
    point: &Cartesian3,
    result: &'a mut Cartesian2,
) -> &'a mut Cartesian2 {
    let tmp = Cartesian4::from_elements_new(point.x, point.y, point.z, 1.0);
    let tmp = Matrix4::multiply_by_vector_new(model_view_projection_matrix, &tmp);
    let tmp = Cartesian4::multiply_by_scalar_new(&tmp, 1.0 / tmp.w);
    let tmp = Matrix4::multiply_by_vector_new(viewport_transformation, &tmp);
    Cartesian2::from_cartesian4(&tmp, result);
    result
}

/// Allocating variant of [`point_to_gl_window_coordinates`].
pub fn point_to_gl_window_coordinates_new(
    model_view_projection_matrix: &Matrix4,
    viewport_transformation: &Matrix4,
    point: &Cartesian3,
) -> Cartesian2 {
    let mut result = Cartesian2::default();
    point_to_gl_window_coordinates(
        model_view_projection_matrix,
        viewport_transformation,
        point,
        &mut result,
    );
    result
}

/// Port of `Transforms.pointToWindowCoordinates`.
///
/// Transform a point from model coordinates to window coordinates.
pub fn point_to_window_coordinates<'a>(
    model_view_projection_matrix: &Matrix4,
    viewport_transformation: &Matrix4,
    point: &Cartesian3,
    result: &'a mut Cartesian2,
) -> &'a mut Cartesian2 {
    point_to_gl_window_coordinates(
        model_view_projection_matrix,
        viewport_transformation,
        point,
        result,
    );
    result.y = 2.0 * viewport_transformation.elements[5] - result.y;
    result
}

/// Allocating variant of [`point_to_window_coordinates`].
pub fn point_to_window_coordinates_new(
    model_view_projection_matrix: &Matrix4,
    viewport_transformation: &Matrix4,
    point: &Cartesian3,
) -> Cartesian2 {
    let mut result = Cartesian2::default();
    point_to_window_coordinates(
        model_view_projection_matrix,
        viewport_transformation,
        point,
        &mut result,
    );
    result
}

/// Port of `Transforms.SWIZZLE_3D_TO_2D_MATRIX` (`@private` in JS): an
/// immutable matrix that swaps x, y, z for 2D.
pub const SWIZZLE_3D_TO_2D_MATRIX: Matrix4 = Matrix4 {
    elements: [
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ],
};

/// Port of `Transforms.basisTo2D` (`@private` in JS).
pub fn basis_to_2d(
    projection: &dyn MapProjection,
    matrix: &Matrix4,
    result: &mut Matrix4,
) {
    let rtc_center = Matrix4::get_translation_new(matrix);
    let ellipsoid = projection.ellipsoid();

    let projected_position;
    if Cartesian3::equals(Some(&rtc_center), Some(&Cartesian3::ZERO)) {
        projected_position = Cartesian3::ZERO;
    } else {
        // Get the 2D center.
        let mut cartographic = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(&rtc_center, &mut cartographic);

        let mut pp = projection.project(&cartographic);
        Cartesian3::from_elements(pp.z, pp.x, pp.y, &mut pp);
        projected_position = pp;
    }

    // Assuming the instances are positioned on the ellipsoid, invert the
    // ellipsoidal transform to get the local transform and then convert to 2D.
    let mut from_enu = Matrix4::default();
    east_north_up_to_fixed_frame(&rtc_center, Some(ellipsoid), &mut from_enu);
    let mut to_enu = Matrix4::default();
    Matrix4::inverse_transformation(&from_enu, &mut to_enu);
    let rotation = Matrix4::get_matrix3_new(matrix);
    Matrix4::multiply_by_matrix3(&to_enu, &rotation, result);
    // Swap x, y, z for 2D.
    let swizzled = Matrix4::multiply_new(&SWIZZLE_3D_TO_2D_MATRIX, result);
    *result = swizzled;
    // Use the projected center. (JS: `Matrix4.setTranslation(result, projectedPosition, result)`;
    // the local copy avoids Rust's aliasing restriction on &self/&mut self.)
    let result_copy = *result;
    Matrix4::set_translation(&result_copy, &projected_position, result);
}

/// Port of `Transforms.ellipsoidTo2DModelMatrix` (`@private` in JS).
pub fn ellipsoid_to_2d_model_matrix(
    projection: &dyn MapProjection,
    center: &Cartesian3,
    result: &mut Matrix4,
) {
    let ellipsoid = projection.ellipsoid();

    let mut from_enu = Matrix4::default();
    east_north_up_to_fixed_frame(center, Some(ellipsoid), &mut from_enu);
    let mut to_enu = Matrix4::default();
    Matrix4::inverse_transformation(&from_enu, &mut to_enu);

    let mut cartographic = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(center, &mut cartographic);
    let mut projected_position = projection.project(&cartographic);
    Cartesian3::from_elements(
        projected_position.z,
        projected_position.x,
        projected_position.y,
        &mut projected_position,
    );

    let translation = Matrix4::from_translation_new(&projected_position);
    Matrix4::multiply(&SWIZZLE_3D_TO_2D_MATRIX, &to_enu, result);
    let multiplied = Matrix4::multiply_new(&translation, result);
    *result = multiplied;
}
