//! 用于 Bevy 的 Cesium 数据源插件。
//!
//! 汇总所有数据源加载器（CZML、GeoJSON、KML、GPX）。

pub mod czml_loader;
pub mod geojson_loader;
pub mod gpx_loader;
pub mod kml_loader;

use bevy::prelude::*;

/// 汇总并挂载四个数据源加载子插件的 Bevy 插件。
pub struct CesiumDataSourcePlugin;

impl Plugin for CesiumDataSourcePlugin {
    /// 以 add_plugins 一次性注册 CZML/GeoJSON/KML/GPX 四个加载器插件。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        app.add_plugins((
            czml_loader::CzmlLoadPlugin,
            geojson_loader::GeoJsonLoadPlugin,
            kml_loader::KmlLoadPlugin,
            gpx_loader::GpxLoadPlugin,
        ));
    }
}
