//! Ported from `packages/engine/Source/Core/Simon1994PlanetaryPositions.js`.
//!
//! Contains functions for finding the Cartesian coordinates of the Sun and the Moon
//! in the Earth-centered inertial frame using Simon et al. 1994 theory.

use crate::cartesian3::Cartesian3;
use crate::julian_date::JulianDate;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::time_constants::{DAYS_PER_JULIAN_CENTURY, SECONDS_PER_DAY};

const TDT_MINUS_TAI: f64 = 32.184;
const J2000D: f64 = 2451545.0;
const METERS_PER_KILOMETER: f64 = 1000.0;
const METERS_PER_AU: f64 = 1.4959787e11;

const MOON_EARTH_MASS_RATIO: f64 = 0.012300034;
const EARTH_FACTOR: f64 =
    (MOON_EARTH_MASS_RATIO / (MOON_EARTH_MASS_RATIO + 1.0)) * -1.0;

// Axes transformation matrix values
const AXES_TRANSFORMATION: [f64; 9] = [
    1.0000000000000002,
    5.619723173785822e-16,
    4.690511510146299e-19,
    -5.154129427414611e-16,
    0.9174820620691819,
    -0.39777715593191376,
    -2.23970096136568e-16,
    0.39777715593191376,
    0.9174820620691819,
];

fn compute_tdb_minus_tt_spice(days_since_j2000: f64) -> f64 {
    let g = 6.239996 + 0.0172019696544 * days_since_j2000;
    1.657e-3 * (g + 1.671e-2 * g.sin()).sin()
}

fn tai_to_tdb(date: &JulianDate) -> JulianDate {
    let mut result = JulianDate::add_seconds_new(date, TDT_MINUS_TAI);
    let days = JulianDate::total_days(&result) - J2000D;
    result = JulianDate::add_seconds_new(&result, compute_tdb_minus_tt_spice(days));
    result
}

fn mean_anomaly_to_eccentric_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let revs = (mean_anomaly / CesiumMath::TWO_PI).floor();
    let ma = mean_anomaly - revs * CesiumMath::TWO_PI;

    let mut iteration_value = ma
        + (eccentricity * ma.sin())
            / (1.0 - (ma + eccentricity).sin() + ma.sin());

    let mut eccentric_anomaly = f64::MAX;
    for _ in 0..50 {
        if (eccentric_anomaly - iteration_value).abs() <= CesiumMath::EPSILON8 {
            break;
        }
        eccentric_anomaly = iteration_value;
        let nr = eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - ma;
        let dnr = 1.0 - eccentricity * eccentric_anomaly.cos();
        iteration_value = eccentric_anomaly - nr / dnr;
    }

    iteration_value + revs * CesiumMath::TWO_PI
}

fn eccentric_anomaly_to_true_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let revs = (eccentric_anomaly / CesiumMath::TWO_PI).floor();
    let ea = eccentric_anomaly - revs * CesiumMath::TWO_PI;

    let ta_x = ea.cos() - eccentricity;
    let ta_y = ea.sin() * (1.0 - eccentricity * eccentricity).sqrt();
    let mut ta = ta_y.atan2(ta_x);
    ta = CesiumMath::zero_to_two_pi(ta);
    if ea < 0.0 {
        ta -= CesiumMath::TWO_PI;
    }
    ta + revs * CesiumMath::TWO_PI
}

fn mean_anomaly_to_true_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let ea = mean_anomaly_to_eccentric_anomaly(mean_anomaly, eccentricity);
    eccentric_anomaly_to_true_anomaly(ea, eccentricity)
}

fn perifocal_to_cartesian_matrix(
    aop: f64,
    inclination: f64,
    raan: f64,
    result: &mut Matrix3,
) {
    let cosap = aop.cos();
    let sinap = aop.sin();
    let cosi = inclination.cos();
    let sini = inclination.sin();
    let cosr = raan.cos();
    let sinr = raan.sin();

    let data = &mut result.elements;
    data[0] = cosr * cosap - sinr * sinap * cosi;
    data[1] = sinr * cosap + cosr * sinap * cosi;
    data[2] = sinap * sini;
    data[3] = -cosr * sinap - sinr * cosap * cosi;
    data[4] = -sinr * sinap + cosr * cosap * cosi;
    data[5] = cosap * sini;
    data[6] = sinr * sini;
    data[7] = -cosr * sini;
    data[8] = cosi;
}

