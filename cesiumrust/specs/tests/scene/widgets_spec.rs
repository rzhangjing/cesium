//! Widgets view model specs
//! Ported from CesiumJS widgets/Source/ (SceneModePicker, SelectionIndicator, I18n)
//!
//! A-class tests: SceneModePickerViewModel, SelectionIndicatorViewModel,
//! Locale/I18n, ProjectionPickerViewModel

use cesium_scene_mode::SceneMode;
use cesium_widgets::{
    I18n, Locale, ProjectionPickerViewModel, ProjectionType, SceneModePickerViewModel,
    SelectionIndicatorViewModel, WidgetStrings,
};

// ─── SceneModePickerViewModel ──────────────────────────────────────────────────

#[test]
fn scene_mode_picker_defaults() {
    let vm = SceneModePickerViewModel::new();
    assert_eq!(vm.selected_mode, SceneMode::Scene3D);
    assert!(!vm.is_dropdown_open);
    assert!(vm.show);
    assert!((vm.morph_duration - 2.0).abs() < 1e-10);
}

#[test]
fn scene_mode_picker_select_modes() {
    let mut vm = SceneModePickerViewModel::new();

    vm.select_2d();
    assert_eq!(vm.selected_mode, SceneMode::Scene2D);
    assert!(!vm.is_dropdown_open);

    vm.select_columbus_view();
    assert_eq!(vm.selected_mode, SceneMode::ColumbusView);

    vm.select_3d();
    assert_eq!(vm.selected_mode, SceneMode::Scene3D);
}

#[test]
fn scene_mode_picker_ignores_morphing() {
    let mut vm = SceneModePickerViewModel::new();
    vm.select_2d();
    vm.select_mode(SceneMode::Morphing);
    // Should not change to Morphing
    assert_eq!(vm.selected_mode, SceneMode::Scene2D);
}

#[test]
fn scene_mode_picker_dropdown() {
    let mut vm = SceneModePickerViewModel::new();
    assert!(!vm.is_dropdown_open);

    vm.toggle_dropdown();
    assert!(vm.is_dropdown_open);

    vm.toggle_dropdown();
    assert!(!vm.is_dropdown_open);

    vm.toggle_dropdown();
    vm.close_dropdown();
    assert!(!vm.is_dropdown_open);
}

#[test]
fn scene_mode_picker_labels() {
    let mut vm = SceneModePickerViewModel::new();
    assert_eq!(vm.current_label(), "3D");

    vm.select_2d();
    assert_eq!(vm.current_label(), "2D");

    vm.select_columbus_view();
    assert_eq!(vm.current_label(), "Columbus View");
}

#[test]
fn scene_mode_picker_available_modes() {
    let modes = SceneModePickerViewModel::available_modes();
    assert_eq!(modes.len(), 3);
    assert!(modes.contains(&SceneMode::Scene3D));
    assert!(modes.contains(&SceneMode::Scene2D));
    assert!(modes.contains(&SceneMode::ColumbusView));
    assert!(!modes.contains(&SceneMode::Morphing));
}

#[test]
fn scene_mode_picker_is_mode_selected() {
    let mut vm = SceneModePickerViewModel::new();
    assert!(vm.is_mode_selected(SceneMode::Scene3D));
    assert!(!vm.is_mode_selected(SceneMode::Scene2D));

    vm.select_2d();
    assert!(!vm.is_mode_selected(SceneMode::Scene3D));
    assert!(vm.is_mode_selected(SceneMode::Scene2D));
}

// ─── SelectionIndicatorViewModel ───────────────────────────────────────────────

#[test]
fn selection_indicator_defaults() {
    let vm = SelectionIndicatorViewModel::new();
    assert!(!vm.show);
    assert_eq!(vm.screen_x, 0.0);
    assert_eq!(vm.screen_y, 0.0);
    assert_eq!(vm.scale, 1.0);
    assert!(!vm.is_animating);
    assert!(!vm.is_on_screen);
}

