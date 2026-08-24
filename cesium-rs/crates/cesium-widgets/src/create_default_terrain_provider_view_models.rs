//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultTerrainProviderViewModels.js`.
//!
//! DEVIATION: the JS default creation functions construct real terrain
//! providers (`CesiumTerrainProvider`/`EllipsoidTerrainProvider`); those
//! require network/GPU resources, so the port yields empty provider lists
//! until the terrain providers are materialized (Track B).

use std::rc::Rc;

use crate::provider_view_model::{
    ProviderCreationOutput, ProviderViewModel, ProviderViewModelOptions, StringProp,
};

fn default_entry(name: &str, tooltip: &str, icon_url: &str) -> ProviderViewModel {
    ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Value(name.to_string())),
        tooltip: Some(StringProp::Value(tooltip.to_string())),
        icon_url: Some(StringProp::Value(icon_url.to_string())),
        category: None,
        creation_function: Some(Rc::new(|| ProviderCreationOutput::Providers(Vec::new()))),
    })
}

/// Creates the default list of terrain provider view models.
pub fn create_default_terrain_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        default_entry(
            "Cesium World Terrain",
            "Cesium default terrain provider",
            "Images/cesium_terrain-64.png",
        ),
        default_entry(
            "Ellipsoid",
            "Smooth ellipsoid (no terrain)",
            "Images/ellipsoid-64.png",
        ),
    ]
}
