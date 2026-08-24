//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetStatistics.js`.
//!
//! Statistics for 3D Tiles tilesets.

use std::collections::HashMap;

/// The per-content counters consumed by
/// [`Cesium3DTilesetStatistics::increment_selection_counts`] and
/// [`Cesium3DTilesetStatistics::increment_load_counts`].
///
/// Rust analogue of the `Cesium3DTileContent` getters the statistics read
/// (`featuresLength`, `pointsLength`, `trianglesLength`,
/// `geometryByteLength`, `batchTableByteLength`, `texturesByteLength`).
#[derive(Debug, Clone, Default)]
pub struct TileContentCounts {
    /// Number of features in the content.
    pub features_length: i32,
    /// Number of points in the content.
    pub points_length: i32,
    /// Number of triangles in the content.
    pub triangles_length: i32,
    /// Size in bytes of the geometry buffers.
    pub geometry_byte_length: i64,
    /// Size in bytes of the batch table (and binary metadata).
    pub batch_table_byte_length: i64,
    /// Size in bytes of the textures (for non-model contents).
    pub textures_byte_length: i64,
    /// The contents nested inside this content (mirrors `innerContents`).
    pub inner_contents: Vec<TileContentCounts>,
}

/// Statistics for a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// Tracks runtime metrics for tile loading, rendering, and memory usage.
/// Mirrors CesiumJS `Cesium3DTilesetStatistics`.
#[derive(Debug, Clone, Default)]
pub struct Cesium3DTilesetStatistics {
    // Rendering statistics
    /// Number of tiles selected for rendering.
    pub selected: i32,
    /// Number of tiles visited.
    pub visited: i32,

    // Loading statistics
    /// Number of commands issued.
    pub number_of_commands: i32,
    /// Number of attempted tile requests.
    pub number_of_attempted_requests: i32,
    /// Number of pending tile requests.
    pub number_of_pending_requests: i32,
    /// Number of tiles currently processing.
    pub number_of_tiles_processing: i32,
    /// Number of tiles with content loaded (does not include empty tiles).
    pub number_of_tiles_with_content_ready: i32,
    /// Number of tiles in tileset JSON (and other tileset JSON files as
    /// they are loaded).
    pub number_of_tiles_total: i32,
    /// Running total of loaded tiles for the lifetime of the session.
    pub number_of_loaded_tiles_total: i32,

    // Features statistics
    /// Number of features rendered.
    pub number_of_features_selected: i32,
    /// Number of features in memory.
    pub number_of_features_loaded: i32,
    /// Number of points rendered.
    pub number_of_points_selected: i32,
    /// Number of points in memory.
    pub number_of_points_loaded: i32,
    /// Number of triangles rendered.
    pub number_of_triangles_selected: i32,

    // Styling statistics
    /// Number of tiles styled.
    pub number_of_tiles_styled: i32,
    /// Number of features styled.
    pub number_of_features_styled: i32,

    // Optimization statistics
    /// Number of tiles culled with the children union optimization.
    pub number_of_tiles_culled_with_children_union: i32,

    // Memory statistics
    /// Size in bytes of the geometry buffers in memory.
    pub geometry_byte_length: i64,
    /// Size in bytes of the textures in memory.
    pub textures_byte_length: i64,
    /// Reference counters of model textures by texture id.
    pub textures_reference_counter_by_id: HashMap<String, i32>,
    /// Batch textures and any binary metadata properties not otherwise
    /// accounted for.
    pub batch_table_byte_length: i64,
}

impl Cesium3DTilesetStatistics {
    /// Creates a new Cesium3DTilesetStatistics with zero values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the per-frame counters.
    ///
    /// Mirrors `Cesium3DTilesetStatistics.prototype.clear`.
    pub fn clear(&mut self) {
        self.selected = 0;
        self.visited = 0;
        self.number_of_commands = 0;
        self.number_of_attempted_requests = 0;
        self.number_of_features_selected = 0;
        self.number_of_points_selected = 0;
        self.number_of_triangles_selected = 0;
        self.number_of_tiles_styled = 0;
        self.number_of_features_styled = 0;
        self.number_of_tiles_culled_with_children_union = 0;
    }

