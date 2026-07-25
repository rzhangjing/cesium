//! Debug inspector domain models.
//!
//! Maps to CesiumJS `Scene/DebugInspector.js` and related debug visualization
//! utilities. Provides runtime inspection of scene state, tilesets, and
//! rendering statistics.

use std::collections::HashMap;

/// Debug inspector for scene diagnostics.
///
/// Maps to CesiumJS `Scene/DebugInspector.js`
#[derive(Debug, Clone, Default)]
pub struct DebugInspector {
    /// Whether the inspector is enabled.
    pub enabled: bool,
    /// Show wireframe rendering.
    pub wireframe: bool,
    /// Show bounding volumes.
    pub show_bounding_volumes: bool,
    /// Show tile coordinates.
    pub show_tile_coordinates: bool,
    /// Show render statistics overlay.
    pub show_statistics: bool,
    /// Show frustum culling visualization.
    pub show_frustums: bool,
    /// Show depth buffer visualization.
    pub show_depth: bool,
    /// Show normals visualization.
    pub show_normals: bool,
    /// Show picking debug colors.
    pub show_pick_debug: bool,
    /// Highlight mode for tiles.
    pub highlight_mode: HighlightMode,
    /// Per-tile debug info.
    pub tile_debug_info: HashMap<u64, TileDebugInfo>,
    /// Frame statistics.
    pub frame_stats: FrameDebugStats,
}

/// Highlight mode for tile inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighlightMode {
    #[default]
    None,
    /// Highlight by depth in the tree.
    Depth,
    /// Highlight by geometric error.
    GeometricError,
    /// Highlight by distance from camera.
    Distance,
    /// Highlight by rendering state.
    RenderState,
    /// Random color per tile.
    RandomColor,
}

/// Per-tile debug information.
#[derive(Debug, Clone, Default)]
pub struct TileDebugInfo {
    pub tile_id: u64,
    pub depth: u32,
    pub geometric_error: f64,
    pub distance_to_camera: f64,
    pub screen_space_error: f64,
    pub is_rendered: bool,
    pub is_visited: bool,
    pub is_selected: bool,
    pub content_type: String,
    pub triangles_count: u64,
    pub vertices_count: u64,
    pub load_time_ms: f64,
}

/// Frame-level debug statistics.
#[derive(Debug, Clone, Default)]
pub struct FrameDebugStats {
    pub frame_number: u64,
    pub draw_calls: u32,
    pub triangles_rendered: u64,
    pub vertices_rendered: u64,
    pub tiles_rendered: u32,
    pub tiles_visited: u32,
    pub tiles_culled: u32,
    pub tiles_loading: u32,
    pub tiles_loaded: u32,
    pub frame_time_ms: f64,
    pub gpu_time_ms: f64,
    pub memory_used_bytes: u64,
    pub texture_count: u32,
    pub shader_count: u32,
    pub buffer_count: u32,
}

impl DebugInspector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all debug visualizations.
    pub fn enable_all(&mut self) {
        self.enabled = true;
        self.wireframe = true;
        self.show_bounding_volumes = true;
        self.show_tile_coordinates = true;
        self.show_statistics = true;
        self.show_frustums = true;
    }

    /// Disable all debug visualizations.
    pub fn disable_all(&mut self) {
        self.enabled = false;
        self.wireframe = false;
        self.show_bounding_volumes = false;
        self.show_tile_coordinates = false;
        self.show_statistics = false;
        self.show_frustums = false;
        self.show_depth = false;
        self.show_normals = false;
        self.show_pick_debug = false;
        self.highlight_mode = HighlightMode::None;
    }

    /// Record tile debug info.
    pub fn record_tile(&mut self, info: TileDebugInfo) {
        self.tile_debug_info.insert(info.tile_id, info);
    }

    /// Get debug info for a specific tile.
    pub fn get_tile_info(&self, tile_id: u64) -> Option<&TileDebugInfo> {
        self.tile_debug_info.get(&tile_id)
    }

    /// Clear per-tile debug info (call at start of frame).
    pub fn clear_tile_info(&mut self) {
        self.tile_debug_info.clear();
    }

    /// Update frame statistics.
    pub fn update_frame_stats(&mut self, stats: FrameDebugStats) {
        self.frame_stats = stats;
    }

    /// Get a summary string for display.
    pub fn summary(&self) -> String {
        let s = &self.frame_stats;
        format!(
            "Frame {} | Draw calls: {} | Tris: {} | Tiles: {} rendered / {} visited / {} culled | {:.2} ms",
            s.frame_number,
            s.draw_calls,
            s.triangles_rendered,
            s.tiles_rendered,
            s.tiles_visited,
            s.tiles_culled,
            s.frame_time_ms,
        )
    }
}

