//! 面向高程数据服务的地形提供者。
//!
//! 提供多种地形后端的统一描述符与采样工具：
//! - Cesium quantized-mesh 地形
//! - 平坦椭球地形
//! - 高程图地形
//! - VRTheWorld 地形

/// 地形提供者的可用性策略。
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AvailabilityStrategy {
    /// 所有瓦片均可用。
    #[default]
    All,
    /// 可用性由裁剪方案决定。
    TilingScheme {
        /// 最小层级。
        minimum_level: u32,
        /// 最大层级。
        maximum_level: u32,
    },
    /// 来自 layer.json 文件的可用性。
    LayerJson,
}

/// 一个 Cesium 地形提供者（quantized-mesh 格式）。
///
/// 从 Ion 或自定义端点获取 quantized-mesh 瓦片，
/// 依据 layer.json 声明的最大层级与可用性发起请求。
#[derive(Debug, Clone)]
pub struct CesiumTerrainProvider {
    /// 地形服务的基础 URL。
    pub url: String,
    /// 是否请求顶点法线。
    pub request_vertex_normals: bool,
    /// 是否请求水掩膜。
    pub request_water_mask: bool,
    /// 可用性策略。
    pub availability: AvailabilityStrategy,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl CesiumTerrainProvider {
    /// 创建一个新的 Cesium 地形提供者。
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            request_vertex_normals: false,
            request_water_mask: false,
            availability: AvailabilityStrategy::All,
            credit: None,
        }
    }

    /// 为光照启用顶点法线。
    pub fn with_vertex_normals(mut self) -> Self {
        self.request_vertex_normals = true;
        self
    }

    /// 启用水掩膜。
    pub fn with_water_mask(mut self) -> Self {
        self.request_water_mask = true;
        self
    }

    /// 生成某个地形瓦片的 URL。
    ///
    /// 格式：`{url}/{level}/{x}/{y}.terrain`
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        let mut url = format!("{}/{}/{}/{}.terrain", base, level, x, y);

        // 为扩展添加查询参数
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

    /// 生成 layer.json 元数据文件的 URL。
    pub fn get_layer_json_url(&self) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/layer.json", base)
    }

    /// 检查在给定层级下某个瓦片是否可用。
    pub fn is_available(&self, level: u32) -> bool {
        match &self.availability {
            AvailabilityStrategy::All => true,
            AvailabilityStrategy::TilingScheme {
                minimum_level,
                maximum_level,
            } => level >= *minimum_level && level <= *maximum_level,
            AvailabilityStrategy::LayerJson => true, // 需要异步检查
        }
    }
}

/// 一个椭球地形提供者（平坦，无高程）。
///
/// 始终返回零高程，适用于无地形数据时的占位场景。
#[derive(Debug, Clone, Default)]
pub struct EllipsoidTerrainProvider;

impl EllipsoidTerrainProvider {
    /// 创建一个新的椭球地形提供者。
    pub fn new() -> Self {
        Self
    }

    /// 返回任意位置的高度（始终为 0）。
    pub fn get_height(&self, _longitude: f64, _latitude: f64) -> f64 {
        0.0
    }
}

/// 一个高程图地形提供者。
///
/// 以规则网格高度采样提供服务端高程数据。
#[derive(Debug, Clone)]
pub struct HeightmapTerrainProvider {
    /// 高程图服务的基础 URL。
    pub url: String,
    /// 每个高程图瓦片的宽度（以采样计）。
    pub width: u32,
    /// 每个高程图瓦片的高度（以采样计）。
    pub height: u32,
    /// 文件扩展名。
    pub file_extension: String,
    /// 最小缩放层级。
    pub minimum_level: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl HeightmapTerrainProvider {
    /// 创建一个新的高程图地形提供者。
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

    /// 设置高程图的尺寸。
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 生成某个高程图瓦片的 URL。
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        format!(
            "{}/{}/{}/{}.{}",
            base, level, x, y, self.file_extension
        )
    }

    /// 检查在给定层级下某个瓦片是否可用。
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// VRTheWorld 地形提供者。
///
/// 从 VRTheWorld 高程服务端按瓦片获取地形数据。
#[derive(Debug, Clone)]
pub struct VrTheWorldTerrainProvider {
    /// VRTheWorld 服务的基础 URL。
    pub url: String,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl VrTheWorldTerrainProvider {
    /// 创建一个新的 VRTheWorld 地形提供者。
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            credit: None,
        }
    }

    /// 生成某个地形瓦片的 URL。
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/{}/{}/{}.tif", base, level, x, y)
    }
}

