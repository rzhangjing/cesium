//! Extended imagery provider specs - ported from Scene/*ImageryProviderSpec.js
//!
//! Covers: TimeDynamicImagery, WmsGetFeatureInfo, Bing quadkey values,
//! WMS bbox computation, ArcGIS, Mapbox, MapboxStyle, SingleTile,
//! TileCoordinates, Ion providers.

use cesium_provider::imagery_provider::{
    ArcGisMapServerImageryProvider, BingMapsImageryProvider, IonImageryProvider,
    MapboxImageryProvider, MapboxStyleImageryProvider, SingleTileImageryProvider,
    TileCoord, TileCoordinatesImageryProvider, TimeDynamicImagery, WmsGetFeatureInfo,
    WmsImageryProvider,
};

// ─── TimeDynamicImagery ────────────────────────────────────────────────────

#[test]
fn time_dynamic_new_empty() {
    let td = TimeDynamicImagery::new();
    assert_eq!(td.interval_count(), 0);
    assert!(!td.interpolate);
}

#[test]
fn time_dynamic_add_interval() {
    let mut td = TimeDynamicImagery::new();
    td.add_interval(0.0, 100.0, "https://example.com/{time}/{z}/{x}/{y}.png");
    assert_eq!(td.interval_count(), 1);
    td.add_interval(100.0, 200.0, "https://example.com/{time}/{z}/{x}/{y}.png");
    assert_eq!(td.interval_count(), 2);
}

#[test]
fn time_dynamic_get_tile_url_in_range() {
    let mut td = TimeDynamicImagery::new();
    td.add_interval(0.0, 100.0, "https://example.com/{time}/{z}/{x}/{y}.png");
    let coord = TileCoord::new(3, 5, 4);
    let url = td.get_tile_url(50.0, &coord).unwrap();
    assert_eq!(url, "https://example.com/50/4/3/5.png");
}

#[test]
fn time_dynamic_get_tile_url_out_of_range() {
    let mut td = TimeDynamicImagery::new();
    td.add_interval(0.0, 100.0, "https://example.com/{time}/{z}/{x}/{y}.png");
    let coord = TileCoord::new(3, 5, 4);
    assert!(td.get_tile_url(150.0, &coord).is_none());
}

#[test]
fn time_dynamic_get_tile_url_at_boundary() {
    let mut td = TimeDynamicImagery::new();
    td.add_interval(10.0, 20.0, "https://tiles.example.com/{z}/{x}/{y}?t={time}");
    let coord = TileCoord::new(1, 2, 3);
    // At start boundary
    let url = td.get_tile_url(10.0, &coord).unwrap();
    assert!(url.contains("t=10"));
    // At stop boundary
    let url = td.get_tile_url(20.0, &coord).unwrap();
    assert!(url.contains("t=20"));
}

#[test]
fn time_dynamic_multiple_intervals() {
    let mut td = TimeDynamicImagery::new();
    td.add_interval(0.0, 50.0, "https://a.example.com/{z}/{x}/{y}.png");
    td.add_interval(50.0, 100.0, "https://b.example.com/{z}/{x}/{y}.png");
    let coord = TileCoord::new(1, 1, 1);
    let url_a = td.get_tile_url(25.0, &coord).unwrap();
    assert!(url_a.starts_with("https://a."));
    let url_b = td.get_tile_url(75.0, &coord).unwrap();
    assert!(url_b.starts_with("https://b."));
}

// ─── WmsGetFeatureInfo ─────────────────────────────────────────────────────

#[test]
fn wms_gfi_creation() {
    let gfi = WmsGetFeatureInfo::new("https://wms.example.com", "roads");
    assert_eq!(gfi.layers, "roads");
    assert_eq!(gfi.info_format, "application/json");
    assert_eq!(gfi.crs, "EPSG:4326");
    assert_eq!(gfi.feature_count, 10);
}

#[test]
fn wms_gfi_with_info_format() {
    let gfi = WmsGetFeatureInfo::new("https://wms.example.com", "roads")
        .with_info_format("text/html");
    assert_eq!(gfi.info_format, "text/html");
}

#[test]
fn wms_gfi_get_url() {
    let gfi = WmsGetFeatureInfo::new("https://wms.example.com", "roads");
    let url = gfi.get_url([-10.0, -5.0, 10.0, 5.0], 256, 256, 128, 64);
    assert!(url.contains("request=GetFeatureInfo"));
    assert!(url.contains("query_layers=roads"));
    assert!(url.contains("i=128"));
    assert!(url.contains("j=64"));
    assert!(url.contains("feature_count=10"));
    // bbox order: south,west,north,east for WMS 1.3.0
    assert!(url.contains("bbox=-5,-10,5,10"));
}

