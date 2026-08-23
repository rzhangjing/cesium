//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultTerrainProviderViewModels.js`.

use crate::provider_view_model::ProviderViewModel;

/// Creates the default list of terrain provider view models.
pub fn create_default_terrain_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        ProviderViewModel::new(
            "Cesium World Terrain",
            "Cesium default terrain provider",
            "Images/cesium_terrain-64.png",
        ),
        ProviderViewModel::new(
            "Ellipsoid",
            "Smooth ellipsoid (no terrain)",
            "Images/ellipsoid-64.png",
        ),
    ]
}