#[test]
fn selection_indicator_show_at() {
    let mut vm = SelectionIndicatorViewModel::new();
    vm.show_at(100.0, 200.0);

    assert!(vm.show);
    assert_eq!(vm.screen_x, 100.0);
    assert_eq!(vm.screen_y, 200.0);
    assert!(vm.is_on_screen);
    assert!(vm.is_animating);
    assert_eq!(vm.animation_progress, 0.0);
}

#[test]
fn selection_indicator_hide() {
    let mut vm = SelectionIndicatorViewModel::new();
    vm.show_at(50.0, 50.0);
    vm.hide();

    assert!(!vm.show);
    assert!(!vm.is_on_screen);
    assert!(!vm.is_animating);
    assert_eq!(vm.animation_progress, 0.0);
}

#[test]
fn selection_indicator_update_position() {
    let mut vm = SelectionIndicatorViewModel::new();
    vm.show_at(10.0, 20.0);
    vm.update_position(300.0, 400.0, true);

    assert_eq!(vm.screen_x, 300.0);
    assert_eq!(vm.screen_y, 400.0);
    assert!(vm.is_on_screen);

    vm.update_position(0.0, 0.0, false);
    assert!(!vm.is_on_screen);
}

// ─── Locale / I18n ─────────────────────────────────────────────────────────────

#[test]
fn locale_code_and_from_code() {
    assert_eq!(Locale::En.code(), "en");
    assert_eq!(Locale::ZhCn.code(), "zh-CN");
    assert_eq!(Locale::Ja.code(), "ja");

    assert_eq!(Locale::from_code("en"), Some(Locale::En));
    assert_eq!(Locale::from_code("zh-CN"), Some(Locale::ZhCn));
    assert_eq!(Locale::from_code("zh"), Some(Locale::ZhCn));
    assert_eq!(Locale::from_code("ja"), Some(Locale::Ja));
    assert_eq!(Locale::from_code("xx"), None);
}

#[test]
fn locale_all() {
    let all = Locale::all();
    assert_eq!(all.len(), 6);
    assert!(all.contains(&Locale::En));
    assert!(all.contains(&Locale::ZhCn));
    assert!(all.contains(&Locale::Ja));
    assert!(all.contains(&Locale::Fr));
    assert!(all.contains(&Locale::De));
    assert!(all.contains(&Locale::Es));
}

#[test]
fn i18n_default_english() {
    let i18n = I18n::new();
    assert_eq!(i18n.current_locale, Locale::En);

    let strings = i18n.strings();
    assert_eq!(strings.animation.play, "Play");
    assert_eq!(strings.animation.pause, "Pause");
    assert_eq!(strings.geocoder.placeholder, "Enter an address or landmark...");
    assert_eq!(strings.info_box.title, "Entity Information");
}

#[test]
fn i18n_switch_locale() {
    let mut i18n = I18n::new();
    i18n.set_locale(Locale::ZhCn);
    assert_eq!(i18n.current_locale, Locale::ZhCn);

    let strings = i18n.strings();
    // Chinese strings should differ from English
    assert_ne!(strings.animation.play, "Play");
}

#[test]
fn widget_strings_english() {
    let ws = WidgetStrings::english();
    assert_eq!(ws.scene_mode_picker.scene_3d, "3D");
    assert_eq!(ws.scene_mode_picker.scene_2d, "2D");
    assert_eq!(ws.scene_mode_picker.columbus_view, "Columbus View");
    assert_eq!(ws.fullscreen.enter, "Full screen");
    assert_eq!(ws.fullscreen.exit, "Exit full screen");
    assert_eq!(ws.vr_button.enter, "Enter VR");
    assert_eq!(ws.navigation_help.rotate, "Left click + drag");
}

// ─── ProjectionPickerViewModel ─────────────────────────────────────────────────

#[test]
fn projection_picker_defaults() {
    let vm = ProjectionPickerViewModel::new();
    assert_eq!(vm.selected_projection, ProjectionType::Perspective);
    assert!(vm.show);
}

#[test]
fn projection_picker_switch() {
    let mut vm = ProjectionPickerViewModel::new();
    vm.select_orthographic();
    assert_eq!(vm.selected_projection, ProjectionType::Orthographic);

    vm.select_perspective();
    assert_eq!(vm.selected_projection, ProjectionType::Perspective);
}
