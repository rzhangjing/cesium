//! Imagery providers for tile-based map services.
//!
//! Maps to CesiumJS imagery providers:
//! - `UrlTemplateImageryProvider`
//! - `WebMapTileServiceImageryProvider` (WMTS)
//! - `WebMapServiceImageryProvider` (WMS)
//! - `TileMapServiceImageryProvider` (TMS)
//! - `OpenStreetMapImageryProvider`
//! - `BingMapsImageryProvider`

use std::collections::HashMap;

/// Tile coordinate (x, y, level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    /// Tile column (x).
    pub x: u32,
    /// Tile row (y).
    pub y: u32,
    /// Zoom level.
    pub level: u32,
}

impl TileCoord {
    /// Creates a new tile coordinate.
    pub fn new(x: u32, y: u32, level: u32) -> Self {
        Self { x, y, level }
    }
}

/// Subdomain selection strategy for load balancing.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SubdomainStrategy {
    /// No subdomains.
    #[default]
    None,
    /// Round-robin through subdomains.
    RoundRobin(Vec<String>),
}

/// An imagery provider that generates tile URLs from a template.
///
/// Maps to CesiumJS `UrlTemplateImageryProvider`
#[derive(Debug, Clone)]
pub struct UrlTemplateImageryProvider {
    /// URL template with placeholders: {x}, {y}, {z}, {s}, {reverseY}.
    pub url_template: String,
    /// Minimum zoom level.
    pub minimum_level: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Subdomain strategy.
    pub subdomains: SubdomainStrategy,
    /// Credit/attribution string.
    pub credit: Option<String>,
}

impl UrlTemplateImageryProvider {
    /// Creates a new URL template imagery provider.
    pub fn new(url_template: impl Into<String>) -> Self {
        Self {
            url_template: url_template.into(),
            minimum_level: 0,
            maximum_level: 25,
            tile_width: 256,
            tile_height: 256,
            subdomains: SubdomainStrategy::None,
            credit: None,
        }
    }

    /// Sets the maximum zoom level.
    pub fn with_max_level(mut self, level: u32) -> Self {
        self.maximum_level = level;
        self
    }

    /// Sets the tile size.
    pub fn with_tile_size(mut self, width: u32, height: u32) -> Self {
        self.tile_width = width;
        self.tile_height = height;
        self
    }

    /// Sets subdomains for load balancing.
    pub fn with_subdomains(mut self, subdomains: Vec<String>) -> Self {
        self.subdomains = SubdomainStrategy::RoundRobin(subdomains);
        self
    }

    /// Generates the URL for a given tile.
    pub fn get_tile_url(&self, coord: &TileCoord, subdomain_index: usize) -> String {
        let mut url = self.url_template.clone();

        url = url.replace("{x}", &coord.x.to_string());
        url = url.replace("{y}", &coord.y.to_string());
        url = url.replace("{z}", &coord.level.to_string());

        // Reverse Y (TMS-style: origin at bottom-left)
        let tiles_y = 1u32 << coord.level;
        let reverse_y = tiles_y - 1 - coord.y;
        url = url.replace("{reverseY}", &reverse_y.to_string());

        // Subdomain
        if let SubdomainStrategy::RoundRobin(subdomains) = &self.subdomains {
            if !subdomains.is_empty() {
                let s = &subdomains[subdomain_index % subdomains.len()];
                url = url.replace("{s}", s);
            }
        } else {
            url = url.replace("{s}", "");
        }

        url
    }

    /// Checks if a tile is available at the given level.
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// A WMTS (Web Map Tile Service) imagery provider.
///
/// Maps to CesiumJS `WebMapTileServiceImageryProvider`
#[derive(Debug, Clone)]
pub struct WmtsImageryProvider {
    /// Base URL of the WMTS service.
    pub url: String,
    /// Layer identifier.
    pub layer: String,
    /// Style identifier.
    pub style: String,
    /// Tile matrix set identifier.
    pub tile_matrix_set_id: String,
    /// Image format (e.g., "image/png").
    pub format: String,
    /// Minimum zoom level.
    pub minimum_level: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl WmtsImageryProvider {
    /// Creates a new WMTS provider.
    pub fn new(url: impl Into<String>, layer: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            layer: layer.into(),
            style: "default".to_string(),
            tile_matrix_set_id: "GoogleMapsCompatible".to_string(),
            format: "image/jpeg".to_string(),
            minimum_level: 0,
            maximum_level: 25,
            tile_width: 256,
            tile_height: 256,
            credit: None,
        }
    }

