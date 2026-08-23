//! Ported from `packages/widgets/Source/BaseLayerPicker/ProviderViewModel.js`.

use crate::command::Command;

/// A view model for a single imagery or terrain provider option.
pub struct ProviderViewModel {
    /// The display name.
    pub name: String,
    /// The tooltip text.
    pub tooltip: String,
    /// The icon URL.
    pub icon_url: String,
    /// Whether this provider is currently selected.
    pub is_selected: bool,
    /// The command to select this provider.
    pub selection_command: Command,
}

impl ProviderViewModel {
    /// Creates a new provider view model.
    pub fn new(name: &str, tooltip: &str, icon_url: &str) -> Self {
        Self {
            name: name.to_string(),
            tooltip: tooltip.to_string(),
            icon_url: icon_url.to_string(),
            is_selected: false,
            selection_command: Command::empty(),
        }
    }
}
