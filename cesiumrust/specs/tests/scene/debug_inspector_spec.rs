//! DebugInspector / PerformanceOverlay / TilesetInspector specs
//! Ported from CesiumJS Scene/DebugInspector.js
//!
//! A-class tests: enable/disable, tile recording, frame stats, summary,
//! performance overlay history/fps/average, tileset inspector select/deselect

use cesium_scene::{
    DebugInspector, FrameDebugStats, HighlightMode, PerformanceOverlay, TileDebugInfo,
    TilesetInspector,
};

// ─── DebugInspector ────────────────────────────────────────────────────────────

#[test]
fn debug_inspector_defaults() {
    let inspector = DebugInspector::new();
    assert!(!inspector.enabled);
    assert!(!inspector.wireframe);
    assert!(!inspector.show_bounding_volumes);
    assert!(!inspector.show_tile_coordinates);
    assert!(!inspector.show_statistics);
    assert!(!inspector.show_frustums);
    assert!(!inspector.show_depth);
    assert!(!inspector.show_normals);
    assert!(!inspector.show_pick_debug);
    assert_eq!(inspector.highlight_mode, HighlightMode::None);
    assert!(inspector.tile_debug_info.is_empty());
}

#[test]
fn debug_inspector_enable_all() {
    let mut inspector = DebugInspector::new();
    inspector.enable_all();

    assert!(inspector.enabled);
    assert!(inspector.wireframe);
    assert!(inspector.show_bounding_volumes);
    assert!(inspector.show_tile_coordinates);
    assert!(inspector.show_statistics);
    assert!(inspector.show_frustums);
}

#[test]
fn debug_inspector_disable_all() {
    let mut inspector = DebugInspector::new();
    inspector.enable_all();
    inspector.show_depth = true;
    inspector.show_normals = true;
    inspector.show_pick_debug = true;
    inspector.highlight_mode = HighlightMode::Depth;

    inspector.disable_all();

    assert!(!inspector.enabled);
    assert!(!inspector.wireframe);
    assert!(!inspector.show_bounding_volumes);
    assert!(!inspector.show_statistics);
    assert!(!inspector.show_depth);
    assert!(!inspector.show_normals);
    assert!(!inspector.show_pick_debug);
    assert_eq!(inspector.highlight_mode, HighlightMode::None);
}

#[test]
fn debug_inspector_record_and_get_tile() {
    let mut inspector = DebugInspector::new();
    inspector.record_tile(TileDebugInfo {
        tile_id: 42,
        depth: 3,
        geometric_error: 100.0,
        distance_to_camera: 500.0,
        screen_space_error: 8.5,
        is_rendered: true,
        is_visited: true,
        is_selected: false,
        content_type: "b3dm".to_string(),
        triangles_count: 15000,
        vertices_count: 8000,
        load_time_ms: 12.5,
    });

    let info = inspector.get_tile_info(42).unwrap();
    assert_eq!(info.depth, 3);
    assert!((info.geometric_error - 100.0).abs() < 1e-10);
    assert!((info.distance_to_camera - 500.0).abs() < 1e-10);
    assert!((info.screen_space_error - 8.5).abs() < 1e-10);
    assert!(info.is_rendered);
    assert!(info.is_visited);
    assert!(!info.is_selected);
    assert_eq!(info.content_type, "b3dm");
    assert_eq!(info.triangles_count, 15000);
    assert!((info.load_time_ms - 12.5).abs() < 1e-10);

    // Non-existent tile
    assert!(inspector.get_tile_info(999).is_none());
}

#[test]
fn debug_inspector_clear_tile_info() {
    let mut inspector = DebugInspector::new();
    inspector.record_tile(TileDebugInfo {
        tile_id: 1,
        ..Default::default()
    });
    inspector.record_tile(TileDebugInfo {
        tile_id: 2,
        ..Default::default()
    });
    assert_eq!(inspector.tile_debug_info.len(), 2);

    inspector.clear_tile_info();
    assert!(inspector.tile_debug_info.is_empty());
    assert!(inspector.get_tile_info(1).is_none());
}

#[test]
fn debug_inspector_frame_stats_and_summary() {
    let mut inspector = DebugInspector::new();
    inspector.update_frame_stats(FrameDebugStats {
        frame_number: 100,
        draw_calls: 256,
        triangles_rendered: 1_500_000,
        tiles_rendered: 128,
        tiles_visited: 200,
        tiles_culled: 72,
        frame_time_ms: 16.67,
        ..Default::default()
    });

    let summary = inspector.summary();
    assert!(summary.contains("Frame 100"));
    assert!(summary.contains("Draw calls: 256"));
    assert!(summary.contains("128 rendered"));
    assert!(summary.contains("200 visited"));
    assert!(summary.contains("72 culled"));
}

#[test]
fn debug_inspector_highlight_modes() {
    let modes = [
        HighlightMode::None,
        HighlightMode::Depth,
        HighlightMode::GeometricError,
        HighlightMode::Distance,
        HighlightMode::RenderState,
        HighlightMode::RandomColor,
    ];

    let mut inspector = DebugInspector::new();
    for mode in &modes {
        inspector.highlight_mode = *mode;
        assert_eq!(inspector.highlight_mode, *mode);
    }
}

// ─── PerformanceOverlay ────────────────────────────────────────────────────────

#[test]
fn performance_overlay_record_and_fps() {
    let mut overlay = PerformanceOverlay::new();
    overlay.record_frame(16.67); // ~60 fps

    assert!((overlay.fps - 1000.0 / 16.67).abs() < 0.1);
    assert_eq!(overlay.history.len(), 1);
    assert!((overlay.frame_time_ms - 16.67).abs() < 1e-10);
}

#[test]
fn performance_overlay_average_and_min_fps() {
    let mut overlay = PerformanceOverlay::new();
    overlay.record_frame(10.0); // 100 fps
    overlay.record_frame(20.0); // 50 fps
    overlay.record_frame(15.0); // ~66.7 fps

    let avg = overlay.average_frame_time();
    assert!((avg - 15.0).abs() < 1e-10); // (10+20+15)/3

    // min_fps = 1000 / max_frame_time = 1000/20 = 50
    assert!((overlay.min_fps() - 50.0).abs() < 1e-10);
}

#[test]
fn performance_overlay_history_limit_120() {
    let mut overlay = PerformanceOverlay::new();
    for i in 0..150 {
        overlay.record_frame(16.0 + i as f64 * 0.01);
    }
    assert_eq!(overlay.history.len(), 120);
}

#[test]
fn performance_overlay_empty() {
    let overlay = PerformanceOverlay::new();
    assert_eq!(overlay.average_frame_time(), 0.0);
    assert_eq!(overlay.min_fps(), 0.0);
    assert_eq!(overlay.fps, 0.0);
}

// ─── TilesetInspector ──────────────────────────────────────────────────────────

#[test]
fn tileset_inspector_select_deselect() {
    let mut inspector = TilesetInspector::new();
    assert!(!inspector.active);
    assert!(inspector.selected_tile.is_none());
    assert!(!inspector.show_content_volume);
    assert!(!inspector.freeze_frame);
    assert!(inspector.max_sse_override.is_none());

    inspector.select_tile(99);
    assert_eq!(inspector.selected_tile, Some(99));

    inspector.deselect();
    assert!(inspector.selected_tile.is_none());
}