    /// Sets the tile matrix set ID.
    pub fn with_tile_matrix_set(mut self, id: impl Into<String>) -> Self {
        self.tile_matrix_set_id = id.into();
        self
    }

    /// Sets the image format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    /// Generates a KVP (Key-Value Pair) request URL for a tile.
    pub fn get_tile_url_kvp(&self, coord: &TileCoord) -> String {
        let separator = if self.url.contains('?') { "&" } else { "?" };
        format!(
            "{}{}service=WMTS&version=1.0.0&request=GetTile&layer={}&style={}&tilematrixset={}&tilematrix={}&tilerow={}&tilecol={}&format={}",
            self.url,
            separator,
            self.layer,
            self.style,
            self.tile_matrix_set_id,
            coord.level,
            coord.y,
            coord.x,
            self.format,
        )
    }

    /// Generates a RESTful request URL for a tile.
    pub fn get_tile_url_rest(&self, coord: &TileCoord) -> String {
        let base = self.url.trim_end_matches('/');
        format!(
            "{}/{}/{}/{}/{}/{}/{}.{}",
            base,
            self.layer,
            self.style,
            self.tile_matrix_set_id,
            coord.level,
            coord.y,
            coord.x,
            self.format_extension(),
        )
    }

    /// Gets the file extension from the format.
    fn format_extension(&self) -> &str {
        match self.format.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            "image/tiff" => "tiff",
            _ => "png",
        }
    }
}

/// A WMS (Web Map Service) imagery provider.
///
/// Maps to CesiumJS `WebMapServiceImageryProvider`
#[derive(Debug, Clone)]
pub struct WmsImageryProvider {
    /// Base URL of the WMS service.
    pub url: String,
    /// Comma-separated layer names.
    pub layers: String,
    /// Image format.
    pub format: String,
    /// Whether to use transparent background.
    pub transparent: bool,
    /// CRS/SRS identifier.
    pub crs: String,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Additional parameters.
    pub parameters: HashMap<String, String>,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl WmsImageryProvider {
    /// Creates a new WMS provider.
    pub fn new(url: impl Into<String>, layers: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            layers: layers.into(),
            format: "image/png".to_string(),
            transparent: true,
            crs: "EPSG:4326".to_string(),
            tile_width: 256,
            tile_height: 256,
            parameters: HashMap::new(),
            credit: None,
        }
    }

    /// Generates a GetMap request URL for a tile.
    ///
    /// Computes the bounding box from tile coordinates assuming
    /// a geographic tiling scheme (EPSG:4326).
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        // Geographic tiling scheme: 2 tiles wide at level 0
        let tiles_x = 2u32 << coord.level;
        let tiles_y = 1u32 << coord.level;

        let west = -180.0 + (coord.x as f64 / tiles_x as f64) * 360.0;
        let east = -180.0 + ((coord.x + 1) as f64 / tiles_x as f64) * 360.0;
        let north = 90.0 - (coord.y as f64 / tiles_y as f64) * 180.0;
        let south = 90.0 - ((coord.y + 1) as f64 / tiles_y as f64) * 180.0;

        let separator = if self.url.contains('?') { "&" } else { "?" };
        let mut url = format!(
            "{}{}service=WMS&version=1.3.0&request=GetMap&layers={}&styles=&crs={}&bbox={},{},{},{}&width={}&height={}&format={}&transparent={}",
            self.url,
            separator,
            self.layers,
            self.crs,
            south, west, north, east,
            self.tile_width,
            self.tile_height,
            self.format,
            self.transparent,
        );

