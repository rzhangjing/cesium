//! Imagery provider URL generation spec tests.
//!
//! Maps to CesiumJS:
//! - Scene/UrlTemplateImageryProviderSpec.js
//! - Scene/WebMapTileServiceImageryProviderSpec.js
//! - Scene/TileMapServiceImageryProviderSpec.js
//! - Scene/OpenStreetMapImageryProviderSpec.js
//!
//! A-class tests: URL template substitution, KVP/REST URL generation, reverse Y.

use cesium_provider::imagery_provider::{
    BingMapsImageryProvider, BingMapStyle, OpenStreetMapImageryProvider, TileCoord,
    TmsImageryProvider, UrlTemplateImageryProvider, WmtsImageryProvider,
};

// === UrlTemplateImageryProvider ===

#[test]
fn url_template_basic_substitution() {
    let p = UrlTemplateImageryProvider::new("https://tiles.example.com/{z}/{x}/{y}.png");
    let coord = TileCoord::new(3, 5, 4);
    let url = p.get_tile_url(&coord, 0);
    assert_eq!(url, "https://tiles.example.com/4/3/5.png");
}

#[test]
fn url_template_reverse_y() {
    let p = UrlTemplateImageryProvider::new("https://tiles.example.com/{z}/{x}/{reverseY}.png");
    let coord = TileCoord::new(0, 0, 1);
    // At level 1, tiles_y = 2, reverseY = 2 - 1 - 0 = 1
    let url = p.get_tile_url(&coord, 0);
    assert_eq!(url, "https://tiles.example.com/1/0/1.png");
}

#[test]
fn url_template_reverse_y_level_2() {
    let p = UrlTemplateImageryProvider::new("https://t.com/{z}/{x}/{reverseY}.png");
    let coord = TileCoord::new(1, 3, 2);
    // At level 2, tiles_y = 4, reverseY = 4 - 1 - 3 = 0
    let url = p.get_tile_url(&coord, 0);
    assert_eq!(url, "https://t.com/2/1/0.png");
}

#[test]
fn url_template_subdomain_round_robin() {
    let p = UrlTemplateImageryProvider::new("https://{s}.tiles.example.com/{z}/{x}/{y}.png")
        .with_subdomains(vec!["a".into(), "b".into(), "c".into()]);
    let coord = TileCoord::new(1, 1, 1);

    let url0 = p.get_tile_url(&coord, 0);
    assert!(url0.contains("a.tiles"));

    let url1 = p.get_tile_url(&coord, 1);
    assert!(url1.contains("b.tiles"));

    let url2 = p.get_tile_url(&coord, 2);
    assert!(url2.contains("c.tiles"));

    // Wraps around
    let url3 = p.get_tile_url(&coord, 3);
    assert!(url3.contains("a.tiles"));
}

#[test]
fn url_template_no_subdomain_removes_placeholder() {
    let p = UrlTemplateImageryProvider::new("https://tiles.example.com/{z}/{x}/{y}.png");
    let coord = TileCoord::new(0, 0, 0);
    let url = p.get_tile_url(&coord, 0);
    assert!(!url.contains("{s}"));
}

#[test]
fn url_template_defaults() {
    let p = UrlTemplateImageryProvider::new("https://t.com/{z}/{x}/{y}.png");
    assert_eq!(p.minimum_level, 0);
    assert_eq!(p.maximum_level, 25);
    assert_eq!(p.tile_width, 256);
    assert_eq!(p.tile_height, 256);
}

#[test]
fn url_template_with_max_level() {
    let p = UrlTemplateImageryProvider::new("https://t.com/{z}/{x}/{y}.png").with_max_level(18);
    assert_eq!(p.maximum_level, 18);
    assert!(p.is_available(18));
    assert!(!p.is_available(19));
}

#[test]
fn url_template_with_tile_size() {
    let p = UrlTemplateImageryProvider::new("https://t.com/{z}/{x}/{y}.png")
        .with_tile_size(512, 512);
    assert_eq!(p.tile_width, 512);
    assert_eq!(p.tile_height, 512);
}

#[test]
fn url_template_availability() {
    let p = UrlTemplateImageryProvider::new("https://t.com/{z}/{x}/{y}.png");
    assert!(p.is_available(0));
    assert!(p.is_available(25));
    assert!(!p.is_available(26));
}

// === WmtsImageryProvider ===

#[test]
fn wmts_defaults() {
    let p = WmtsImageryProvider::new("https://wmts.example.com", "satellite");
    assert_eq!(p.layer, "satellite");
    assert_eq!(p.style, "default");
    assert_eq!(p.tile_matrix_set_id, "GoogleMapsCompatible");
    assert_eq!(p.format, "image/jpeg");
    assert_eq!(p.tile_width, 256);
    assert_eq!(p.tile_height, 256);
}

#[test]
fn wmts_kvp_url() {
    let p = WmtsImageryProvider::new("https://wmts.example.com", "roads");
    let coord = TileCoord::new(5, 10, 3);
    let url = p.get_tile_url_kvp(&coord);

    assert!(url.contains("service=WMTS"));
    assert!(url.contains("request=GetTile"));
    assert!(url.contains("layer=roads"));
    assert!(url.contains("style=default"));
    assert!(url.contains("tilematrixset=GoogleMapsCompatible"));
    assert!(url.contains("tilematrix=3"));
    assert!(url.contains("tilerow=10"));
    assert!(url.contains("tilecol=5"));
    assert!(url.contains("format=image/jpeg"));
}

