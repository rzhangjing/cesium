//! I18n (internationalization) spec tests.
//!
//! Maps to CesiumJS:
//! - Widgets/I18nSpec.js (locale management, string resources, fallback)
//!
//! A-class tests: locale codes, string resources, I18n manager, key-path lookup.

use cesium_widgets::i18n::{I18n, Locale, WidgetStrings};

// === Locale ===

#[test]
fn locale_code_values() {
    assert_eq!(Locale::En.code(), "en");
    assert_eq!(Locale::ZhCn.code(), "zh-CN");
    assert_eq!(Locale::Ja.code(), "ja");
    assert_eq!(Locale::Fr.code(), "fr");
    assert_eq!(Locale::De.code(), "de");
    assert_eq!(Locale::Es.code(), "es");
}

#[test]
fn locale_from_code_valid() {
    assert_eq!(Locale::from_code("en"), Some(Locale::En));
    assert_eq!(Locale::from_code("zh-CN"), Some(Locale::ZhCn));
    assert_eq!(Locale::from_code("zh"), Some(Locale::ZhCn));
    assert_eq!(Locale::from_code("ja"), Some(Locale::Ja));
    assert_eq!(Locale::from_code("fr"), Some(Locale::Fr));
    assert_eq!(Locale::from_code("de"), Some(Locale::De));
    assert_eq!(Locale::from_code("es"), Some(Locale::Es));
}

#[test]
fn locale_from_code_invalid() {
    assert_eq!(Locale::from_code("xx"), None);
    assert_eq!(Locale::from_code(""), None);
    assert_eq!(Locale::from_code("EN"), None);
    assert_eq!(Locale::from_code("zh-TW"), None);
}

