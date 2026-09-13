//! Widgets/HomeButtonSpec.js, FullscreenButtonSpec.js, NavigationHelpButtonSpec.js, VRButtonSpec.js
//! → Rust integration tests

use cesium_widgets::{
    HomeButtonViewModel, FullscreenButtonViewModel,
    NavigationHelpButtonViewModel, VRButtonViewModel,
};

// === HomeButtonViewModel ===

#[test]
fn test_home_button_default() {
    let vm = HomeButtonViewModel::default();
    assert!(vm.show);
    assert!(vm.duration > 0.0);
}

// === FullscreenButtonViewModel ===

#[test]
fn test_fullscreen_button_default() {
    let vm = FullscreenButtonViewModel::default();
    assert!(!vm.is_fullscreen);
    assert!(vm.show);
}

// === NavigationHelpButtonViewModel ===

#[test]
fn test_navigation_help_button_default() {
    let vm = NavigationHelpButtonViewModel::default();
    assert!(!vm.is_help_visible);
    assert!(vm.show);
    assert!(!vm.show_touch);
}

// === VRButtonViewModel ===

#[test]
fn test_vr_button_default() {
    let vm = VRButtonViewModel::default();
    assert!(!vm.is_vr_active);
    assert!(vm.show);
}
