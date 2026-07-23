//! Terrain providers for elevation data services.
//!
//! Maps to CesiumJS terrain providers:
//! - `CesiumTerrainProvider` (quantized-mesh)
//! - `EllipsoidTerrainProvider` (flat)
//! - `VRTheWorldTerrainProvider`
//! - Custom heightmap providers

/// Terrain provider availability strategy.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AvailabilityStrategy {
    /// All tiles are available.
    #[default]
    All,
    /// Availability is determined by a tiling scheme.
    TilingScheme {
        /// Minimum level.
        minimum_level: u32,
        /// Maximum level.
        maximum_level: u32,
    },
    /// Availability from a layer.json file.
    LayerJson,
}

/// A Cesium terrain provider (quantized-mesh format).
///
/// Maps to CesiumJS `CesiumTerrainProvider`
#[derive(Debug, Clone)]
pub struct CesiumTerrainProvider {
    /// Base URL of the terrain service.
    pub url: String,
    /// Whether to request vertex normals.
    pub request_vertex_normals: bool,
    /// Whether to request water mask.
    pub request_water_mask: bool,
    /// Availability strategy.
    pub availability: AvailabilityStrategy,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl CesiumTerrainProvider {
    /// Creates a new Cesium terrain provider.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            request_vertex_normals: false,
            request_water_mask: false,
            availability: AvailabilityStrategy::All,
            credit: None,
        }
    }

    /// Enables vertex normals for lighting.
    pub fn with_vertex_normals(mut self) -> Self {
        self.request_vertex_normals = true;
        self
    }

    /// Enables water mask.
    pub fn with_water_mask(mut self) -> Self {
        self.request_water_mask = true;
        self
    }

    /// Generates the URL for a terrain tile.
    ///
    /// Format: `{url}/{level}/{x}/{y}.terrain`
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        let mut url = format!("{}/{}/{}/{}.terrain", base, level, x, y);

        // Add query parameters for extensions
        let mut params = Vec::new();
        if self.request_vertex_normals {
            params.push("extensions=octvertexnormals");
        }
        if self.request_water_mask {
            params.push("extensions=watermask");
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        url
    }

    /// Generates the URL for the layer.json metadata file.
    pub fn get_layer_json_url(&self) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/layer.json", base)
    }

    /// Checks if a tile is available at the given level.
    pub fn is_available(&self, level: u32) -> bool {
        match &self.availability {
            AvailabilityStrategy::All => true,
            AvailabilityStrategy::TilingScheme {
                minimum_level,
                maximum_level,
            } => level >= *minimum_level && level <= *maximum_level,
            AvailabilityStrategy::LayerJson => true, // Would need async check
        }
    }
}

/// An ellipsoid terrain provider (flat, no elevation).
///
/// Maps to CesiumJS `EllipsoidTerrainProvider`
#[derive(Debug, Clone, Default)]
pub struct EllipsoidTerrainProvider;

impl EllipsoidTerrainProvider {
    /// Creates a new ellipsoid terrain provider.
    pub fn new() -> Self {
        Self
    }

    /// Returns height at any position (always 0).
    pub fn get_height(&self, _longitude: f64, _latitude: f64) -> f64 {
        0.0
    }
}

/// A heightmap terrain provider.
///
/// Maps to CesiumJS `HeightmapTerrainProvider`
#[derive(Debug, Clone)]
pub struct HeightmapTerrainProvider {
    /// Base URL of the heightmap service.
    pub url: String,
    /// Width of each heightmap tile in samples.
    pub width: u32,
    /// Height of each heightmap tile in samples.
    pub height: u32,
    /// File extension.
    pub file_extension: String,
    /// Minimum zoom level.
    pub minimum_level: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl HeightmapTerrainProvider {
    /// Creates a new heightmap terrain provider.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            width: 65,
            height: 65,
            file_extension: "terrain".to_string(),
            minimum_level: 0,
            maximum_level: 25,
            credit: None,
        }
    }

    /// Sets the heightmap dimensions.
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Generates the URL for a heightmap tile.
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        format!(
            "{}/{}/{}/{}.{}",
            base, level, x, y, self.file_extension
        )
    }

    /// Checks if a tile is available at the given level.
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// VRTheWorld terrain provider.
///
/// Maps to CesiumJS `VRTheWorldTerrainProvider`
#[derive(Debug, Clone)]
pub struct VrTheWorldTerrainProvider {
    /// Base URL of the VRTheWorld service.
    pub url: String,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl VrTheWorldTerrainProvider {
    /// Creates a new VRTheWorld terrain provider.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            credit: None,
        }
    }

    /// Generates the URL for a terrain tile.
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/{}/{}/{}.tif", base, level, x, y)
    }
}