/// 用于 layer.json 解析的地形提供者配置。
#[derive(Debug, Clone, Default)]
pub struct TerrainLayerConfig {
    /// 瓦片格式（"quantized-mesh-1.0" 或 "heightmap-1.0"）。
    pub format: String,
    /// 可用的最小层级。
    pub min_level: u32,
    /// 最大层级。
    pub max_level: u32,
    /// 是否有可用的顶点法线。
    pub has_vertex_normals: bool,
    /// 是否有可用的水掩膜。
    pub has_water_mask: bool,
    /// 投影（"EPSG:4326" 或 "EPSG:3857"）。
    pub projection: String,
    /// 裁剪方案边界 [west, south, east, north]，以度为单位。
    pub bounds: Option<[f64; 4]>,
}

impl TerrainLayerConfig {
    /// 解析 layer.json 内容。
    pub fn from_json(json: &str) -> Result<Self, String> {
        // 针对 layer.json 的简单 JSON 解析
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

/// 按键从 JSON 中提取一个字符串值。
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let colon = json[start..].find(':')? + start;
    let quote_start = json[colon..].find('"')? + colon + 1;
    let quote_end = json[quote_start..].find('"')? + quote_start;
    Some(json[quote_start..quote_end].to_string())
}

/// 按键从 JSON 中提取一个数值。
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let colon = json[start..].find(':')? + start;
    let value_start = colon + 1;

    // 跳过空白并找到数字
    let remaining = json[value_start..].trim_start();
    let end = remaining
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+' && c != 'e' && c != 'E')
        .unwrap_or(remaining.len());

    remaining[..end].parse().ok()
}

/// 一个集成了裁剪方案的统一地形提供者。
///
/// 枚举各具体后端，便于上层统一分发请求。
#[derive(Debug, Clone)]
pub enum TerrainProviderKind {
    /// Cesium 地形（quantized-mesh）。
    Cesium(CesiumTerrainProvider),
    /// 平坦椭球地形。
    Ellipsoid(EllipsoidTerrainProvider),
    /// 高程图地形。
    Heightmap(HeightmapTerrainProvider),
    /// VRTheWorld 地形。
    VrTheWorld(VrTheWorldTerrainProvider),
}

/// 带裁剪方案与可用性的地形提供者描述符。
///
/// 聚合提供者类型、裁剪方案、最大层级与法线/水掩膜能力标志。
#[derive(Debug, Clone)]
pub struct TerrainProviderDescriptor {
    /// 提供者类型。
    pub kind: TerrainProviderKind,
    /// 该提供者使用的裁剪方案。
    pub tiling_scheme: crate::tiling_scheme::TilingScheme,
    /// 该提供者是否有顶点法线。
    pub has_vertex_normals: bool,
    /// 该提供者是否有水掩膜。
    pub has_water_mask: bool,
    /// 最大可用层级。
    pub maximum_level: u32,
}

impl TerrainProviderDescriptor {
    /// 为 Cesium 地形提供者创建描述符。
    pub fn cesium(provider: CesiumTerrainProvider, max_level: u32) -> Self {
        Self {
            has_vertex_normals: provider.request_vertex_normals,
            has_water_mask: provider.request_water_mask,
            kind: TerrainProviderKind::Cesium(provider),
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            maximum_level: max_level,
        }
    }

    /// 为椭球地形提供者创建描述符。
    pub fn ellipsoid() -> Self {
        Self {
            kind: TerrainProviderKind::Ellipsoid(EllipsoidTerrainProvider),
            tiling_scheme: crate::tiling_scheme::TilingScheme::geographic(),
            has_vertex_normals: false,
            has_water_mask: false,
            maximum_level: 0,
        }
    }

    /// 为高程图地形提供者创建描述符。
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

    /// 获取给定瓦片坐标的瓦片 URL。
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> Option<String> {
        match &self.kind {
            TerrainProviderKind::Cesium(p) => Some(p.get_tile_url(level, x, y)),
            TerrainProviderKind::Ellipsoid(_) => None,
            TerrainProviderKind::Heightmap(p) => Some(p.get_tile_url(level, x, y)),
            TerrainProviderKind::VrTheWorld(p) => Some(p.get_tile_url(level, x, y)),
        }
    }

    /// 检查在给定层级下某个瓦片是否可用。
    pub fn is_available(&self, level: u32) -> bool {
        level <= self.maximum_level
    }
}

/// 高程采样结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledHeight {
    /// 经度（弧度）。
    pub longitude: f64,
    /// 纬度（弧度）。
    pub latitude: f64,
    /// 采样得到的高度（米，若无数据则为 None）。
    pub height: Option<f64>,
}

