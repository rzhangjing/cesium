//! Core fidelity spec mirrors for the A7 geocoder batch.
//!
//! Mirrors:
//! - `packages/engine/Specs/Core/PeliasGeocoderServiceSpec.js`
//! - `packages/engine/Specs/Core/IonGeocoderServiceSpec.js` (scene-free
//!   cases; the JS `options.scene` credit registration is a DEVIATION)
//! - `packages/engine/Specs/Core/CartographicGeocoderServiceSpec.js`
//! - `packages/engine/Specs/Core/BingMapsGeocoderServiceSpec.js`
//! - `packages/engine/Specs/Core/GoogleGeocoderServicesSpec.js`
//! - `packages/engine/Specs/Core/OpenCageGeocoderServiceSpec.js`
//!
//! DEVIATION: the JS specs spy on `Resource.prototype.fetchJson` /
//! `loadAndExecuteScript`; the Rust port injects the fetch backends
//! (mock backend pattern) instead.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use cesium_core::bing_maps_geocoder_service::{
    BingMapsGeocoderService, BingMapsGeocoderServiceOptions,
};
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic_geocoder_service::CartographicGeocoderService;
use cesium_core::credit::Credit;
use cesium_core::geocode_type::GeocodeType;
use cesium_core::geocoder_service::{
    get_credits_from_result, GeocodeDestination, GeocoderService,
};
use cesium_core::google_geocoder_service::{
    GoogleGeocoderService, GoogleGeocoderServiceOptions,
};
use cesium_core::ion;
use cesium_core::ion_geocode_provider_type::IonGeocodeProviderType;
use cesium_core::ion_geocoder_service::{IonGeocoderService, IonGeocoderServiceOptions};
use cesium_core::open_cage_geocoder_service::OpenCageGeocoderService;
use cesium_core::pelias_geocoder_service::PeliasGeocoderService;
use cesium_core::rectangle::Rectangle;

fn panics(mut f: impl FnMut() + std::panic::UnwindSafe) -> bool {
    catch_unwind(AssertUnwindSafe(&mut f)).is_err()
}

// ── Core/PeliasGeocoderService ───────────────────────────────────────

#[test]
fn pelias_constructor_throws_without_url() {
    assert!(panics(|| {
        let _ = PeliasGeocoderService::new(None);
    }));
}

#[test]
fn pelias_returns_geocoder_results() {
    let mut service = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));

    let query = "some query";
    let data = serde_json::json!({
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [-75.172489, 39.927828],
                },
                "properties": {
                    "label": "1826 S 16th St, Philadelphia, PA, USA",
                },
            },
        ],
    });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode(query, GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].display_name,
        "1826 S 16th St, Philadelphia, PA, USA"
    );
    match &results[0].destination {
        GeocodeDestination::Cartesian3(cartesian) => {
            assert_eq!(
                *cartesian,
                Cartesian3::from_degrees_new(-75.172489, 39.927828, None, None)
            );
        }
        _ => panic!("expected a Cartesian3 destination"),
    }
}

#[test]
fn pelias_returns_geocoder_results_with_attributions() {
    let mut service = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));

    let data = serde_json::json!({
        "attributions": [
            { "html": "Credit", "collapsible": true },
        ],
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [-75.172489, 39.927828],
                },
                "properties": {
                    "label": "1826 S 16th St, Philadelphia, PA, USA",
                },
            },
        ],
    });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("some query", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    let attributions = results[0].attributions.as_ref().unwrap();
    assert_eq!(attributions.len(), 1);
    assert_eq!(attributions[0].html, "Credit");

    // `GeocoderService.getCreditsFromResult` mapping (collapsible=true →
    // showOnScreen=false).
    let credits = get_credits_from_result(&results[0]).unwrap();
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].html(), "Credit");
    assert!(!credits[0].show_on_screen());
}

