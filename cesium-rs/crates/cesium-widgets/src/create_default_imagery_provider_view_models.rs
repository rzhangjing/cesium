//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultImageryProviderViewModels.js`.
//!
//! DEVIATION: the JS default creation functions construct real imagery
//! providers (`createTileMapServiceImageryProvider`, `OpenStreetMapImageryProvider`);
//! those require GPU/network resources, so the port yields empty provider
//! lists until the imagery providers are materialized (Track B).

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

/// Creates the default list of imagery provider view models.
pub fn create_default_imagery_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        default_entry(
            "Bing Maps Aerial",
            "Bing Maps aerial imagery",
            "Images/bing-aerial-64.png",
        ),
        default_entry(
            "Bing Maps Aerial with Labels",
            "Bing Maps aerial imagery with labels",
            "Images/bing-aerial-labels-64.png",
        ),
        default_entry(
            "Bing Maps Roads",
            "Bing Maps road imagery",
            "Images/bing-road-64.png",
        ),
        default_entry(
            "OpenStreetMap",
            "OpenStreetMap imagery",
            "Images/openstreetmap-64.png",
        ),
    ]
}