/// Performance overlay data for HUD display.
#[derive(Debug, Clone, Default)]
pub struct PerformanceOverlay {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub gpu_frame_time_ms: f64,
    pub draw_calls: u32,
    pub triangles: u64,
    pub texture_memory_mb: f64,
    pub buffer_memory_mb: f64,
    pub tile_memory_mb: f64,
    pub history: Vec<f64>,
}

impl PerformanceOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a frame time sample.
    pub fn record_frame(&mut self, frame_time_ms: f64) {
        self.frame_time_ms = frame_time_ms;
        self.fps = if frame_time_ms > 0.0 { 1000.0 / frame_time_ms } else { 0.0 };
        self.history.push(frame_time_ms);
        if self.history.len() > 120 {
            self.history.remove(0);
        }
    }

    /// Average frame time over history.
    pub fn average_frame_time(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }

    /// Minimum FPS over history.
    pub fn min_fps(&self) -> f64 {
        let max_time = self.history.iter().cloned().fold(0.0f64, f64::max);
        if max_time > 0.0 { 1000.0 / max_time } else { 0.0 }
    }
}

/// Tileset inspector for examining 3D Tiles content.
#[derive(Debug, Clone, Default)]
pub struct TilesetInspector {
    /// Whether the inspector is active.
    pub active: bool,
    /// Currently selected tile ID.
    pub selected_tile: Option<u64>,
    /// Show content bounding volume.
    pub show_content_volume: bool,
    /// Show viewer request volume.
    pub show_viewer_volume: bool,
    /// Colorize by tileset.
    pub colorize_tileset: bool,
    /// Freeze frame (stop updating).
    pub freeze_frame: bool,
    /// Maximum screen space error override.
    pub max_sse_override: Option<f64>,
}

impl TilesetInspector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a tile for inspection.
    pub fn select_tile(&mut self, tile_id: u64) {
        self.selected_tile = Some(tile_id);
    }

    /// Deselect the current tile.
    pub fn deselect(&mut self) {
        self.selected_tile = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_inspector_toggle() {
        let mut inspector = DebugInspector::new();
        assert!(!inspector.enabled);

        inspector.enable_all();
        assert!(inspector.enabled);
        assert!(inspector.wireframe);
        assert!(inspector.show_bounding_volumes);
        assert!(inspector.show_statistics);

        inspector.disable_all();
        assert!(!inspector.enabled);
        assert!(!inspector.wireframe);
    }

    #[test]
    fn test_tile_debug_info() {
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
        assert_eq!(info.triangles_count, 15000);
        assert!(info.is_rendered);

        inspector.clear_tile_info();
        assert!(inspector.get_tile_info(42).is_none());
    }

    #[test]
    fn test_frame_stats_summary() {
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
    }

    #[test]
    fn test_performance_overlay() {
        let mut overlay = PerformanceOverlay::new();
        overlay.record_frame(16.67);
        overlay.record_frame(14.0);
        overlay.record_frame(20.0);

        assert!(overlay.fps > 0.0);
        assert_eq!(overlay.history.len(), 3);
        assert!(overlay.average_frame_time() > 15.0);
        assert!(overlay.min_fps() < 60.0);
    }

    #[test]
    fn test_performance_overlay_history_limit() {
        let mut overlay = PerformanceOverlay::new();
        for _ in 0..150 {
            overlay.record_frame(16.0);
        }
        assert_eq!(overlay.history.len(), 120);
    }

    #[test]
    fn test_tileset_inspector() {
        let mut inspector = TilesetInspector::new();
        assert!(!inspector.active);
        assert!(inspector.selected_tile.is_none());

        inspector.select_tile(99);
        assert_eq!(inspector.selected_tile, Some(99));

        inspector.deselect();
        assert!(inspector.selected_tile.is_none());
    }

    #[test]
    fn test_highlight_modes() {
        let mut inspector = DebugInspector::new();
        assert_eq!(inspector.highlight_mode, HighlightMode::None);

        inspector.highlight_mode = HighlightMode::Depth;
        assert_eq!(inspector.highlight_mode, HighlightMode::Depth);

        inspector.highlight_mode = HighlightMode::RandomColor;
        assert_eq!(inspector.highlight_mode, HighlightMode::RandomColor);
    }
}
