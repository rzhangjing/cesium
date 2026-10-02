//! 用于基于瓦片的地图服务的影像提供者。
//!
//! 统一描述各类瓦片地图服务后端，按模板或协议生成瓦片 URL：
//! - URL 模板、WMTS、WMS、TMS
//! - OpenStreetMap、Bing Maps、ArcGIS MapServer
//! - Mapbox、Ion、Google Earth Enterprise 等

use std::collections::HashMap;

/// 瓦片坐标（x, y, level）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    /// 瓦片列（x）。
    pub x: u32,
    /// 瓦片行（y）。
    pub y: u32,
    /// 缩放层级。
    pub level: u32,
}

impl TileCoord {
    /// 创建一个新的瓦片坐标。
    pub fn new(x: u32, y: u32, level: u32) -> Self {
        Self { x, y, level }
    }
}

/// 用于负载均衡的子域名选择策略。
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SubdomainStrategy {
    /// 无子域名。
    #[default]
    None,
    /// 在子域名间轮询。
    RoundRobin(Vec<String>),
}

/// 一个从模板生成瓦片 URL 的影像提供者。
///
/// 支持在 URL 模板中按 {x}/{y}/{z} 等占位符展开为实际地址。
#[derive(Debug, Clone)]
pub struct UrlTemplateImageryProvider {
    /// 带有占位符的 URL 模板：{x}, {y}, {z}, {s}, {reverseY}。
    pub url_template: String,
    /// 最小缩放层级。
    pub minimum_level: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 子域名策略。
    pub subdomains: SubdomainStrategy,
    /// 署名/来源归属字符串。
    pub credit: Option<String>,
}

impl UrlTemplateImageryProvider {
    /// 创建一个新的 URL 模板影像提供者。
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

    /// 设置最大缩放层级。
    pub fn with_max_level(mut self, level: u32) -> Self {
        self.maximum_level = level;
        self
    }

    /// 设置瓦片尺寸。
    pub fn with_tile_size(mut self, width: u32, height: u32) -> Self {
        self.tile_width = width;
        self.tile_height = height;
        self
    }

    /// 设置用于负载均衡的子域名。
    pub fn with_subdomains(mut self, subdomains: Vec<String>) -> Self {
        self.subdomains = SubdomainStrategy::RoundRobin(subdomains);
        self
    }

