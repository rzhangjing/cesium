//! EllipsoidRhumbLine - a rhumb line (loxodrome) on an ellipsoid.
//! Faithful port of CesiumJS `Source/Core/EllipsoidRhumbLine.js`

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils::{equals_epsilon, negative_pi_to_pi, sign, EPSILON10, EPSILON12, EPSILON14, EPSILON8, PI_OVER_TWO};

fn calculate_m(ellipticity: f64, major: f64, latitude: f64) -> f64 {
    if ellipticity == 0.0 {
        return major * latitude;
    }

    let e2 = ellipticity * ellipticity;
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let e8 = e6 * e2;
    let e10 = e8 * e2;
    let e12 = e10 * e2;
    let phi = latitude;
    let sin2_phi = (2.0 * phi).sin();
    let sin4_phi = (4.0 * phi).sin();
    let sin6_phi = (6.0 * phi).sin();
    let sin8_phi = (8.0 * phi).sin();
    let sin10_phi = (10.0 * phi).sin();
    let sin12_phi = (12.0 * phi).sin();

    major
        * ((1.0 - e2 / 4.0 - (3.0 * e4) / 64.0 - (5.0 * e6) / 256.0
            - (175.0 * e8) / 16384.0
            - (441.0 * e10) / 65536.0
            - (4851.0 * e12) / 1048576.0)
            * phi
            - ((3.0 * e2) / 8.0
                + (3.0 * e4) / 32.0
                + (45.0 * e6) / 1024.0
                + (105.0 * e8) / 4096.0
                + (2205.0 * e10) / 131072.0
                + (6237.0 * e12) / 524288.0)
                * sin2_phi
            + ((15.0 * e4) / 256.0
                + (45.0 * e6) / 1024.0
                + (525.0 * e8) / 16384.0
                + (1575.0 * e10) / 65536.0
                + (155925.0 * e12) / 8388608.0)
                * sin4_phi
            - ((35.0 * e6) / 3072.0
                + (175.0 * e8) / 12288.0
                + (3675.0 * e10) / 262144.0
                + (13475.0 * e12) / 1048576.0)
                * sin6_phi
            + ((315.0 * e8) / 131072.0
                + (2205.0 * e10) / 524288.0
                + (43659.0 * e12) / 8388608.0)
                * sin8_phi
            - ((693.0 * e10) / 1310720.0 + (6237.0 * e12) / 5242880.0) * sin10_phi
            + ((1001.0 * e12) / 8388608.0) * sin12_phi)
}

