//! Ported from `packages/engine/Source/Core/EllipsoidRhumbLine.js`.
//!
//! Computes the rhumb line between two points on an ellipsoid.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;

/// Computes the rhumb line path between two points on an ellipsoid.
pub struct EllipsoidRhumbLine {
    start: Cartographic,
    end: Cartographic,
    heading: f64,
    ellipsoid_ref: Ellipsoid,
    distance: f64,
    ellipticity: f64,
    ellipticity_squared: f64,
}

impl EllipsoidRhumbLine {
    /// Creates a new EllipsoidRhumbLine.
    ///
    /// DEVIATION: the JS constructor takes `(start, end, ellipsoid)` and
    /// requires `start`/`end` to be defined; this port accepts `Option`
    /// values (defaulting to a zero `Cartographic`) and an unused
    /// `start_heading` argument kept for source compatibility with existing
    /// call sites (the heading is always derived from the endpoints, exactly
    /// like the JS implementation).
    pub fn new(
        start: Option<Cartographic>,
        end: Option<Cartographic>,
        _start_heading: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let start = start.unwrap_or_default();
        let end = end.unwrap_or_default();

        let mut line = Self {
            start,
            end,
            heading: 0.0,
            ellipsoid_ref: ellipsoid,
            distance: 0.0,
            ellipticity: 0.0,
            ellipticity_squared: 0.0,
        };
        let start = line.start.clone();
        let end = line.end.clone();
        line.compute_properties(&start, &end);
        line
    }

    /// Gets the start position.
    pub fn start(&self) -> &Cartographic {
        &self.start
    }

    /// Gets the end position.
    pub fn end(&self) -> &Cartographic {
        &self.end
    }

    /// Gets the start heading.
    pub fn start_heading(&self) -> f64 {
        self.heading
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid_ref
    }

    /// Gets the rhumb line surface distance (mirrors the JS
    /// `surfaceDistance` property).
    pub fn rhumb_distance(&self) -> f64 {
        self.distance
    }

    /// Sets the start and end points of the rhumb line.
    ///
    /// Mirrors `EllipsoidRhumbLine.prototype.setEndPoints`.
    pub fn set_end_points(&mut self, start: &Cartographic, end: &Cartographic) {
        self.compute_properties(start, end);
    }

