//! Ported from `packages/engine/Source/Core/BingMapsGeocoderService.js`
//! (127 lines).
//!
//! Provides geocoding through Bing Maps.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::credit::Credit;
use crate::developer_error::throw_developer_error;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{
    GeocodeDestination, GeocoderResult, GeocoderService,
};
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource, ResourceOptions};

const URL: &str = "https://dev.virtualearth.net/REST/v1/Locations";

/// The JSON(P)-fetch backend used by [`BingMapsGeocoderService::geocode`].
///
/// DEVIATION: the JS service calls `resource.fetchJsonp("jsonp")`; the
/// headless port injects a backend closure receiving the derived
/// [`Resource`]. The default backend mirrors an empty Bing response
/// (`{ "resourceSets": [] }`).
pub type BingFetchBackend = dyn Fn(&Resource) -> Value + Send + Sync;

fn default_fetch_backend() -> Arc<BingFetchBackend> {
    Arc::new(|_resource| serde_json::json!({ "resourceSets": [] }))
}

/// Options for [`BingMapsGeocoderService::new`].
#[derive(Default, Clone)]
pub struct BingMapsGeocoderServiceOptions {
    /// A key to use with the Bing Maps geocoding service (`key`,
    /// required).
    pub key: Option<String>,
    /// A Bing Maps culture code to return results in a specific culture
    /// and language (`culture`).
    pub culture: Option<String>,
}

/// Provides geocoding through Bing Maps.
pub struct BingMapsGeocoderService {
    key: String,
    resource: Resource,
    credit: Credit,
    fetch_backend: Arc<BingFetchBackend>,
}

impl BingMapsGeocoderService {
    /// Creates a new BingMapsGeocoderService.
    ///
    /// Port of `new BingMapsGeocoderService(options)`.
    ///
    /// # Panics
    /// Mirrors the `options.key is required.` DeveloperError when `key`
    /// is missing (debug builds).
    pub fn new(options: Option<BingMapsGeocoderServiceOptions>) -> Self {
        let options = options.unwrap_or_default();
        #[cfg(debug_assertions)]
        if options.key.is_none() {
            throw_developer_error("options.key is required.");
        }
        let key = options.key.unwrap_or_default();

        let mut query_parameters = HashMap::from([("key".to_string(), key.clone())]);
        if let Some(culture) = &options.culture {
            query_parameters.insert("culture".to_string(), culture.clone());
        }

        let resource = Resource::with_options(ResourceOptions {
            url: Some(URL.to_string()),
            query_parameters: Some(query_parameters),
            ..Default::default()
        });

        let credit = Credit::new(
            "<img src=\"http://dev.virtualearth.net/Branding/logo_powered_by.png\"/>",
            false,
        );

        Self {
            key,
            resource,
            credit,
            fetch_backend: default_fetch_backend(),
        }
    }

    /// Replaces the JSON-fetch backend (mock injection point; see
    /// [`BingFetchBackend`]).
    pub fn set_fetch_backend(&mut self, backend: Arc<BingFetchBackend>) {
        self.fetch_backend = backend;
    }

    /// The URL endpoint for the Bing geocoder service (`url` property).
    pub fn url(&self) -> &str {
        URL
    }

    /// The key for the Bing geocoder service (`key` property).
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl GeocoderService for BingMapsGeocoderService {
    fn credit(&self) -> Option<Credit> {
        Some(self.credit.clone_credit())
    }

    /// Performs the geocode query.
    ///
    /// Port of `BingMapsGeocoderService.prototype.geocode` (synchronous;
    /// see the [`crate::geocoder_service::GeocoderService`] DEVIATION
    /// note).
    fn geocode(&self, query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        let query_parameters =
            HashMap::from([("query".to_string(), query.to_string())]);
        let resource = self.resource.get_derived_resource_with_options(
            DerivedResourceOptions {
                query_parameters: Some(&query_parameters),
                ..Default::default()
            },
        );

        let result = (self.fetch_backend)(&resource);

        let resource_sets = result
            .get("resourceSets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if resource_sets.is_empty() {
            return Vec::new();
        }

        let resources = resource_sets[0]
            .get("resources")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        resources
            .iter()
            .map(|resource| {
                let bbox = resource.get("bbox").and_then(Value::as_array).unwrap();
                let south = bbox[0].as_f64().unwrap_or_default();
                let west = bbox[1].as_f64().unwrap_or_default();
                let north = bbox[2].as_f64().unwrap_or_default();
                let east = bbox[3].as_f64().unwrap_or_default();
                GeocoderResult {
                    display_name: resource
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    destination: GeocodeDestination::Rectangle(Rectangle::from_degrees(
                        west, south, east, north,
                    )),
                    attributions: None,
                    attribution: None,
                }
            })
            .collect()
    }
}
