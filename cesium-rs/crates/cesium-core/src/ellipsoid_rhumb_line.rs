//! Ported from `packages/engine/Source/Core/EllipsoidRhumbLine.js`.
//!
//! Computes the rhumb line between two points on an ellipsoid.

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;

/// Computes the rhumb line path between two points on an ellipsoid.
pub struct EllipsoidRhumbLine {
    start: Cartographic,
    end: Cartographic,
    start_heading: f64,
    ellipsoid_ref: Ellipsoid,
    rhumb_distance: f64,
}

impl EllipsoidRhumbLine {
    /// Creates a new EllipsoidRhumbLine.
    pub fn new(
        start: Option<Cartographic>,
        end: Option<Cartographic>,
        start_heading: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let start = start.unwrap_or_default();
        let end = end.unwrap_or_default();

        let mut line = Self {
            start,
            end,
            start_heading: start_heading.unwrap_or(0.0),
            ellipsoid_ref: ellipsoid,
            rhumb_distance: 0.0,
        };
        line.compute_constants();
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
        self.start_heading
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid_ref
    }

    /// Gets the rhumb line distance.
    pub fn rhumb_distance(&self) -> f64 {
        self.rhumb_distance
    }

    /// Interpolates a position along the rhumb line at a given fraction (0.0 to 1.0).
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        Cartographic {
            longitude: self.start.longitude + (self.end.longitude - self.start.longitude) * fraction,
            latitude: self.start.latitude + (self.end.latitude - self.start.latitude) * fraction,
            height: self.start.height + (self.end.height - self.start.height) * fraction,
        }
    }

    /// Interpolates a position at a given surface distance.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        if self.rhumb_distance.abs() < 1e-10 {
            return self.start.clone();
        }
        let fraction = distance / self.rhumb_distance;
        self.interpolate_using_fraction(fraction)
    }

    fn compute_constants(&mut self) {
        let a = self.ellipsoid_ref.maximum_radius();
        let b = self.ellipsoid_ref.minimum_radius();
        let e = (1.0 - (b * b) / (a * a)).sqrt();

        let lat1 = self.start.latitude;
        let lat2 = self.end.latitude;
        let lon1 = self.start.longitude;
        let lon2 = self.end.longitude;

        let d_lon = lon2 - lon1;

        // Compute meridional parts
        let mp1 = meridional_parts(e, lat1);
        let mp2 = meridional_parts(e, lat2);
        let d_mp = mp2 - mp1;

        // Compute heading
        if d_mp.abs() < 1e-10 {
            self.start_heading = if d_lon >= 0.0 {
                std::f64::consts::FRAC_PI_2
            } else {
                -std::f64::consts::FRAC_PI_2
            };
        } else {
            self.start_heading = d_lon.atan2(d_mp);
        }

        // Compute distance
        let d_lat = lat2 - lat1;
        if d_lat.abs() > 1e-10 {
            let q = d_lat / d_mp;
            self.rhumb_distance =
                (d_lat * d_lat + q * q * d_lon * d_lon).sqrt() * a;
        } else {
            self.rhumb_distance = (d_lon * d_lon).abs() * lat1.cos() * a;
        }
    }
}

fn meridional_parts(e: f64, latitude: f64) -> f64 {
    let sin_lat = latitude.sin();
    let e_sin = e * sin_lat;
    // M(φ) = ln(tan(π/4 + φ/2)) - (e/2) * ln((1 + e*sin(φ))/(1 - e*sin(φ)))
    let log_tan = ((std::f64::consts::FRAC_PI_4 + latitude / 2.0).tan()).ln();
    let e_term = (e / 2.0) * ((1.0 + e_sin) / (1.0 - e_sin)).ln();
    log_tan - e_term
}