fn calculate_inverse_m(m: f64, ellipticity: f64, major: f64) -> f64 {
    let d = m / major;

    if ellipticity == 0.0 {
        return d;
    }

    let d2 = d * d;
    let d3 = d2 * d;
    let d4 = d3 * d;
    let e = ellipticity;
    let e2 = e * e;
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let e8 = e6 * e2;
    let e10 = e8 * e2;
    let e12 = e10 * e2;
    let sin2_d = (2.0 * d).sin();
    let cos2_d = (2.0 * d).cos();
    let sin4_d = (4.0 * d).sin();
    let cos4_d = (4.0 * d).cos();
    let sin6_d = (6.0 * d).sin();
    let cos6_d = (6.0 * d).cos();
    let sin8_d = (8.0 * d).sin();
    let cos8_d = (8.0 * d).cos();
    let sin10_d = (10.0 * d).sin();
    let cos10_d = (10.0 * d).cos();
    let sin12_d = (12.0 * d).sin();

    d + (d * e2) / 4.0
        + (7.0 * d * e4) / 64.0
        + (15.0 * d * e6) / 256.0
        + (579.0 * d * e8) / 16384.0
        + (1515.0 * d * e10) / 65536.0
        + (16837.0 * d * e12) / 1048576.0
        + ((3.0 * d * e4) / 16.0 + (45.0 * d * e6) / 256.0
            - (d * (32.0 * d2 - 561.0) * e8) / 4096.0
            - (d * (232.0 * d2 - 1677.0) * e10) / 16384.0
            + (d * (399985.0 - 90560.0 * d2 + 512.0 * d4) * e12) / 5242880.0)
            * cos2_d
        + ((21.0 * d * e6) / 256.0 + (483.0 * d * e8) / 4096.0
            - (d * (224.0 * d2 - 1969.0) * e10) / 16384.0
            - (d * (33152.0 * d2 - 112599.0) * e12) / 1048576.0)
            * cos4_d
        + ((151.0 * d * e8) / 4096.0
            + (4681.0 * d * e10) / 65536.0
            + (1479.0 * d * e12) / 16384.0
            - (453.0 * d3 * e12) / 32768.0)
            * cos6_d
        + ((1097.0 * d * e10) / 65536.0 + (42783.0 * d * e12) / 1048576.0) * cos8_d
        + ((8011.0 * d * e12) / 1048576.0) * cos10_d
        + ((3.0 * e2) / 8.0
            + (3.0 * e4) / 16.0
            + (213.0 * e6) / 2048.0
            - (3.0 * d2 * e6) / 64.0
            + (255.0 * e8) / 4096.0
            - (33.0 * d2 * e8) / 512.0
            + (20861.0 * e10) / 524288.0
            - (33.0 * d2 * e10) / 512.0
            + (d4 * e10) / 1024.0
            + (28273.0 * e12) / 1048576.0
            - (471.0 * d2 * e12) / 8192.0
            + (9.0 * d4 * e12) / 4096.0)
            * sin2_d
        + ((21.0 * e4) / 256.0
            + (21.0 * e6) / 256.0
            + (533.0 * e8) / 8192.0
            - (21.0 * d2 * e8) / 512.0
            + (197.0 * e10) / 4096.0
            - (315.0 * d2 * e10) / 4096.0
            + (584039.0 * e12) / 16777216.0
            - (12517.0 * d2 * e12) / 131072.0
            + (7.0 * d4 * e12) / 2048.0)
            * sin4_d
        + ((151.0 * e6) / 6144.0
            + (151.0 * e8) / 4096.0
            + (5019.0 * e10) / 131072.0
            - (453.0 * d2 * e10) / 16384.0
            + (26965.0 * e12) / 786432.0
            - (8607.0 * d2 * e12) / 131072.0)
            * sin6_d
        + ((1097.0 * e8) / 131072.0
            + (1097.0 * e10) / 65536.0
            + (225797.0 * e12) / 10485760.0
            - (1097.0 * d2 * e12) / 65536.0)
            * sin8_d
        + ((8011.0 * e10) / 2621440.0 + (8011.0 * e12) / 1048576.0) * sin10_d
        + ((293393.0 * e12) / 251658240.0) * sin12_d
}

fn calculate_sigma(ellipticity: f64, latitude: f64) -> f64 {
    if ellipticity == 0.0 {
        return (0.5 * (PI_OVER_TWO + latitude)).tan().ln();
    }

    let e_sin_l = ellipticity * latitude.sin();
    (0.5 * (PI_OVER_TWO + latitude)).tan().ln()
        - (ellipticity / 2.0) * ((1.0 + e_sin_l) / (1.0 - e_sin_l)).ln()
}

fn calculate_heading(
    ellipticity: f64,
    first_longitude: f64,
    first_latitude: f64,
    second_longitude: f64,
    second_latitude: f64,
) -> f64 {
    let sigma1 = calculate_sigma(ellipticity, first_latitude);
    let sigma2 = calculate_sigma(ellipticity, second_latitude);
    (negative_pi_to_pi(second_longitude - first_longitude)).atan2(sigma2 - sigma1)
}

fn calculate_arc_length(
    ellipticity: f64,
    ellipticity_squared: f64,
    major: f64,
    minor: f64,
    heading: f64,
    first_latitude: f64,
    second_latitude: f64,
    delta_longitude: f64,
) -> f64 {
    let distance;

    // Check to see if the rhumb line has constant latitude
    if equals_epsilon(heading.abs(), PI_OVER_TWO, EPSILON8, EPSILON8) {
        // If heading is close to 90 degrees
        if major == minor {
            distance = major * first_latitude.cos() * negative_pi_to_pi(delta_longitude);
        } else {
            let sin_phi = first_latitude.sin();
            distance = (major * first_latitude.cos() * negative_pi_to_pi(delta_longitude))
                / (1.0 - ellipticity_squared * sin_phi * sin_phi).sqrt();
        }
    } else {
        let m1 = calculate_m(ellipticity, major, first_latitude);
        let m2 = calculate_m(ellipticity, major, second_latitude);
        distance = (m2 - m1) / heading.cos();
    }
    distance.abs()
}

