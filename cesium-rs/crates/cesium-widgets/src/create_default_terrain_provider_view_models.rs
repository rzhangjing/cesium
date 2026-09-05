//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultTerrainProviderViewModels.js`.
//!
//! DEVIATION: the JS `Cesium World Terrain` creation function
//! (`createWorldTerrainAsync`) requires the ion network endpoint, so that
//! entry yields an empty provider list. The Ellipsoid entry is backed by the
//! materialized `EllipsoidTerrainProvider`, so its creation function returns
//! a real provider handle.

use std::rc::Rc;

use cesium_core::ellipsoid_terrain_provider::EllipsoidTerrainProvider;

use crate::provider_view_model::{
    ProviderCreationOutput, ProviderViewModel, ProviderViewModelOptions, StringProp,
};

fn entry(
    name: &str,
    tooltip: &str,
    icon_url: &str,
    creation_function: Rc<dyn Fn() -> ProviderCreationOutput>,
) -> ProviderViewModel {
    ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Value(name.to_string())),
        tooltip: Some(StringProp::Value(tooltip.to_string())),
        icon_url: Some(StringProp::Value(icon_url.to_string())),
        category: None,
        creation_function: Some(creation_function),
    })
}

/// Creates the default list of terrain provider view models.
pub fn create_default_terrain_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        entry(
            "Cesium World Terrain",
            "Cesium default terrain provider",
            "Images/cesium_terrain-64.png",
            Rc::new(|| ProviderCreationOutput::Providers(Vec::new())),
        ),
        entry(
            "Ellipsoid",
            "Smooth ellipsoid (no terrain)",
            "Images/ellipsoid-64.png",
            Rc::new(|| {
                ProviderCreationOutput::Providers(vec![Rc::new(
                    EllipsoidTerrainProvider::new(None, None),
                )])
            }),
        ),
    ]
}
