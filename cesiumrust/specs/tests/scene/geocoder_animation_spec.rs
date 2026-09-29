//! Geocoder + Animation 控件视图模型规范
//! 移植自 CesiumJS widgets/Source/Geocoder + Animation
//!
//! A 类测试：GeocoderViewModel 搜索/结果/导航、
//! ShuttleRing 角度↔倍率转换、AnimationViewModel

use cesium_widgets::{AnimationViewModel, GeocoderViewModel, ShuttleRing};

// ─── GeocoderViewModel ─────────────────────────────────────────────────────────

#[test]
fn geocoder_defaults() {
    let vm = GeocoderViewModel::new();
    assert!(vm.search_text.is_empty());
    assert!(!vm.is_searching);
    assert!(vm.results.is_empty());
    assert!(!vm.show_results);
    assert!(vm.selected_index.is_none());
    assert!(vm.show);
    assert!(vm.auto_complete);
    assert_eq!(vm.min_chars, 3);
    assert!((vm.flight_duration - 1.5).abs() < 1e-10);
}

#[test]
fn geocoder_set_search_text_clears_when_short() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("ab"); // < min_chars
    assert_eq!(vm.search_text, "ab");
    assert!(vm.results.is_empty());
    assert!(!vm.show_results);
}

#[test]
fn geocoder_should_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("ab");
    assert!(!vm.should_search()); // 太短

    vm.set_search_text("abc");
    assert!(vm.should_search()); // 恰好 min_chars
}

#[test]
fn geocoder_begin_and_complete_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("New York");
    vm.begin_search();
    assert!(vm.is_searching);

    // 带结果完成
    let results = vec![cesium_widgets::geocoder::GeocoderSearchResult {
        display_name: "New York, NY".to_string(),
        destination: cesium_widgets::geocoder::GeocoderSearchDestination::Point {
            longitude: -1.2921,
            latitude: 0.7106,
            height: None,
        },
    }];
    vm.complete_search(results);

    assert!(!vm.is_searching);
    assert_eq!(vm.results.len(), 1);
    assert!(vm.show_results);
    assert_eq!(vm.selected_index, Some(0));
}

#[test]
fn geocoder_complete_search_empty() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("xyz123");
    vm.begin_search();
    vm.complete_search(vec![]);

    assert!(!vm.is_searching);
    assert!(vm.results.is_empty());
    assert!(!vm.show_results);
    assert!(vm.selected_index.is_none());
}

#[test]
fn geocoder_clear_search() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.clear_search();

    assert!(vm.search_text.is_empty());
    assert!(vm.results.is_empty());
    assert!(!vm.show_results);
    assert!(!vm.is_searching);
}

#[test]
fn geocoder_navigation() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("New");
    vm.begin_search();

    let results = vec![
        cesium_widgets::geocoder::GeocoderSearchResult {
            display_name: "Result 1".to_string(),
            destination: cesium_widgets::geocoder::GeocoderSearchDestination::Point {
                longitude: 0.0,
                latitude: 0.0,
                height: None,
            },
        },
        cesium_widgets::geocoder::GeocoderSearchResult {
            display_name: "Result 2".to_string(),
            destination: cesium_widgets::geocoder::GeocoderSearchDestination::Point {
                longitude: 1.0,
                latitude: 1.0,
                height: None,
            },
        },
    ];
    vm.complete_search(results);
    assert_eq!(vm.selected_index, Some(0));

    vm.select_next();
    assert_eq!(vm.selected_index, Some(1));

    vm.select_next(); // 环绕
    assert_eq!(vm.selected_index, Some(0));

    vm.select_previous(); // 回绕
    assert_eq!(vm.selected_index, Some(1));
}

#[test]
fn geocoder_activate_selected() {
    let mut vm = GeocoderViewModel::new();
    vm.set_search_text("test");
    vm.begin_search();
    vm.complete_search(vec![cesium_widgets::geocoder::GeocoderSearchResult {
        display_name: "Paris, France".to_string(),
        destination: cesium_widgets::geocoder::GeocoderSearchDestination::Point {
            longitude: 0.0407,
            latitude: 0.8517,
            height: None,
        },
    }]);

    let result = vm.activate_selected().unwrap();
    assert_eq!(result.display_name, "Paris, France");
    assert_eq!(vm.search_text, "Paris, France");
    assert!(!vm.show_results);
}

// ─── ShuttleRing ───────────────────────────────────────────────────────────────

#[test]
fn shuttle_ring_default_ticks() {
    let sr = ShuttleRing::default();
    assert_eq!(sr.ticks.len(), 16);
    assert_eq!(sr.ticks[0], -1000.0);
    assert_eq!(sr.ticks[15], 1000.0);
}

#[test]
fn shuttle_ring_angle_to_multiplier_linear() {
    let sr = ShuttleRing::default();
    // 在 ±15° 内 → 线性 [-1, 1]
    assert!((sr.angle_to_multiplier(0.0) - 0.0).abs() < 1e-10);
    assert!((sr.angle_to_multiplier(15.0) - 1.0).abs() < 1e-10);
    assert!((sr.angle_to_multiplier(-15.0) - (-1.0)).abs() < 1e-10);
    assert!((sr.angle_to_multiplier(7.5) - 0.5).abs() < 1e-10);
}

#[test]
fn shuttle_ring_angle_to_multiplier_log() {
    let sr = ShuttleRing::default();
    // 超出 ±15° → 对数
    let m = sr.angle_to_multiplier(105.0); // 最大角度
    assert!(m > 100.0); // 应接近 1000

    let m_neg = sr.angle_to_multiplier(-105.0);
    assert!(m_neg < -100.0);
}

#[test]
fn shuttle_ring_multiplier_to_angle() {
    let sr = ShuttleRing::default();
    // 倍率 0 → 角度 0
    assert!((sr.multiplier_to_angle(0.0, false) - 0.0).abs() < 1e-10);
    // 倍率 1 → 角度 15
    assert!((sr.multiplier_to_angle(1.0, false) - 15.0).abs() < 1e-10);
    // 倍率 -1 → 角度 -15
    assert!((sr.multiplier_to_angle(-1.0, false) - (-15.0)).abs() < 1e-10);
    // system clock → 总是 15
    assert!((sr.multiplier_to_angle(5.0, true) - 15.0).abs() < 1e-10);
}

#[test]
fn shuttle_ring_roundtrip() {
    let sr = ShuttleRing::default();
    // 角度 → 倍率 → 角度应往返一致
    let angle = 60.0;
    let mult = sr.angle_to_multiplier(angle);
    let angle_back = sr.multiplier_to_angle(mult, false);
    assert!((angle - angle_back).abs() < 0.01);
}

// ─── AnimationViewModel ────────────────────────────────────────────────────────

#[test]
fn animation_view_model_defaults() {
    let vm = AnimationViewModel::new();
    assert!(!vm.is_playing);
    assert!((vm.multiplier - 1.0).abs() < 1e-10);
    assert!(!vm.is_system_clock);
}

#[test]
fn animation_view_model_play_pause() {
    let mut vm = AnimationViewModel::new();
    vm.play();
    assert!(vm.is_playing);
    assert!(vm.multiplier > 0.0);

    vm.pause();
    assert!(!vm.is_playing);
}

#[test]
fn animation_view_model_reverse() {
    let mut vm = AnimationViewModel::new();
    vm.play_reverse();
    assert!(vm.is_playing);
    assert!(vm.multiplier < 0.0);
}
