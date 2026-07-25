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

/// A unified terrain provider with tiling scheme integration.
///
/// Maps to CesiumJS `TerrainProvider` interface
#[derive(Debug, Clone)]
pub enum TerrainProviderKind {
    /// Cesium terrain (quantized-mesh).
    Cesium(CesiumTerrainProvider),
    /// Flat ellipsoid terrain.
    Ellipsoid(EllipsoidTerrainProvider),
    /// Heightmap terrain.
    Heightmap(HeightmapTerrainProvider),
    /// VRTheWorld terrain.
    VrTheWorld(VrTheWorldTerrainProvider),
}

/// Terrain provider descriptor with tiling scheme and availability.
///
/// Maps to CesiumJS `TerrainProvider` base interface
#[derive(Debug, Clone)]
pub struct TerrainProviderDescriptor {
    /// The provider kind.
    pub kind: TerrainProviderKind,
    /// The tiling scheme used by this provider.
    pub tiling_scheme: crate::tiling_scheme::TilingScheme,
    /// Whether the provider has vertex normals.
    pub has_vertex_normals: bool,
    /// Whether the provider has a water mask.
    pub has_water_mask: bool,
    /// Maximum available level.
    pub maximum_level: u32,
}

impl TerrainProviderDescriptor {
    /// Creates a descriptor for a Cesium terrain provider.
    pub fn cesium(provider: CesiumTerrainProvider, max_level: u32) -> Self {
        Self {
            has_vertex_normals: provider.request_vertex_normals,
            has_water_mask: provider.request_water_mask,
            kind: TerrainProviderKind::Cesium(provider),
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            maximum_level: max_level,
        }
    }

    /// Creates a descriptor for an ellipsoid terrain provider.
    pub fn ellipsoid() -> Self {
        Self {
            kind: TerrainProviderKind::Ellipsoid(EllipsoidTerrainProvider),
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            has_vertex_normals: false,
            has_water_mask: false,
            maximum_level: 0,
        }
    }

    /// Creates a descriptor for a heightmap terrain provider.
    pub fn heightmap(provider: HeightmapTerrainProvider) -> Self {
        let max_level = provider.maximum_level;
        Self {
            kind: TerrainProviderKind::Heightmap(provider),
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            has_vertex_normals: false,
            has_water_mask: false,
            maximum_level: max_level,
        }
    }

    /// Gets the tile URL for a given tile coordinate.
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> Option<String> {
        match &self.kind {
            TerrainProviderKind::Cesium(p) => Some(p.get_tile_url(level, x, y)),
            TerrainProviderKind::Ellipsoid(_) => None,
            TerrainProviderKind::Heightmap(p) => Some(p.get_tile_url(level, x, y)),
            TerrainProviderKind::VrTheWorld(p) => Some(p.get_tile_url(level, x, y)),
        }
    }

    /// Checks if a tile is available at the given level.
    pub fn is_available(&self, level: u32) -> bool {
        level <= self.maximum_level
    }
}

/// Height sampling result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledHeight {
    /// Longitude in radians.
    pub longitude: f64,
    /// Latitude in radians.
    pub latitude: f64,
    /// Sampled height in meters (None if no data).
    pub height: Option<f64>,
}

/// Parameters for height sampling from a heightmap grid.
#[derive(Debug, Clone)]
pub struct HeightmapSampleParams<'a> {
    /// Height data grid (row-major, height x width).
    pub heightmap: &'a [f64],
    /// Number of columns in the heightmap.
    pub grid_width: usize,
    /// Number of rows in the heightmap.
    pub grid_height: usize,
    /// West edge of the tile (radians).
    pub tile_west: f64,
    /// South edge of the tile (radians).
    pub tile_south: f64,
    /// East edge of the tile (radians).
    pub tile_east: f64,
    /// North edge of the tile (radians).
    pub tile_north: f64,
    /// Minimum height value in the grid.
    pub min_height: f64,
    /// Maximum height value in the grid.
    pub max_height: f64,
}