#[test]
fn wms_gfi_url_with_existing_query() {
    let gfi = WmsGetFeatureInfo::new("https://wms.example.com?token=abc", "buildings");
    let url = gfi.get_url([0.0, 0.0, 1.0, 1.0], 512, 512, 256, 256);
    assert!(url.contains("&service=WMS"));
    assert!(url.contains("token=abc"));
}

// ─── Bing Quadkey ──────────────────────────────────────────────────────────

#[test]
fn bing_quadkey_level_0() {
    let coord = TileCoord::new(0, 0, 0);
    let qk = BingMapsImageryProvider::tile_to_quadkey(&coord);
    assert_eq!(qk, "");
}

#[test]
fn bing_quadkey_level_1() {
    // Level 1: x=0,y=0 → "0"; x=1,y=0 → "1"; x=0,y=1 → "2"; x=1,y=1 → "3"
    assert_eq!(BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(0, 0, 1)), "0");
    assert_eq!(BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(1, 0, 1)), "1");
    assert_eq!(BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(0, 1, 1)), "2");
    assert_eq!(BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(1, 1, 1)), "3");
}

#[test]
fn bing_quadkey_level_2() {
    // x=3, y=2, level=2: bit1(1): x&2=2→+1, y&2=2→+2 = "3"; bit0(0): x&1=1→+1, y&1=0→+0 = "1"
    let qk = BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(3, 2, 2));
    assert_eq!(qk, "31");
}

#[test]
fn bing_quadkey_level_3() {
    // x=5, y=3, level=3
    // bit2: x&4=4→+1, y&4=0→+0 = "1"
    // bit1: x&2=0→+0, y&2=2→+2 = "2"
    // bit0: x&1=1→+1, y&1=1→+2 = "3"
    let qk = BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(5, 3, 3));
    assert_eq!(qk, "123");
}

#[test]
fn bing_tile_url_contains_quadkey() {
    let provider = BingMapsImageryProvider::new("key123");
    let coord = TileCoord::new(1, 1, 1);
    let url = provider.get_tile_url(&coord);
    // quadkey for (1,1,1) = "3"
    assert!(url.contains("aerial3.jpeg"));
}

// ─── WMS bbox computation ──────────────────────────────────────────────────

#[test]
fn wms_bbox_level_0_tile_0_0() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    let coord = TileCoord::new(0, 0, 0);
    let url = provider.get_tile_url(&coord);
    // Level 0: tiles_x=2, tiles_y=1
    // west=-180, east=0, north=90, south=-90
    assert!(url.contains("bbox=-90,-180,90,0"));
}

#[test]
fn wms_bbox_level_1_tile_1_0() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    let coord = TileCoord::new(1, 0, 1);
    let url = provider.get_tile_url(&coord);
    // Level 1: tiles_x=4, tiles_y=2
    // x=1: west=-180+(1/4)*360=-90, east=-180+(2/4)*360=0
    // y=0: north=90-(0/2)*180=90, south=90-(1/2)*180=0
    assert!(url.contains("bbox=0,-90,90,0"));
}

#[test]
fn wms_url_with_custom_params() {
    let mut provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    provider.parameters.insert("time".to_string(), "2024-01-01".to_string());
    let coord = TileCoord::new(0, 0, 0);
    let url = provider.get_tile_url(&coord);
    assert!(url.contains("time=2024-01-01"));
}

// ─── ArcGIS MapServer ──────────────────────────────────────────────────────

#[test]
fn arcgis_creation() {
    let provider = ArcGisMapServerImageryProvider::new("https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer");
    assert_eq!(provider.tile_width, 256);
    assert_eq!(provider.maximum_level, 23);
    assert!(provider.use_https);
}

#[test]
fn arcgis_get_tile_url() {
    let provider = ArcGisMapServerImageryProvider::new("https://services.arcgisonline.com/MapServer");
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url(&coord);
    assert_eq!(url, "https://services.arcgisonline.com/MapServer/tile/4/5/3&f=image");
}

#[test]
fn arcgis_with_layers() {
    let provider = ArcGisMapServerImageryProvider::new("https://example.com/MapServer")
        .with_layers("0,1,2");
    let coord = TileCoord::new(1, 1, 1);
    let url = provider.get_tile_url(&coord);
    assert!(url.contains("layers=show:0,1,2"));
}