/// Terrain provider configuration for layer.json parsing.
#[derive(Debug, Clone, Default)]
pub struct TerrainLayerConfig {
    /// Tile format ("quantized-mesh-1.0" or "heightmap-1.0").
    pub format: String,
    /// Available levels.
    pub min_level: u32,
    /// Maximum level.
    pub max_level: u32,
    /// Whether vertex normals are available.
    pub has_vertex_normals: bool,
    /// Whether water mask is available.
    pub has_water_mask: bool,
    /// Projection ("EPSG:4326" or "EPSG:3857").
    pub projection: String,
    /// Tiling scheme bounds [west, south, east, north] in degrees.
    pub bounds: Option<[f64; 4]>,
}

impl TerrainLayerConfig {
    /// Parses a layer.json content.
    pub fn from_json(json: &str) -> Result<Self, String> {
        // Simple JSON parsing for layer.json
        let mut config = Self::default();

        if let Some(format) = extract_json_string(json, "format") {
            config.format = format;
        }
        if let Some(min) = extract_json_number(json, "minLevel") {
            config.min_level = min as u32;
        }
        if let Some(max) = extract_json_number(json, "maxLevel") {
            config.max_level = max as u32;
        }
        if let Some(proj) = extract_json_string(json, "projection") {
            config.projection = proj;
        }

        config.has_vertex_normals = json.contains("octvertexnormals");
        config.has_water_mask = json.contains("watermask");

        Ok(config)
    }
}

/// Extracts a string value from JSON by key.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let colon = json[start..].find(':')? + start;
    let quote_start = json[colon..].find('"')? + colon + 1;
    let quote_end = json[quote_start..].find('"')? + quote_start;
    Some(json[quote_start..quote_end].to_string())
}

/// Extracts a number value from JSON by key.
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let colon = json[start..].find(':')? + start;
    let value_start = colon + 1;

    // Skip whitespace and find the number
    let remaining = json[value_start..].trim_start();
    let end = remaining
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+' && c != 'e' && c != 'E')
        .unwrap_or(remaining.len());

    remaining[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cesium_terrain_url() {
        let provider = CesiumTerrainProvider::new("https://terrain.example.com/tiles");
        let url = provider.get_tile_url(5, 10, 15);
        assert_eq!(url, "https://terrain.example.com/tiles/5/10/15.terrain");
    }

    #[test]
    fn test_cesium_terrain_url_with_normals() {
        let provider = CesiumTerrainProvider::new("https://terrain.example.com")
            .with_vertex_normals();
        let url = provider.get_tile_url(3, 1, 2);
        assert!(url.contains("extensions=octvertexnormals"));
    }

    #[test]
    fn test_cesium_terrain_layer_json() {
        let provider = CesiumTerrainProvider::new("https://terrain.example.com/");
        assert_eq!(
            provider.get_layer_json_url(),
            "https://terrain.example.com/layer.json"
        );
    }

    #[test]
    fn test_ellipsoid_terrain() {
        let provider = EllipsoidTerrainProvider::new();
        assert_eq!(provider.get_height(0.0, 0.0), 0.0);
        assert_eq!(provider.get_height(1.5, 0.8), 0.0);
    }

    #[test]
    fn test_heightmap_terrain_url() {
        let provider = HeightmapTerrainProvider::new("https://heightmap.example.com");
        let url = provider.get_tile_url(4, 8, 12);
        assert_eq!(url, "https://heightmap.example.com/4/8/12.terrain");
    }

    #[test]
    fn test_heightmap_availability() {
        let provider = HeightmapTerrainProvider::new("https://example.com");
        assert!(provider.is_available(0));
        assert!(provider.is_available(25));
        assert!(!provider.is_available(26));
    }

    #[test]
    fn test_vrtheworld_url() {
        let provider = VrTheWorldTerrainProvider::new("https://vrtheworld.example.com");
        let url = provider.get_tile_url(2, 3, 4);
        assert_eq!(url, "https://vrtheworld.example.com/2/3/4.tif");
    }

    #[test]
    fn test_terrain_layer_config_parse() {
        let json = r#"{
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "minLevel": 0,
            "maxLevel": 22,
            "projection": "EPSG:4326",
            "extensions": ["octvertexnormals", "watermask"]
        }"#;

        let config = TerrainLayerConfig::from_json(json).unwrap();
        assert_eq!(config.format, "quantized-mesh-1.0");
        assert_eq!(config.min_level, 0);
        assert_eq!(config.max_level, 22);
        assert_eq!(config.projection, "EPSG:4326");
        assert!(config.has_vertex_normals);
        assert!(config.has_water_mask);
    }

    #[test]
    fn test_cesium_terrain_availability() {
        let provider = CesiumTerrainProvider::new("https://example.com");
        assert!(provider.is_available(0));
        assert!(provider.is_available(100)); // All available by default
    }
}
