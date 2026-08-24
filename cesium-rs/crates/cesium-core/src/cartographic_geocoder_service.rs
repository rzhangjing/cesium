//! Ported from `packages/engine/Source/Core/CartographicGeocoderService.js`
//! (74 lines).
//!
//! Geocodes queries containing longitude and latitude coordinates and an
//! optional height. Query format: `longitude latitude (height)` with
//! longitude/latitude in degrees and height in meters.

use crate::cartesian3::Cartesian3;
use crate::credit::Credit;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{GeocodeDestination, GeocoderResult, GeocoderService};

/// Geocodes queries containing longitude and latitude coordinates and an
/// optional height.
#[derive(Default, Clone, Copy)]
pub struct CartographicGeocoderService {
    _private: (),
}

impl CartographicGeocoderService {
    /// Creates a new CartographicGeocoderService.
    ///
    /// Port of `new CartographicGeocoderService()`.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl GeocoderService for CartographicGeocoderService {
    fn credit(&self) -> Option<Credit> {
        // JS getter returns `undefined`.
        None
    }

    /// Parses the query into a cartographic destination.
    ///
    /// Port of `CartographicGeocoderService.prototype.geocode` (synchronous;
    /// see the [`crate::geocoder_service::GeocoderService`] DEVIATION note).
    fn geocode(&self, query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        // JS: `query.match(/[^\s,\n]+/g)`; the JS code would throw a
        // TypeError on a fully-empty query (`null.length`). The port treats
        // "no tokens" as no results.
        // DEVIATION: see above. See docs/deviations.md.
        let split_query: Vec<&str> = query
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|token| !token.is_empty())
            .collect();

        if split_query.len() == 2 || split_query.len() == 3 {
            let mut longitude = split_query[0].parse::<f64>().ok();
            let mut latitude = split_query[1].parse::<f64>().ok();
            let height = if split_query.len() == 3 {
                split_query[2].parse::<f64>().ok()
            } else {
                Some(300.0)
            };

            if longitude.is_none() && latitude.is_none() {
                // Port of the `^(\d+.?\d*)([nsew])` N/S/E/W fallback parse.
                let coord_test =
                    regex::Regex::new(r"(?i)^(\d+.?\d*)([nsew])").unwrap();
                for token in &split_query {
                    if let Some(split_coord) = coord_test.captures(token) {
                        let direction = &split_coord[2];
                        if regex::Regex::new("(?i)^[ns]").unwrap().is_match(direction) {
                            let value: f64 = split_coord[1].parse().unwrap_or(f64::NAN);
                            latitude = Some(
                                if regex::Regex::new("(?i)^[n]").unwrap().is_match(direction) {
                                    value
                                } else {
                                    -value
                                },
                            );
                        } else if regex::Regex::new("(?i)^[ew]").unwrap().is_match(direction) {
                            let value: f64 = split_coord[1].parse().unwrap_or(f64::NAN);
                            longitude = Some(
                                if regex::Regex::new("(?i)^[e]").unwrap().is_match(direction) {
                                    value
                                } else {
                                    -value
                                },
                            );
                        }
                    }
                }
            }

            if let (Some(longitude), Some(latitude), Some(height)) = (longitude, latitude, height)
            {
                let result = GeocoderResult {
                    display_name: query.to_string(),
                    destination: GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(
                        longitude,
                        latitude,
                        Some(height),
                        None,
                    )),
                    attributions: None,
                    attribution: None,
                };
                return vec![result];
            }
        }
        Vec::new()
    }
}