#[test]
fn locale_all_returns_six() {
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
fn locale_default_is_en() {
    assert_eq!(Locale::default(), Locale::En);
}

// === WidgetStrings ===

#[test]
fn widget_strings_english() {
    let s = WidgetStrings::english();
    assert_eq!(s.animation.play, "Play");
    assert_eq!(s.animation.pause, "Pause");
    assert_eq!(s.animation.play_reverse, "Play Reverse");
    assert_eq!(s.animation.play_forward, "Play Forward");
    assert_eq!(s.animation.realtime, "Today (real-time)");
    assert_eq!(s.animation.multiplier_label, "Speed");
    assert_eq!(s.timeline.tooltip, "Timeline");
    assert_eq!(s.scene_mode_picker.scene_3d, "3D");
    assert_eq!(s.scene_mode_picker.scene_2d, "2D");
    assert_eq!(s.scene_mode_picker.columbus_view, "Columbus View");
    assert_eq!(s.geocoder.placeholder, "Enter an address or landmark...");
    assert_eq!(s.geocoder.search, "Search");
    assert_eq!(s.geocoder.no_results, "No results found");
    assert_eq!(s.fullscreen.enter, "Full screen");
    assert_eq!(s.fullscreen.exit, "Exit full screen");
    assert_eq!(s.info_box.title, "Entity Information");
    assert_eq!(s.info_box.close, "Close");
    assert_eq!(s.info_box.no_selection, "No entity selected");
    assert_eq!(s.vr_button.enter, "Enter VR");
    assert_eq!(s.vr_button.exit, "Exit VR");
}

#[test]
fn widget_strings_chinese() {
    let s = WidgetStrings::chinese();
    assert_eq!(s.animation.play, "播放");
    assert_eq!(s.animation.pause, "暂停");
    assert_eq!(s.animation.play_reverse, "反向播放");
    assert_eq!(s.animation.play_forward, "正向播放");
    assert_eq!(s.animation.realtime, "今天（实时）");
    assert_eq!(s.animation.multiplier_label, "速度");
    assert_eq!(s.timeline.tooltip, "时间线");
    assert_eq!(s.scene_mode_picker.scene_3d, "三维");
    assert_eq!(s.scene_mode_picker.scene_2d, "二维");
    assert_eq!(s.scene_mode_picker.columbus_view, "哥伦布视图");
    assert_eq!(s.geocoder.placeholder, "输入地址或地标...");
    assert_eq!(s.geocoder.search, "搜索");
    assert_eq!(s.fullscreen.enter, "全屏");
    assert_eq!(s.fullscreen.exit, "退出全屏");
    assert_eq!(s.info_box.title, "实体信息");
    assert_eq!(s.info_box.close, "关闭");
    assert_eq!(s.vr_button.enter, "进入VR");
    assert_eq!(s.vr_button.exit, "退出VR");
}

#[test]
fn widget_strings_japanese() {
    let s = WidgetStrings::japanese();
    assert_eq!(s.animation.play, "再生");
    assert_eq!(s.animation.pause, "一時停止");
    assert_eq!(s.animation.play_reverse, "逆再生");
    assert_eq!(s.timeline.tooltip, "タイムライン");
    assert_eq!(s.scene_mode_picker.columbus_view, "コロンバスビュー");
    assert_eq!(s.geocoder.search, "検索");
    assert_eq!(s.fullscreen.enter, "フルスクリーン");
    assert_eq!(s.info_box.title, "エンティティ情報");
    assert_eq!(s.vr_button.enter, "VR開始");
}

#[test]
fn widget_strings_default_is_english() {
    let s = WidgetStrings::default();
    assert_eq!(s.animation.play, "Play");
}

#[test]
fn widget_strings_navigation_help() {
    let s = WidgetStrings::english();
    assert_eq!(s.navigation_help.tooltip, "Navigation Instructions");
    assert_eq!(s.navigation_help.mouse_title, "Mouse Navigation");
    assert_eq!(s.navigation_help.touch_title, "Touch Navigation");
    assert_eq!(s.navigation_help.rotate, "Left click + drag");
    assert_eq!(s.navigation_help.zoom, "Right click + drag, or scroll");
    assert_eq!(s.navigation_help.pan, "Ctrl + left click + drag");
    assert_eq!(s.navigation_help.tilt, "Ctrl + middle click + drag");
}

#[test]
fn widget_strings_base_layer_picker() {
    let s = WidgetStrings::english();
    assert_eq!(s.base_layer_picker.tooltip, "Imagery / Terrain");
    assert_eq!(s.base_layer_picker.imagery, "Imagery");
    assert_eq!(s.base_layer_picker.terrain, "Terrain");

    let zh = WidgetStrings::chinese();
    assert_eq!(zh.base_layer_picker.tooltip, "影像 / 地形");
    assert_eq!(zh.base_layer_picker.imagery, "影像");
    assert_eq!(zh.base_layer_picker.terrain, "地形");
}

// === I18n Manager ===

#[test]
fn i18n_default_locale_en() {
    let i18n = I18n::new();
    assert_eq!(i18n.current_locale, Locale::En);
    assert_eq!(i18n.strings().animation.play, "Play");
}

#[test]
fn i18n_set_locale_changes_strings() {
    let mut i18n = I18n::new();
    i18n.set_locale(Locale::ZhCn);
    assert_eq!(i18n.current_locale, Locale::ZhCn);
    assert_eq!(i18n.strings().animation.play, "播放");

    i18n.set_locale(Locale::Ja);
    assert_eq!(i18n.strings().animation.play, "再生");
}

#[test]
fn i18n_fallback_to_english() {
    let mut i18n = I18n::new();
    // French is not registered by default
    i18n.set_locale(Locale::Fr);
    assert_eq!(i18n.strings().animation.play, "Play");
}

#[test]
fn i18n_strings_for_specific_locale() {
    let i18n = I18n::new();
    // Current locale is En, but get strings for ZhCn
    let zh = i18n.strings_for(Locale::ZhCn);
    assert_eq!(zh.animation.play, "播放");

    // Unregistered locale falls back to English
    let fr = i18n.strings_for(Locale::Fr);
    assert_eq!(fr.animation.play, "Play");
}

#[test]
fn i18n_register_custom_locale() {
    let mut i18n = I18n::new();
    let german = WidgetStrings {
        animation: cesium_widgets::i18n::AnimationStrings {
            play: "Abspielen".to_string(),
            pause: "Pause".to_string(),
            play_reverse: "Rückwärts".to_string(),
            play_forward: "Vorwärts".to_string(),
            realtime: "Heute (Echtzeit)".to_string(),
            multiplier_label: "Geschwindigkeit".to_string(),
        },
        ..WidgetStrings::english()
    };
    i18n.register_locale(Locale::De, german);
    i18n.set_locale(Locale::De);
    assert_eq!(i18n.strings().animation.play, "Abspielen");
    assert_eq!(i18n.strings().animation.pause, "Pause");
}

#[test]
fn i18n_available_locales_default() {
    let i18n = I18n::new();
    let locales = i18n.available_locales();
    // Default registers: En, ZhCn, Ja
    assert_eq!(locales.len(), 3);
    assert!(locales.contains(&Locale::En));
    assert!(locales.contains(&Locale::ZhCn));
    assert!(locales.contains(&Locale::Ja));
}

#[test]
fn i18n_available_locales_after_register() {
    let mut i18n = I18n::new();
    i18n.register_locale(Locale::Fr, WidgetStrings::english());
    let locales = i18n.available_locales();
    assert_eq!(locales.len(), 4);
    assert!(locales.contains(&Locale::Fr));
}

#[test]
fn i18n_get_key_path() {
    let i18n = I18n::new();
    assert_eq!(i18n.get("animation.play"), Some("Play"));
    assert_eq!(i18n.get("animation.pause"), Some("Pause"));
    assert_eq!(i18n.get("animation.play_reverse"), Some("Play Reverse"));
    assert_eq!(i18n.get("animation.play_forward"), Some("Play Forward"));
    assert_eq!(i18n.get("animation.realtime"), Some("Today (real-time)"));
    assert_eq!(i18n.get("animation.multiplier_label"), Some("Speed"));
    assert_eq!(i18n.get("geocoder.placeholder"), Some("Enter an address or landmark..."));
    assert_eq!(i18n.get("geocoder.search"), Some("Search"));
    assert_eq!(i18n.get("geocoder.no_results"), Some("No results found"));
    assert_eq!(i18n.get("fullscreen.enter"), Some("Full screen"));
    assert_eq!(i18n.get("fullscreen.exit"), Some("Exit full screen"));
    assert_eq!(i18n.get("info_box.title"), Some("Entity Information"));
    assert_eq!(i18n.get("info_box.close"), Some("Close"));
    assert_eq!(i18n.get("info_box.no_selection"), Some("No entity selected"));
}

#[test]
fn i18n_get_invalid_keys() {
    let i18n = I18n::new();
    assert_eq!(i18n.get("invalid.key"), None);
    assert_eq!(i18n.get("animation.nonexistent"), None);
    assert_eq!(i18n.get("nodot"), None);
    assert_eq!(i18n.get(""), None);
}

#[test]
fn i18n_get_respects_current_locale() {
    let mut i18n = I18n::new();
    i18n.set_locale(Locale::ZhCn);
    assert_eq!(i18n.get("animation.play"), Some("播放"));
    assert_eq!(i18n.get("geocoder.search"), Some("搜索"));
    assert_eq!(i18n.get("fullscreen.enter"), Some("全屏"));
    assert_eq!(i18n.get("info_box.close"), Some("关闭"));
}
