//! Geocoder widget view model.
//!
//! Maps to CesiumJS `Geocoder/GeocoderViewModel.js`.

/// A geocoder search result for display.
#[derive(Debug, Clone, PartialEq)]
pub struct GeocoderSearchResult {
    /// Display name of the result.
    pub display_name: String,
    /// Destination description (rectangle or point).
    pub destination: GeocoderSearchDestination,
}

/// Destination of a geocoder result.
#[derive(Debug, Clone, PartialEq)]
pub enum GeocoderSearchDestination {
    /// A rectangle [west, south, east, north] in radians.
    Rectangle([f64; 4]),
    /// A point with longitude, latitude, and optional height.
    Point {
        /// Longitude in radians.
        longitude: f64,
        /// Latitude in radians.
        latitude: f64,
        /// Height in meters.
        height: Option<f64>,
    },
}

/// Geocoder widget view model.
///
/// Provides search-as-you-type geocoding functionality.
#[derive(Debug, Clone)]
pub struct GeocoderViewModel {
    /// The current search text.
    pub search_text: String,
    /// Whether a search is in progress.
    pub is_searching: bool,
    /// Search results.
    pub results: Vec<GeocoderSearchResult>,
    /// Whether the results panel is visible.
    pub show_results: bool,
    /// Index of the currently highlighted result.
    pub selected_index: Option<usize>,
    /// Whether the widget is visible.
    pub show: bool,
    /// Whether autocomplete is enabled.
    pub auto_complete: bool,
    /// Minimum characters before triggering search.
    pub min_chars: usize,
    /// Flight duration to destination in seconds.
    pub flight_duration: f64,
    /// Placeholder text for the input.
    pub placeholder: String,
}

impl Default for GeocoderViewModel {
    fn default() -> Self {
        Self {
            search_text: String::new(),
            is_searching: false,
            results: Vec::new(),
            show_results: false,
            selected_index: None,
            show: true,
            auto_complete: true,
            min_chars: 3,
            flight_duration: 1.5,
            placeholder: "Enter an address or landmark...".to_string(),
        }
    }
}

impl GeocoderViewModel {
    /// Create a new geocoder view model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the search text.
    pub fn set_search_text(&mut self, text: impl Into<String>) {
        self.search_text = text.into();
        self.selected_index = None;
        if self.search_text.len() < self.min_chars {
            self.results.clear();
            self.show_results = false;
        }
    }

    /// Check if the search text is long enough to trigger a search.
    pub fn should_search(&self) -> bool {
        self.search_text.len() >= self.min_chars && !self.is_searching
    }

    /// Begin a search operation.
    pub fn begin_search(&mut self) {
        if self.should_search() {
            self.is_searching = true;
        }
    }

    /// Complete a search with results.
    pub fn complete_search(&mut self, results: Vec<GeocoderSearchResult>) {
        self.is_searching = false;
        self.results = results;
        self.show_results = !self.results.is_empty();
        self.selected_index = if self.results.is_empty() { None } else { Some(0) };
    }

    /// Clear the search.
    pub fn clear_search(&mut self) {
        self.search_text.clear();
        self.results.clear();
        self.show_results = false;
        self.selected_index = None;
        self.is_searching = false;
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(0) => self.results.len() - 1,
            Some(i) => i - 1,
            None => 0,
        });
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(i) if i >= self.results.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        });
    }

    /// Get the currently selected result.
    pub fn selected_result(&self) -> Option<&GeocoderSearchResult> {
        self.results.get(self.selected_index?)
    }

    /// Activate the selected result (fly to destination).
    pub fn activate_selected(&mut self) -> Option<GeocoderSearchResult> {
        let result = self.selected_result()?.clone();
        self.search_text = result.display_name.clone();
        self.show_results = false;
        Some(result)
    }

    /// Hide the results panel.
    pub fn hide_results(&mut self) {
        self.show_results = false;
    }

    /// Show the results panel.
    pub fn show_results_panel(&mut self) {
        if !self.results.is_empty() {
            self.show_results = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_results() -> Vec<GeocoderSearchResult> {
        vec![
            GeocoderSearchResult {
                display_name: "New York, NY".to_string(),
                destination: GeocoderSearchDestination::Point {
                    longitude: -1.2921,
                    latitude: 0.7106,
                    height: None,
                },
            },
            GeocoderSearchResult {
                display_name: "New Orleans, LA".to_string(),
                destination: GeocoderSearchDestination::Point {
                    longitude: -1.5708,
                    latitude: 0.5236,
                    height: None,
                },
            },
            GeocoderSearchResult {
                display_name: "United States".to_string(),
                destination: GeocoderSearchDestination::Rectangle([
                    -2.2092, 0.3194, -1.1636, 0.8538,
                ]),
            },
        ]
    }

    #[test]
    fn test_default() {
        let vm = GeocoderViewModel::default();
        assert!(vm.search_text.is_empty());
        assert!(!vm.is_searching);
        assert!(vm.results.is_empty());
        assert!(vm.auto_complete);
        assert_eq!(vm.min_chars, 3);
    }

    #[test]
    fn test_set_search_text() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        assert_eq!(vm.search_text, "New");
        assert!(vm.should_search());
    }

    #[test]
    fn test_min_chars() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("Ne");
        assert!(!vm.should_search());
        vm.set_search_text("New");
        assert!(vm.should_search());
    }

    #[test]
    fn test_search_flow() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New York");
        vm.begin_search();
        assert!(vm.is_searching);

        vm.complete_search(sample_results());
        assert!(!vm.is_searching);
        assert_eq!(vm.results.len(), 3);
        assert!(vm.show_results);
        assert_eq!(vm.selected_index, Some(0));
    }

    #[test]
    fn test_navigation() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());

        assert_eq!(vm.selected_index, Some(0));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(1));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(2));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(0)); // Wrap around

        vm.select_previous();
        assert_eq!(vm.selected_index, Some(2)); // Wrap back
    }

    #[test]
    fn test_activate_selected() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());

        let result = vm.activate_selected().unwrap();
        assert_eq!(result.display_name, "New York, NY");
        assert_eq!(vm.search_text, "New York, NY");
        assert!(!vm.show_results);
    }

    #[test]
    fn test_clear_search() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());
        vm.clear_search();

        assert!(vm.search_text.is_empty());
        assert!(vm.results.is_empty());
        assert!(!vm.show_results);
        assert!(vm.selected_index.is_none());
    }

    #[test]
    fn test_empty_results() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("xyzzy");
        vm.begin_search();
        vm.complete_search(vec![]);

        assert!(!vm.show_results);
        assert!(vm.selected_index.is_none());
        assert!(vm.selected_result().is_none());
    }
}
