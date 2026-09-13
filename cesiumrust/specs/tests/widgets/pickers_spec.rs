//! Widgets/SceneModePickerSpec.js, ProjectionPickerSpec.js, BaseLayerPickerSpec.js
//! → Rust integration tests

use cesium_widgets::{
    SceneModePickerViewModel, ProjectionPickerViewModel, ProjectionType,
    BaseLayerPickerViewModel, ProviderViewModel, ProviderCategory,
};

// === SceneModePickerViewModel ===

#[test]
fn test_scene_mode_picker_default() {
    let vm = SceneModePickerViewModel::default();
    assert!(vm.show);
}

// === ProjectionPickerViewModel ===

#[test]
fn test_projection_picker_default() {
    let vm = ProjectionPickerViewModel::default();
    assert!(vm.show);
}

#[test]
fn test_projection_type_variants() {
    assert_ne!(ProjectionType::Perspective, ProjectionType::Orthographic);
    assert_eq!(ProjectionType::default(), ProjectionType::Perspective);
}

// === BaseLayerPickerViewModel ===

#[test]
fn test_base_layer_picker_default() {
    let vm = BaseLayerPickerViewModel::default();
    assert!(vm.show);
}

// === ProviderViewModel ===

#[test]
fn test_provider_view_model_new() {
    let provider = ProviderViewModel::new("OpenStreetMap", "Imagery");
    assert_eq!(provider.name, "OpenStreetMap");
    assert_eq!(provider.category, "Imagery");
    assert!(!provider.is_selected);
}

// === ProviderCategory ===

#[test]
fn test_provider_category_new() {
    let cat = ProviderCategory::new("Imagery");
    assert_eq!(cat.name, "Imagery");
    assert!(cat.providers.is_empty());
}