#[test]
fn pelias_returns_no_geocoder_results_if_pelias_has_no_results() {
    let mut service = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));

    let data = serde_json::json!({ "features": [] });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("some query", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn pelias_calls_search_endpoint_if_specified() {
    let mut service = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));

    let query = "some query";
    let captured = Arc::new(Mutex::new(None::<(String, Option<String>)>));
    let captured_clone = Arc::clone(&captured);
    service.set_fetch_backend(Arc::new(move |resource| {
        *captured_clone.lock().unwrap() = Some((
            resource.raw_url().to_string(),
            resource.get_query_parameter("text").map(str::to_string),
        ));
        serde_json::json!({ "features": [] })
    }));

    let _ = service.geocode(query, GeocodeType::Search);
    let (url, text) = captured.lock().unwrap().clone().unwrap();
    assert!(url.ends_with("search"), "unexpected endpoint url: {url}");
    assert_eq!(text.as_deref(), Some(query));
}

#[test]
fn pelias_calls_autocomplete_endpoint_if_specified() {
    let mut service = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));

    let query = "some query";
    let captured = Arc::new(Mutex::new(None::<(String, Option<String>)>));
    let captured_clone = Arc::clone(&captured);
    service.set_fetch_backend(Arc::new(move |resource| {
        *captured_clone.lock().unwrap() = Some((
            resource.raw_url().to_string(),
            resource.get_query_parameter("text").map(str::to_string),
        ));
        serde_json::json!({ "features": [] })
    }));

    let _ = service.geocode(query, GeocodeType::Autocomplete);
    let (url, text) = captured.lock().unwrap().clone().unwrap();
    assert!(
        url.ends_with("autocomplete"),
        "unexpected endpoint url: {url}"
    );
    assert_eq!(text.as_deref(), Some(query));
}

// ── Core/IonGeocoderService ──────────────────────────────────────────

#[test]
fn ion_creates_with_default_parameters() {
    let service = IonGeocoderService::new(None);

    assert_eq!(service.access_token(), ion::default_access_token());
    assert_eq!(service.server().url(), ion::default_server());
    assert_eq!(
        service.geocode_provider_type(),
        IonGeocodeProviderType::Default
    );
}

#[test]
fn ion_creates_with_specified_parameters() {
    let access_token = "123456";
    let server = "http://not.ion.invalid/";
    let geocode_provider_type = IonGeocodeProviderType::Google;

    let service = IonGeocoderService::new(Some(IonGeocoderServiceOptions {
        access_token: Some(access_token.to_string()),
        server: Some(server.to_string()),
        geocode_provider_type: Some(geocode_provider_type),
    }));

    assert_eq!(service.access_token(), access_token);
    assert_eq!(service.server().url(), server);
    assert_eq!(service.geocode_provider_type(), geocode_provider_type);
}

#[test]
fn ion_calls_inner_geocoder_and_returns_result() {
    let mut service = IonGeocoderService::new(None);

    let data = serde_json::json!({
        "features": [
            {
                "type": "Feature",
                "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
                "properties": { "label": "results" },
            },
        ],
    });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let result = service.geocode("some query", GeocodeType::Search);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].display_name, "results");
}

#[test]
fn ion_credit_returns_expected_value() {
    let service = IonGeocoderService::new(None);
    assert!(service.credit().is_none());
}

#[test]
fn ion_setting_geocode_provider_type_updates_pelias_url_for_google() {
    let mut service = IonGeocoderService::new(Some(IonGeocoderServiceOptions {
        geocode_provider_type: Some(IonGeocodeProviderType::Default),
        ..Default::default()
    }));

    service.set_geocode_provider_type(IonGeocodeProviderType::Google);
    assert_eq!(
        service.pelias().url().get_query_parameter("geocoder"),
        Some("google")
    );
}

#[test]
fn ion_setting_geocode_provider_type_updates_pelias_url_for_bing() {
    let mut service = IonGeocoderService::new(Some(IonGeocoderServiceOptions {
        geocode_provider_type: Some(IonGeocodeProviderType::Default),
        ..Default::default()
    }));

    service.set_geocode_provider_type(IonGeocodeProviderType::Bing);
    assert_eq!(
        service.pelias().url().get_query_parameter("geocoder"),
        Some("bing")
    );
}