fn interpolate_using_surface_distance(
    start: &Cartographic,
    heading: f64,
    distance: f64,
    major: f64,
    ellipticity: f64,
) -> Cartographic {
    if distance == 0.0 {
        return *start;
    }

    let ellipticity_squared = ellipticity * ellipticity;

    let longitude;
    let latitude;

    // Check to see if the rhumb line has constant latitude
    if (PI_OVER_TWO - heading.abs()).abs() > EPSILON8 {
        // Calculate latitude of the second point
        let m1 = calculate_m(ellipticity, major, start.latitude);
        let delta_m = distance * heading.cos();
        let m2 = m1 + delta_m;
        latitude = calculate_inverse_m(m2, ellipticity, major);

        // Now find the longitude of the second point
        if heading.abs() < EPSILON10 {
            longitude = negative_pi_to_pi(start.longitude);
        } else {
            let sigma1 = calculate_sigma(ellipticity, start.latitude);
            let sigma2 = calculate_sigma(ellipticity, latitude);
            let delta_longitude = heading.tan() * (sigma2 - sigma1);
            longitude = negative_pi_to_pi(start.longitude + delta_longitude);
        }
    } else {
        // If heading is close to 90 degrees
        latitude = start.latitude;
        let local_rad;

        if ellipticity == 0.0 {
            local_rad = major * start.latitude.cos();
        } else {
            let sin_phi = start.latitude.sin();
            local_rad =
                (major * start.latitude.cos()) / (1.0 - ellipticity_squared * sin_phi * sin_phi).sqrt();
        }

        let delta_longitude = distance / local_rad;
        if heading > 0.0 {
            longitude = negative_pi_to_pi(start.longitude + delta_longitude);
        } else {
            longitude = negative_pi_to_pi(start.longitude - delta_longitude);
        }
    }

    Cartographic::from_radians(longitude, latitude, 0.0)
}

/// A rhumb line (loxodrome) on an ellipsoid.
/// Maps to CesiumJS `EllipsoidRhumbLine`
#[derive(Clone, Debug)]
pub struct EllipsoidRhumbLine {
    start: Cartographic,
    end: Cartographic,
    heading: f64,
    distance: f64,
    ellipticity: f64,
    ellipticity_squared: f64,
    major: f64,
    #[allow(dead_code)]
    minor: f64,
}

impl EllipsoidRhumbLine {
    /// Creates a new rhumb line from start and end cartographic points on the given ellipsoid.
    pub fn new(start: &Cartographic, end: &Cartographic, ellipsoid: &Ellipsoid) -> Self {
        let major = ellipsoid.maximum_radius();
        let minor = ellipsoid.minimum_radius();
        let major_squared = major * major;
        let minor_squared = minor * minor;
        let ellipticity_squared = (major_squared - minor_squared) / major_squared;
        let ellipticity = ellipticity_squared.sqrt();

        let heading = calculate_heading(
            ellipticity,
            start.longitude,
            start.latitude,
            end.longitude,
            end.latitude,
        );

        let delta_longitude = end.longitude - start.longitude;
        let distance = calculate_arc_length(
            ellipticity,
            ellipticity_squared,
            major,
            minor,
            heading,
            start.latitude,
            end.latitude,
            delta_longitude,
        );

        let mut s = *start;
        s.height = 0.0;
        let mut e = *end;
        e.height = 0.0;

        Self {
            start: s,
            end: e,
            heading,
            distance,
            ellipticity,
            ellipticity_squared,
            major,
            minor,
        }
    }

    /// Sets new endpoints and recomputes properties.
    pub fn set_end_points(&mut self, start: &Cartographic, end: &Cartographic) {
        let heading = calculate_heading(
            self.ellipticity,
            start.longitude,
            start.latitude,
            end.longitude,
            end.latitude,
        );

        let delta_longitude = end.longitude - start.longitude;
        let distance = calculate_arc_length(
            self.ellipticity,
            self.ellipticity_squared,
            self.major,
            self.minor,
            heading,
            start.latitude,
            end.latitude,
            delta_longitude,
        );

        let mut s = *start;
        s.height = 0.0;
        let mut e = *end;
        e.height = 0.0;

        self.start = s;
        self.end = e;
        self.heading = heading;
        self.distance = distance;
    }

    /// Gets the surface distance between start and end.
    pub fn surface_distance(&self) -> f64 {
        self.distance
    }

    /// Gets the heading of the rhumb line.
    pub fn heading(&self) -> f64 {
        self.heading
    }

    /// Gets the start point.
    pub fn start(&self) -> Cartographic {
        self.start
    }

    /// Gets the end point.
    pub fn end(&self) -> Cartographic {
        self.end
    }

