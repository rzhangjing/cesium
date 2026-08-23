//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultImageryProviderViewModels.js`.

use crate::provider_view_model::ProviderViewModel;

/// Creates the default list of imagery provider view models.
pub fn create_default_imagery_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        ProviderViewModel::new(
            "Bing Maps Aerial",
            "Bing Maps aerial imagery",
            "Images/bing-aerial-64.png",
        ),
        ProviderViewModel::new(
            "Bing Maps Aerial with Labels",
            "Bing Maps aerial imagery with labels",
            "Images/bing-aerial-labels-64.png",
        ),
        ProviderViewModel::new(
            "Bing Maps Roads",
            "Bing Maps road imagery",
            "Images/bing-road-64.png",
        ),
        ProviderViewModel::new(
            "OpenStreetMap",
            "OpenStreetMap imagery",
            "Images/openstreetmap-64.png",
        ),
    ]
}
