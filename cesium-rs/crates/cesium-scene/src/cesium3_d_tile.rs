//! Ported from `packages/engine/Source/Scene/Cesium3DTile.js`.
//!
//! A single tile in a 3D Tiles tileset.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::julian_date::JulianDate;
use cesium_core::matrix4::Matrix4;

use crate::cesium3_d_tile_content_state::Cesium3DTileContentState;
use crate::cesium3_d_tile_refine::Cesium3DTileRefine;

/// A single tile in a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// When a tile is first created, its content is not loaded; the content is loaded
/// on-demand when needed based on the view.
///
/// Mirrors CesiumJS `Cesium3DTile` (2529 lines).
pub struct Cesium3DTile {
    // ---- transforms ----
    /// The local transform of this tile.
    pub transform: Matrix4,
    /// The final computed transform (parent * local).
    pub computed_transform: Matrix4,
    /// The initial transform (without vertical exaggeration).
    initial_transform: Matrix4,

    // ---- hierarchy ----
    /// This tile's parent, or `None` if root.
    pub parent: Option<usize>,
    /// This tile's children.
    pub children: Vec<usize>,

    // ---- bounding volumes ----
    /// The bounding volume for this tile.
    pub bounding_volume: Option<BoundingSphere>,
    /// The content bounding volume (tight-fit around features).
    pub content_bounding_volume: Option<BoundingSphere>,

    // ---- geometric error ----
    /// The error, in meters, introduced if this tile is rendered and its children are not.
    pub geometric_error: f64,
    /// Scaled geometric error (accounts for geometricErrorScale).
    geometric_error_scale: f64,

    // ---- refinement ----
    /// The refinement type (ADD or REPLACE).
    pub refine: Cesium3DTileRefine,

    // ---- content ----
    /// The content state.
    pub content_state: Cesium3DTileContentState,
    /// Whether the tile has no content.
    pub has_empty_content: bool,
    /// Whether the tile's content points to an external tileset.
    pub has_tileset_content: bool,
    /// Whether the tile has renderable content.
    pub has_renderable_content: bool,
    /// Whether the tile has multiple contents.
    pub has_multiple_contents: bool,
    /// Number of features in the content.
    pub features_length: i32,

    // ---- expiration ----
    /// Time in seconds after content is ready when content expires.
    pub expire_duration: f64,
    /// The date when content expires.
    pub expire_date: Option<JulianDate>,

    // ---- traversal state ----
    /// Whether this tile was selected last frame.
    pub was_selected_last_frame: bool,
    /// Whether this tile is visible in the current frame.
    pub is_visible: bool,
    /// The screen space error for this tile in pixels.
    pub screen_space_error: f64,
    /// The depth of this tile in the tileset tree.
    pub depth: i32,

    // ---- caching ----
    /// The time when this tile was last selected for rendering.
    pub last_selected_time: f64,
    /// The number of frames this tile has been loading.
    pub loading_frames_count: i32,

    // ---- vertical exaggeration ----
    vertical_exaggeration: f64,
    vertical_exaggeration_relative_height: f64,

    /// The tile's local center forRTC.
    pub center: Cartesian3,
}

impl Cesium3DTile {
    /// Creates a new Cesium3DTile with default values.
    pub fn new() -> Self {
        Self {
            transform: Matrix4::IDENTITY,
            computed_transform: Matrix4::IDENTITY,
            initial_transform: Matrix4::IDENTITY,
            parent: None,
            children: Vec::new(),
            bounding_volume: None,
            content_bounding_volume: None,
            geometric_error: 0.0,
            geometric_error_scale: 1.0,
            refine: Cesium3DTileRefine::Replace,
            content_state: Cesium3DTileContentState::Unloaded,
            has_empty_content: false,
            has_tileset_content: false,
            has_renderable_content: true,
            has_multiple_contents: false,
            features_length: 0,
            expire_duration: 0.0,
            expire_date: None,
            was_selected_last_frame: false,
            is_visible: false,
            screen_space_error: 0.0,
            depth: 0,
            last_selected_time: 0.0,
            loading_frames_count: 0,
            vertical_exaggeration: 1.0,
            vertical_exaggeration_relative_height: 0.0,
            center: Cartesian3::ZERO,
        }
    }

    /// Returns the geometric error scale.
    pub fn geometric_error_scale(&self) -> f64 {
        self.geometric_error_scale
    }

    /// Updates the geometric error scale based on vertical exaggeration.
    pub fn update_geometric_error_scale(&mut self) {
        self.geometric_error_scale = self.geometric_error * self.vertical_exaggeration;
    }

    /// Sets the vertical exaggeration.
    pub fn set_vertical_exaggeration(&mut self, exaggeration: f64, relative_height: f64) {
        self.vertical_exaggeration = exaggeration;
        self.vertical_exaggeration_relative_height = relative_height;
        self.update_geometric_error_scale();
    }

    /// Returns whether the content is ready.
    pub fn content_ready(&self) -> bool {
        self.content_state == Cesium3DTileContentState::Ready
    }

    /// Returns whether the content is loading.
    pub fn content_loading(&self) -> bool {
        self.content_state == Cesium3DTileContentState::Loading
    }

    /// Returns whether the content has failed.
    pub fn content_failed(&self) -> bool {
        self.content_state == Cesium3DTileContentState::Failed
    }

    /// Returns whether this tile is a leaf (no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

impl Default for Cesium3DTile {
    fn default() -> Self { Self::new() }
}
