//! Ported from `packages/engine/Source/Core/IonGeocoderService.js`
//! (147 lines).
//!
//! Provides geocoding through Cesium ion.

use std::collections::HashMap;
use std::sync::Arc;

use crate::credit::Credit;
use crate::geocode_type::GeocodeType;
use crate::geocoder_service::{GeocoderResult, GeocoderService};
use crate::ion;
use crate::ion_geocode_provider_type::IonGeocodeProviderType;
use crate::pelias_geocoder_service::{PeliasFetchBackend, PeliasGeocoderService};
use crate::resource::{QueryValue, Resource};

/// Maps an [`IonGeocodeProviderType`] to its `geocoder` query parameter.
///
/// Mirrors `providerToParameterMap` / `providerToQueryParameter`
/// (`DEFAULT` maps to `undefined`, i.e. no parameter).
fn provider_to_query_parameter(provider: IonGeocodeProviderType) -> Option<&'static str> {
    match provider {
        IonGeocodeProviderType::Google => Some("google"),
        IonGeocodeProviderType::Bing => Some("bing"),
        IonGeocodeProviderType::Default => None,
    }
}

/// Maps a `geocoder` query parameter back to its
/// [`IonGeocodeProviderType`].
///
/// Mirrors `queryParameterToProvider` (`None` / missing parameter maps to
/// `DEFAULT`).
fn query_parameter_to_provider(parameter: Option<&str>) -> IonGeocodeProviderType {
    match parameter {
        Some("google") => IonGeocodeProviderType::Google,
        Some("bing") => IonGeocodeProviderType::Bing,
        // JS: the DEFAULT entry's undefined value matches; unknown
        // parameters would crash the JS lookup — treated as DEFAULT here.
        _ => IonGeocodeProviderType::Default,
    }
}

/// Options for [`IonGeocoderService::new`].
///
/// DEVIATION: the JS `options.scene` is required and only used to register
/// the default-token static credit
/// (`scene.frameState.creditDisplay.addStaticCredit`); the headless port
/// has no Scene/CreditDisplay, so the field is dropped.
#[derive(Default, Clone)]
pub struct IonGeocoderServiceOptions {
    /// The access token to use (`accessToken`, default
    /// `Ion.defaultAccessToken`).
    pub access_token: Option<String>,
    /// The Cesium ion API server (`server`, default `Ion.defaultServer`).
    pub server: Option<String>,
    /// The geocoder the Cesium ion API server should use
    /// (`geocodeProviderType`, default `IonGeocodeProviderType.DEFAULT`).
    pub geocode_provider_type: Option<IonGeocodeProviderType>,
}

/// Provides geocoding through Cesium ion.
pub struct IonGeocoderService {
    access_token: String,
    server: Resource,
    pelias: PeliasGeocoderService,
}

impl IonGeocoderService {
    /// Creates a new IonGeocoderService.
    ///
    /// Port of `new IonGeocoderService(options)`.
    ///
    /// DEVIATION: the JS `validateIonGeocodeProviderType` debug check has
    /// no Rust counterpart (the provider type is an enum and cannot be
    /// invalid); the `options.scene` credit registration is dropped (see
    /// [`IonGeocoderServiceOptions`]).
    pub fn new(options: Option<IonGeocoderServiceOptions>) -> Self {
        let options = options.unwrap_or_default();

        let geocode_provider_type = options
            .geocode_provider_type
            .unwrap_or(IonGeocodeProviderType::Default);

        let access_token = options
            .access_token
            .unwrap_or_else(ion::default_access_token);
        let mut server = Resource::create_if_needed(
            &options.server.unwrap_or_else(ion::default_server),
        );
        server.append_forward_slash();

        // DEVIATION: the JS registers `Ion.getDefaultTokenCredit(token)` as
        // a static credit on `options.scene.frameState.creditDisplay`; the
        // headless port has no CreditDisplay.

        let mut search_endpoint = server.get_derived_resource("v1/geocode");
        if !access_token.is_empty() {
            let params =
                HashMap::from([("access_token".to_string(), access_token.clone())]);
            search_endpoint.append_query_parameters(&params);
        }

        let mut pelias = PeliasGeocoderService::from_resource(search_endpoint);
        // geocodeProviderType isn't stored here directly but instead relies
        // on the query parameters of the pelias url; use the setter logic
        // to update the value.
        let mut service = Self {
            access_token,
            server,
            pelias,
        };
        service.set_geocode_provider_type(geocode_provider_type);
        service
    }

    /// The access token used (`_accessToken`).
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// The Cesium ion API server resource (`_server`).
    pub fn server(&self) -> &Resource {
        &self.server
    }

    /// The inner Pelias service (`_pelias`).
    pub fn pelias(&self) -> &PeliasGeocoderService {
        &self.pelias
    }

    /// Replaces the JSON-fetch backend of the inner Pelias service (mock
    /// injection point; see [`PeliasFetchBackend`]).
    pub fn set_fetch_backend(&mut self, backend: Arc<PeliasFetchBackend>) {
        self.pelias.set_fetch_backend(backend);
    }

    /// The geocoding service that the Cesium ion API server should use to
    /// fulfill geocoding requests (`geocodeProviderType` getter).
    ///
    /// Reads the `geocoder` query parameter of the pelias url.
    pub fn geocode_provider_type(&self) -> IonGeocodeProviderType {
        query_parameter_to_provider(self.pelias.url().get_query_parameter("geocoder"))
    }

    /// Sets the geocoding service (`geocodeProviderType` setter).
    ///
    /// Mirrors the JS setter: merges the `geocoder` parameter into the
    /// pelias url query parameters, deleting it for `DEFAULT` (so no
    /// `&geocoder=undefined` is sent).
    pub fn set_geocode_provider_type(&mut self, geocode_provider_type: IonGeocodeProviderType) {
        let mut query: HashMap<String, String> = self
            .pelias
            .url()
            .query_parameters()
            .iter()
            .filter_map(|(key, value)| {
                // Only single-valued parameters participate (JS plain
                // object spread of `queryParameters`).
                match value {
                    QueryValue::Single(v) => Some((key.clone(), v.clone())),
                    _ => None,
                }
            })
            .collect();

        match provider_to_query_parameter(geocode_provider_type) {
            Some(parameter) => {
                query.insert("geocoder".to_string(), parameter.to_string());
            }
            None => {
                // Delete the geocoder parameter to prevent sending
                // `&geocoder=undefined` in the query.
                query.remove("geocoder");
            }
        }

        let url = self.pelias.url_mut();
        // JS `setQueryParameters` replaces the parameters (clone
        // semantics); clear first, then apply the merged set.
        let existing: Vec<String> = url
            .query_parameters()
            .iter()
            .map(|(key, _)| key.clone())
            .collect();
        for key in existing {
            url.remove_query_parameter(&key);
        }
        url.set_query_parameters(&query, false);
    }
}

impl GeocoderService for IonGeocoderService {
    /// The `credit` property: always `undefined` for the ion service.
    fn credit(&self) -> Option<Credit> {
        None
    }

    /// Performs the geocode query (delegates to the inner Pelias service).
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        self.pelias.geocode(query, geocode_type)
    }
}