#[test]
fn ion_setting_geocode_provider_type_updates_pelias_url_for_default() {
    let mut service = IonGeocoderService::new(Some(IonGeocoderServiceOptions {
        geocode_provider_type: Some(IonGeocodeProviderType::Google),
        ..Default::default()
    }));

    service.set_geocode_provider_type(IonGeocodeProviderType::Default);
    // Make sure the parameter is deleted, not set to "undefined".
    assert_eq!(
        service.pelias().url().get_query_parameter("geocoder"),
        None
    );
    assert_eq!(service.geocode_provider_type(), IonGeocodeProviderType::Default);
}

// DEVIATION: "throws if setting invalid geocodeProviderType" cannot be
// mirrored — `IonGeocodeProviderType` is an enum and cannot be invalid.

// ── Core/CartographicGeocoderService ─────────────────────────────────

#[test]
fn cartographic_returns_cartesian_with_matching_coordinates_for_ns_ew_input() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode("35N 75W", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(-75.0, 35.0, Some(300.0), None))
    );
}

#[test]
fn cartographic_returns_cartesian_with_matching_coordinates_for_ew_ns_input() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode("75W 35N", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(-75.0, 35.0, Some(300.0), None))
    );
}

#[test]
fn cartographic_returns_cartesian_for_long_lat_height_input() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode(" 1.0, 2.0, 3.0 ", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None))
    );
}

#[test]
fn cartographic_returns_cartesian_for_long_lat_input() {
    let service = CartographicGeocoderService::new();
    let default_height = 300.0;
    let results = service.geocode(" 1.0, 2.0 ", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Cartesian3(Cartesian3::from_degrees_new(
            1.0,
            2.0,
            Some(default_height),
            None
        ))
    );
}

#[test]
fn cartographic_returns_empty_array_for_input_with_only_longitudinal_coordinates() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode(" 1e 1e ", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn cartographic_returns_empty_array_for_input_with_only_one_nsew_coordinate() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode(" 1e 1 ", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn cartographic_returns_empty_array_for_input_with_only_one_number() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode(" 2.0 ", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn cartographic_returns_empty_array_for_string() {
    let service = CartographicGeocoderService::new();
    let results = service.geocode(" aoeu ", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

// ── Core/BingMapsGeocoderService ─────────────────────────────────────

fn bing_mock_data() -> serde_json::Value {
    serde_json::json!({
        "resourceSets": [
            {
                "resources": [
                    { "name": "a", "bbox": [32.0, 3.0, 3.0, 4.0] },
                ],
            },
        ],
    })
}

#[test]
fn bing_returns_geocoder_results() {
    let query = "some query";
    let key = "not_the_real_key;";
    let data = bing_mock_data();

    let captured = Arc::new(Mutex::new(None::<(Option<String>, Option<String>, Option<String>)>));
    let captured_clone = Arc::clone(&captured);
    let mut service = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some(key.to_string()),
        culture: None,
    }));
    service.set_fetch_backend(Arc::new(move |resource| {
        *captured_clone.lock().unwrap() = Some((
            resource.get_query_parameter("query").map(str::to_string),
            resource.get_query_parameter("key").map(str::to_string),
            resource.get_query_parameter("culture").map(str::to_string),
        ));
        data.clone()
    }));

    let results = service.geocode(query, GeocodeType::Search);
    let (got_query, got_key, got_culture) = captured.lock().unwrap().clone().unwrap();
    assert_eq!(got_query.as_deref(), Some(query));
    assert_eq!(got_key.as_deref(), Some(key));
    assert_eq!(got_culture, None);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].display_name, "a");
    // bbox = [south, west, north, east].
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Rectangle(Rectangle::from_degrees(3.0, 32.0, 4.0, 3.0))
    );
}