fn elements_to_cartesian(
    semimajor_axis: f64,
    eccentricity: f64,
    inclination: f64,
    longitude_of_perigee: f64,
    longitude_of_node: f64,
    mean_longitude: f64,
    result: &mut Cartesian3,
) {
    let mut inc = inclination;
    let mut lon_node = longitude_of_node;
    if inc < 0.0 {
        inc = -inc;
        lon_node += std::f64::consts::PI;
    }

    let aop = longitude_of_perigee - lon_node;
    let true_anomaly = mean_anomaly_to_true_anomaly(
        mean_longitude - longitude_of_perigee,
        eccentricity,
    );

    let mut p2c = Matrix3::default();
    perifocal_to_cartesian_matrix(aop, inc, lon_node, &mut p2c);

    let semilatus = semimajor_axis * (1.0 - eccentricity) * (1.0 + eccentricity);
    let costheta = true_anomaly.cos();
    let sintheta = true_anomaly.sin();
    let denom = 1.0 + eccentricity * costheta;
    let radius = semilatus / denom;

    result.x = radius * costheta;
    result.y = radius * sintheta;
    result.z = 0.0;

    let temp = *result;
    Matrix3::multiply_by_vector(&p2c, &temp, result);
}

fn compute_simon_earth_moon_barycenter(date: &JulianDate, result: &mut Cartesian3) {
    let tdb = tai_to_tdb(date);
    let epoch_day = J2000D;
    let x = tdb.day_number as f64 - epoch_day
        + (tdb.seconds_of_day - 0.0) / SECONDS_PER_DAY;
    let t = x / (DAYS_PER_JULIAN_CENTURY * 10.0);
    let u = 0.3595362 * t;

    let rpd = CesiumMath::RADIANS_PER_DEGREE;
    let rpa = CesiumMath::RADIANS_PER_ARCSECOND;

    let semi_major_axis = METERS_PER_AU * 1.0000010178
        + 64e-7 * METERS_PER_AU * (16002.0_f64 * u).cos()
        + -150e-7 * METERS_PER_AU * (16002.0_f64 * u).sin()
        + -152e-7 * METERS_PER_AU * (21863.0_f64 * u).cos()
        + -46e-7 * METERS_PER_AU * (21863.0_f64 * u).sin()
        + 62e-7 * METERS_PER_AU * (32004.0_f64 * u).cos()
        + 68e-7 * METERS_PER_AU * (32004.0_f64 * u).sin()
        + -8e-7 * METERS_PER_AU * (10931.0_f64 * u).cos()
        + 54e-7 * METERS_PER_AU * (10931.0_f64 * u).sin()
        + 32e-7 * METERS_PER_AU * (14529.0_f64 * u).cos()
        + 14e-7 * METERS_PER_AU * (14529.0_f64 * u).sin()
        + -41e-7 * METERS_PER_AU * (16368.0_f64 * u).cos()
        + 24e-7 * METERS_PER_AU * (16368.0_f64 * u).sin()
        + 19e-7 * METERS_PER_AU * (15318.0_f64 * u).cos()
        + -28e-7 * METERS_PER_AU * (15318.0_f64 * u).sin()
        + -11e-7 * METERS_PER_AU * (32794.0_f64 * u).cos()
        + 22e-7 * METERS_PER_AU * (32794.0_f64 * u).sin();

    let mean_longitude = 100.46645683 * rpd
        + 1295977422.83429 * rpa * t
        + -325e-7 * (10.0_f64 * u).cos()
        + -105e-7 * (10.0_f64 * u).sin()
        + -322e-7 * (16002.0_f64 * u).cos()
        + -137e-7 * (16002.0_f64 * u).sin()
        + -79e-7 * (21863.0_f64 * u).cos()
        + 258e-7 * (21863.0_f64 * u).sin()
        + 232e-7 * (10931.0_f64 * u).cos()
        + 35e-7 * (10931.0_f64 * u).sin()
        + -52e-7 * (1473.0_f64 * u).cos()
        + -116e-7 * (1473.0_f64 * u).sin()
        + 97e-7 * (32004.0_f64 * u).cos()
        + -88e-7 * (32004.0_f64 * u).sin()
        + 55e-7 * (4387.0_f64 * u).cos()
        + -112e-7 * (4387.0_f64 * u).sin()
        + -41e-7 * (73.0_f64 * u).cos()
        + -80e-7 * (73.0_f64 * u).sin();

    let eccentricity = 0.0167086342 - 0.0004203654 * t;
    let lon_perigee = 102.93734808 * rpd + 11612.3529 * rpa * t;
    let inclination = 469.97289 * rpa * t;
    let lon_node = 174.87317577 * rpd - 8679.27034 * rpa * t;

    elements_to_cartesian(
        semi_major_axis,
        eccentricity,
        inclination,
        lon_perigee,
        lon_node,
        mean_longitude,
        result,
    );
}