    /// Creates a rhumb line from a start point, heading, and distance.
    ///
    /// Maps to `EllipsoidRhumbLine.fromStartHeadingDistance`.
    pub fn from_start_heading_distance(
        start: &Cartographic,
        heading: f64,
        distance: f64,
        ellipsoid: &Ellipsoid,
    ) -> Self {
        let major = ellipsoid.maximum_radius();
        let minor = ellipsoid.minimum_radius();
        let major_squared = major * major;
        let minor_squared = minor * minor;
        let ellipticity_squared = (major_squared - minor_squared) / major_squared;
        let ellipticity = ellipticity_squared.sqrt();

        let heading = negative_pi_to_pi(heading);
        let end = interpolate_using_surface_distance(start, heading, distance, major, ellipticity);

        Self::new(start, &end, ellipsoid)
    }

    /// Finds the intersection of the rhumb line with the given longitude.
    ///
    /// Returns `None` for N-S lines where the longitude doesn't match.
    /// Maps to `EllipsoidRhumbLine.prototype.findIntersectionWithLongitude`.
    pub fn find_intersection_with_longitude(&self, intersection_longitude: f64) -> Option<Cartographic> {
        let ellipticity = self.ellipticity;
        let heading = self.heading;
        let abs_heading = heading.abs();
        let start = &self.start;

        let mut intersection_longitude = negative_pi_to_pi(intersection_longitude);

        if equals_epsilon(intersection_longitude.abs(), std::f64::consts::PI, EPSILON14, EPSILON14) {
            intersection_longitude = sign(start.longitude) * std::f64::consts::PI;
        }

        // E-W rhumb line (heading ~ ±PI/2)
        if (PI_OVER_TWO - abs_heading).abs() <= EPSILON8 {
            return Some(Cartographic::from_radians(intersection_longitude, start.latitude, 0.0));
        }

        // N-S rhumb line (heading ~ 0 or PI)
        if equals_epsilon((PI_OVER_TWO - abs_heading).abs(), PI_OVER_TWO, EPSILON8, EPSILON8) {
            if equals_epsilon(intersection_longitude, start.longitude, EPSILON12, EPSILON12) {
                return None;
            }
            let latitude = PI_OVER_TWO * sign(PI_OVER_TWO - heading);
            return Some(Cartographic::from_radians(intersection_longitude, latitude, 0.0));
        }

        // Iterative solver from Equation 9 from http://edwilliams.org/ellipsoid/ellipsoid.pdf
        let phi1 = start.latitude;
        let e_sin_phi1 = ellipticity * phi1.sin();
        let left_component = (0.5 * (PI_OVER_TWO + phi1)).tan()
            * ((intersection_longitude - start.longitude) / heading.tan()).exp();
        let denominator = (1.0 + e_sin_phi1) / (1.0 - e_sin_phi1);

        let mut new_phi = start.latitude;
        let new_phi_result;
        loop {
            let phi = new_phi;
            let e_sin_phi = ellipticity * phi.sin();
            let numerator = (1.0 + e_sin_phi) / (1.0 - e_sin_phi);
            new_phi = 2.0
                * (left_component * (numerator / denominator).powf(ellipticity / 2.0)).atan()
                - PI_OVER_TWO;
            if equals_epsilon(new_phi, phi, EPSILON12, EPSILON12) {
                new_phi_result = new_phi;
                break;
            }
        }

        Some(Cartographic::from_radians(intersection_longitude, new_phi_result, 0.0))
    }

    /// Finds the intersection of the rhumb line with the given latitude.
    ///
    /// Returns `None` for E-W lines (constant latitude).
    /// Maps to `EllipsoidRhumbLine.prototype.findIntersectionWithLatitude`.
    pub fn find_intersection_with_latitude(&self, intersection_latitude: f64) -> Option<Cartographic> {
        let ellipticity = self.ellipticity;
        let heading = self.heading;
        let start = &self.start;

        // E-W rhumb line: no intersection or infinite intersections
        if equals_epsilon(heading.abs(), PI_OVER_TWO, EPSILON8, EPSILON8) {
            return None;
        }

        let sigma1 = calculate_sigma(ellipticity, start.latitude);
        let sigma2 = calculate_sigma(ellipticity, intersection_latitude);
        let delta_longitude = heading.tan() * (sigma2 - sigma1);
        let longitude = negative_pi_to_pi(start.longitude + delta_longitude);

        Some(Cartographic::from_radians(longitude, intersection_latitude, 0.0))
    }

    /// Interpolates a point at the given fraction (0..1) along the rhumb line.
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        self.interpolate_using_surface_distance(fraction * self.distance)
    }

    /// Interpolates a point at the given surface distance along the rhumb line.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        interpolate_using_surface_distance(
            &self.start,
            self.heading,
            distance,
            self.major,
            self.ellipticity,
        )
    }
}
