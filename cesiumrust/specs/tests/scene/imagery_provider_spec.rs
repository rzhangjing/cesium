//! ImageryProvider specs - ported from Scene/*ImageryProviderSpec
//! Covers: ImageryProviderDescriptor, ImageryProviderKind, BingMapStyle,
//! UrlTemplate, WMTS, WMS, TMS, OSM, Bing providers

use cesium_provider::imagery_provider::{
    BingMapStyle, BingMapsImageryProvider, ImageryProviderDescriptor, ImageryProviderKind,
    OpenStreetMapImageryProvider, TileCoord, TmsImageryProvider, UrlTemplateImageryProvider,
    WmsImageryProvider, WmtsImageryProvider,
};

// ─── ImageryProviderKind ────────────────────────────────────────────────────

#[test]
fn imagery_provider_kind_url_template() {
    let provider = UrlTemplateImageryProvider::new("https://example.com/{z}/{x}/{y}.png");
    let kind = ImageryProviderKind::UrlTemplate(provider);
    assert!(matches!(kind, ImageryProviderKind::UrlTemplate(_)));
}

#[test]
fn imagery_provider_kind_wmts() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1");
    let kind = ImageryProviderKind::Wmts(provider);
    assert!(matches!(kind, ImageryProviderKind::Wmts(_)));
}

#[test]
fn imagery_provider_kind_wms() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    let kind = ImageryProviderKind::Wms(provider);
    assert!(matches!(kind, ImageryProviderKind::Wms(_)));
}

#[test]
fn imagery_provider_kind_tms() {
    let provider = TmsImageryProvider::new("https://tms.example.com");
    let kind = ImageryProviderKind::Tms(provider);
    assert!(matches!(kind, ImageryProviderKind::Tms(_)));
}

#[test]
fn imagery_provider_kind_osm() {
    let provider = OpenStreetMapImageryProvider::new();
    let kind = ImageryProviderKind::Osm(provider);
    assert!(matches!(kind, ImageryProviderKind::Osm(_)));
}

#[test]
fn imagery_provider_kind_bing() {
    let provider = BingMapsImageryProvider::new("fake_key");
    let kind = ImageryProviderKind::Bing(provider);
    assert!(matches!(kind, ImageryProviderKind::Bing(_)));
}

// ─── BingMapStyle ───────────────────────────────────────────────────────────

#[test]
fn bing_map_style_variants() {
    assert_ne!(BingMapStyle::Aerial, BingMapStyle::Road);
    assert_ne!(BingMapStyle::Road, BingMapStyle::AerialWithLabels);
    assert_ne!(BingMapStyle::CanvasDark, BingMapStyle::CanvasLight);
}

#[test]
fn bing_map_style_imagery_set() {
    assert_eq!(BingMapStyle::Aerial.imagery_set(), "Aerial");
    assert_eq!(BingMapStyle::Road.imagery_set(), "Road");
    assert_eq!(BingMapStyle::AerialWithLabels.imagery_set(), "AerialWithLabels");
}

// ─── UrlTemplateImageryProvider ─────────────────────────────────────────────

#[test]
fn url_template_provider_creation() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png");
    assert!(provider.url_template.contains("{z}"));
    assert_eq!(provider.minimum_level, 0);
    assert_eq!(provider.maximum_level, 25);
    assert_eq!(provider.tile_width, 256);
    assert_eq!(provider.tile_height, 256);
}

#[test]
fn url_template_provider_builder() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png")
        .with_max_level(18)
        .with_tile_size(512, 512);
    assert_eq!(provider.maximum_level, 18);
    assert_eq!(provider.tile_width, 512);
    assert_eq!(provider.tile_height, 512);
}

#[test]
fn url_template_get_tile_url() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png");
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url(&coord, 0);
    assert_eq!(url, "https://tile.example.com/4/3/5.png");
}

#[test]
fn url_template_reverse_y() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{reverseY}.png");
    let coord = TileCoord::new(0, 0, 1);
    let url = provider.get_tile_url(&coord, 0);
    // At level 1, tiles_y = 2, reverseY = 2 - 1 - 0 = 1
    assert_eq!(url, "https://tile.example.com/1/0/1.png");
}

#[test]
fn url_template_subdomains() {
    let provider = UrlTemplateImageryProvider::new("https://{s}.tile.example.com/{z}/{x}/{y}.png")
        .with_subdomains(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let coord = TileCoord::new(1, 2, 3);
    let url = provider.get_tile_url(&coord, 1);
    assert!(url.starts_with("https://b.tile.example.com/"));
}

#[test]
fn url_template_is_available() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png")
        .with_max_level(18);
    assert!(provider.is_available(0));
    assert!(provider.is_available(18));
    assert!(!provider.is_available(19));
}

// ─── WmtsImageryProvider ────────────────────────────────────────────────────

#[test]
fn wmts_provider_creation() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1");
    assert_eq!(provider.layer, "layer1");
    assert_eq!(provider.style, "default");
    assert_eq!(provider.tile_matrix_set_id, "GoogleMapsCompatible");
    assert_eq!(provider.format, "image/jpeg");
}

#[test]
fn wmts_provider_builder() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1")
        .with_tile_matrix_set("EPSG:4326")
        .with_format("image/png");
    assert_eq!(provider.tile_matrix_set_id, "EPSG:4326");
    assert_eq!(provider.format, "image/png");
}