        for (key, value) in &self.parameters {
            url.push_str(&format!("&{}={}", key, value));
        }

        url
    }
}

/// A TMS (Tile Map Service) imagery provider.
///
/// Maps to CesiumJS `TileMapServiceImageryProvider`
#[derive(Debug, Clone)]
pub struct TmsImageryProvider {
    /// Base URL of the TMS service.
    pub url: String,
    /// File extension (png, jpg).
    pub file_extension: String,
    /// Minimum zoom level.
    pub minimum_level: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl TmsImageryProvider {
    /// Creates a new TMS provider.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            file_extension: "png".to_string(),
            minimum_level: 0,
            maximum_level: 25,
            credit: None,
        }
    }

    /// Generates the URL for a tile.
    /// TMS uses bottom-left origin (reverse Y).
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let tiles_y = 1u32 << coord.level;
        let tms_y = tiles_y - 1 - coord.y;
        let base = self.url.trim_end_matches('/');
        format!(
            "{}/{}/{}/{}.{}",
            base, coord.level, coord.x, tms_y, self.file_extension
        )
    }
}

/// OpenStreetMap imagery provider.
///
/// Maps to CesiumJS `OpenStreetMapImageryProvider`
#[derive(Debug, Clone)]
pub struct OpenStreetMapImageryProvider {
    /// Base URL.
    pub url: String,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl Default for OpenStreetMapImageryProvider {
    fn default() -> Self {
        Self {
            url: "https://tile.openstreetmap.org".to_string(),
            maximum_level: 19,
            credit: Some("© OpenStreetMap contributors".to_string()),
        }
    }
}

impl OpenStreetMapImageryProvider {
    /// Creates a new OSM provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates the URL for a tile.
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/{}/{}/{}.png", base, coord.level, coord.x, coord.y)
    }
}

/// Bing Maps imagery provider.
///
/// Maps to CesiumJS `BingMapsImageryProvider`
#[derive(Debug, Clone)]
pub struct BingMapsImageryProvider {
    /// Bing Maps key.
    pub key: String,
    /// Map style (Aerial, Road, AerialWithLabels).
    pub map_style: BingMapStyle,
    /// Culture (language).
    pub culture: String,
}

/// Bing Maps style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BingMapStyle {
    /// Aerial imagery.
    Aerial,
    /// Road map.
    Road,
    /// Aerial with labels.
    AerialWithLabels,
    /// Canvas dark.
    CanvasDark,
    /// Canvas light.
    CanvasLight,
}

impl BingMapStyle {
    /// Gets the Bing Maps quadkey imagery set.
    pub fn imagery_set(&self) -> &str {
        match self {
            Self::Aerial => "Aerial",
            Self::Road => "Road",
            Self::AerialWithLabels => "AerialWithLabels",
            Self::CanvasDark => "CanvasDark",
            Self::CanvasLight => "CanvasLight",
        }
    }
}

impl BingMapsImageryProvider {
    /// Creates a new Bing Maps provider.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            map_style: BingMapStyle::Aerial,
            culture: "en-US".to_string(),
        }
    }

    /// Converts tile coordinates to a Bing Maps quadkey.
    pub fn tile_to_quadkey(coord: &TileCoord) -> String {
        let mut quadkey = String::with_capacity(coord.level as usize);
        for i in (0..coord.level).rev() {
            let mut digit = 0u8;
            let mask = 1u32 << i;
            if coord.x & mask != 0 {
                digit += 1;
            }
            if coord.y & mask != 0 {
                digit += 2;
            }
            quadkey.push(char::from(b'0' + digit));
        }
        quadkey
    }

    /// Generates a Bing Maps tile URL (simplified, without metadata).
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let quadkey = Self::tile_to_quadkey(coord);
        // Subdomain based on quadkey hash
        let subdomain = (coord.x + coord.y) % 4;
        format!(
            "https://ecn.t{}.tiles.virtualearth.net/tiles/{}{}.jpeg?g=1&mkt={}",
            subdomain,
            self.map_style.imagery_set().to_lowercase(),
            quadkey,
            self.culture,
        )
    }
}

