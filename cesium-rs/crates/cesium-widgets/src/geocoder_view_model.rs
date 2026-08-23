//! Ported from `packages/widgets/Source/Geocoder/GeocoderViewModel.js`.

/// The view model for the Geocoder widget.
///
/// Provides location search functionality.
pub struct GeocoderViewModel {
    /// The search text.
    pub search_text: String,
    /// Whether a search is in progress.
    pub is_searching: bool,
    /// Whether the geocoder suggestions panel is visible.
    pub suggestions_visible: bool,
}

impl GeocoderViewModel {
    /// Creates a new geocoder view model.
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            is_searching: false,
            suggestions_visible: false,
        }
    }

    /// Initiates a search.
    pub fn search(&mut self) {
        if !self.search_text.is_empty() {
            self.is_searching = true;
            // DEVIATION: Requires geocoder service integration
        }
    }
}

impl Default for GeocoderViewModel {
    fn default() -> Self { Self::new() }
}