#[test]
fn wmts_get_tile_url_kvp() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1");
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url_kvp(&coord);
    assert!(url.contains("service=WMTS"));
    assert!(url.contains("layer=layer1"));
    assert!(url.contains("tilerow=5"));
    assert!(url.contains("tilecol=3"));
}

#[test]
fn wmts_get_tile_url_rest() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1");
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url_rest(&coord);
    assert!(url.contains("layer1/default/GoogleMapsCompatible/4/5/3"));
}

// ─── WmsImageryProvider ─────────────────────────────────────────────────────

#[test]
fn wms_provider_creation() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    assert_eq!(provider.layers, "layer1");
    assert_eq!(provider.format, "image/png");
    assert!(provider.transparent);
    assert_eq!(provider.crs, "EPSG:4326");
}

#[test]
fn wms_get_tile_url() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    let coord = TileCoord::new(0, 0, 0);
    let url = provider.get_tile_url(&coord);
    assert!(url.contains("service=WMS"));
    assert!(url.contains("request=GetMap"));
    assert!(url.contains("layers=layer1"));
}

// ─── TmsImageryProvider ─────────────────────────────────────────────────────

#[test]
fn tms_provider_creation() {
    let provider = TmsImageryProvider::new("https://tms.example.com");
    assert!(provider.url.contains("tms"));
    assert_eq!(provider.file_extension, "png");
}

#[test]
fn tms_get_tile_url() {
    let provider = TmsImageryProvider::new("https://tms.example.com");
    let coord = TileCoord::new(1, 0, 1);
    let url = provider.get_tile_url(&coord);
    // TMS reverses Y: tiles_y=2, tms_y = 2-1-0 = 1
    assert_eq!(url, "https://tms.example.com/1/1/1.png");
}

// ─── OpenStreetMapImageryProvider ───────────────────────────────────────────

#[test]
fn osm_provider_creation() {
    let provider = OpenStreetMapImageryProvider::new();
    assert!(provider.url.contains("tile.openstreetmap.org"));
    assert_eq!(provider.maximum_level, 19);
    assert!(provider.credit.is_some());
}

#[test]
fn osm_get_tile_url() {
    let provider = OpenStreetMapImageryProvider::new();
    let coord = TileCoord::new(3, 5, 4);
    let url = provider.get_tile_url(&coord);
    assert_eq!(url, "https://tile.openstreetmap.org/4/3/5.png");
}

// ─── BingMapsImageryProvider ────────────────────────────────────────────────

#[test]
fn bing_provider_creation() {
    let provider = BingMapsImageryProvider::new("fake_key");
    assert_eq!(provider.key, "fake_key");
    assert_eq!(provider.map_style, BingMapStyle::Aerial);
    assert_eq!(provider.culture, "en-US");
}

#[test]
fn bing_tile_to_quadkey() {
    let coord = TileCoord::new(3, 5, 3);
    let quadkey = BingMapsImageryProvider::tile_to_quadkey(&coord);
    assert_eq!(quadkey.len(), 3);
}

#[test]
fn bing_get_tile_url() {
    let provider = BingMapsImageryProvider::new("fake_key");
    let coord = TileCoord::new(1, 1, 2);
    let url = provider.get_tile_url(&coord);
    assert!(url.contains("virtualearth.net"));
    assert!(url.contains("aerial"));
}

// ─── ImageryProviderDescriptor ──────────────────────────────────────────────

#[test]
fn descriptor_url_template() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png");
    let desc = ImageryProviderDescriptor::url_template(provider);
    assert!(matches!(desc.kind, ImageryProviderKind::UrlTemplate(_)));
    assert_eq!(desc.tile_width, 256);
    assert_eq!(desc.tile_height, 256);
    assert!(!desc.has_time_dynamic);
}

#[test]
fn descriptor_wmts() {
    let provider = WmtsImageryProvider::new("https://wmts.example.com", "layer1");
    let desc = ImageryProviderDescriptor::wmts(provider);
    assert!(matches!(desc.kind, ImageryProviderKind::Wmts(_)));
    assert_eq!(desc.maximum_level, 25);
}

#[test]
fn descriptor_wms() {
    let provider = WmsImageryProvider::new("https://wms.example.com", "layer1");
    let desc = ImageryProviderDescriptor::wms(provider);
    assert!(matches!(desc.kind, ImageryProviderKind::Wms(_)));
}

#[test]
fn descriptor_osm() {
    let provider = OpenStreetMapImageryProvider::new();
    let desc = ImageryProviderDescriptor::osm(provider);
    assert!(matches!(desc.kind, ImageryProviderKind::Osm(_)));
    assert_eq!(desc.maximum_level, 19);
}

#[test]
fn descriptor_get_tile_url() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png");
    let desc = ImageryProviderDescriptor::url_template(provider);
    let coord = TileCoord::new(1, 2, 3);
    let url = desc.get_tile_url(&coord, 0);
    assert_eq!(url, "https://tile.example.com/3/1/2.png");
}

#[test]
fn descriptor_is_available() {
    let provider = UrlTemplateImageryProvider::new("https://tile.example.com/{z}/{x}/{y}.png")
        .with_max_level(18);
    let desc = ImageryProviderDescriptor::url_template(provider);
    assert!(desc.is_available(10));
    assert!(!desc.is_available(19));
}

// ─── TileCoord ──────────────────────────────────────────────────────────────

#[test]
fn tile_coord_creation() {
    let coord = TileCoord::new(5, 10, 3);
    assert_eq!(coord.x, 5);
    assert_eq!(coord.y, 10);
    assert_eq!(coord.level, 3);
}