/// 从高程图网格采样高程的参数。
#[derive(Debug, Clone)]
pub struct HeightmapSampleParams<'a> {
    /// 高程数据网格（行优先，height x width）。
    pub heightmap: &'a [f64],
    /// 高程图的列数。
    pub grid_width: usize,
    /// 高程图的行数。
    pub grid_height: usize,
    /// 瓦片的西边界（弧度）。
    pub tile_west: f64,
    /// 瓦片的南边界（弧度）。
    pub tile_south: f64,
    /// 瓦片的东边界（弧度）。
    pub tile_east: f64,
    /// 瓦片的北边界（弧度）。
    pub tile_north: f64,
    /// 网格中的最小高程值。
    pub min_height: f64,
    /// 网格中的最大高程值。
    pub max_height: f64,
}

/// 使用双线性插值在某个位置采样地形高度。
///
/// 定位包围该点的四个采样，按经纬权重插值得到高程。
pub fn sample_height_bilinear(
    params: &HeightmapSampleParams<'_>,
    longitude: f64,
    latitude: f64,
) -> Option<f64> {
    let heightmap = params.heightmap;
    let grid_width = params.grid_width;
    let grid_height = params.grid_height;

    if heightmap.len() < grid_width * grid_height {
        // 数据不足以覆盖整张网格时放弃采样。
        return None;
    }

    // 检查边界
    if longitude < params.tile_west || longitude > params.tile_east
        || latitude < params.tile_south || latitude > params.tile_north
    {
        return None;
    }

    // 计算小数形式的网格位置
    let fx = (longitude - params.tile_west) / (params.tile_east - params.tile_west)
        * (grid_width - 1) as f64;
    let fy = (params.tile_north - latitude) / (params.tile_north - params.tile_south)
        * (grid_height - 1) as f64;

    // 取左下角整数采样并钳制，保证 x1/y1 不越界。
    let x0 = (fx as usize).min(grid_width - 2);
    let y0 = (fy as usize).min(grid_height - 2);
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    // tx/ty 为落在单元内的小数位置，作为插值权重。
    let tx = fx - x0 as f64;
    let ty = fy - y0 as f64;

    // 双线性插值
    let h00 = heightmap[y0 * grid_width + x0];
    let h10 = heightmap[y0 * grid_width + x1];
    let h01 = heightmap[y1 * grid_width + x0];
    let h11 = heightmap[y1 * grid_width + x1];

    let h = h00 * (1.0 - tx) * (1.0 - ty)
        + h10 * tx * (1.0 - ty)
        + h01 * (1.0 - tx) * ty
        + h11 * tx * ty;

    // 钳制到有效范围
    Some(h.clamp(params.min_height, params.max_height))
}

/// 从 quantized mesh 数据采样高程的参数。
#[derive(Debug, Clone)]
pub struct QuantizedSampleParams<'a> {
    /// 量化顶点数据 [u0..un, v0..vn, h0..hn]。
    pub quantized_vertices: &'a [u16],
    /// 顶点数量。
    pub vertex_count: usize,
    /// 西边界（弧度）。
    pub tile_west: f64,
    /// 南边界（弧度）。
    pub tile_south: f64,
    /// 东边界（弧度）。
    pub tile_east: f64,
    /// 北边界（弧度）。
    pub tile_north: f64,
    /// 最小高程。
    pub min_height: f64,
    /// 最大高程。
    pub max_height: f64,
}

/// 从 quantized mesh 数据采样地形高度。
pub fn sample_height_quantized(
    params: &QuantizedSampleParams<'_>,
    longitude: f64,
    latitude: f64,
) -> Option<f64> {
    let quantized_vertices = params.quantized_vertices;
    let vertex_count = params.vertex_count;

    if quantized_vertices.len() < vertex_count * 3 {
        // 顶点数据须容纳 u/v/h 三段，否则放弃。
        return None;
    }

    // 找到最近的顶点
    let u_query = ((longitude - params.tile_west) / (params.tile_east - params.tile_west)
        * 32767.0) as u16;
    let v_query = ((latitude - params.tile_south) / (params.tile_north - params.tile_south)
        * 32767.0) as u16;

    // 以最近顶点的高程作为采样结果。
    let mut best_dist = u32::MAX;
    let mut best_height = 0u16;

    for i in 0..vertex_count {
        let u = quantized_vertices[i];
        let v = quantized_vertices[vertex_count + i];
        let h = quantized_vertices[vertex_count * 2 + i];

        // 用曼哈顿距离比较 u/v 偏差，逐顶点取最近者。
        let du = (u as i32 - u_query as i32).unsigned_abs();
        let dv = (v as i32 - v_query as i32).unsigned_abs();
        let dist = du + dv;

        if dist < best_dist {
            best_dist = dist;
            best_height = h;
        }
    }

    // 反量化高程
    // 将量化高度线性还原回 [min,max] 区间。
    let t = best_height as f64 / 32767.0;
    Some(params.min_height + t * (params.max_height - params.min_height))
}