#[test]
fn wmts_kvp_url_with_existing_query() {
    let p = WmtsImageryProvider::new("https://wmts.example.com?token=abc", "layer1");
    let coord = TileCoord::new(0, 0, 0);
    let url = p.get_tile_url_kvp(&coord);
    assert!(url.contains("?token=abc&service=WMTS"));
}

#[test]
fn wmts_rest_url() {
    let p = WmtsImageryProvider::new("https://wmts.example.com/", "satellite");
    let coord = TileCoord::new(3, 7, 4);
    let url = p.get_tile_url_rest(&coord);
    assert_eq!(
        url,
        "https://wmts.example.com/satellite/default/GoogleMapsCompatible/4/7/3.jpg"
    );
}

#[test]
fn wmts_rest_url_png_format() {
    let p = WmtsImageryProvider::new("https://wmts.example.com", "layer")
        .with_format("image/png");
    let coord = TileCoord::new(1, 2, 3);
    let url = p.get_tile_url_rest(&coord);
    assert!(url.ends_with(".png"));
}

#[test]
fn wmts_with_tile_matrix_set() {
    let p = WmtsImageryProvider::new("https://wmts.example.com", "layer")
        .with_tile_matrix_set("EPSG:4326");
    assert_eq!(p.tile_matrix_set_id, "EPSG:4326");
}

// === TmsImageryProvider ===

#[test]
fn tms_defaults() {
    let p = TmsImageryProvider::new("https://tms.example.com/tiles");
    assert_eq!(p.file_extension, "png");
    assert_eq!(p.minimum_level, 0);
    assert_eq!(p.maximum_level, 25);
}

#[test]
fn tms_reverse_y_url() {
    let p = TmsImageryProvider::new("https://tms.example.com/tiles");
    let coord = TileCoord::new(3, 5, 4);
    // At level 4, tiles_y = 16, tms_y = 16 - 1 - 5 = 10
    let url = p.get_tile_url(&coord);
    assert_eq!(url, "https://tms.example.com/tiles/4/3/10.png");
}

#[test]
fn tms_reverse_y_level_0() {
    let p = TmsImageryProvider::new("https://tms.example.com");
    let coord = TileCoord::new(0, 0, 0);
    // At level 0, tiles_y = 1, tms_y = 1 - 1 - 0 = 0
    let url = p.get_tile_url(&coord);
    assert_eq!(url, "https://tms.example.com/0/0/0.png");
}

#[test]
fn tms_trailing_slash() {
    let p = TmsImageryProvider::new("https://tms.example.com/");
    let coord = TileCoord::new(1, 1, 1);
    // At level 1, tiles_y = 2, tms_y = 2 - 1 - 1 = 0
    let url = p.get_tile_url(&coord);
    assert_eq!(url, "https://tms.example.com/1/1/0.png");
}

// === OpenStreetMapImageryProvider ===

#[test]
fn osm_defaults() {
    let p = OpenStreetMapImageryProvider::new();
    assert_eq!(p.url, "https://tile.openstreetmap.org");
    assert_eq!(p.maximum_level, 19);
    assert!(p.credit.is_some());
}

#[test]
fn osm_tile_url() {
    let p = OpenStreetMapImageryProvider::new();
    let coord = TileCoord::new(3, 5, 4);
    let url = p.get_tile_url(&coord);
    assert_eq!(url, "https://tile.openstreetmap.org/4/3/5.png");
}

#[test]
fn osm_tile_url_level_0() {
    let p = OpenStreetMapImageryProvider::new();
    let coord = TileCoord::new(0, 0, 0);
    let url = p.get_tile_url(&coord);
    assert_eq!(url, "https://tile.openstreetmap.org/0/0/0.png");
}

// === BingMapsImageryProvider ===

#[test]
fn bing_defaults() {
    let p = BingMapsImageryProvider::new("test-key");
    assert_eq!(p.key, "test-key");
    assert_eq!(p.map_style, BingMapStyle::Aerial);
    assert_eq!(p.culture, "en-US");
}

#[test]
fn bing_quadkey_level_1() {
    // Level 1: 4 quadrants
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(0, 0, 1)),
        "0"
    );
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(1, 0, 1)),
        "1"
    );
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(0, 1, 1)),
        "2"
    );
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(1, 1, 1)),
        "3"
    );
}

#[test]
fn bing_quadkey_level_2() {
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(3, 3, 2)),
        "33"
    );
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(0, 0, 2)),
        "00"
    );
}

#[test]
fn bing_quadkey_level_3() {
    // x=5 (101), y=3 (011) at level 3
    // i=2: mask=4, x&4=4→+1, y&4=0→digit=1
    // i=1: mask=2, x&2=0, y&2=2→+2→digit=2
    // i=0: mask=1, x&1=1→+1, y&1=1→+2→digit=3
    assert_eq!(
        BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(5, 3, 3)),
        "123"
    );
}

#[test]
fn bing_tile_url_contains_quadkey() {
    let p = BingMapsImageryProvider::new("key");
    let coord = TileCoord::new(1, 1, 1);
    let url = p.get_tile_url(&coord);
    assert!(url.contains("aerial3"));
    assert!(url.contains("virtualearth.net"));
}

#[test]
fn bing_style_imagery_set() {
    assert_eq!(BingMapStyle::Aerial.imagery_set(), "Aerial");
    assert_eq!(BingMapStyle::Road.imagery_set(), "Road");
    assert_eq!(BingMapStyle::AerialWithLabels.imagery_set(), "AerialWithLabels");
    assert_eq!(BingMapStyle::CanvasDark.imagery_set(), "CanvasDark");
    assert_eq!(BingMapStyle::CanvasLight.imagery_set(), "CanvasLight");
}
