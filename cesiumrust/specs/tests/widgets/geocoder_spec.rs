//! Geocoder specs - ported from Widgets/GeocoderSpec, GeocoderViewModelSpec
//! Covers: GeocoderViewModel, GeocoderSearchResult

use cesium_widgets::geocoder::{GeocoderSearchDestination, GeocoderSearchResult, GeocoderViewModel};

// ─── GeocoderViewModel ──────────────────────────────────────────────────────

#[test]
fn geocoder_viewmodel_new() {
    let vm = GeocoderViewModel::new();
    assert!(vm.search_text.is_empty());
    assert!(!vm.is_searching);
    assert!(vm.results.is_empty());
}

#[test]
fn geocoder_set_search_text() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("San Francisco");
    assert_eq!(vm.search_text, "San Francisco");
}

#[test]
fn geocoder_should_search_min_chars() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("ab");
    assert!(!vm.should_search(), "should not search with < min_chars");
    vm.set_search_text("abc");
    assert!(vm.should_search(), "should search with >= min_chars");
}

#[test]
fn geocoder_begin_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("Tokyo");
    vm.begin_search();
    assert!(vm.is_searching);
}

#[test]
fn geocoder_complete_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("Tokyo");
    vm.begin_search();
    let results = vec![
        GeocoderSearchResult {
            display_name: "Tokyo, Japan".to_string(),
            destination: GeocoderSearchDestination::Rectangle([2.4, 0.6, 2.5, 0.7]),
        },
        GeocoderSearchResult {
            display_name: "Tokyo, TX".to_string(),
            destination: GeocoderSearchDestination::Point {
                longitude: -1.7,
                latitude: 0.6,
                height: None,
            },
        },
    ];
    vm.complete_search(results);
    assert!(!vm.is_searching);
    assert_eq!(vm.results.len(), 2);
    assert!(vm.show_results);
}

#[test]
fn geocoder_clear_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.clear_search();
    assert!(vm.search_text.is_empty());
    assert!(vm.results.is_empty());
    assert!(!vm.show_results);
}

#[test]
fn geocoder_select_next_previous() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.complete_search(vec![
        GeocoderSearchResult {
            display_name: "Result 1".to_string(),
            destination: GeocoderSearchDestination::Rectangle([0.0, 0.0, 1.0, 1.0]),
        },
        GeocoderSearchResult {
            display_name: "Result 2".to_string(),
            destination: GeocoderSearchDestination::Rectangle([1.0, 1.0, 2.0, 2.0]),
        },
    ]);
    // complete_search sets selected_index to Some(0)
    assert_eq!(vm.selected_index, Some(0));
    vm.select_next();
    assert_eq!(vm.selected_index, Some(1));
    vm.select_next();
    // wraps around
    assert_eq!(vm.selected_index, Some(0));
    vm.select_previous();
    // wraps to last
    assert_eq!(vm.selected_index, Some(1));
}

#[test]
fn geocoder_selected_result() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.complete_search(vec![GeocoderSearchResult {
        display_name: "Only Result".to_string(),
        destination: GeocoderSearchDestination::Point {
            longitude: 0.0,
            latitude: 0.0,
            height: Some(100.0),
        },
    }]);
    // complete_search auto-selects first result
    let result = vm.selected_result().unwrap();
    assert_eq!(result.display_name, "Only Result");
}

#[test]
fn geocoder_activate_selected() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.complete_search(vec![GeocoderSearchResult {
        display_name: "Activated".to_string(),
        destination: GeocoderSearchDestination::Rectangle([0.0, 0.0, 1.0, 1.0]),
    }]);
    vm.select_next();
    let activated = vm.activate_selected();
    assert!(activated.is_some());
    assert_eq!(vm.search_text, "Activated");
    assert!(!vm.show_results);
}

#[test]
fn geocoder_hide_show_results() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.complete_search(vec![GeocoderSearchResult {
        display_name: "R".to_string(),
        destination: GeocoderSearchDestination::Point {
            longitude: 0.0,
            latitude: 0.0,
            height: None,
        },
    }]);
    assert!(vm.show_results);
    vm.hide_results();
    assert!(!vm.show_results);
    vm.show_results_panel();
    assert!(vm.show_results);
}