// ============================================================================
// ArcGISTerrainProvider
// ============================================================================

/// ArcGIS 地形提供者（ImageServer 或 ElevationService）。
///
/// 按瓦片坐标拼接服务 URL，用于常见的 ArcGIS 高程服务模式。
#[derive(Debug, Clone)]
pub struct ArcGisTerrainProvider {
    /// ArcGIS 地形服务的基础 URL。
    pub url: String,
    /// 是否使用 HTTPS。
    pub use_https: bool,
    /// 瓦片宽度（像素）。
    pub tile_width: u32,
    /// 瓦片高度（像素）。
    pub tile_height: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl ArcGisTerrainProvider {
    /// 创建一个新的 ArcGIS 地形提供者。
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

    /// 设置署名。
    pub fn with_credit(mut self, credit: impl Into<String>) -> Self {
        self.credit = Some(credit.into());
        self
    }

    /// 获取给定坐标的瓦片 URL。
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        // 按 {url}/tile/{level}/{y}/{x} 约定拼接瓦片地址。
        format!(
            "{}/tile/{}/{}/{}",
            self.url.trim_end_matches('/'),
            level,
            y,
            x
        )
    }
}

/// Google Earth Enterprise 地形提供者。
///
/// 基于 GEE 元数据的 quadkey 瓦片方案获取地形。
#[derive(Debug, Clone, PartialEq)]
pub struct GoogleEarthEnterpriseTerrainProvider {
    /// Google Earth Enterprise 服务器的基础 URL。
    pub url: String,
    /// 地形数据库的路径。
    pub path: String,
    /// 瓦片宽度。
    pub tile_width: u32,
    /// 瓦片高度。
    pub tile_height: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl GoogleEarthEnterpriseTerrainProvider {
    /// 创建一个新的 Google Earth Enterprise 地形提供者。
    pub fn new(url: &str, path: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            path: path.to_string(),
            tile_width: 32,
            tile_height: 32,
            maximum_level: 23,
            credit: None,
        }
    }

    /// 设置署名。
    pub fn with_credit(mut self, credit: &str) -> Self {
        self.credit = Some(credit.to_string());
        self
    }

    /// 获取给定瓦片坐标的瓦片 URL。
    pub fn get_tile_url(&self, level: u32, x: u32, y: u32) -> String {
        format!(
            "{}/query?request=TerrainMaps&path={}&version=1&x={}&y={}&z={}",
            self.url, self.path, x, y, level
        )
    }

    /// 获取元数据 URL。
    pub fn get_metadata_url(&self) -> String {
        format!("{}/query?request=DatabaseMetadata&path={}", self.url, self.path)
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
        assert!(provider.is_available(100)); // 默认全部可用
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
        // 3x3 平坦高程图，位于 100m
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
        // 2x2 高程图：0, 100, 0, 100（西-东梯度）
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
        let h = sample_height_bilinear(&params, 2.0, 0.5); // 东侧范围外
        assert!(h.is_none());
    }

    #[test]
    fn test_sample_height_quantized() {
        // 4 个顶点：u=[0, 32767, 0, 32767], v=[0, 0, 32767, 32767], h=[0, 16383, 32767, 16383]
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

        // 在东南角查询（lon=1.0, lat=0.0）→ u=32767, v=0 → 最近顶点 1（h=16383）
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

        // 在西南角查询（u=0, v=0）- 最近的是顶点 0（h=0）
        let h = sample_height_quantized(&params, 0.0, 0.0);
        assert!(h.unwrap().abs() < 1.0);
    }

    #[test]
    fn test_google_earth_enterprise_terrain() {
        let provider = GoogleEarthEnterpriseTerrainProvider::new(
            "https://gee.example.com",
            "/terrain",
        ).with_credit("GEE Terrain");

        assert_eq!(provider.tile_width, 32);
        assert_eq!(provider.maximum_level, 23);
        assert_eq!(provider.credit, Some("GEE Terrain".to_string()));

        let url = provider.get_tile_url(4, 8, 12);
        assert!(url.contains("request=TerrainMaps"));
        assert!(url.contains("path=/terrain"));
        assert!(url.contains("x=8"));
        assert!(url.contains("y=12"));
        assert!(url.contains("z=4"));

        let meta_url = provider.get_metadata_url();
        assert!(meta_url.contains("request=DatabaseMetadata"));
    }
}
