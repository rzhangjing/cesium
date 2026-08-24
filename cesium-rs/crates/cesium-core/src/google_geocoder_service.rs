//! Ported from `packages/engine/Source/Core/GoogleGeocoderService.js`
//! (111 lines).
//!
//! Provides geocoding through Google.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::credit::Credit;
use crate::developer_error::throw_developer_error;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{
    GeocodeDestination, GeocoderAttribution, GeocoderResult, GeocoderService,
};
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource, ResourceOptions};
use crate::runtime_error::RuntimeError;

const API_URL: &str = "https://maps.googleapis.com/maps/api/geocode/json";
const CREDIT_HTML: &str =
    "<img alt=\"Google\" src=\"https://assets.ion.cesium.com/google-credit.png\" style=\"vertical-align:-5px\">";

/// The JSON-fetch backend used by [`GoogleGeocoderService::geocode`].
///
/// DEVIATION: the JS service calls `resource.fetchJson()`; the headless port
/// injects a backend closure receiving the derived [`Resource`]. The default
/// backend mirrors a zero-results response (`{ "status": "ZERO_RESULTS" }`).
pub type GoogleFetchBackend = dyn Fn(&Resource) -> Value + Send + Sync;

fn default_fetch_backend() -> Arc<GoogleFetchBackend> {
    Arc::new(|_resource| serde_json::json!({ "status": "ZERO_RESULTS" }))
}

/// Options for [`GoogleGeocoderService::new`].
#[derive(Default, Clone)]
pub struct GoogleGeocoderServiceOptions {
    /// An API key to use with the Google geocoding service (`key`,
    /// required).
    pub key: Option<String>,
}

/// Provides geocoding through Google.
pub struct GoogleGeocoderService {
    resource: Resource,
    credit: Credit,
    fetch_backend: Arc<GoogleFetchBackend>,
}

impl GoogleGeocoderService {
    /// Creates a new GoogleGeocoderService.
    ///
    /// Port of `new GoogleGeocoderService(options)`.
    ///
    /// # Panics
    /// Mirrors the `options.key is required.` DeveloperError when `key` is
    /// missing (debug builds).
    pub fn new(options: Option<GoogleGeocoderServiceOptions>) -> Self {
        let options = options.unwrap_or_default();
        #[cfg(debug_assertions)]
        if options.key.is_none() {
            throw_developer_error("options.key is required.");
        }
        let key = options.key.unwrap_or_default();

        let resource = Resource::with_options(ResourceOptions {
            url: Some(API_URL.to_string()),
            query_parameters: Some(HashMap::from([("key".to_string(), key)])),
            ..Default::default()
        });

        let credit = Credit::new(CREDIT_HTML, true);

        Self {
            resource,
            credit,
            fetch_backend: default_fetch_backend(),
        }
    }

    /// Replaces the JSON-fetch backend (mock injection point; see
    /// [`GoogleFetchBackend`]).
    pub fn set_fetch_backend(&mut self, backend: Arc<GoogleFetchBackend>) {
        self.fetch_backend = backend;
    }

    /// The endpoint resource (`_resource`, exposed for spec fidelity).
    pub fn resource(&self) -> &Resource {
        &self.resource
    }
}

impl GeocoderService for GoogleGeocoderService {
    fn credit(&self) -> Option<Credit> {
        Some(self.credit.clone_credit())
    }

    /// Get a list of possible locations that match a search string.
    ///
    /// Port of `GoogleGeocoderService.prototype.geocode` (synchronous; see
    /// the [`crate::geocoder_service::GeocoderService`] DEVIATION note).
    ///
    /// # Panics
    /// Mirrors the `RuntimeError` thrown when the service returns a status
    /// other than `OK` or `ZERO_RESULTS`.
    fn geocode(&self, query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        let query_parameters =
            HashMap::from([("address".to_string(), query.to_string())]);
        let resource = self.resource.get_derived_resource_with_options(
            DerivedResourceOptions {
                query_parameters: Some(&query_parameters),
                ..Default::default()
            },
        );

        let response = (self.fetch_backend)(&resource);

        let status = response
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if status == "ZERO_RESULTS" {
            return Vec::new();
        }

        if status != "OK" {
            let error_message = response
                .get("error_message")
                .and_then(Value::as_str)
                .unwrap_or("undefined");
            // JS: `throw new RuntimeError(...)`.
            panic!(
                "{}",
                RuntimeError::new(Some(&format!(
                    "GoogleGeocoderService got a bad response {status}: {error_message}"
                )))
            );
        }

        let results = response
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        results
            .iter()
            .map(|result| {
                let viewport = &result["geometry"]["viewport"];
                let south_west = &viewport["southwest"];
                let north_east = &viewport["northeast"];
                GeocoderResult {
                    display_name: result
                        .get("formatted_address")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    destination: GeocodeDestination::Rectangle(Rectangle::from_degrees(
                        south_west["lng"].as_f64().unwrap_or_default(),
                        south_west["lat"].as_f64().unwrap_or_default(),
                        north_east["lng"].as_f64().unwrap_or_default(),
                        north_east["lat"].as_f64().unwrap_or_default(),
                    )),
                    attributions: None,
                    // JS fidelity: Google maps a singular `attribution` key
                    // (unused by `getCreditsFromResult`).
                    attribution: Some(GeocoderAttribution {
                        html: CREDIT_HTML.to_string(),
                        collapsible: Some(false),
                    }),
                }
            })
            .collect()
    }
}