    /// Provides the location of a point at the indicated portion along the
    /// rhumb line.
    ///
    /// Mirrors `EllipsoidRhumbLine.prototype.interpolateUsingFraction`
    /// (which delegates to `interpolateUsingSurfaceDistance` with
    /// `fraction * distance` — the interpolation is *not* linear in
    /// longitude/latitude).
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        self.interpolate_using_surface_distance(fraction * self.distance)
    }

    /// Provides the location of a point at the indicated distance along the
    /// rhumb line.
    ///
    /// # Panics
    /// Panics (debug) if start and end are not distinct.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if self.distance == 0.0 {
                throw_developer_error(
                    "EllipsoidRhumbLine must have distinct start and end set.",
                );
            }
        }
        //>>includeEnd('debug');

        interpolate_using_surface_distance_impl(
            &self.start,
            self.heading,
            distance,
            self.ellipsoid_ref.maximum_radius(),
            self.ellipticity,
        )
    }

    /// Provides the location of a point at the indicated latitude along the
    /// rhumb line.
    ///
    /// If the latitude is outside the range of start and end points, the
    /// first intersection with the latitude from that start point in the
    /// direction of the heading is returned. This follows the spiral property
    /// of a rhumb line.
    ///
    /// Returns `None` if there is no intersection or infinite intersections.
    ///
    /// # Panics
    /// Panics (debug) if start and end are not distinct.
    pub fn find_intersection_with_latitude(
        &self,
        intersection_latitude: f64,
    ) -> Option<Cartographic> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if self.distance == 0.0 {
                throw_developer_error(
                    "EllipsoidRhumbLine must have distinct start and end set.",
                );
            }
        }
        //>>includeEnd('debug');

        let ellipticity = self.ellipticity;
        let heading = self.heading;
        let start = &self.start;

        // If start and end have same latitude, return undefined since it's
        // either no intersection or infinite intersections
        if CesiumMath::equals_epsilon(
            heading.abs(),
            CesiumMath::PI_OVER_TWO,
            Some(CesiumMath::EPSILON8),
            None,
        ) {
            return None;
        }

        // Can be solved using the same equations from
        // interpolateUsingSurfaceDistance
        let sigma1 = calculate_sigma(ellipticity, start.latitude);
        let sigma2 = calculate_sigma(ellipticity, intersection_latitude);
        let delta_longitude = heading.tan() * (sigma2 - sigma1);
        let longitude = CesiumMath::negative_pi_to_pi(start.longitude + delta_longitude);

        Some(Cartographic {
            longitude,
            latitude: intersection_latitude,
            height: 0.0,
        })
    }

    /// Mirrors the private JS `computeProperties`.
    fn compute_properties(&mut self, start: &Cartographic, end: &Cartographic) {
        let mut first_cartesian = Cartesian3::default();
        self.ellipsoid_ref
            .cartographic_to_cartesian(start, &mut first_cartesian);
        let first_raw = first_cartesian.clone();
        Cartesian3::normalize(&first_raw, &mut first_cartesian);
        let mut last_cartesian = Cartesian3::default();
        self.ellipsoid_ref
            .cartographic_to_cartesian(end, &mut last_cartesian);
        let last_raw = last_cartesian.clone();
        Cartesian3::normalize(&last_raw, &mut last_cartesian);

        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            let dot = (first_cartesian.x * last_cartesian.x
                + first_cartesian.y * last_cartesian.y
                + first_cartesian.z * last_cartesian.z)
                .clamp(-1.0, 1.0);
            let angle_between = dot.acos();
            if (angle_between.abs() - std::f64::consts::PI).abs() < 0.0125 {
                throw_developer_error(
                    "EllipsoidRhumbLine start and end must be distinct.",
                );
            }
        }
        //>>includeEnd('debug');

        let major = self.ellipsoid_ref.maximum_radius();
        let minor = self.ellipsoid_ref.minimum_radius();
        let major_squared = major * major;
        let minor_squared = minor * minor;
        self.ellipticity_squared = (major_squared - minor_squared) / major_squared;
        self.ellipticity = self.ellipticity_squared.sqrt();

        let mut start_clone = start.clone();
        start_clone.height = 0.0;
        self.start = start_clone;

        let mut end_clone = end.clone();
        end_clone.height = 0.0;
        self.end = end_clone;

        self.heading = calculate_heading(
            self.ellipticity,
            start.longitude,
            start.latitude,
            end.longitude,
            end.latitude,
        );
        self.distance = calculate_arc_length(
            self.heading,
            self.ellipticity,
            self.ellipticity_squared,
            major,
            minor,
            start.longitude,
            start.latitude,
            end.longitude,
            end.latitude,
        );
    }
}

