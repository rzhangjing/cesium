//! Extended widgets specs: Buttons, BaseLayerPicker, InfoBox
//! Ported from CesiumJS widgets/Source/ (HomeButton, FullscreenButton, NavigationHelp,
//! VRButton, BaseLayerPicker, InfoBox)
//!
//! A-class tests: ToggleButton, HomeButton, FullscreenButton, NavigationHelp,
//! VRButton, BaseLayerPicker categories/providers/selection, InfoBox show/clear/tracking

use cesium_widgets::{
    BaseLayerPickerViewModel, FullscreenButtonViewModel, HomeButtonViewModel, InfoBoxViewModel,
    NavigationHelpButtonViewModel, ProviderCategory, ProviderViewModel, ToggleButtonViewModel,
    VRButtonViewModel,
};

// ─── ToggleButtonViewModel ─────────────────────────────────────────────────────

#[test]
fn toggle_button_new_and_toggle() {
    let mut btn = ToggleButtonViewModel::new("Test Button");
    assert!(!btn.is_toggled);
    assert!(btn.show);
    assert!(btn.is_enabled);
    assert_eq!(btn.tooltip, "Test Button");

    btn.toggle();
    assert!(btn.is_toggled);

    btn.toggle();
    assert!(!btn.is_toggled);
}

#[test]
fn toggle_button_disabled_no_toggle() {
    let mut btn = ToggleButtonViewModel::new("Disabled");
    btn.is_enabled = false;

    btn.toggle();
    assert!(!btn.is_toggled); // Should not change

    btn.set_toggled(true);
    assert!(!btn.is_toggled); // Should not change
}

// ─── HomeButtonViewModel ───────────────────────────────────────────────────────

#[test]
fn home_button_defaults() {
    let btn = HomeButtonViewModel::new();
    assert_eq!(btn.tooltip, "View Home");
    assert!(btn.show);
    assert!((btn.duration - 1.5).abs() < 1e-10);
    assert_eq!(btn.home_longitude, 0.0);
    assert_eq!(btn.home_latitude, 0.0);
    assert!((btn.home_height - 15_000_000.0).abs() < 1e-10);
}

#[test]
fn home_button_set_home() {
    let mut btn = HomeButtonViewModel::new();
    btn.set_home(1.0, 0.5, 1000.0);

    let (lon, lat, h) = btn.home_position();
    assert!((lon - 1.0).abs() < 1e-10);
    assert!((lat - 0.5).abs() < 1e-10);
    assert!((h - 1000.0).abs() < 1e-10);
}

// ─── FullscreenButtonViewModel ─────────────────────────────────────────────────

#[test]
fn fullscreen_button_toggle() {
    let mut btn = FullscreenButtonViewModel::new();
    assert!(!btn.is_fullscreen);
    assert!(btn.is_supported);
    assert_eq!(btn.current_tooltip(), "Full screen");

    btn.toggle_fullscreen();
    assert!(btn.is_fullscreen);
    assert_eq!(btn.current_tooltip(), "Exit full screen");

    btn.toggle_fullscreen();
    assert!(!btn.is_fullscreen);
}

#[test]
fn fullscreen_button_unsupported() {
    let mut btn = FullscreenButtonViewModel::new();
    btn.is_supported = false;

    btn.toggle_fullscreen();
    assert!(!btn.is_fullscreen); // Should not change
}

// ─── NavigationHelpButtonViewModel ─────────────────────────────────────────────

#[test]
fn navigation_help_toggle() {
    let mut btn = NavigationHelpButtonViewModel::new();
    assert!(!btn.is_help_visible);
    assert!(!btn.show_touch);

    btn.toggle_help();
    assert!(btn.is_help_visible);

    btn.hide_help();
    assert!(!btn.is_help_visible);

    btn.show_help();
    assert!(btn.is_help_visible);
}

// ─── VRButtonViewModel ─────────────────────────────────────────────────────────

#[test]
fn vr_button_defaults() {
    let btn = VRButtonViewModel::new();
    assert!(btn.show);
    assert!(!btn.is_vr_active);
}

// ─── BaseLayerPickerViewModel ──────────────────────────────────────────────────