    /// 为给定瓦片生成 URL。
    pub fn get_tile_url(&self, coord: &TileCoord, subdomain_index: usize) -> String {
        let mut url = self.url_template.clone();

        url = url.replace("{x}", &coord.x.to_string());
        url = url.replace("{y}", &coord.y.to_string());
        url = url.replace("{z}", &coord.level.to_string());

        // 反转 Y（TMS 风格：原点在左下角）
        let tiles_y = 1u32 << coord.level;
        let reverse_y = tiles_y - 1 - coord.y;
        url = url.replace("{reverseY}", &reverse_y.to_string());

        // 子域名
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

    /// 检查在给定层级下某个瓦片是否可用。
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// 一个 WMTS（Web Map Tile Service）影像提供者。
///
/// 依据标准 WMTS 图层/矩阵集参数拼接瓦片请求。
#[derive(Debug, Clone)]
pub struct WmtsImageryProvider {
    /// WMTS 服务的基础 URL。
    pub url: String,
    /// 图层标识符。
    pub layer: String,
    /// 样式标识符。
    pub style: String,
    /// 瓦片矩阵集标识符。
    pub tile_matrix_set_id: String,
    /// 图像格式（例如 "image/png"）。
    pub format: String,
    /// 最小缩放层级。
    pub minimum_level: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl WmtsImageryProvider {
    /// 创建一个新的 WMTS 提供者。
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

    /// 设置瓦片矩阵集 ID。
    pub fn with_tile_matrix_set(mut self, id: impl Into<String>) -> Self {
        self.tile_matrix_set_id = id.into();
        self
    }

    /// 设置图像格式。
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    /// 为某个瓦片生成一个 KVP（Key-Value Pair）请求 URL。
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

    /// 为某个瓦片生成一个 RESTful 请求 URL。
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

    /// 从格式中获取文件扩展名。
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

/// 一个 WMS（Web Map Service）影像提供者。
///
/// 以 GetMap 请求按包围盒与层级获取地图影像。
#[derive(Debug, Clone)]
pub struct WmsImageryProvider {
    /// WMS 服务的基础 URL。
    pub url: String,
    /// 以逗号分隔的图层名。
    pub layers: String,
    /// 图像格式。
    pub format: String,
    /// 是否使用透明背景。
    pub transparent: bool,
    /// CRS/SRS 标识符。
    pub crs: String,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 附加参数。
    pub parameters: HashMap<String, String>,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl WmsImageryProvider {
    /// 创建一个新的 WMS 提供者。
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

    /// 为某个瓦片生成一个 GetMap 请求 URL。
    ///
    /// 假设采用地理裁剪方案（EPSG:4326），
    /// 从瓦片坐标计算边界框。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        // 地理裁剪方案：在层级 0 为 2 瓦宽
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

/// 一个 TMS（Tile Map Service）影像提供者。
///
/// 按 TMS 约定（y 轴向上）拼接瓦片地址。
#[derive(Debug, Clone)]
pub struct TmsImageryProvider {
    /// TMS 服务的基础 URL。
    pub url: String,
    /// 文件扩展名（png、jpg）。
    pub file_extension: String,
    /// 最小缩放层级。
    pub minimum_level: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl TmsImageryProvider {
    /// 创建一个新的 TMS 提供者。
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            file_extension: "png".to_string(),
            minimum_level: 0,
            maximum_level: 25,
            credit: None,
        }
    }

    /// 为某个瓦片生成 URL。
    /// TMS 使用左下角原点（反转 Y）。
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

/// OpenStreetMap 影像提供者。
///
/// 从 OSM 瓦片服务端按标准缩放方案获取底图。
#[derive(Debug, Clone)]
pub struct OpenStreetMapImageryProvider {
    /// 基础 URL。
    pub url: String,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl Default for OpenStreetMapImageryProvider {
    /// 默认使用 OSM 官方瓦片服务器，最大层级 19。
    fn default() -> Self {
        Self {
            url: "https://tile.openstreetmap.org".to_string(),
            maximum_level: 19,
            credit: Some("© OpenStreetMap contributors".to_string()),
        }
    }
}

impl OpenStreetMapImageryProvider {
    /// 创建一个新的 OSM 提供者。
    pub fn new() -> Self {
        Self::default()
    }

    /// 为某个瓦片生成 URL。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let base = self.url.trim_end_matches('/');
        format!("{}/{}/{}/{}.png", base, coord.level, coord.x, coord.y)
    }
}

/// Bing Maps 影像提供者。
///
/// 按 Bing 影像样式与子域名轮询策略获取瓦片。
#[derive(Debug, Clone)]
pub struct BingMapsImageryProvider {
    /// Bing Maps 密钥。
    pub key: String,
    /// 地图样式（Aerial、Road、AerialWithLabels）。
    pub map_style: BingMapStyle,
    /// 语言区域（language）。
    pub culture: String,
}

/// Bing Maps 样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BingMapStyle {
    /// 航拍影像。
    Aerial,
    /// 道路地图。
    Road,
    /// 带注记的航拍影像。
    AerialWithLabels,
    /// 深色画布。
    CanvasDark,
    /// 浅色画布。
    CanvasLight,
}

impl BingMapStyle {
    /// 获取 Bing Maps 的 quadkey 影像集。
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
    /// 创建一个新的 Bing Maps 提供者。
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            map_style: BingMapStyle::Aerial,
            culture: "en-US".to_string(),
        }
    }

    /// 将瓦片坐标转换为 Bing Maps 的 quadkey。
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

    /// 生成一个 Bing Maps 瓦片 URL（简化版，不含元数据）。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let quadkey = Self::tile_to_quadkey(coord);
        // 基于 quadkey 哈希选择子域名
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

/// 带有裁剪方案的统一影像提供者描述符。
///
/// 聚合各后端类型及其瓦片方案、尺寸与能力标志。
#[derive(Debug, Clone)]
pub struct ImageryProviderDescriptor {
    /// 提供者类型。
    pub kind: ImageryProviderKind,
    /// 该提供者使用的裁剪方案。
    pub tiling_scheme: crate::tiling_scheme::TilingScheme,
    /// 最小缩放层级。
    pub minimum_level: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
    /// 该提供者是否支持时间动态影像。
    pub has_time_dynamic: bool,
}

/// 影像提供者的类型。
#[derive(Debug, Clone)]
pub enum ImageryProviderKind {
    /// URL 模板提供者。
    UrlTemplate(UrlTemplateImageryProvider),
    /// WMTS 提供者。
    Wmts(WmtsImageryProvider),
    /// WMS 提供者。
    Wms(WmsImageryProvider),
    /// TMS 提供者。
    Tms(TmsImageryProvider),
    /// OpenStreetMap 提供者。
    Osm(OpenStreetMapImageryProvider),
    /// Bing Maps 提供者。
    Bing(BingMapsImageryProvider),
}

impl ImageryProviderDescriptor {
    /// 从 URL 模板提供者创建描述符。
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

    /// 从 WMTS 提供者创建描述符。
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

    /// 从 WMS 提供者创建描述符。
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

    /// 从 OSM 提供者创建描述符。
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

    /// 获取给定坐标的瓦片 URL。
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

    /// 检查在给定层级下某个瓦片是否可用。
    pub fn is_available(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }
}

/// 时间动态影像区间。
///
/// 由起始时间与对应的 URL 参数构成。
#[derive(Debug, Clone)]
pub struct TimeDynamicInterval {
    /// 起始时间（自 epoch 起的秒数）。
    pub start: f64,
    /// 结束时间（自 epoch 起的秒数）。
    pub stop: f64,
    /// 该区间的 URL 模板（可能包含 {time} 占位符）。
    pub url_template: String,
}

/// 时间动态影像提供者。
///
/// 一组时间区间与其对应影像参数的映射。
///
/// 根据时刻定位命中的区间索引。
#[derive(Debug, Clone)]
pub struct TimeDynamicImagery {
    /// 带有相关 URL 的时间区间。
    pub intervals: Vec<TimeDynamicInterval>,
    /// 是否在区间之间进行插值。
    pub interpolate: bool,
}

impl TimeDynamicImagery {
    /// 创建一个新的时间动态影像提供者。
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
            interpolate: false,
        }
    }

    /// 添加一个时间区间。
    pub fn add_interval(&mut self, start: f64, stop: f64, url_template: impl Into<String>) {
        self.intervals.push(TimeDynamicInterval {
            start,
            stop,
            url_template: url_template.into(),
        });
    }

    /// 获取给定时间和瓦片坐标的 URL。
    pub fn get_tile_url(&self, time: f64, coord: &TileCoord) -> Option<String> {
        // 找到包含该时间的区间
        let interval = self.intervals.iter().find(|i| time >= i.start && time <= i.stop)?;

        // 替换占位符
        let mut url = interval.url_template.clone();
        url = url.replace("{x}", &coord.x.to_string());
        url = url.replace("{y}", &coord.y.to_string());
        url = url.replace("{z}", &coord.level.to_string());
        url = url.replace("{time}", &time.to_string());

        Some(url)
    }

    /// 返回区间的数量。
    pub fn interval_count(&self) -> usize {
        self.intervals.len()
    }
}

impl Default for TimeDynamicImagery {
    /// 默认等价于调用 new()，初始不含任何时间区间。
    fn default() -> Self {
        Self::new()
    }
}

/// WMS GetFeatureInfo 请求构建器。
///
/// 按像素坐标反算经纬度并拼接要素查询参数。
#[derive(Debug, Clone)]
pub struct WmsGetFeatureInfo {
    /// WMS 服务的基础 URL。
    pub url: String,
    /// 以逗号分隔的图层名。
    pub layers: String,
    /// 信息格式（例如 "application/json"、"text/html"）。
    pub info_format: String,
    /// CRS/SRS 标识符。
    pub crs: String,
    /// 要素数量上限。
    pub feature_count: u32,
}

impl WmsGetFeatureInfo {
    /// 创建一个新的 GetFeatureInfo 构建器。
    pub fn new(url: impl Into<String>, layers: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            layers: layers.into(),
            info_format: "application/json".to_string(),
            crs: "EPSG:4326".to_string(),
            feature_count: 10,
        }
    }

    /// 设置信息格式。
    pub fn with_info_format(mut self, format: impl Into<String>) -> Self {
        self.info_format = format.into();
        self
    }

    /// 生成一个 GetFeatureInfo 请求 URL。
    ///
    /// # 参数
    /// * `bbox` - 边界框 [west, south, east, north]，以度为单位
    /// * `width` - 图像宽度（以像素计）
    /// * `height` - 图像高度（以像素计）
    /// * `x` - 点击处 x 坐标（以像素计）
    /// * `y` - 点击处 y 坐标（以像素计）
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

// ============================================================================
// ArcGISMapServerImageryProvider
// ============================================================================

/// ArcGIS MapServer 影像提供者。
///
/// 按 MapServer 导出接口拼接瓦片 URL，支持图层与图像格式参数。
#[derive(Debug, Clone, PartialEq)]
pub struct ArcGisMapServerImageryProvider {
    /// ArcGIS MapServer 的基础 URL。
    pub url: String,
    /// 要显示的图层 ID（以逗号分隔）。
    pub layers: Option<String>,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
    /// 是否使用 HTTPS。
    pub use_https: bool,
}

impl ArcGisMapServerImageryProvider {
    /// 创建一个新的 ArcGIS MapServer 提供者。
    pub fn new(url: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            layers: None,
            tile_width: 256,
            tile_height: 256,
            maximum_level: 23,
            credit: None,
            use_https: true,
        }
    }

    /// 设置要显示的图层。
    pub fn with_layers(mut self, layers: &str) -> Self {
        self.layers = Some(layers.to_string());
        self
    }

    /// 获取给定坐标的瓦片 URL。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        let layers_param = self
            .layers
            .as_ref()
            .map(|l| format!("&layers=show:{}", l))
            .unwrap_or_default();

        format!(
            "{}/tile/{}/{}/{}{}&f=image",
            self.url, coord.level, coord.y, coord.x, layers_param
        )
    }
}

