//! 用于 Bevy 的 Cesium 数据源插件。
//!
//! 汇总所有数据源加载器（CZML、GeoJSON、KML、GPX）。

pub mod czml_loader;
pub mod geojson_loader;
pub mod gpx_loader;
pub mod kml_loader;

use bevy::prelude::*;

pub struct CesiumDataSourcePlugin;

impl Plugin for CesiumDataSourcePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            czml_loader::CzmlLoadPlugin,
            geojson_loader::GeoJsonLoadPlugin,
            kml_loader::KmlLoadPlugin,
            gpx_loader::GpxLoadPlugin,
        ));
    }
}