#[test]
fn bing_uses_supplied_culture() {
    let query = "some query";
    let key = "not_the_real_key;";
    let data = bing_mock_data();

    let captured = Arc::new(Mutex::new(None::<Option<String>>));
    let captured_clone = Arc::clone(&captured);
    let mut service = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some(key.to_string()),
        culture: Some("ja".to_string()),
    }));
    service.set_fetch_backend(Arc::new(move |resource| {
        *captured_clone.lock().unwrap() =
            Some(resource.get_query_parameter("culture").map(str::to_string));
        data.clone()
    }));

    let results = service.geocode(query, GeocodeType::Search);
    assert_eq!(captured.lock().unwrap().clone().unwrap().as_deref(), Some("ja"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].display_name, "a");
    assert!(matches!(
        results[0].destination,
        GeocodeDestination::Rectangle(_)
    ));
}

#[test]
fn bing_returns_no_geocoder_results_if_bing_has_no_results() {
    let data = serde_json::json!({ "resourceSets": [] });
    let mut service = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some(String::new()),
        culture: None,
    }));
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("some query", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn bing_returns_no_geocoder_results_if_bing_has_results_but_no_resources() {
    let data = serde_json::json!({ "resourceSets": [ { "resources": [] } ] });
    let mut service = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some(String::new()),
        culture: None,
    }));
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("some query", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn bing_credit_returns_expected_value() {
    let service = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some(String::new()),
        culture: None,
    }));

    let credit: Credit = service.credit().unwrap();
    assert_eq!(
        credit.html(),
        "<img src=\"http://dev.virtualearth.net/Branding/logo_powered_by.png\"/>"
    );
    assert!(!credit.show_on_screen());
}

// ── Core/GoogleGeocoderService ───────────────────────────────────────

#[test]
fn google_constructor_throws_without_key() {
    assert!(panics(|| {
        let _ = GoogleGeocoderService::new(None);
    }));
}

#[test]
fn google_constructor_sets_key_on_resource() {
    let key = "0123456789abcdef0123456789abcdef";
    let service = GoogleGeocoderService::new(Some(GoogleGeocoderServiceOptions {
        key: Some(key.to_string()),
    }));
    assert_eq!(
        service.resource().url(),
        format!("https://maps.googleapis.com/maps/api/geocode/json?key={key}")
    );
}

#[test]
fn google_geocode_returns_results_for_status_ok() {
    let service_fn = || {
        let mut service = GoogleGeocoderService::new(Some(GoogleGeocoderServiceOptions {
            key: Some("key".to_string()),
        }));
        let data = serde_json::json!({
            "results": [
                {
                    "formatted_address":
                        "1600 Amphitheatre Pkwy, Mountain View, CA 94043, USA",
                    "geometry": {
                        "viewport": {
                            "northeast": { "lat": 37.4237349802915, "lng": -122.083183169709 },
                            "southwest": { "lat": 37.4210370197085, "lng": -122.085881130292 },
                        },
                    },
                },
            ],
            "status": "OK",
        });
        service.set_fetch_backend(Arc::new(move |_resource| data.clone()));
        service.geocode("query", GeocodeType::Search)
    };
    let results = service_fn();

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].display_name,
        "1600 Amphitheatre Pkwy, Mountain View, CA 94043, USA"
    );
    let expected = Rectangle::from_degrees(
        -122.085881130292,
        37.4210370197085,
        -122.083183169709,
        37.4237349802915,
    );
    match &results[0].destination {
        GeocodeDestination::Rectangle(rectangle) => {
            assert_eq!(rectangle.west, expected.west);
            assert_eq!(rectangle.south, expected.south);
            assert_eq!(rectangle.east, expected.east);
            assert_eq!(rectangle.north, expected.north);
        }
        _ => panic!("expected a Rectangle destination"),
    }
    let attribution = results[0].attribution.as_ref().unwrap();
    assert_eq!(
        attribution.html,
        "<img alt=\"Google\" src=\"https://assets.ion.cesium.com/google-credit.png\" style=\"vertical-align:-5px\">"
    );
    assert_eq!(attribution.collapsible, Some(false));
}