// ============================================================================
// MapboxImageryProvider
// ============================================================================

/// Mapbox 影像提供者。
///
/// 按 Mapbox 瓦片 API 模板拼接影像 URL。
#[derive(Debug, Clone, PartialEq)]
pub struct MapboxImageryProvider {
    /// 地图 ID（例如 "mapbox.satellite"）。
    pub map_id: String,
    /// 访问令牌。
    pub access_token: String,
    /// 瓦片尺寸（256 或 512）。
    pub tile_size: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl MapboxImageryProvider {
    /// 创建一个新的 Mapbox 提供者。
    pub fn new(map_id: &str, access_token: &str) -> Self {
        Self {
            map_id: map_id.to_string(),
            access_token: access_token.to_string(),
            tile_size: 512,
            maximum_level: 22,
            credit: Some("© Mapbox © OpenStreetMap".to_string()),
        }
    }

    /// 获取给定坐标的瓦片 URL。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        format!(
            "https://api.mapbox.com/v4/{}/{}/{}/{}.png?access_token={}",
            self.map_id, coord.level, coord.x, coord.y, self.access_token
        )
    }
}

// ============================================================================
// MapboxStyleImageryProvider
// ============================================================================

/// Mapbox Style 影像提供者（使用 Mapbox Styles API）。
///
/// 基于 Styles API，按样式 ID 与缩放层级动态拼接瓦片 URL。
#[derive(Debug, Clone, PartialEq)]
pub struct MapboxStyleImageryProvider {
    /// 样式 ID（例如 "mapbox/streets-v11"）。
    pub style_id: String,
    /// 访问令牌。
    pub access_token: String,
    /// 瓦片尺寸。
    pub tile_size: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl MapboxStyleImageryProvider {
    /// 创建一个新的 Mapbox Style 提供者。
    pub fn new(style_id: &str, access_token: &str) -> Self {
        Self {
            style_id: style_id.to_string(),
            access_token: access_token.to_string(),
            tile_size: 512,
            maximum_level: 22,
            credit: Some("© Mapbox © OpenStreetMap".to_string()),
        }
    }

    /// 获取给定坐标的瓦片 URL。
    pub fn get_tile_url(&self, coord: &TileCoord) -> String {
        format!(
            "https://api.mapbox.com/styles/v1/{}/tiles/{}/{}/{}?access_token={}",
            self.style_id, coord.level, coord.x, coord.y, self.access_token
        )
    }
}

// ============================================================================
// SingleTileImageryProvider
// ============================================================================

/// 单瓦片影像提供者（在整个 globe 上显示一张图像）。
///
/// 将单张图像按给定矩形投影覆盖整个地球。
#[derive(Debug, Clone, PartialEq)]
pub struct SingleTileImageryProvider {
    /// 图像的 URL。
    pub url: String,
    /// 图像覆盖的矩形 [west, south, east, north]，以弧度表示。
    pub rectangle: [f64; 4],
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl SingleTileImageryProvider {
    /// 创建一个新的单瓦片提供者。
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            rectangle: [-std::f64::consts::PI, -std::f64::consts::FRAC_PI_2, std::f64::consts::PI, std::f64::consts::FRAC_PI_2],
            credit: None,
        }
    }

