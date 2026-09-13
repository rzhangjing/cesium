//! Base layer picker view model.
//!
//! Maps to CesiumJS `BaseLayerPicker/BaseLayerPickerViewModel.js`.

/// A category of providers (e.g., "Imagery", "Terrain").
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderCategory {
    /// Category name.
    pub name: String,
    /// Provider view models in this category.
    pub providers: Vec<ProviderViewModel>,
}

impl ProviderCategory {
    /// Create a new provider category.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            providers: Vec::new(),
        }
    }

    /// Add a provider to this category.
    pub fn add_provider(&mut self, provider: ProviderViewModel) {
        self.providers.push(provider);
    }

    /// Get the number of providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

/// A view model representing a single imagery/terrain provider option.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderViewModel {
    /// Display name.
    pub name: String,
    /// Tooltip text.
    pub tooltip: String,
    /// Icon URL or identifier.
    pub icon_url: String,
    /// Provider category name.
    pub category: String,
    /// Whether this provider is currently selected.
    pub is_selected: bool,
    /// The provider creation parameters (URL, key, etc.).
    pub creation_parameters: serde_json::Value,
}

impl ProviderViewModel {
    /// Create a new provider view model.
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            tooltip: name.clone(),
            icon_url: String::new(),
            is_selected: false,
            creation_parameters: serde_json::Value::Null,
            name,
            category: category.into(),
        }
    }

    /// Set the tooltip.
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }

    /// Set the icon URL.
    pub fn with_icon(mut self, icon_url: impl Into<String>) -> Self {
        self.icon_url = icon_url.into();
        self
    }

    /// Set creation parameters.
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.creation_parameters = params;
        self
    }
}

/// Base layer picker view model.
///
/// Controls selection of imagery and terrain providers.
#[derive(Debug, Clone)]
pub struct BaseLayerPickerViewModel {
    /// Whether the picker dropdown is open.
    pub is_dropdown_open: bool,
    /// Whether the widget is visible.
    pub show: bool,
    /// Provider categories.
    pub categories: Vec<ProviderCategory>,
    /// Index of the selected imagery provider (within its category).
    pub selected_imagery_index: Option<(usize, usize)>,
    /// Index of the selected terrain provider (within its category).
    pub selected_terrain_index: Option<(usize, usize)>,
}

impl Default for BaseLayerPickerViewModel {
    fn default() -> Self {
        Self {
            is_dropdown_open: false,
            show: true,
            categories: Vec::new(),
            selected_imagery_index: None,
            selected_terrain_index: None,
        }
    }
}

impl BaseLayerPickerViewModel {
    /// Create a new base layer picker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a provider category.
    pub fn add_category(&mut self, category: ProviderCategory) {
        self.categories.push(category);
    }