/// A unified imagery provider descriptor with tiling scheme.
///
/// Maps to CesiumJS `ImageryProvider` base interface
#[derive(Debug, Clone)]
pub struct ImageryProviderDescriptor {
    /// The provider kind.
    pub kind: ImageryProviderKind,
    /// The tiling scheme used by this provider.
    pub tiling_scheme: crate::tiling_scheme::TilingScheme,
    /// Minimum zoom level.
    pub minimum_level: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
    /// Whether the provider supports time-dynamic imagery.
    pub has_time_dynamic: bool,
}

/// The kind of imagery provider.
#[derive(Debug, Clone)]
pub enum ImageryProviderKind {
    /// URL template provider.
    UrlTemplate(UrlTemplateImageryProvider),
    /// WMTS provider.
    Wmts(WmtsImageryProvider),
    /// WMS provider.
    Wms(WmsImageryProvider),
    /// TMS provider.
    Tms(TmsImageryProvider),
    /// OpenStreetMap provider.
    Osm(OpenStreetMapImageryProvider),
    /// Bing Maps provider.
    Bing(BingMapsImageryProvider),
}

impl ImageryProviderDescriptor {
    /// Creates a descriptor from a URL template provider.
    pub fn url_template(provider: UrlTemplateImageryProvider) -> Self {
        let max_level = provider.maximum_level;
        Self {
            tiling_scheme: crate::tiling_scheme::TilingScheme::web_mercator(),
            minimum_level: provider.minimum_level,
            maximum_level: max_level,
            tile_width: provider.tile_width,
            tile_height: provider.tile_height,
            credit: provider.credit.clone(),
            has_time_dynamic: false,
            kind: ImageryProviderKind::UrlTemplate(provider),
        }
    }

    /// Creates a descriptor from a WMTS provider.
    pub fn wmts(provider: WmtsImageryProvider) -> Self {
        let max_level = provider.maximum_level;
        Self {
            tiling_scheme: crate::tiling_scheme::TilingScheme::web_mercator(),
            minimum_level: provider.minimum_level,
            maximum_level: max_level,
            tile_width: provider.tile_width,
            tile_height: provider.tile_height,
            credit: provider.credit.clone(),
            has_time_dynamic: false,
            kind: ImageryProviderKind::Wmts(provider),
        }
    }

    /// Creates a descriptor from a WMS provider.
    pub fn wms(provider: WmsImageryProvider) -> Self {
        Self {
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            minimum_level: 0,
            maximum_level: 25,
            tile_width: provider.tile_width,
            tile_height: provider.tile_height,
            credit: provider.credit.clone(),
            has_time_dynamic: false,
            kind: ImageryProviderKind::Wms(provider),
        }
    }

    /// Creates a descriptor from an OSM provider.
    pub fn osm(provider: OpenStreetMapImageryProvider) -> Self {
        let max_level = provider.maximum_level;
        Self {
            tiling_scheme: crate::tiling_scheme::TilingScheme::web_mercator(),
            minimum_level: 0,
            maximum_level: max_level,
            tile_width: 256,
            tile_height: 256,
            credit: provider.credit.clone(),
            has_time_dynamic: false,
            kind: ImageryProviderKind::Osm(provider),
        }
    }

    /// Gets the tile URL for a given coordinate.
    pub fn get_tile_url(&self, coord: &TileCoord, subdomain_index: usize) -> String {
        match &self.kind {
            ImageryProviderKind::UrlTemplate(p) => p.get_tile_url(coord, subdomain_index),
            ImageryProviderKind::Wmts(p) => p.get_tile_url_kvp(coord),
            ImageryProviderKind::Wms(p) => p.get_tile_url(coord),
            ImageryProviderKind::Tms(p) => p.get_tile_url(coord),
            ImageryProviderKind::Osm(p) => p.get_tile_url(coord),
            ImageryProviderKind::Bing(p) => p.get_tile_url(coord),
        }
    }