    /// 使用指定的矩形创建。
    pub fn with_rectangle(mut self, rectangle: [f64; 4]) -> Self {
        self.rectangle = rectangle;
        self
    }

    /// 获取图像 URL（始终返回同一个 URL）。
    pub fn get_tile_url(&self, _coord: &TileCoord) -> String {
        self.url.clone()
    }
}

// ============================================================================
// TileCoordinatesImageryProvider
// ============================================================================

/// 一个调试用的提供者，在每个瓦片上绘制瓦片坐标。
///
/// 用于调试：按层级与 x/y 生成瓦片坐标文本覆盖层。
#[derive(Debug, Clone, PartialEq)]
pub struct TileCoordinatesImageryProvider {
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 背景颜色 [R, G, B, A]。
    pub color: [f64; 4],
    /// 文本颜色 [R, G, B, A]。
    pub text_color: [f64; 4],
}

impl Default for TileCoordinatesImageryProvider {
    /// 默认 256×256 瓦片，半透明黑底配黄色坐标文本。
    fn default() -> Self {
        Self {
            tile_width: 256,
            tile_height: 256,
            color: [0.0, 0.0, 0.0, 0.5],
            text_color: [1.0, 1.0, 0.0, 1.0],
        }
    }
}

impl TileCoordinatesImageryProvider {
    /// 创建一个新的瓦片坐标提供者。
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取某个瓦片要显示的文本。
    pub fn get_tile_text(&self, coord: &TileCoord) -> String {
        format!("L{}: X{} Y{}", coord.level, coord.x, coord.y)
    }
}

