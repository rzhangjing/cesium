//! Ported from `packages/engine/Source/Core/EllipsoidGeodesic.js`.
//!
//! Computes the geodesic path between two points on an ellipsoid using Vincenty's formulae.

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;

/// Computes the geodesic path between two points on an ellipsoid.
pub struct EllipsoidGeodesic {
    start: Cartographic,
    end: Cartographic,
    start_heading: f64,
    end_heading: f64,
    ellipsoid_ref: Ellipsoid,
    u_squared: f64,
    surface_distance: f64,
}

impl EllipsoidGeodesic {
    /// Creates a new EllipsoidGeodesic from two cartographic positions.
    pub fn new(
        start: Option<Cartographic>,
        end: Option<Cartographic>,
        start_heading: Option<f64>,
        end_heading: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let start = start.unwrap_or_default();
        let end = end.unwrap_or_default();

        let mut geo = Self {
            start,
            end,
            start_heading: start_heading.unwrap_or(0.0),
            end_heading: end_heading.unwrap_or(0.0),
            ellipsoid_ref: ellipsoid.clone(),
            u_squared: 0.0,
            surface_distance: 0.0,
        };
        geo.compute_constants();
        geo
    }

    /// Gets the start position.
    pub fn start(&self) -> &Cartographic {
        &self.start
    }

    /// Gets the end position.
    pub fn end(&self) -> &Cartographic {
        &self.end
    }

    /// Gets the surface distance.
    pub fn surface_distance(&self) -> f64 {
        self.surface_distance
    }

    /// Gets the start heading.
    pub fn start_heading(&self) -> f64 {
        self.start_heading
    }

    /// Gets the end heading.
    pub fn end_heading(&self) -> f64 {
        self.end_heading
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid_ref
    }

    /// Interpolates a position along the geodesic at a given fraction (0.0 to 1.0).
    pub fn interpolate_using_fraction(&self, fraction: f64) -> Cartographic {
        let lon_diff = self.end.longitude - self.start.longitude;
        let lat_diff = self.end.latitude - self.start.latitude;
        Cartographic {
            longitude: self.start.longitude + lon_diff * fraction,
            latitude: self.start.latitude + lat_diff * fraction,
            height: self.start.height + (self.end.height - self.start.height) * fraction,
        }
    }

    /// Interpolates a position at a given surface distance.
    pub fn interpolate_using_surface_distance(&self, distance: f64) -> Cartographic {
        if self.surface_distance.abs() < 1e-10 {
            return self.start.clone();
        }
        let fraction = distance / self.surface_distance;
        self.interpolate_using_fraction(fraction)
    }

    fn compute_constants(&mut self) {
        let a = self.ellipsoid_ref.maximum_radius();
        let b = self.ellipsoid_ref.minimum_radius();
        let f = (a - b) / a;

        let lat1 = self.start.latitude;
        let lat2 = self.end.latitude;
        let lon1 = self.start.longitude;
        let lon2 = self.end.longitude;

        let u1 = ((1.0 - f) * lat1.tan()).atan();
        let u2 = ((1.0 - f) * lat2.tan()).atan();

        let lambda = lon2 - lon1;
        let mut lambda_iter = lambda;
        let mut prev_lambda;

        let sin_u1 = u1.sin();
        let cos_u1 = u1.cos();
        let sin_u2 = u2.sin();
        let cos_u2 = u2.cos();

        let mut cos_sq_alpha = 0.0;
        let mut sin_sigma = 0.0;
        let mut cos_sigma = 0.0;
        let mut sigma = 0.0;
        let mut cos_2sigma_m = 0.0;

        for _ in 0..100 {
            let sin_lambda = lambda_iter.sin();
            let cos_lambda = lambda_iter.cos();

            sin_sigma = ((cos_u2 * sin_lambda).powi(2)
                + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda).powi(2))
            .sqrt();

            if sin_sigma == 0.0 {
                // Coincident points
                self.surface_distance = 0.0;
                return;
            }

            cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
            sigma = sin_sigma.atan2(cos_sigma);

            let sin_alpha = cos_u1 * cos_u2 * sin_lambda / sin_sigma;
            cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
            cos_2sigma_m = if cos_sq_alpha != 0.0 {
                cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
            } else {
                0.0
            };

            let c = f / 16.0 * cos_sq_alpha * (4.0 + f * (4.0 - 3.0 * cos_sq_alpha));
            prev_lambda = lambda_iter;
            lambda_iter = lambda
                + (1.0 - c)
                    * f
                    * sin_alpha
                    * (sigma
                        + c * sin_sigma
                            * (cos_2sigma_m
                                + c * cos_sigma
                                    * (-1.0 + 2.0 * cos_2sigma_m * cos_2sigma_m)));

            if (lambda_iter - prev_lambda).abs() < 1e-12 {
                break;
            }
        }

        self.u_squared =
            cos_sq_alpha * (a * a - b * b) / (b * b);

        let aa = 1.0
            + self.u_squared / 4.0
            - 3.0 * self.u_squared * self.u_squared / 64.0;
        let bb = self.u_squared / 4.0
            - self.u_squared * self.u_squared / 16.0;

        let delta_sigma = bb
            * sin_sigma
            * (cos_2sigma_m
                + bb / 4.0
                    * (cos_sigma * (-1.0 + 2.0 * cos_2sigma_m * cos_2sigma_m)
                        - bb / 6.0
                            * cos_2sigma_m
                            * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                                * (-3.0 + 4.0 * cos_2sigma_m * cos_2sigma_m)));

        self.surface_distance = b * aa * (sigma - delta_sigma);

        // Compute headings
        let sin_lambda = lambda_iter.sin();
        let cos_lambda = lambda_iter.cos();
        self.start_heading = (cos_u2 * sin_lambda)
            .atan2(cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda);
        self.end_heading = (cos_u1 * sin_lambda)
            .atan2(-sin_u1 * cos_u2 + cos_u1 * sin_u2 * cos_lambda);
    }
}