fn compute_simon_moon(date: &JulianDate, result: &mut Cartesian3) {
    let tdb = tai_to_tdb(date);
    let epoch_day = J2000D;
    let x = tdb.day_number as f64 - epoch_day
        + (tdb.seconds_of_day - 0.0) / SECONDS_PER_DAY;
    let t = x / DAYS_PER_JULIAN_CENTURY;
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;

    let rpd = CesiumMath::RADIANS_PER_DEGREE;
    let rpa = CesiumMath::RADIANS_PER_ARCSECOND;

    let mut semi_major_axis = (383397.7725 + 0.004 * t) * METERS_PER_KILOMETER;
    let mut eccentricity = 0.055545526 - 0.000000016 * t;
    let inc_constant = 5.15668983 * rpd;
    let mut inc_sec = -0.00008 * t + 0.02966 * t2 - 0.000042 * t3 - 0.00000013 * t4;
    let lop_constant = 83.35324312 * rpd;
    let mut lop_sec = 14643420.2669 * t - 38.2702 * t2 - 0.045047 * t3 + 0.00021301 * t4;
    let lon_constant = 125.04455501 * rpd;
    let mut lon_sec = -6967919.3631 * t + 6.3602 * t2 + 0.007625 * t3 - 0.00003586 * t4;
    let ml_constant = 218.31664563 * rpd;
    let mut ml_sec = 1732559343.4847 * t - 6.391 * t2 + 0.006588 * t3 - 0.00003169 * t4;

    // Delaunay arguments
    let d_ang = 297.85019547 * rpd
        + rpa * (1602961601.209 * t - 6.3706 * t2 + 0.006593 * t3 - 0.00003169 * t4);
    let f_ang = 93.27209062 * rpd
        + rpa * (1739527262.8478 * t - 12.7512 * t2 - 0.001037 * t3 + 0.00000417 * t4);
    let l = 134.96340251 * rpd
        + rpa * (1717915923.2178 * t + 31.8792 * t2 + 0.051635 * t3 - 0.0002447 * t4);
    let _lprime = 357.52910918 * rpd
        + rpa * (129596581.0481 * t - 0.5532 * t2 + 0.000136 * t3 - 0.00001149 * t4);
    let psi = 310.17137918 * rpd
        - rpa * (6967051.436 * t + 6.2068 * t2 + 0.007618 * t3 - 0.00003219 * t4);

    let two_d = 2.0 * d_ang;
    let four_d = 4.0 * d_ang;
    let six_d = 6.0 * d_ang;
    let two_l = 2.0 * l;
    let three_l = 3.0 * l;
    let four_l = 4.0 * l;
    let two_f = 2.0 * f_ang;

    semi_major_axis += 3400.4 * (two_d).cos() * METERS_PER_KILOMETER
        - 635.6 * (two_d - l).cos() * METERS_PER_KILOMETER
        - 235.6 * l.cos() * METERS_PER_KILOMETER
        + 218.1 * (two_d - l).cos() * METERS_PER_KILOMETER
        + 181.0 * (two_d + l).cos() * METERS_PER_KILOMETER;

    eccentricity += 0.014216 * (two_d - l).cos()
        + 0.008551 * (two_d - two_l).cos()
        - 0.001383 * l.cos()
        + 0.001356 * (two_d + l).cos()
        - 0.001147 * (four_d - three_l).cos()
        - 0.000914 * (four_d - two_l).cos()
        + 0.000869 * (two_d - l).cos()
        - 0.000627 * two_d.cos()
        - 0.000394 * (four_d - four_l).cos()
        + 0.000282 * (two_d - two_l).cos()
        - 0.000279 * (d_ang - l).cos()
        - 0.000236 * two_l.cos()
        + 0.000231 * four_d.cos()
        + 0.000229 * (six_d - four_l).cos()
        - 0.000201 * (two_l - two_f).cos();

    inc_sec += 486.26 * (two_d - two_f).cos()
        - 40.13 * two_d.cos()
        + 37.51 * two_f.cos()
        + 25.73 * (two_l - two_f).cos()
        + 19.97 * (two_d - two_f).cos();

    lop_sec += -55609.0 * (two_d - l).sin()
        - 34711.0 * (two_d - two_l).sin()
        - 9792.0 * l.sin()
        + 9385.0 * (four_d - three_l).sin()
        + 7505.0 * (four_d - two_l).sin()
        + 5318.0 * (two_d + l).sin()
        + 3484.0 * (four_d - four_l).sin()
        - 3417.0 * (two_d - l).sin()
        - 2530.0 * (six_d - four_l).sin()
        - 2376.0 * two_d.sin()
        - 2075.0 * (two_d - three_l).sin()
        - 1883.0 * two_l.sin()
        - 1736.0 * (six_d - 5.0 * l).sin()
        + 1626.0 * _lprime.sin()
        - 1370.0 * (six_d - three_l).sin();

    lon_sec += -5392.0 * (two_d - two_f).sin()
        - 441.0 * two_d.sin()
        + 423.0 * two_f.sin()
        - 288.0 * (two_l - two_f).sin();

    ml_sec += -3332.9 * two_d.sin()
        + 1197.4 * (two_d - l).sin()
        - 662.5 * _lprime.sin()
        + 396.3 * l.sin()
        - 218.0 * (two_d - _lprime).sin();

    // Table 5 terms
    let two_psi = 2.0 * psi;
    let three_psi = 3.0 * psi;
    inc_sec += 46.997 * psi.cos() * t
        - 0.614 * (two_d - two_f + psi).cos() * t
        + 0.614 * (two_d - two_f - psi).cos() * t
        - 0.0297 * two_psi.cos() * t2
        - 0.0335 * psi.cos() * t2
        + 0.0012 * (two_d - two_f + two_psi).cos() * t2
        - 0.00016 * psi.cos() * t3
        + 0.00004 * three_psi.cos() * t3
        + 0.00004 * two_psi.cos() * t3;

    let perigee_and_mean = 2.116 * psi.sin() * t
        - 0.111 * (two_d - two_f - psi).sin() * t
        - 0.0015 * psi.sin() * t2;
    lop_sec += perigee_and_mean;
    ml_sec += perigee_and_mean;
    lon_sec += -520.77 * psi.sin() * t
        + 13.66 * (two_d - two_f + psi).sin() * t
        + 1.12 * (two_d - psi).sin() * t
        - 1.06 * (two_f - psi).sin() * t
        + 0.66 * two_psi.sin() * t2
        + 0.371 * psi.sin() * t2
        - 0.035 * (two_d - two_f + two_psi).sin() * t2
        - 0.015 * (two_d - two_f + psi).sin() * t2
        + 0.0014 * psi.sin() * t3
        - 0.0011 * three_psi.sin() * t3
        - 0.0009 * two_psi.sin() * t3;

    let inclination = inc_constant + inc_sec * rpa;
    let lon_perigee = lop_constant + lop_sec * rpa;
    let mean_longitude = ml_constant + ml_sec * rpa;
    let lon_node = lon_constant + lon_sec * rpa;

    elements_to_cartesian(
        semi_major_axis,
        eccentricity,
        inclination,
        lon_perigee,
        lon_node,
        mean_longitude,
        result,
    );
}