// ============================================================================
// IonImageryProvider
// ============================================================================

/// Cesium Ion 影像提供者。
///
/// 通过 Ion 资产端点解析出实际的瓦片服务地址。
#[derive(Debug, Clone, PartialEq)]
pub struct IonImageryProvider {
    /// Ion 资产 ID。
    pub asset_id: u64,
    /// Ion 访问令牌。
    pub access_token: Option<String>,
    /// Ion 服务器 URL。
    pub server: String,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl IonImageryProvider {
    /// 创建一个新的 Ion 影像提供者。
    pub fn new(asset_id: u64) -> Self {
        Self {
            asset_id,
            access_token: None,
            server: "https://api.cesium.com".to_string(),
            credit: None,
        }
    }

    /// 设置访问令牌。
    pub fn with_access_token(mut self, token: &str) -> Self {
        self.access_token = Some(token.to_string());
        self
    }

    /// 获取该资产的端点 URL。
    pub fn get_endpoint_url(&self) -> String {
        let token_param = self
            .access_token
            .as_ref()
            .map(|t| format!("?access_token={}", t))
            .unwrap_or_default();
        format!("{}/v1/assets/{}/endpoint{}", self.server, self.asset_id, token_param)
    }
}

/// Google Earth Enterprise 影像提供者。
///
/// 通过 GEE query 接口按 x/y/z 请求影像瓦片与数据库元数据
#[derive(Debug, Clone, PartialEq)]
pub struct GoogleEarthEnterpriseImageryProvider {
    /// Google Earth Enterprise 服务器的基础 URL。
    pub url: String,
    /// 影像数据库的路径。
    pub path: String,
    /// 影像的通道 ID。
    pub channel: u32,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl GoogleEarthEnterpriseImageryProvider {
    /// 创建一个新的 Google Earth Enterprise 影像提供者。
    pub fn new(url: &str, path: &str, channel: u32) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            path: path.to_string(),
            channel,
            tile_width: 256,
            tile_height: 256,
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
            "{}/query?request=ImageryMaps&channel={}&version=1&x={}&y={}&z={}",
            self.url, self.channel, x, y, level
        )
    }