    /// Checks if a tile is available at the given level.
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// Time-dynamic imagery interval.
/// Maps to CesiumJS time-dynamic imagery support
#[derive(Debug, Clone)]
pub struct TimeDynamicInterval {
    /// Start time (seconds since epoch).
    pub start: f64,
    /// Stop time (seconds since epoch).
    pub stop: f64,
    /// URL template for this interval (may include {time} placeholder).
    pub url_template: String,
}

/// Time-dynamic imagery provider.
///
/// Maps to CesiumJS `TimeDynamicImagery`
#[derive(Debug, Clone)]
pub struct TimeDynamicImagery {
    /// Time intervals with associated URLs.
    pub intervals: Vec<TimeDynamicInterval>,
    /// Whether to interpolate between intervals.
    pub interpolate: bool,
}

impl TimeDynamicImagery {
    /// Creates a new time-dynamic imagery provider.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
            interpolate: false,
        }
    }

    /// Adds a time interval.
    pub fn add_interval(&mut self, start: f64, stop: f64, url_template: impl Into<String>) {
        self.intervals.push(TimeDynamicInterval {
            start,
            stop,
            url_template: url_template.into(),
        });
    }

    /// Gets the URL for a given time and tile coordinate.
    pub fn get_tile_url(&self, time: f64, coord: &TileCoord) -> Option<String> {
        // Find the interval containing this time
        let interval = self.intervals.iter().find(|i| time >= i.start && time <= i.stop)?;

        // Replace placeholders
        let mut url = interval.url_template.clone();
        url = url.replace("{x}", &coord.x.to_string());
        url = url.replace("{y}", &coord.y.to_string());
        url = url.replace("{z}", &coord.level.to_string());
        url = url.replace("{time}", &time.to_string());

        Some(url)
    }

    /// Returns the number of intervals.
    pub fn interval_count(&self) -> usize {
        self.intervals.len()
    }
}

impl Default for TimeDynamicImagery {
    fn default() -> Self {
        Self::new()
    }
}

/// WMS GetFeatureInfo request builder.
///
/// Maps to CesiumJS WMS GetFeatureInfo support
#[derive(Debug, Clone)]
pub struct WmsGetFeatureInfo {
    /// Base URL of the WMS service.
    pub url: String,
    /// Comma-separated layer names.
    pub layers: String,
    /// Info format (e.g., "application/json", "text/html").
    pub info_format: String,
    /// CRS/SRS identifier.
    pub crs: String,
    /// Feature count limit.
    pub feature_count: u32,
}