/// Computes the position of the Sun in the Earth-centered inertial frame.
pub fn compute_sun_position_in_earth_inertial_frame(
    julian_date: &JulianDate,
    result: &mut Cartesian3,
) {
    let mut translation = Cartesian3::default();
    compute_simon_earth_moon_barycenter(julian_date, &mut translation);
    Cartesian3::negate(&translation, result);

    compute_simon_moon(julian_date, &mut translation);
    let earth_pos = Cartesian3::multiply_by_scalar_new(&translation, EARTH_FACTOR);

    let temp = Cartesian3::subtract_new(result, &earth_pos);
    *result = temp;

    let axes_mtx = Matrix3::from_array_new(&AXES_TRANSFORMATION, 0);
    let temp2 = Matrix3::multiply_by_vector_new(&axes_mtx, result);
    *result = temp2;
}

/// Computes the position of the Moon in the Earth-centered inertial frame.
pub fn compute_moon_position_in_earth_inertial_frame(
    julian_date: &JulianDate,
    result: &mut Cartesian3,
) {
    compute_simon_moon(julian_date, result);
    let axes_mtx = Matrix3::from_array_new(&AXES_TRANSFORMATION, 0);
    let temp = Matrix3::multiply_by_vector_new(&axes_mtx, result);
    *result = temp;
}
