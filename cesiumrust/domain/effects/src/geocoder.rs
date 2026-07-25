//! Geocoding service interface and types.
//!
//! Maps to CesiumJS:
//! - `Core/GeocoderService.js`
//! - `Core/GeocodeType.js`
//! - `Core/BingMapsGeocoderService.js`
//! - `Core/PeliasGeocoderService.js`
//! - `Core/OpenCageGeocoderService.js`

use serde::{Deserialize, Serialize};

/// The type of geocode to perform.
///
/// Maps to CesiumJS `Core/GeocodeType.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeocodeType {
    /// Search for a location by name/address.
    #[default]
    Search,
    /// Reverse geocode a location to get name/address.
    Reverse,
}

/// A result from a geocoding operation.
///
/// Maps to CesiumJS `GeocoderService.Result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeocoderResult {
    /// Display name for the location.
    pub display_name: String,
    /// Destination as a bounding rectangle [west, south, east, north] in radians,
    /// or a point [lon, lat, height] in radians/meters.
    pub destination: GeocoderDestination,
    /// Attribution credits for the result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributions: Vec<GeocoderAttribution>,
}

/// Destination of a geocoder result - either a rectangle or a point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeocoderDestination {
    /// A bounding rectangle [west, south, east, north] in radians.
    Rectangle([f64; 4]),
    /// A point [longitude, latitude] in radians with optional height.
    Point {
        /// Longitude in radians.
        longitude: f64,
        /// Latitude in radians.
        latitude: f64,
        /// Height in meters (optional).
        #[serde(default)]
        height: Option<f64>,
    },
}

/// An attribution credit from a geocoder result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeocoderAttribution {
    /// Whether the attribution must be shown.
    #[serde(default)]
    pub mandatory: bool,
    /// Whether the attribution is collapsible.
    #[serde(default)]
    pub collapsible: bool,
    /// HTML content of the attribution.
    pub html: String,
}

/// Trait for geocoding services.
///
/// Maps to CesiumJS `Core/GeocoderService.js`.
pub trait GeocoderService {
    /// Get the credit to display after a geocode is performed.
    fn credit(&self) -> Option<&str>;

    /// Perform a geocode operation.
    ///
    /// Returns a list of results matching the query.
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult>;
}

/// A mock geocoder service for testing.
#[derive(Debug, Clone, Default)]
pub struct MockGeocoderService {
    /// Results to return for any query.
    pub results: Vec<GeocoderResult>,
    /// Credit string.
    pub credit: Option<String>,
}

impl MockGeocoderService {
    /// Create a new mock geocoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with predefined results.
    pub fn with_results(results: Vec<GeocoderResult>) -> Self {
        Self {
            results,
            credit: None,
        }
    }
}

impl GeocoderService for MockGeocoderService {
    fn credit(&self) -> Option<&str> {
        self.credit.as_deref()
    }

    fn geocode(&self, _query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        self.results.clone()
    }
}

/// Parse credits from geocoder result attributions.
///
/// Maps to CesiumJS `GeocoderService.getCreditsFromResult`.
pub fn get_credits_from_result(result: &GeocoderResult) -> Vec<&GeocoderAttribution> {
    result.attributions.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocode_type_default() {
        assert_eq!(GeocodeType::default(), GeocodeType::Search);
    }

    #[test]
    fn test_geocoder_result_rectangle() {
        let result = GeocoderResult {
            display_name: "New York, NY".to_string(),
            destination: GeocoderDestination::Rectangle([
                -1.2985, 0.7086, -1.2968, 0.7098,
            ]),
            attributions: vec![],
        };
        assert_eq!(result.display_name, "New York, NY");
        if let GeocoderDestination::Rectangle(r) = result.destination {
            assert!(r[0] < r[2]); // west < east
            assert!(r[1] < r[3]); // south < north
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn test_geocoder_result_point() {
        let result = GeocoderResult {
            display_name: "Eiffel Tower".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0407,
                latitude: 0.8517,
                height: Some(330.0),
            },
            attributions: vec![],
        };
        if let GeocoderDestination::Point { longitude, latitude, height } = result.destination {
            assert!((longitude - 0.0407).abs() < 1e-4);
            assert!((latitude - 0.8517).abs() < 1e-4);
            assert_eq!(height, Some(330.0));
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_geocoder_attribution() {
        let attr = GeocoderAttribution {
            mandatory: true,
            collapsible: false,
            html: "<a href='https://example.com'>Example</a>".to_string(),
        };
        assert!(attr.mandatory);
        assert!(!attr.collapsible);
    }

    #[test]
    fn test_mock_geocoder_service() {
        let results = vec![GeocoderResult {
            display_name: "Test Location".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0,
                latitude: 0.0,
                height: None,
            },
            attributions: vec![],
        }];

        let service = MockGeocoderService::with_results(results);
        let geocode_results = service.geocode("test", GeocodeType::Search);
        assert_eq!(geocode_results.len(), 1);
        assert_eq!(geocode_results[0].display_name, "Test Location");
    }

    #[test]
    fn test_mock_geocoder_credit() {
        let mut service = MockGeocoderService::new();
        assert!(service.credit().is_none());

        service.credit = Some("Test Credit".to_string());
        assert_eq!(service.credit(), Some("Test Credit"));
    }

    #[test]
    fn test_get_credits_from_result() {
        let result = GeocoderResult {
            display_name: "Test".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0,
                latitude: 0.0,
                height: None,
            },
            attributions: vec![
                GeocoderAttribution {
                    mandatory: true,
                    collapsible: false,
                    html: "Credit 1".to_string(),
                },
                GeocoderAttribution {
                    mandatory: false,
                    collapsible: true,
                    html: "Credit 2".to_string(),
                },
            ],
        };

        let credits = get_credits_from_result(&result);
        assert_eq!(credits.len(), 2);
        assert!(credits[0].mandatory);
        assert!(!credits[1].mandatory);
    }

    #[test]
    fn test_geocoder_result_serialization() {
        let result = GeocoderResult {
            display_name: "Test".to_string(),
            destination: GeocoderDestination::Rectangle([0.0, 0.0, 1.0, 1.0]),
            attributions: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));

        let deserialized: GeocoderResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.display_name, "Test");
    }
}