#[test]
fn base_layer_picker_defaults() {
    let vm = BaseLayerPickerViewModel::new();
    assert!(!vm.is_dropdown_open);
    assert!(vm.show);
    assert!(vm.categories.is_empty());
    assert!(vm.selected_imagery_index.is_none());
    assert!(vm.selected_terrain_index.is_none());
}

#[test]
fn base_layer_picker_add_categories() {
    let mut vm = BaseLayerPickerViewModel::new();

    let mut imagery_cat = ProviderCategory::new("Imagery");
    imagery_cat.add_provider(
        ProviderViewModel::new("Bing Maps", "Imagery")
            .with_tooltip("Bing Maps Aerial")
            .with_icon("bing.png"),
    );
    imagery_cat.add_provider(ProviderViewModel::new("OpenStreetMap", "Imagery"));

    let terrain_cat = ProviderCategory::new("Terrain");

    vm.categories.push(imagery_cat);
    vm.categories.push(terrain_cat);

    assert_eq!(vm.categories.len(), 2);
    assert_eq!(vm.categories[0].provider_count(), 2);
    assert_eq!(vm.categories[1].provider_count(), 0);
}

#[test]
fn provider_view_model_builder() {
    let pvm = ProviderViewModel::new("Sentinel-2", "Imagery")
        .with_tooltip("Sentinel-2 Cloudless")
        .with_icon("sentinel.png")
        .with_parameters(serde_json::json!({"url": "https://example.com"}));

    assert_eq!(pvm.name, "Sentinel-2");
    assert_eq!(pvm.category, "Imagery");
    assert_eq!(pvm.tooltip, "Sentinel-2 Cloudless");
    assert_eq!(pvm.icon_url, "sentinel.png");
    assert!(!pvm.is_selected);
    assert_eq!(pvm.creation_parameters["url"], "https://example.com");
}

// ─── InfoBoxViewModel ──────────────────────────────────────────────────────────

#[test]
fn info_box_defaults() {
    let vm = InfoBoxViewModel::new();
    assert!(vm.show);
    assert!(!vm.is_frame_visible);
    assert!(vm.title.is_empty());
    assert!(vm.description.is_empty());
    assert!(vm.show_close);
    assert!(!vm.has_content);
    assert!(!vm.is_tracking);
    assert!(vm.camera_view_offset.is_none());
}

#[test]
fn info_box_show_entity() {
    let mut vm = InfoBoxViewModel::new();
    vm.show_entity("My Entity", "A test entity description");

    assert_eq!(vm.title, "My Entity");
    assert_eq!(vm.description, "A test entity description");
    assert!(vm.has_content);
    assert!(vm.is_frame_visible);
}

#[test]
fn info_box_clear() {
    let mut vm = InfoBoxViewModel::new();
    vm.show_entity("Title", "Desc");
    vm.set_tracking(true);
    vm.set_camera_offset([1.0, 2.0, 3.0]);

    vm.clear();

    assert!(vm.title.is_empty());
    assert!(vm.description.is_empty());
    assert!(!vm.has_content);
    assert!(!vm.is_frame_visible);
    assert!(!vm.is_tracking);
    assert!(vm.camera_view_offset.is_none());
}

#[test]
fn info_box_toggle_frame() {
    let mut vm = InfoBoxViewModel::new();

    // No content → toggle does nothing
    vm.toggle_frame();
    assert!(!vm.is_frame_visible);

    // With content → toggle works
    vm.show_entity("T", "D");
    assert!(vm.is_frame_visible);
    vm.toggle_frame();
    assert!(!vm.is_frame_visible);
    vm.toggle_frame();
    assert!(vm.is_frame_visible);
}

#[test]
fn info_box_close() {
    let mut vm = InfoBoxViewModel::new();
    vm.show_entity("T", "D");
    vm.close();
    assert!(!vm.is_frame_visible);
    assert!(vm.has_content); // Content preserved
}

#[test]
fn info_box_summary() {
    let mut vm = InfoBoxViewModel::new();
    assert_eq!(vm.summary(), "");

    vm.show_entity("T", "Short desc");
    assert_eq!(vm.summary(), "Short desc");
}