    /// Toggle the dropdown.
    pub fn toggle_dropdown(&mut self) {
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// Close the dropdown.
    pub fn close_dropdown(&mut self) {
        self.is_dropdown_open = false;
    }

    /// Select an imagery provider by category and provider index.
    pub fn select_imagery(&mut self, category_idx: usize, provider_idx: usize) {
        // Deselect previous
        if let Some((ci, pi)) = self.selected_imagery_index {
            if let Some(cat) = self.categories.get_mut(ci) {
                if let Some(prov) = cat.providers.get_mut(pi) {
                    prov.is_selected = false;
                }
            }
        }

        // Select new
        if let Some(cat) = self.categories.get_mut(category_idx) {
            if let Some(prov) = cat.providers.get_mut(provider_idx) {
                prov.is_selected = true;
                self.selected_imagery_index = Some((category_idx, provider_idx));
            }
        }

        self.is_dropdown_open = false;
    }

    /// Select a terrain provider by category and provider index.
    pub fn select_terrain(&mut self, category_idx: usize, provider_idx: usize) {
        // Deselect previous
        if let Some((ci, pi)) = self.selected_terrain_index {
            if let Some(cat) = self.categories.get_mut(ci) {
                if let Some(prov) = cat.providers.get_mut(pi) {
                    prov.is_selected = false;
                }
            }
        }

        // Select new
        if let Some(cat) = self.categories.get_mut(category_idx) {
            if let Some(prov) = cat.providers.get_mut(provider_idx) {
                prov.is_selected = true;
                self.selected_terrain_index = Some((category_idx, provider_idx));
            }
        }

        self.is_dropdown_open = false;
    }

    /// Get the currently selected imagery provider.
    pub fn selected_imagery_provider(&self) -> Option<&ProviderViewModel> {
        let (ci, pi) = self.selected_imagery_index?;
        self.categories.get(ci)?.providers.get(pi)
    }

    /// Get the currently selected terrain provider.
    pub fn selected_terrain_provider(&self) -> Option<&ProviderViewModel> {
        let (ci, pi) = self.selected_terrain_index?;
        self.categories.get(ci)?.providers.get(pi)
    }

    /// Get the total number of providers across all categories.
    pub fn total_provider_count(&self) -> usize {
        self.categories.iter().map(|c| c.provider_count()).sum()
    }

    /// Get the button tooltip showing the current selection.
    pub fn button_tooltip(&self) -> String {
        if let Some(prov) = self.selected_imagery_provider() {
            format!("Current imagery: {}", prov.name)
        } else {
            "Select base layer".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_picker() -> BaseLayerPickerViewModel {
        let mut vm = BaseLayerPickerViewModel::new();

        let mut imagery_cat = ProviderCategory::new("Imagery");
        imagery_cat.add_provider(
            ProviderViewModel::new("Bing Maps", "Imagery")
                .with_tooltip("Bing Maps aerial imagery"),
        );
        imagery_cat.add_provider(
            ProviderViewModel::new("OpenStreetMap", "Imagery")
                .with_tooltip("OSM street map"),
        );

        let mut terrain_cat = ProviderCategory::new("Terrain");
        terrain_cat.add_provider(
            ProviderViewModel::new("Cesium World Terrain", "Terrain")
                .with_tooltip("High-res terrain"),
        );

        vm.add_category(imagery_cat);
        vm.add_category(terrain_cat);
        vm
    }

    #[test]
    fn test_default() {
        let vm = BaseLayerPickerViewModel::default();
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert!(vm.categories.is_empty());
        assert!(vm.selected_imagery_index.is_none());
    }

    #[test]
    fn test_add_categories() {
        let vm = make_test_picker();
        assert_eq!(vm.categories.len(), 2);
        assert_eq!(vm.total_provider_count(), 3);
    }

    #[test]
    fn test_select_imagery() {
        let mut vm = make_test_picker();
        vm.select_imagery(0, 1); // OpenStreetMap
        let selected = vm.selected_imagery_provider().unwrap();
        assert_eq!(selected.name, "OpenStreetMap");
        assert!(selected.is_selected);
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_select_imagery_deselects_previous() {
        let mut vm = make_test_picker();
        vm.select_imagery(0, 0); // Bing
        vm.select_imagery(0, 1); // OSM
        assert!(!vm.categories[0].providers[0].is_selected);
        assert!(vm.categories[0].providers[1].is_selected);
    }

    #[test]
    fn test_select_terrain() {
        let mut vm = make_test_picker();
        vm.select_terrain(1, 0);
        let selected = vm.selected_terrain_provider().unwrap();
        assert_eq!(selected.name, "Cesium World Terrain");
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = make_test_picker();
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_button_tooltip() {
        let mut vm = make_test_picker();
        assert_eq!(vm.button_tooltip(), "Select base layer");
        vm.select_imagery(0, 0);
        assert_eq!(vm.button_tooltip(), "Current imagery: Bing Maps");
    }

    #[test]
    fn test_provider_view_model_builder() {
        let prov = ProviderViewModel::new("Test", "Cat")
            .with_tooltip("A tooltip")
            .with_icon("icon.png")
            .with_parameters(serde_json::json!({"url": "http://example.com"}));
        assert_eq!(prov.name, "Test");
        assert_eq!(prov.tooltip, "A tooltip");
        assert_eq!(prov.icon_url, "icon.png");
        assert_eq!(prov.creation_parameters["url"], "http://example.com");
    }

    #[test]
    fn test_invalid_selection() {
        let mut vm = make_test_picker();
        vm.select_imagery(99, 0); // Invalid category
        // Selection should not be set for invalid indices
        assert!(vm.selected_imagery_index.is_none());
        assert!(vm.selected_imagery_provider().is_none());
    }
}