    /// 获取元数据 URL。
    pub fn get_metadata_url(&self) -> String {
        format!("{}/query?request=DatabaseMetadata&path={}", self.url, self.path)
    }
}

/// Google Earth Enterprise Maps 影像提供者。
///
/// 通过 GEE Maps 接口以三层索引（layer/tile）拼接栅格瓦片 URL
#[derive(Debug, Clone, PartialEq)]
pub struct GoogleEarthEnterpriseMapsProvider {
    /// Google Earth Enterprise Maps 服务器的基础 URL。
    pub url: String,
    /// 通道 ID。
    pub channel: u32,
    /// 瓦片宽度（以像素计）。
    pub tile_width: u32,
    /// 瓦片高度（以像素计）。
    pub tile_height: u32,
    /// 最大缩放层级。
    pub maximum_level: u32,
    /// 署名/来源归属。
    pub credit: Option<String>,
}

impl GoogleEarthEnterpriseMapsProvider {
    /// 创建一个新的 Google Earth Enterprise Maps 提供者。
    pub fn new(url: &str, channel: u32) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            channel,
            tile_width: 256,
            tile_height: 256,
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
            "{}/query?request=ImageryMaps&channel={}&version=1&x={}&y={}&z={}",
            self.url, self.channel, x, y, level
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

        // 层级 2：tiles_y = 4, reverseY = 4 - 1 - 1 = 2
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
        assert_eq!(url3, "https://a.example.com/1/0/0.png"); // 回绕
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

        // 层级 1：tiles_y = 2, tms_y = 2 - 1 - 0 = 1
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
        // 来自 Bing Maps 文档的已知 quadkey 示例
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
        // 层级 3，瓦片 (3, 5) → quadkey "213"
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

        // 时间位于第一个区间内
        let url = td.get_tile_url(50.0, &TileCoord::new(1, 2, 3)).unwrap();
        assert!(url.contains("50"));
        assert!(url.contains("/3/1/2.png"));

        // 时间位于第二个区间内
        let url = td.get_tile_url(150.0, &TileCoord::new(1, 2, 3)).unwrap();
        assert!(url.contains("late"));

        // 时间在所有区间之外
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

        // WMS 1.3.0 的 BBOX 应为 south,west,north,east
        assert!(url.contains("bbox=20,10,40,30"));
    }

    #[test]
    fn test_google_earth_enterprise_imagery() {
        let provider = GoogleEarthEnterpriseImageryProvider::new(
            "https://gee.example.com",
            "/dbRoot",
            100,
        );
        assert_eq!(provider.tile_width, 256);
        assert_eq!(provider.maximum_level, 23);

        let url = provider.get_tile_url(5, 10, 15);
        assert!(url.contains("request=ImageryMaps"));
        assert!(url.contains("channel=100"));
        assert!(url.contains("x=10"));
        assert!(url.contains("y=15"));
        assert!(url.contains("z=5"));

        let meta_url = provider.get_metadata_url();
        assert!(meta_url.contains("request=DatabaseMetadata"));
        assert!(meta_url.contains("path=/dbRoot"));
    }

    #[test]
    fn test_google_earth_enterprise_maps() {
        let provider = GoogleEarthEnterpriseMapsProvider::new(
            "https://maps.example.com/",
            200,
        ).with_credit("Google Earth Enterprise");

        assert_eq!(provider.channel, 200);
        assert_eq!(provider.credit, Some("Google Earth Enterprise".to_string()));

        let url = provider.get_tile_url(3, 4, 5);
        assert!(url.contains("channel=200"));
        assert!(url.contains("x=4"));
    }
}
