//! Ported from `packages/widgets/Source/BaseLayerPicker/BaseLayerPickerViewModel.js`.

use crate::provider_view_model::ProviderViewModel;

/// The view model for the BaseLayerPicker widget.
///
/// Allows the user to choose the base imagery and terrain layers.
pub struct BaseLayerPickerViewModel {
    /// The available imagery providers.
    pub imagery_providers: Vec<ProviderViewModel>,
    /// The available terrain providers.
    pub terrain_providers: Vec<ProviderViewModel>,
    /// The index of the selected imagery provider.
    pub selected_imagery_index: Option<usize>,
    /// The index of the selected terrain provider.
    pub selected_terrain_index: Option<usize>,
    /// Whether the drop-down panel is visible.
    pub drop_down_visible: bool,
}

impl BaseLayerPickerViewModel {
    /// Creates a new base layer picker view model.
    pub fn new() -> Self {
        Self {
            imagery_providers: Vec::new(),
            terrain_providers: Vec::new(),
            selected_imagery_index: None,
            selected_terrain_index: None,
            drop_down_visible: false,
        }
    }

    /// Toggles the drop-down visibility.
    pub fn toggle_drop_down(&mut self) {
        self.drop_down_visible = !self.drop_down_visible;
    }
}

impl Default for BaseLayerPickerViewModel {
    fn default() -> Self { Self::new() }
}
