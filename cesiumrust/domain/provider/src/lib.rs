//! cesium-provider：面向基于瓦片服务的影像与地形提供者。
//!
//! 状态（P2 代码健康审计，2026-09-27）：已实现并由 `cesium-specs`
//! 套件覆盖，但**尚未接入任何生产运行时路径** —— 没有
//! 适配器或应用 crate 依赖它。作为功能对等的领域模型
//! 保留，以供未来适配器桥接使用；不要将其
//! 视为已交付的能力。参见 docs/ARCHITECTURE.md "Test-only domain crates"。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! 子模块：
//! - [`imagery_provider`]：各类影像提供者（URL 模板/WMTS/WMS/TMS/ArcGIS/Mapbox/Ion 等）
//! - [`terrain_provider`]：各类地形提供者（Cesium/Ellipsoid/Heightmap 等）
//! - [`tiling_scheme`]：地理/Web Mercator 切分方案与瓦片可用性四叉树
//! - [`google_earth_enterprise`]：Google Earth Enterprise 元数据与解码

pub mod imagery_provider;
pub mod terrain_provider;
pub mod tiling_scheme;
pub mod google_earth_enterprise;

pub use imagery_provider::{
    ArcGisMapServerImageryProvider, BingMapStyle, BingMapsImageryProvider,
    GoogleEarthEnterpriseImageryProvider, GoogleEarthEnterpriseMapsProvider,
    ImageryProviderDescriptor, ImageryProviderKind, IonImageryProvider,
    MapboxImageryProvider, MapboxStyleImageryProvider, OpenStreetMapImageryProvider,
    SingleTileImageryProvider, SubdomainStrategy, TileCoordinatesImageryProvider,
    TileCoord, TimeDynamicImagery, TmsImageryProvider, UrlTemplateImageryProvider,
    WmsGetFeatureInfo, WmsImageryProvider, WmtsImageryProvider,
};
pub use terrain_provider::{
    ArcGisTerrainProvider, AvailabilityStrategy, CesiumTerrainProvider, EllipsoidTerrainProvider,
    GoogleEarthEnterpriseTerrainProvider, HeightmapSampleParams, HeightmapTerrainProvider,
    QuantizedSampleParams, SampledHeight, TerrainLayerConfig, TerrainProviderDescriptor,
    TerrainProviderKind, VrTheWorldTerrainProvider, sample_height_bilinear, sample_height_quantized,
};
pub use tiling_scheme::{
    GeographicTilingScheme, TileAvailability, TilingScheme, WebMercatorTilingScheme,
};