// ─── Mapbox ────────────────────────────────────────────────────────────────

#[test]
fn mapbox_creation() {
    let provider = MapboxImageryProvider::new("mapbox.satellite", "pk.token123");
    assert_eq!(provider.map_id, "mapbox.satellite");
    assert_eq!(provider.access_token, "pk.token123");
    assert_eq!(provider.tile_size, 512);
    assert_eq!(provider.maximum_level, 22);
}

#[test]
fn mapbox_get_tile_url() {
    let provider = MapboxImageryProvider::new("mapbox.satellite", "pk.abc");
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url(&coord);
    assert_eq!(url, "https://api.mapbox.com/v4/mapbox.satellite/4/3/5.png?access_token=pk.abc");
}

// ─── MapboxStyle ───────────────────────────────────────────────────────────

#[test]
fn mapbox_style_creation() {
    let provider = MapboxStyleImageryProvider::new("mapbox/streets-v11", "pk.xyz");
    assert_eq!(provider.style_id, "mapbox/streets-v11");
    assert_eq!(provider.tile_size, 512);
}

#[test]
fn mapbox_style_get_tile_url() {
    let provider = MapboxStyleImageryProvider::new("mapbox/streets-v11", "pk.xyz");
    let coord = TileCoord::new(2, 3, 4);
    let url = provider.get_tile_url(&coord);
    assert_eq!(url, "https://api.mapbox.com/styles/v1/mapbox/streets-v11/tiles/4/2/3?access_token=pk.xyz");
}

// ─── SingleTile ────────────────────────────────────────────────────────────

#[test]
fn single_tile_creation() {
    let provider = SingleTileImageryProvider::new("https://example.com/world.png");
    assert_eq!(provider.url, "https://example.com/world.png");
    // Default rectangle covers the whole globe
    assert!((provider.rectangle[0] - (-std::f64::consts::PI)).abs() < 1e-10);
    assert!((provider.rectangle[2] - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn single_tile_with_rectangle() {
    let provider = SingleTileImageryProvider::new("https://example.com/region.png")
        .with_rectangle([-1.0, -0.5, 1.0, 0.5]);
    assert_eq!(provider.rectangle, [-1.0, -0.5, 1.0, 0.5]);
}

#[test]
fn single_tile_always_same_url() {
    let provider = SingleTileImageryProvider::new("https://example.com/world.png");
    let url1 = provider.get_tile_url(&TileCoord::new(0, 0, 0));
    let url2 = provider.get_tile_url(&TileCoord::new(5, 10, 3));
    assert_eq!(url1, url2);
    assert_eq!(url1, "https://example.com/world.png");
}

// ─── TileCoordinates ───────────────────────────────────────────────────────

#[test]
fn tile_coordinates_creation() {
    let provider = TileCoordinatesImageryProvider::new();
    assert_eq!(provider.tile_width, 256);
    assert_eq!(provider.tile_height, 256);
}

#[test]
fn tile_coordinates_get_tile_text() {
    let provider = TileCoordinatesImageryProvider::new();
    let text = provider.get_tile_text(&TileCoord::new(3, 5, 4));
    assert_eq!(text, "L4: X3 Y5");
}

#[test]
fn tile_coordinates_text_level_0() {
    let provider = TileCoordinatesImageryProvider::new();
    let text = provider.get_tile_text(&TileCoord::new(0, 0, 0));
    assert_eq!(text, "L0: X0 Y0");
}

// ─── Ion ───────────────────────────────────────────────────────────────────

#[test]
fn ion_creation() {
    let provider = IonImageryProvider::new(12345);
    assert_eq!(provider.asset_id, 12345);
    assert!(provider.access_token.is_none());
    assert_eq!(provider.server, "https://api.cesium.com");
}

#[test]
fn ion_with_access_token() {
    let provider = IonImageryProvider::new(12345).with_access_token("my_token");
    assert_eq!(provider.access_token.as_deref(), Some("my_token"));
}

#[test]
fn ion_endpoint_url_without_token() {
    let provider = IonImageryProvider::new(12345);
    let url = provider.get_endpoint_url();
    assert_eq!(url, "https://api.cesium.com/v1/assets/12345/endpoint");
}

#[test]
fn ion_endpoint_url_with_token() {
    let provider = IonImageryProvider::new(99).with_access_token("abc");
    let url = provider.get_endpoint_url();
    assert_eq!(url, "https://api.cesium.com/v1/assets/99/endpoint?access_token=abc");
}