impl WmsGetFeatureInfo {
    /// Creates a new GetFeatureInfo builder.
    pub fn new(url: impl Into<String>, layers: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            layers: layers.into(),
            info_format: "application/json".to_string(),
            crs: "EPSG:4326".to_string(),
            feature_count: 10,
        }
    }

    /// Sets the info format.
    pub fn with_info_format(mut self, format: impl Into<String>) -> Self {
        self.info_format = format.into();
        self
    }

    /// Generates a GetFeatureInfo request URL.
    ///
    /// # Arguments
    /// * `bbox` - Bounding box [west, south, east, north] in degrees
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `x` - Click x coordinate in pixels
    /// * `y` - Click y coordinate in pixels
    pub fn get_url(
        &self,
        bbox: [f64; 4],
        width: u32,
        height: u32,
        x: u32,
        y: u32,
    ) -> String {
        let separator = if self.url.contains('?') { "&" } else { "?" };
        format!(
            "{}{}service=WMS&version=1.3.0&request=GetFeatureInfo&layers={}&query_layers={}&crs={}&bbox={},{},{},{}&width={}&height={}&i={}&j={}&feature_count={}&info_format={}",
            self.url,
            separator,
            self.layers,
            self.layers,
            self.crs,
            bbox[1], bbox[0], bbox[3], bbox[2],
            width,
            height,
            x,
            y,
            self.feature_count,
            self.info_format,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_template_basic() {
        let provider = UrlTemplateImageryProvider::new(
            "https://example.com/{z}/{x}/{y}.png",
        );

        let url = provider.get_tile_url(&TileCoord::new(3, 2, 2), 0);
        assert_eq!(url, "https://example.com/2/3/2.png");
    }

    #[test]
    fn test_url_template_reverse_y() {
        let provider = UrlTemplateImageryProvider::new(
            "https://example.com/{z}/{x}/{reverseY}.png",
        );

        // Level 2: tiles_y = 4, reverseY = 4 - 1 - 1 = 2
        let url = provider.get_tile_url(&TileCoord::new(0, 1, 2), 0);
        assert_eq!(url, "https://example.com/2/0/2.png");
    }

    #[test]
    fn test_url_template_subdomains() {
        let provider = UrlTemplateImageryProvider::new(
            "https://{s}.example.com/{z}/{x}/{y}.png",
        )
        .with_subdomains(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        let url0 = provider.get_tile_url(&TileCoord::new(0, 0, 1), 0);
        assert_eq!(url0, "https://a.example.com/1/0/0.png");

        let url1 = provider.get_tile_url(&TileCoord::new(0, 0, 1), 1);
        assert_eq!(url1, "https://b.example.com/1/0/0.png");

        let url3 = provider.get_tile_url(&TileCoord::new(0, 0, 1), 3);
        assert_eq!(url3, "https://a.example.com/1/0/0.png"); // Wraps around
    }

    #[test]
    fn test_url_template_availability() {
        let provider = UrlTemplateImageryProvider::new("https://example.com/{z}/{x}/{y}.png")
            .with_max_level(18);

        assert!(provider.is_available(0));
        assert!(provider.is_available(18));
        assert!(!provider.is_available(19));
    }

    #[test]
    fn test_wmts_kvp() {
        let provider = WmtsImageryProvider::new(
            "https://example.com/wmts",
            "satellite",
        );

        let url = provider.get_tile_url_kvp(&TileCoord::new(1, 2, 3));
        assert!(url.contains("service=WMTS"));
        assert!(url.contains("layer=satellite"));
        assert!(url.contains("tilecol=1"));
        assert!(url.contains("tilerow=2"));
        assert!(url.contains("tilematrix=3"));
    }

    #[test]
    fn test_wmts_rest() {
        let provider = WmtsImageryProvider::new(
            "https://example.com/wmts",
            "satellite",
        )
        .with_format("image/png");

        let url = provider.get_tile_url_rest(&TileCoord::new(1, 2, 3));
        assert_eq!(
            url,
            "https://example.com/wmts/satellite/default/GoogleMapsCompatible/3/2/1.png"
        );
    }

    #[test]
    fn test_wms_get_map() {
        let provider = WmsImageryProvider::new(
            "https://example.com/wms",
            "roads,cities",
        );

        let url = provider.get_tile_url(&TileCoord::new(0, 0, 0));
        assert!(url.contains("service=WMS"));
        assert!(url.contains("request=GetMap"));
        assert!(url.contains("layers=roads,cities"));
        assert!(url.contains("bbox="));
    }

    #[test]
    fn test_tms_url() {
        let provider = TmsImageryProvider::new("https://example.com/tms");

        // Level 1: tiles_y = 2, tms_y = 2 - 1 - 0 = 1
        let url = provider.get_tile_url(&TileCoord::new(0, 0, 1));
        assert_eq!(url, "https://example.com/tms/1/0/1.png");
    }

    #[test]
    fn test_osm_url() {
        let provider = OpenStreetMapImageryProvider::new();
        let url = provider.get_tile_url(&TileCoord::new(1, 2, 3));
        assert_eq!(url, "https://tile.openstreetmap.org/3/1/2.png");
    }

    #[test]
    fn test_bing_quadkey() {
        // Known quadkey examples from Bing Maps documentation
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
        // Level 3, tile (3, 5) → quadkey "213"
        assert_eq!(
            BingMapsImageryProvider::tile_to_quadkey(&TileCoord::new(3, 5, 3)),
            "213"
        );
    }

    #[test]
    fn test_bing_url() {
        let provider = BingMapsImageryProvider::new("test_key");
        let url = provider.get_tile_url(&TileCoord::new(1, 1, 1));
        assert!(url.contains("tiles.virtualearth.net"));
        assert!(url.contains("aerial3"));
    }

    #[test]
    fn test_imagery_descriptor_url_template() {
        let provider = UrlTemplateImageryProvider::new("https://example.com/{z}/{x}/{y}.png")
            .with_max_level(18);
        let desc = ImageryProviderDescriptor::url_template(provider);

        assert_eq!(desc.maximum_level, 18);
        assert!(desc.is_available(10));
        assert!(!desc.is_available(19));
        assert!(!desc.has_time_dynamic);

        let url = desc.get_tile_url(&TileCoord::new(1, 2, 3), 0);
        assert_eq!(url, "https://example.com/3/1/2.png");
    }

    #[test]
    fn test_imagery_descriptor_wmts() {
        let provider = WmtsImageryProvider::new("https://wmts.example.com", "satellite");
        let desc = ImageryProviderDescriptor::wmts(provider);

        assert_eq!(desc.maximum_level, 25);
        let url = desc.get_tile_url(&TileCoord::new(1, 2, 3), 0);
        assert!(url.contains("service=WMTS"));
    }

    #[test]
    fn test_imagery_descriptor_wms() {
        let provider = WmsImageryProvider::new("https://wms.example.com", "roads");
        let desc = ImageryProviderDescriptor::wms(provider);

        let url = desc.get_tile_url(&TileCoord::new(0, 0, 0), 0);
        assert!(url.contains("service=WMS"));
        assert!(url.contains("request=GetMap"));
    }

    #[test]
    fn test_imagery_descriptor_osm() {
        let provider = OpenStreetMapImageryProvider::new();
        let desc = ImageryProviderDescriptor::osm(provider);

        assert_eq!(desc.maximum_level, 19);
        let url = desc.get_tile_url(&TileCoord::new(1, 2, 3), 0);
        assert_eq!(url, "https://tile.openstreetmap.org/3/1/2.png");
    }

    #[test]
    fn test_time_dynamic_imagery() {
        let mut td = TimeDynamicImagery::new();
        td.add_interval(0.0, 100.0, "https://example.com/{time}/{z}/{x}/{y}.png");
        td.add_interval(100.0, 200.0, "https://example.com/late/{z}/{x}/{y}.png");

        assert_eq!(td.interval_count(), 2);

        // Time within first interval
        let url = td.get_tile_url(50.0, &TileCoord::new(1, 2, 3)).unwrap();
        assert!(url.contains("50"));
        assert!(url.contains("/3/1/2.png"));

        // Time within second interval
        let url = td.get_tile_url(150.0, &TileCoord::new(1, 2, 3)).unwrap();
        assert!(url.contains("late"));

        // Time outside all intervals
        let url = td.get_tile_url(300.0, &TileCoord::new(1, 2, 3));
        assert!(url.is_none());
    }

    #[test]
    fn test_wms_get_feature_info() {
        let gfi = WmsGetFeatureInfo::new("https://wms.example.com", "roads,cities")
            .with_info_format("text/html");

        let url = gfi.get_url([-180.0, -90.0, 180.0, 90.0], 256, 256, 128, 128);

        assert!(url.contains("request=GetFeatureInfo"));
        assert!(url.contains("query_layers=roads,cities"));
        assert!(url.contains("i=128"));
        assert!(url.contains("j=128"));
        assert!(url.contains("info_format=text/html"));
        assert!(url.contains("feature_count=10"));
    }

    #[test]
    fn test_wms_get_feature_info_bbox() {
        let gfi = WmsGetFeatureInfo::new("https://wms.example.com", "layer1");
        let url = gfi.get_url([10.0, 20.0, 30.0, 40.0], 512, 512, 256, 256);

        // BBOX should be south,west,north,east for WMS 1.3.0
        assert!(url.contains("bbox=20,10,40,30"));
    }
}