/// Samples terrain height at a position using bilinear interpolation.
///
/// Maps to CesiumJS `sampleTerrain` / `sampleTerrainMostDetailed`
pub fn sample_height_bilinear(
    params: &HeightmapSampleParams<'_>,
    longitude: f64,
    latitude: f64,
) -> Option<f64> {
    let heightmap = params.heightmap;
    let grid_width = params.grid_width;
    let grid_height = params.grid_height;

    if heightmap.len() < grid_width * grid_height {
        return None;
    }

    // Check bounds
    if longitude < params.tile_west || longitude > params.tile_east
        || latitude < params.tile_south || latitude > params.tile_north
    {
        return None;
    }

    // Compute fractional grid position
    let fx = (longitude - params.tile_west) / (params.tile_east - params.tile_west)
        * (grid_width - 1) as f64;
    let fy = (params.tile_north - latitude) / (params.tile_north - params.tile_south)
        * (grid_height - 1) as f64;

    let x0 = (fx as usize).min(grid_width - 2);
    let y0 = (fy as usize).min(grid_height - 2);
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let tx = fx - x0 as f64;
    let ty = fy - y0 as f64;

    // Bilinear interpolation
    let h00 = heightmap[y0 * grid_width + x0];
    let h10 = heightmap[y0 * grid_width + x1];
    let h01 = heightmap[y1 * grid_width + x0];
    let h11 = heightmap[y1 * grid_width + x1];

    let h = h00 * (1.0 - tx) * (1.0 - ty)
        + h10 * tx * (1.0 - ty)
        + h01 * (1.0 - tx) * ty
        + h11 * tx * ty;

    // Clamp to valid range
    Some(h.clamp(params.min_height, params.max_height))
}

/// Parameters for height sampling from quantized mesh data.
#[derive(Debug, Clone)]
pub struct QuantizedSampleParams<'a> {
    /// Quantized vertex data [u0..un, v0..vn, h0..hn].
    pub quantized_vertices: &'a [u16],
    /// Number of vertices.
    pub vertex_count: usize,
    /// West edge (radians).
    pub tile_west: f64,
    /// South edge (radians).
    pub tile_south: f64,
    /// East edge (radians).
    pub tile_east: f64,
    /// North edge (radians).
    pub tile_north: f64,
    /// Minimum height.
    pub min_height: f64,
    /// Maximum height.
    pub max_height: f64,
}

/// Samples terrain height from quantized mesh data.
pub fn sample_height_quantized(
    params: &QuantizedSampleParams<'_>,
    longitude: f64,
    latitude: f64,
) -> Option<f64> {
    let quantized_vertices = params.quantized_vertices;
    let vertex_count = params.vertex_count;

    if quantized_vertices.len() < vertex_count * 3 {
        return None;
    }

    // Find the nearest vertex
    let u_query = ((longitude - params.tile_west) / (params.tile_east - params.tile_west)
        * 32767.0) as u16;
    let v_query = ((latitude - params.tile_south) / (params.tile_north - params.tile_south)
        * 32767.0) as u16;

    let mut best_dist = u32::MAX;
    let mut best_height = 0u16;

    for i in 0..vertex_count {
        let u = quantized_vertices[i];
        let v = quantized_vertices[vertex_count + i];
        let h = quantized_vertices[vertex_count * 2 + i];

        let du = (u as i32 - u_query as i32).unsigned_abs();
        let dv = (v as i32 - v_query as i32).unsigned_abs();
        let dist = du + dv;

        if dist < best_dist {
            best_dist = dist;
            best_height = h;
        }
    }

    // Dequantize height
    let t = best_height as f64 / 32767.0;
    Some(params.min_height + t * (params.max_height - params.min_height))
}

// ============================================================================
// ArcGISTerrainProvider
// ============================================================================

/// ArcGIS terrain provider (ImageServer or ElevationService).
///
/// Maps to CesiumJS `ArcGISTerrainProvider` (not yet in CesiumJS, but common pattern).
#[derive(Debug, Clone)]
pub struct ArcGisTerrainProvider {
    /// Base URL of the ArcGIS terrain service.
    pub url: String,
    /// Whether to use HTTPS.
    pub use_https: bool,
    /// Tile width in pixels.
    pub tile_width: u32,
    /// Tile height in pixels.
    pub tile_height: u32,
    /// Maximum zoom level.
    pub maximum_level: u32,
    /// Credit/attribution.
    pub credit: Option<String>,
}