/// Mirrors the private JS `calculateM` (meridional arc length series).
#[allow(clippy::too_many_arguments)]
fn calculate_m(ellipticity: f64, major: f64, latitude: f64) -> f64 {
    if ellipticity == 0.0 {
        // sphere
        return major * latitude;
    }

    let e = ellipticity;
    let e2 = e * e;
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
        * ((1.0
            - e2 / 4.0
            - (3.0 * e4) / 64.0
            - (5.0 * e6) / 256.0
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

/// Mirrors the private JS `calculateInverseM` (inverse meridional arc
/// length series).
fn calculate_inverse_m(m: f64, ellipticity: f64, major: f64) -> f64 {
    let d = m / major;

    if ellipticity == 0.0 {
        // sphere
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
        + ((3.0 * d * e4) / 16.0
            + (45.0 * d * e6) / 256.0
            - (d * (32.0 * d2 - 561.0) * e8) / 4096.0
            - (d * (232.0 * d2 - 1677.0) * e10) / 16384.0
            + (d * (399985.0 - 90560.0 * d2 + 512.0 * d4) * e12) / 5242880.0)
            * cos2_d
        + ((21.0 * d * e6) / 256.0
            + (483.0 * d * e8) / 4096.0
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

/// Mirrors the private JS `calculateSigma` (isometric latitude).
fn calculate_sigma(ellipticity: f64, latitude: f64) -> f64 {
    if ellipticity == 0.0 {
        // sphere
        return ((0.5 * (CesiumMath::PI_OVER_TWO + latitude)).tan()).ln();
    }

    let e_sin_l = ellipticity * latitude.sin();
    ((0.5 * (CesiumMath::PI_OVER_TWO + latitude)).tan()).ln()
        - (ellipticity / 2.0) * ((1.0 + e_sin_l) / (1.0 - e_sin_l)).ln()
}

/// Mirrors the private JS `calculateHeading`.
fn calculate_heading(
    ellipticity: f64,
    first_longitude: f64,
    first_latitude: f64,
    second_longitude: f64,
    second_latitude: f64,
) -> f64 {
    let sigma1 = calculate_sigma(ellipticity, first_latitude);
    let sigma2 = calculate_sigma(ellipticity, second_latitude);
    CesiumMath::negative_pi_to_pi(second_longitude - first_longitude).atan2(sigma2 - sigma1)
}

/// Mirrors the private JS `calculateArcLength`.
#[allow(clippy::too_many_arguments)]
fn calculate_arc_length(
    heading: f64,
    ellipticity: f64,
    ellipticity_squared: f64,
    major: f64,
    minor: f64,
    first_longitude: f64,
    first_latitude: f64,
    second_longitude: f64,
    second_latitude: f64,
) -> f64 {
    let delta_longitude = second_longitude - first_longitude;

    let distance;

    //Check to see if the rhumb line has constant latitude
    //This equation will diverge if heading gets close to 90 degrees
    if CesiumMath::equals_epsilon(
        heading.abs(),
        CesiumMath::PI_OVER_TWO,
        Some(CesiumMath::EPSILON8),
        None,
    ) {
        //If heading is close to 90 degrees
        if (major - minor).abs() < f64::EPSILON {
            distance =
                major * first_latitude.cos() * CesiumMath::negative_pi_to_pi(delta_longitude);
        } else {
            let sin_phi = first_latitude.sin();
            distance = (major
                * first_latitude.cos()
                * CesiumMath::negative_pi_to_pi(delta_longitude))
                / (1.0 - ellipticity_squared * sin_phi * sin_phi).sqrt();
        }
    } else {
        let m1 = calculate_m(ellipticity, major, first_latitude);
        let m2 = calculate_m(ellipticity, major, second_latitude);

        distance = (m2 - m1) / heading.cos();
    }
    distance.abs()
}

/// Mirrors the private JS `interpolateUsingSurfaceDistance` free function.
fn interpolate_using_surface_distance_impl(
    start: &Cartographic,
    heading: f64,
    distance: f64,
    major: f64,
    ellipticity: f64,
) -> Cartographic {
    // Mirrors JS `distance === 0.0`; the tiny tolerance guards against f64
    // rounding when the caller computes `fraction * distance` (1:1 hardening).
    if distance.abs() < 1e-15 {
        return start.clone();
    }

    let ellipticity_squared = ellipticity * ellipticity;

    let longitude;
    let latitude;
    let delta_longitude;

    //Check to see if the rhumb line has constant latitude
    //This won't converge if heading is close to 90 degrees
    if (CesiumMath::PI_OVER_TWO - heading.abs()).abs() > CesiumMath::EPSILON8 {
        //Calculate latitude of the second point
        let m1 = calculate_m(ellipticity, major, start.latitude);
        let delta_m = distance * heading.cos();
        let m2 = m1 + delta_m;
        latitude = calculate_inverse_m(m2, ellipticity, major);

        //Now find the longitude of the second point

        // Check to see if the rhumb line has constant longitude
        if heading.abs() < CesiumMath::EPSILON10 {
            longitude = CesiumMath::negative_pi_to_pi(start.longitude);
        } else {
            let sigma1 = calculate_sigma(ellipticity, start.latitude);
            let sigma2 = calculate_sigma(ellipticity, latitude);
            delta_longitude = heading.tan() * (sigma2 - sigma1);
            longitude = CesiumMath::negative_pi_to_pi(start.longitude + delta_longitude);
        }
    } else {
        //If heading is close to 90 degrees
        latitude = start.latitude;
        let local_rad;

        if ellipticity == 0.0 {
            // sphere
            local_rad = major * start.latitude.cos();
        } else {
            let sin_phi = start.latitude.sin();
            local_rad = (major * start.latitude.cos())
                / (1.0 - ellipticity_squared * sin_phi * sin_phi).sqrt();
        }

        delta_longitude = distance / local_rad;
        if heading > 0.0 {
            longitude = CesiumMath::negative_pi_to_pi(start.longitude + delta_longitude);
        } else {
            longitude = CesiumMath::negative_pi_to_pi(start.longitude - delta_longitude);
        }
    }

    Cartographic {
        longitude,
        latitude,
        height: 0.0,
    }
}