    /// Increments the counters for the points, triangles, and features
    /// that are currently selected for rendering.
    ///
    /// Mirrors `incrementSelectionCounts(content)`; called recursively for
    /// the given content and all its inner contents.
    pub fn increment_selection_counts(&mut self, content: &TileContentCounts) {
        self.number_of_features_selected += content.features_length;
        self.number_of_points_selected += content.points_length;
        self.number_of_triangles_selected += content.triangles_length;

        // Recursive calls on all inner contents
        for inner in &content.inner_contents {
            self.increment_selection_counts(inner);
        }
    }

    /// Increments the counters for the number of features and points that
    /// are currently loaded, and the lengths (size in bytes) of the
    /// occupied memory.
    ///
    /// Mirrors `incrementLoadCounts(content)`; called recursively for the
    /// given content and all its inner contents.
    ///
    /// DEVIATION: CesiumJS special-cases `Model3DTileContent` by
    /// reference-counting shared textures by texture id; the Rust port
    /// adds `textures_byte_length` directly, which matches the behaviour
    /// for every non-model content.
    pub fn increment_load_counts(&mut self, content: &TileContentCounts) {
        self.number_of_features_loaded += content.features_length;
        self.number_of_points_loaded += content.points_length;
        self.geometry_byte_length += content.geometry_byte_length;
        self.batch_table_byte_length += content.batch_table_byte_length;
        self.textures_byte_length += content.textures_byte_length;

        // Recursive calls on all inner contents
        for inner in &content.inner_contents {
            self.increment_load_counts(inner);
        }
    }

    /// Decrements the counters for the number of features and points that
    /// are currently loaded, and the lengths (size in bytes) of the
    /// occupied memory.
    ///
    /// Mirrors `decrementLoadCounts(content)`; called recursively for the
    /// given content and all its inner contents.
    ///
    /// DEVIATION: see [`Self::increment_load_counts`] for the
    /// model-content texture reference counting deviation.
    pub fn decrement_load_counts(&mut self, content: &TileContentCounts) {
        self.number_of_features_loaded -= content.features_length;
        self.number_of_points_loaded -= content.points_length;
        self.geometry_byte_length -= content.geometry_byte_length;
        self.batch_table_byte_length -= content.batch_table_byte_length;
        self.textures_byte_length -= content.textures_byte_length;

        // Recursive calls on all inner contents
        for inner in &content.inner_contents {
            self.decrement_load_counts(inner);
        }
    }

    /// Copies every field from `statistics` into `result`.
    ///
    /// Mirrors `Cesium3DTilesetStatistics.clone(statistics, result)`.
    pub fn clone_into(statistics: &Self, result: &mut Self) {
        result.selected = statistics.selected;
        result.visited = statistics.visited;
        result.number_of_commands = statistics.number_of_commands;
        result.number_of_attempted_requests = statistics.number_of_attempted_requests;
        result.number_of_pending_requests = statistics.number_of_pending_requests;
        result.number_of_tiles_processing = statistics.number_of_tiles_processing;
        result.number_of_tiles_with_content_ready =
            statistics.number_of_tiles_with_content_ready;
        result.number_of_tiles_total = statistics.number_of_tiles_total;
        result.number_of_features_selected = statistics.number_of_features_selected;
        result.number_of_features_loaded = statistics.number_of_features_loaded;
        result.number_of_points_selected = statistics.number_of_points_selected;
        result.number_of_points_loaded = statistics.number_of_points_loaded;
        result.number_of_triangles_selected = statistics.number_of_triangles_selected;
        result.number_of_tiles_styled = statistics.number_of_tiles_styled;
        result.number_of_features_styled = statistics.number_of_features_styled;
        result.number_of_tiles_culled_with_children_union =
            statistics.number_of_tiles_culled_with_children_union;
        result.geometry_byte_length = statistics.geometry_byte_length;
        result.textures_byte_length = statistics.textures_byte_length;
        result.textures_reference_counter_by_id =
            statistics.textures_reference_counter_by_id.clone();
        result.batch_table_byte_length = statistics.batch_table_byte_length;
    }
}