impl ArcGisTerrainProvider {
    /// Creates a new ArcGIS terrain provider.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            use_https: true,
            tile_width: 256,
            tile_height: 256,
            maximum_level: 23,
            credit: None,
        }
    }

    /// Sets the credit.
    pub fn with_credit(mut self, credit: impl Into<String>) -> Self {
        self.credit = Some(credit.into());
        self
    }

    /// Gets the tile URL for a given coordinate.
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        format!(
            "{}/tile/{}/{}/{}",
            self.url.trim_end_matches('/'),
            level,
            y,
            x
        )
    }
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

    #[test]
    fn test_terrain_provider_descriptor_cesium() {
        let provider = CesiumTerrainProvider::new("https://terrain.example.com")
            .with_vertex_normals()
            .with_water_mask();
        let desc = TerrainProviderDescriptor::cesium(provider, 18);

        assert!(desc.has_vertex_normals);
        assert!(desc.has_water_mask);
        assert_eq!(desc.maximum_level, 18);
        assert!(desc.is_available(10));
        assert!(!desc.is_available(19));

        let url = desc.get_tile_url(5, 10, 15).unwrap();
        assert!(url.contains("terrain.example.com"));
    }

    #[test]
    fn test_terrain_provider_descriptor_ellipsoid() {
        let desc = TerrainProviderDescriptor::ellipsoid();
        assert!(!desc.has_vertex_normals);
        assert!(!desc.has_water_mask);
        assert_eq!(desc.maximum_level, 0);
        assert!(desc.get_tile_url(0, 0, 0).is_none());
    }

    #[test]
    fn test_terrain_provider_descriptor_heightmap() {
        let provider = HeightmapTerrainProvider::new("https://hm.example.com");
        let desc = TerrainProviderDescriptor::heightmap(provider);
        assert_eq!(desc.maximum_level, 25);
        let url = desc.get_tile_url(3, 1, 2).unwrap();
        assert!(url.contains("hm.example.com"));
    }

    #[test]
    fn test_sample_height_bilinear_flat() {
        // 3x3 flat heightmap at 100m
        let heightmap = vec![100.0; 9];
        let params = HeightmapSampleParams {
            heightmap: &heightmap,
            grid_width: 3,
            grid_height: 3,
            tile_west: 0.0,
            tile_south: 0.0,
            tile_east: 1.0,
            tile_north: 1.0,
            min_height: 0.0,
            max_height: 200.0,
        };
        let h = sample_height_bilinear(&params, 0.5, 0.5);
        assert!((h.unwrap() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_sample_height_bilinear_gradient() {
        // 2x2 heightmap: 0, 100, 0, 100 (west-east gradient)
        let heightmap = vec![0.0, 100.0, 0.0, 100.0];
        let params = HeightmapSampleParams {
            heightmap: &heightmap,
            grid_width: 2,
            grid_height: 2,
            tile_west: 0.0,
            tile_south: 0.0,
            tile_east: 1.0,
            tile_north: 1.0,
            min_height: 0.0,
            max_height: 200.0,
        };
        let h = sample_height_bilinear(&params, 0.5, 0.5);
        assert!((h.unwrap() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_sample_height_bilinear_out_of_bounds() {
        let heightmap = vec![100.0; 4];
        let params = HeightmapSampleParams {
            heightmap: &heightmap,
            grid_width: 2,
            grid_height: 2,
            tile_west: 0.0,
            tile_south: 0.0,
            tile_east: 1.0,
            tile_north: 1.0,
            min_height: 0.0,
            max_height: 200.0,
        };
        let h = sample_height_bilinear(&params, 2.0, 0.5); // Outside east
        assert!(h.is_none());
    }

    #[test]
    fn test_sample_height_quantized() {
        // 4 vertices: u=[0, 32767, 0, 32767], v=[0, 0, 32767, 32767], h=[0, 16383, 32767, 16383]
        let vertices: Vec<u16> = vec![
            0, 32767, 0, 32767,       // u
            0, 0, 32767, 32767,       // v
            0, 16383, 32767, 16383,   // h
        ];
        let params = QuantizedSampleParams {
            quantized_vertices: &vertices,
            vertex_count: 4,
            tile_west: 0.0,
            tile_south: 0.0,
            tile_east: 1.0,
            tile_north: 1.0,
            min_height: 0.0,
            max_height: 1000.0,
        };

        // Query at SE corner (lon=1.0, lat=0.0) → u=32767, v=0 → nearest vertex 1 (h=16383)
        let h = sample_height_quantized(&params, 1.0, 0.0);
        let height = h.unwrap();
        // h=16383/32767 * 1000 ≈ 500
        assert!((height - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_sample_height_quantized_corner() {
        let vertices: Vec<u16> = vec![
            0, 32767, 0, 32767,
            0, 0, 32767, 32767,
            0, 16383, 32767, 16383,
        ];
        let params = QuantizedSampleParams {
            quantized_vertices: &vertices,
            vertex_count: 4,
            tile_west: 0.0,
            tile_south: 0.0,
            tile_east: 1.0,
            tile_north: 1.0,
            min_height: 0.0,
            max_height: 1000.0,
        };

        // Query at SW corner (u=0, v=0) - nearest is vertex 0 (h=0)
        let h = sample_height_quantized(&params, 0.0, 0.0);
        assert!(h.unwrap().abs() < 1.0);
    }
}
