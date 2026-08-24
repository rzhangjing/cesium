//! Ported from `packages/engine/Source/Core/OpenCageGeocoderService.js`
//! (134 lines).
//!
//! Provides geocoding via a OpenCage server.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::cartesian3::Cartesian3;
use crate::check;
use crate::credit::Credit;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{GeocodeDestination, GeocoderResult, GeocoderService};
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource};

/// The JSON-fetch backend used by [`OpenCageGeocoderService::geocode`].
///
/// DEVIATION: the JS service calls `resource.fetchJson()`; the headless port
/// injects a backend closure receiving the derived [`Resource`]. The default
/// backend mirrors an empty OpenCage response (`{ "results": [] }`).
pub type OpenCageFetchBackend = dyn Fn(&Resource) -> Value + Send + Sync;

fn default_fetch_backend() -> Arc<OpenCageFetchBackend> {
    Arc::new(|_resource| serde_json::json!({ "results": [] }))
}

/// Provides geocoding via a OpenCage server.
pub struct OpenCageGeocoderService {
    url: Resource,
    params: HashMap<String, String>,
    credit: Credit,
    fetch_backend: Arc<OpenCageFetchBackend>,
}

impl OpenCageGeocoderService {
    /// Creates a new OpenCageGeocoderService.
    ///
    /// Port of `new OpenCageGeocoderService(url, apiKey, params)`.
    ///
    /// # Panics
    /// Mirrors `Check.defined("url", url)` / `Check.defined("apiKey",
    /// apiKey)` (debug builds).
    pub fn new(
        url: Option<&str>,
        api_key: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            check::defined("url", url.as_ref());
            check::defined("apiKey", api_key.as_ref());
        }

        let mut url = Resource::create_if_needed(url.unwrap_or_default());
        url.append_forward_slash();
        url.set_query_parameters(
            &HashMap::from([("key".to_string(), api_key.unwrap_or_default().to_string())]),
            false,
        );

        let credit = Credit::new(
            "Geodata copyright <a href=\"https://www.openstreetmap.org/\">OpenStreetMap</a> contributors",
            false,
        );

        Self {
            url,
            params: params.unwrap_or_default(),
            credit,
            fetch_backend: default_fetch_backend(),
        }
    }

    /// The Resource used to access the OpenCage endpoint (`url` property).
    pub fn url(&self) -> &Resource {
        &self.url
    }

    /// Optional params passed to OpenCage in order to customize geocoding
    /// (`params` property).
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// Replaces the JSON-fetch backend (mock injection point; see
    /// [`OpenCageFetchBackend`]).
    pub fn set_fetch_backend(&mut self, backend: Arc<OpenCageFetchBackend>) {
        self.fetch_backend = backend;
    }
}

impl GeocoderService for OpenCageGeocoderService {
    fn credit(&self) -> Option<Credit> {
        Some(self.credit.clone_credit())
    }

    /// Performs the geocode query.
    ///
    /// Port of `OpenCageGeocoderService.prototype.geocode` (synchronous; see
    /// the [`crate::geocoder_service::GeocoderService`] DEVIATION note).
    fn geocode(&self, query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        // JS: `combine(this._params, { q: query })` (second object wins).
        let mut query_parameters = self.params.clone();
        query_parameters.insert("q".to_string(), query.to_string());

        let resource = self.url.get_derived_resource_with_options(
            DerivedResourceOptions {
                url: Some("json"),
                query_parameters: Some(&query_parameters),
                ..Default::default()
            },
        );

        let response = (self.fetch_backend)(&resource);

        let results = response
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        results
            .iter()
            .map(|result_object| {
                let destination = if let Some(bounds) =
                    result_object.get("bounds").filter(|b| !b.is_null())
                {
                    let southwest = &bounds["southwest"];
                    let northeast = &bounds["northeast"];
                    GeocodeDestination::Rectangle(Rectangle::from_degrees(
                        southwest["lng"].as_f64().unwrap_or_default(),
                        southwest["lat"].as_f64().unwrap_or_default(),
                        northeast["lng"].as_f64().unwrap_or_default(),
                        northeast["lat"].as_f64().unwrap_or_default(),
                    ))
                } else {
                    let geometry = &result_object["geometry"];
                    GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(
                        geometry["lng"].as_f64().unwrap_or_default(),
                        geometry["lat"].as_f64().unwrap_or_default(),
                        None,
                        None,
                    ))
                };

                GeocoderResult {
                    display_name: result_object
                        .get("formatted")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    destination,
                    attributions: None,
                    attribution: None,
                }
            })
            .collect()
    }
}
