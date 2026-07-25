//! Widgets/InfoBoxSpec.js, SelectionIndicatorSpec.js, i18n
//! → Rust integration tests

use cesium_widgets::{InfoBoxViewModel, SelectionIndicatorViewModel, Locale, I18n};

// === InfoBoxViewModel ===

#[test]
fn test_info_box_default() {
    let vm = InfoBoxViewModel::default();
    assert!(vm.show);
    assert!(vm.title.is_empty());
    assert!(vm.description.is_empty());
}

#[test]
fn test_info_box_with_content() {
    let mut vm = InfoBoxViewModel::default();
    vm.title = "Test Entity".to_string();
    vm.description = "<p>Hello</p>".to_string();
    vm.has_content = true;
    vm.show = true;
    assert!(vm.show);
    assert!(vm.has_content);
    assert_eq!(vm.title, "Test Entity");
}

// === SelectionIndicatorViewModel ===

#[test]
fn test_selection_indicator_default() {
    let vm = SelectionIndicatorViewModel::default();
    assert!(!vm.show);
    assert_eq!(vm.scale, 1.0);
}

#[test]
fn test_selection_indicator_position() {
    let mut vm = SelectionIndicatorViewModel::default();
    vm.screen_x = 500.0;
    vm.screen_y = 300.0;
    vm.show = true;
    assert_eq!(vm.screen_x, 500.0);
    assert_eq!(vm.screen_y, 300.0);
}

// === Locale ===

#[test]
fn test_locale_default() {
    assert_eq!(Locale::default(), Locale::En);
}

#[test]
fn test_locale_variants() {
    assert_ne!(Locale::En, Locale::ZhCn);
    assert_ne!(Locale::ZhCn, Locale::Ja);
}

// === I18n ===

#[test]
fn test_i18n_default() {
    let i18n = I18n::default();
    assert_eq!(i18n.current_locale, Locale::En);
}
