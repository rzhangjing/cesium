//! Ported from `packages/widgets/Source/BaseLayerPicker/createDefaultImageryProviderViewModels.js`.
//!
//! DEVIATION: the JS default creation functions for the Cesium ion entries
//! (`createWorldImageryAsync`) require the ion network endpoint, so those
//! entries yield empty provider lists. The OpenStreetMap entry is backed by
//! the materialized `OpenStreetMapImageryProvider` (Track B), so its
//! creation function returns a real provider handle.

use std::rc::Rc;

use cesium_scene::open_street_map_imagery_provider::OpenStreetMapImageryProvider;

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

/// An entry whose provider requires the ion network endpoint; yields an
/// empty provider list until that endpoint is available.
fn ion_entry(name: &str, tooltip: &str, icon_url: &str) -> ProviderViewModel {
    entry(
        name,
        tooltip,
        icon_url,
        Rc::new(|| ProviderCreationOutput::Providers(Vec::new())),
    )
}

/// Creates the default list of imagery provider view models.
pub fn create_default_imagery_provider_view_models() -> Vec<ProviderViewModel> {
    vec![
        ion_entry(
            "Bing Maps Aerial",
            "Bing Maps aerial imagery",
            "Images/bing-aerial-64.png",
        ),
        ion_entry(
            "Bing Maps Aerial with Labels",
            "Bing Maps aerial imagery with labels",
            "Images/bing-aerial-labels-64.png",
        ),
        ion_entry(
            "Bing Maps Roads",
            "Bing Maps road imagery",
            "Images/bing-road-64.png",
        ),
        entry(
            "OpenStreetMap",
            "OpenStreetMap imagery",
            "Images/openstreetmap-64.png",
            Rc::new(|| {
                ProviderCreationOutput::Providers(vec![Rc::new(
                    OpenStreetMapImageryProvider::new("https://tile.openstreetmap.org/"),
                )])
            }),
        ),
    ]
}
