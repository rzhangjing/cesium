//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetStatistics.js`.
//!
//! Statistics for 3D Tiles tilesets.

/// Statistics for a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// Tracks runtime metrics for tile loading, rendering, and memory usage.
/// Mirrors CesiumJS `Cesium3DTilesetStatistics` (192 lines).
pub struct Cesium3DTilesetStatistics {
    // ---- per-frame ----
    /// Number of tiles visited in the current frame.
    pub visited: i32,
    /// Number of tiles selected for rendering in the current frame.
    pub selected: i32,
    /// Number of tiles with features in the command list.
    pub number_of_commands: i32,
    /// Number of tiles loading.
    pub number_of_tiles_loading: i32,
    /// Number of tiles with content fetched this frame.
    pub number_of_tiles_with_content_ready: i32,

    // ---- total (accumulated) ----
    /// Total number of tiles in the tileset.
    pub number_of_tiles_total: i32,
    /// Total features across all loaded tiles.
    pub number_of_features_total: i32,
    /// Total bytes of loaded tile content.
    pub number_of_bytes_total: i64,

    // ---- peak ----
    /// Peak number of tiles loading simultaneously.
    pub number_of_tiles_loading_peak: i32,
    /// Peak number of features loaded.
    pub number_of_features_loaded_peak: i32,
    /// Peak bytes loaded.
    pub number_of_bytes_loaded_peak: i64,

    // ---- attempt counts ----
    /// Total number of tile content requests attempted.
    pub number_of_attempted_requests: i32,
    /// Number of requests that succeeded.
    pub number_of_successful_requests: i32,
    /// Number of requests that failed.
    pub number_of_failed_requests: i32,

    // ---- deferred ----
    /// Number of tiles deferred because they are not yet needed.
    pub number_of_tiles_with_deferred_callbacks: i32,

    // ---- timing ----
    /// Time spent loading tile content (ms).
    pub tile_load_time_total_ms: f64,
    /// Average tile load time (ms).
    pub tile_load_time_average_ms: f64,
}

impl Cesium3DTilesetStatistics {
    /// Creates a new Cesium3DTilesetStatistics with zero values.
    pub fn new() -> Self {
        Self {
            visited: 0,
            selected: 0,
            number_of_commands: 0,
            number_of_tiles_loading: 0,
            number_of_tiles_with_content_ready: 0,
            number_of_tiles_total: 0,
            number_of_features_total: 0,
            number_of_bytes_total: 0,
            number_of_tiles_loading_peak: 0,
            number_of_features_loaded_peak: 0,
            number_of_bytes_loaded_peak: 0,
            number_of_attempted_requests: 0,
            number_of_successful_requests: 0,
            number_of_failed_requests: 0,
            number_of_tiles_with_deferred_callbacks: 0,
            tile_load_time_total_ms: 0.0,
            tile_load_time_average_ms: 0.0,
        }
    }

    /// Resets per-frame statistics to zero.
    pub fn reset_per_frame(&mut self) {
        self.visited = 0;
        self.selected = 0;
        self.number_of_commands = 0;
        self.number_of_tiles_loading = 0;
        self.number_of_tiles_with_content_ready = 0;
    }
}

impl Default for Cesium3DTilesetStatistics {
    fn default() -> Self { Self::new() }
}