#[test]
fn google_returns_empty_array_for_status_zero_results() {
    let mut service = GoogleGeocoderService::new(Some(GoogleGeocoderServiceOptions {
        key: Some("key".to_string()),
    }));
    let data = serde_json::json!({ "status": "ZERO_RESULTS" });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("test", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

// ── Core/OpenCageGeocoderService ─────────────────────────────────────

const OPENCAGE_ENDPOINT: &str = "https://api.opencagedata.com/geocode/v1/";
const OPENCAGE_API_KEY: &str = "c2a490d593b14612aefa6ec2e6b77c47";

#[test]
fn open_cage_constructor_throws_without_url() {
    assert!(panics(|| {
        let _ = OpenCageGeocoderService::new(None, Some(OPENCAGE_API_KEY), None);
    }));
}

#[test]
fn open_cage_constructor_throws_without_api_key() {
    assert!(panics(|| {
        let _ = OpenCageGeocoderService::new(Some(OPENCAGE_ENDPOINT), None, None);
    }));
}

#[test]
fn open_cage_returns_geocoder_results() {
    let mut service =
        OpenCageGeocoderService::new(Some(OPENCAGE_ENDPOINT), Some(OPENCAGE_API_KEY), None);

    let data = serde_json::json!({
        "results": [
            {
                "bounds": {
                    "northeast": { "lat": -22.6790826, "lng": 14.5269016 },
                    "southwest": { "lat": -22.6792826, "lng": 14.5267016 },
                },
                "formatted": "Beryl's Restaurant, Woermann St, Swakopmund, Namibia",
                "geometry": { "lat": -22.6795394, "lng": 14.5276006 },
            },
        ],
    });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("-22.6792,+14.5272", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].display_name,
        "Beryl's Restaurant, Woermann St, Swakopmund, Namibia"
    );
    assert!(matches!(
        results[0].destination,
        GeocodeDestination::Rectangle(_)
    ));
}

#[test]
fn open_cage_returns_geocoder_result_as_a_cartesian3_if_no_bounds_are_provided() {
    let mut service =
        OpenCageGeocoderService::new(Some(OPENCAGE_ENDPOINT), Some(OPENCAGE_API_KEY), None);

    let data = serde_json::json!({
        "results": [
            {
                "formatted": "Beryl's Restaurant, Woermann St, Swakopmund, Namibia",
                "geometry": { "lat": -22.6795394, "lng": 14.5276006 },
            },
        ],
    });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("-22.6792,+14.5272", GeocodeType::Search);
    assert_eq!(results.len(), 1);
    let expected_destination =
        Cartesian3::from_degrees_new(14.5276006, -22.6795394, None, None);
    assert_eq!(
        results[0].destination,
        GeocodeDestination::Cartesian3(expected_destination)
    );
}

#[test]
fn open_cage_returns_no_geocoder_results_if_open_cage_has_no_results() {
    let mut service =
        OpenCageGeocoderService::new(Some(OPENCAGE_ENDPOINT), Some(OPENCAGE_API_KEY), None);

    let data = serde_json::json!({ "results": [] });
    service.set_fetch_backend(Arc::new(move |_resource| data.clone()));

    let results = service.geocode("", GeocodeType::Search);
    assert_eq!(results.len(), 0);
}

#[test]
fn open_cage_credit_returns_expected_value() {
    let service =
        OpenCageGeocoderService::new(Some(OPENCAGE_ENDPOINT), Some(OPENCAGE_API_KEY), None);

    let credit = service.credit().unwrap();
    assert_eq!(
        credit.html(),
        "Geodata copyright <a href=\"https://www.openstreetmap.org/\">OpenStreetMap</a> contributors"
    );
    assert!(!credit.show_on_screen());
}
