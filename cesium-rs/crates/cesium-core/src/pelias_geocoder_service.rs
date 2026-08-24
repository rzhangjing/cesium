//! Ported from `packages/engine/Source/Core/PeliasGeocoderService.js`
//! (106 lines).
//!
//! Provides geocoding via a Pelias server.

use std::sync::Arc;

use serde_json::Value;

use crate::cartesian3::Cartesian3;
use crate::check;
use crate::credit::Credit;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{
    GeocodeDestination, GeocoderAttribution, GeocoderResult, GeocoderService,
};
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource};

/// The JSON-fetch backend used by [`PeliasGeocoderService::geocode`].
///
/// DEVIATION: the JS service calls `resource.fetchJson()` (XHR); the
/// headless port injects a backend closure receiving the derived
/// [`Resource`]. The default backend mirrors an empty Pelias response
/// (`{ "features": [] }`).
pub type PeliasFetchBackend = dyn Fn(&Resource) -> Value + Send + Sync;

fn default_fetch_backend() -> Arc<PeliasFetchBackend> {
    Arc::new(|_resource| serde_json::json!({ "features": [] }))
}

/// Provides geocoding via a [Pelias](https://pelias.io/) server.
pub struct PeliasGeocoderService {
    url: Resource,
    fetch_backend: Arc<PeliasFetchBackend>,
}

impl PeliasGeocoderService {
    /// Creates a new PeliasGeocoderService.
    ///
    /// Port of `new PeliasGeocoderService(url)`; `url` mirrors the JS
    /// `Resource|string` argument (`Resource.createIfNeeded`).
    ///
    /// # Panics
    /// Mirrors the `Check.defined("url", url)` DeveloperError when `url`
    /// is `None` (debug builds).
    pub fn new(url: Option<&str>) -> Self {
        #[cfg(debug_assertions)]
        check::defined("url", url.as_ref());

        let mut resource = Resource::create_if_needed(url.unwrap_or_default());
        resource.append_forward_slash();
        Self {
            url: resource,
            fetch_backend: default_fetch_backend(),
        }
    }

    /// Replaces the JSON-fetch backend (mock injection point; see
    /// [`PeliasFetchBackend`]).
    pub fn set_fetch_backend(&mut self, backend: Arc<PeliasFetchBackend>) {
        self.fetch_backend = backend;
    }

    /// Creates a service from an already-built [`Resource`].
    ///
    /// Mirrors the `Resource` branch of `Resource.createIfNeeded` in the JS
    /// constructor (used by
    /// [`crate::ion_geocoder_service::IonGeocoderService`]).
    pub fn from_resource(mut url: Resource) -> Self {
        url.append_forward_slash();
        Self {
            url,
            fetch_backend: default_fetch_backend(),
        }
    }

    /// The Resource used to access the Pelias endpoint (`url` property).
    pub fn url(&self) -> &Resource {
        &self.url
    }

    /// Mutable access to the endpoint Resource (used by
    /// [`crate::ion_geocoder_service::IonGeocoderService`] to manage the
    /// `geocoder` query parameter, mirroring `this._pelias.url` access).
    pub fn url_mut(&mut self) -> &mut Resource {
        &mut self.url
    }

    /// Parses a fetched Pelias response into geocoder results.
    ///
    /// Mirrors the `resource.fetchJson().then(...)` mapping of
    /// `PeliasGeocoderService.prototype.geocode`.
    fn parse_results(&self, results: &Value) -> Vec<GeocoderResult> {
        let attributions = results.get("attributions").and_then(|value| {
            value.as_array().map(|entries| {
                entries
                    .iter()
                    .map(|entry| GeocoderAttribution {
                        html: entry
                            .get("html")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        collapsible: entry.get("collapsible").and_then(Value::as_bool),
                    })
                    .collect()
            })
        });

        let Some(features) = results.get("features").and_then(Value::as_array) else {
            return Vec::new();
        };

        features
            .iter()
            .map(|feature| {
                let destination = if let Some(bbox) = feature.get("bbox") {
                    let bbox = bbox.as_array().unwrap();
                    GeocodeDestination::Rectangle(Rectangle::from_degrees(
                        bbox[0].as_f64().unwrap_or_default(),
                        bbox[1].as_f64().unwrap_or_default(),
                        bbox[2].as_f64().unwrap_or_default(),
                        bbox[3].as_f64().unwrap_or_default(),
                    ))
                } else {
                    let coordinates = &feature["geometry"]["coordinates"];
                    let lon = coordinates[0].as_f64().unwrap_or_default();
                    let lat = coordinates[1].as_f64().unwrap_or_default();
                    GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(
                        lon,
                        lat,
                        None,
                        None,
                    ))
                };

                GeocoderResult {
                    display_name: feature["properties"]["label"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    destination,
                    attributions: attributions.clone(),
                    attribution: None,
                }
            })
            .collect()
    }
}

impl GeocoderService for PeliasGeocoderService {
    /// The `credit` property: always `undefined` for Pelias.
    fn credit(&self) -> Option<Credit> {
        None
    }

    /// Performs the geocode query.
    ///
    /// Port of `PeliasGeocoderService.prototype.geocode` (synchronous; see
    /// the [`crate::geocoder_service::GeocoderService`] DEVIATION note).
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        let endpoint = if geocode_type == GeocodeType::Autocomplete {
            "autocomplete"
        } else {
            "search"
        };
        let query_parameters =
            std::collections::HashMap::from([("text".to_string(), query.to_string())]);
        let resource = self.url.get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(endpoint),
            query_parameters: Some(&query_parameters),
            ..Default::default()
        });

        let response = (self.fetch_backend)(&resource);
        self.parse_results(&response)
    }
}
