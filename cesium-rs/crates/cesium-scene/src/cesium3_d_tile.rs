//! Ported from `packages/engine/Source/Scene/Cesium3DTile.js`.
//!
//! A single tile in a 3D Tiles tileset, plus the typed representation of a
//! tileset.json tile header.

use serde::{Deserialize, Serialize};

use cesium_core::cartesian3::Cartesian3;
use cesium_core::julian_date::JulianDate;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::rectangle::Rectangle;
use cesium_core::runtime_error::RuntimeError;

use crate::cesium3_d_tile_content_state::Cesium3DTileContentState;
use crate::cesium3_d_tile_refine::Cesium3DTileRefine;
use crate::tile_bounding_volume::TileBoundingVolume;

/// The `expire` property of a tileset.json tile header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpireHeader {
    /// Duration in seconds after content is ready that content expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Date when the content expires (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// A bounding volume in a tileset.json header (`box`, `region` or
/// `sphere`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundingVolumeHeader {
    /// An array of 12 numbers that define an oriented bounding box.
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_: Option<Vec<f64>>,
    /// An array of six numbers that define a bounding geographic region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<Vec<f64>>,
    /// An array of four numbers that define a bounding sphere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sphere: Option<Vec<f64>>,
}

/// The `content` (or an entry of `contents`) property of a tileset.json
/// tile header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentHeader {
    /// The URI of the tile content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// DEPRECATED: the URL of the tile content (use `uri` instead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// An optional tight-fit bounding volume around the content.
    #[serde(default, rename = "boundingVolume", skip_serializing_if = "Option::is_none")]
    pub bounding_volume: Option<BoundingVolumeHeader>,
}

/// The JSON header of a tile in a tileset.json (`root` and `children`
/// entries).
///
/// Mirrors the tile object consumed by the CesiumJS `Cesium3DTile`
/// constructor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cesium3DTileHeader {
    /// A floating-point 4x4 transformation matrix (column major),
    /// optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Vec<f64>>,
    /// The bounding volume of the tile.
    #[serde(default, rename = "boundingVolume", skip_serializing_if = "Option::is_none")]
    pub bounding_volume: Option<BoundingVolumeHeader>,
    /// An optional volume where the content of this tile is requested.
    #[serde(default, rename = "viewerRequestVolume", skip_serializing_if = "Option::is_none")]
    pub viewer_request_volume: Option<BoundingVolumeHeader>,
    /// The error, in meters, introduced if this tile is rendered and its
    /// children are not.
    #[serde(default, rename = "geometricError", skip_serializing_if = "Option::is_none")]
    pub geometric_error: Option<f64>,
    /// Specifies how a tile's content is refined ("REPLACE" or "ADD").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refine: Option<String>,
    /// The tile content (3D Tiles 1.0 schema).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentHeader>,
    /// The tile contents array (3D Tiles 1.1 schema).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<ContentHeader>>,
    /// The children tiles.
    #[serde(default)]
    pub children: Vec<Cesium3DTileHeader>,
    /// Content expiration metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire: Option<ExpireHeader>,
}

/// The subset of a parent tile's state needed to construct a child tile.
///
/// Rust analogue of the `parent` parameter of the CesiumJS `Cesium3DTile`
/// constructor (the constructor only reads the parent's transforms,
/// geometric error and refine).
#[derive(Debug, Clone)]
pub struct ParentTileContext {
    /// The parent's computed transform.
    pub computed_transform: Matrix4,
    /// The parent's initial transform.
    pub initial_transform: Matrix4,
    /// The parent's geometric error.
    pub geometric_error: f64,
    /// The parent's refine mode.
    pub refine: Cesium3DTileRefine,
}

impl Cesium3DTile {
    /// Extracts the [`ParentTileContext`] needed to build children of
    /// this tile.
    pub fn parent_context(&self) -> ParentTileContext {
        ParentTileContext {
            computed_transform: self.computed_transform,
            initial_transform: self.initial_transform,
            geometric_error: self.geometric_error,
            refine: self.refine,
        }
    }
}

/// A single tile in a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// When a tile is first created, its content is not loaded; the content is
/// loaded on-demand when needed based on the view.
///
/// DEVIATION: CesiumJS tiles are reference-counted objects holding a
/// `parent`/`children` pointer graph; the Rust port stores tiles in a flat
/// `Vec<Cesium3DTile>` owned by the tileset and references them by index.
pub struct Cesium3DTile {
    // ---- transforms ----
    /// The local transform of this tile.
    pub transform: Matrix4,
    /// The final computed transform (parent * local).
    pub computed_transform: Matrix4,
    /// The initial transform (without any runtime model matrix changes).
    pub initial_transform: Matrix4,

    // ---- hierarchy ----
    /// This tile's parent, or `None` if root.
    pub parent: Option<usize>,
    /// This tile's children.
    pub children: Vec<usize>,

    // ---- bounding volumes ----
    /// The bounding volume for this tile.
    pub bounding_volume: Option<TileBoundingVolume>,
    /// The content bounding volume (tight-fit around features).
    pub content_bounding_volume: Option<TileBoundingVolume>,
    /// The viewer request volume.
    pub viewer_request_volume: Option<TileBoundingVolume>,

    // ---- geometric error ----
    /// The error, in meters, introduced if this tile is rendered and its children are not.
    pub geometric_error: f64,
    /// Scaled geometric error (accounts for vertical exaggeration).
    geometric_error_scale: f64,

    // ---- refinement ----
    /// The refinement type (ADD or REPLACE).
    pub refine: Cesium3DTileRefine,

    // ---- content ----
    /// The URI of the tile content, if any.
    pub content_uri: Option<String>,
    /// The content state.
    pub content_state: Cesium3DTileContentState,
    /// Whether the tile has no content.
    pub has_empty_content: bool,
    /// Whether the tile's content points to an external tileset.
    pub has_tileset_content: bool,
    /// Whether the tile's content is an implicit tileset.
    pub has_implicit_content: bool,
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
    /// The distance from the camera to the closest point on the tile.
    pub distance_to_camera: f64,
    /// The depth of the tile center along the camera z axis (tie breaker).
    pub center_z_depth: f64,
    /// The frame number when this tile was last visited.
    pub visited_frame: u64,
    /// The frame number when this tile was last touched in the cache.
    pub touched_frame: u64,
    /// The frame number when this tile was last selected.
    pub selected_frame: u64,
    /// The frame number when this tile was last requested for loading.
    pub requested_frame: u64,

    // ---- caching ----
    /// The time when this tile was last selected for rendering.
    pub last_selected_time: f64,
    /// The number of frames this tile has been loading.
    pub loading_frames_count: i32,

    // ---- vertical exaggeration ----
    vertical_exaggeration: f64,
    vertical_exaggeration_relative_height: f64,

    /// The tile's center (center of the bounding volume's bounding
    /// sphere).
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
            viewer_request_volume: None,
            geometric_error: 0.0,
            geometric_error_scale: 1.0,
            refine: Cesium3DTileRefine::Replace,
            content_uri: None,
            content_state: Cesium3DTileContentState::Unloaded,
            has_empty_content: false,
            has_tileset_content: false,
            has_implicit_content: false,
            has_renderable_content: true,
            has_multiple_contents: false,
            features_length: 0,
            expire_duration: 0.0,
            expire_date: None,
            was_selected_last_frame: false,
            is_visible: false,
            screen_space_error: 0.0,
            depth: 0,
            distance_to_camera: 0.0,
            center_z_depth: 0.0,
            visited_frame: 0,
            touched_frame: 0,
            selected_frame: 0,
            requested_frame: 0,
            last_selected_time: 0.0,
            loading_frames_count: 0,
            vertical_exaggeration: 1.0,
            vertical_exaggeration_relative_height: 0.0,
            center: Cartesian3::ZERO,
        }
    }

    /// Creates a tile from its JSON header, mirroring the CesiumJS
    /// `Cesium3DTile(tileset, baseResource, header, parent)` constructor.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the header has no `boundingVolume`
    /// or the bounding volume contains none of `box` / `region` / `sphere`.
    pub fn from_header(
        header: &Cesium3DTileHeader,
        parent: Option<&ParentTileContext>,
        tileset_model_matrix: &Matrix4,
        tileset_geometric_error: f64,
    ) -> Result<Self, RuntimeError> {
        let mut tile = Self::new();

        let has_contents_array = header.contents.is_some();
        let contents = header.contents.as_deref().unwrap_or(&[]);
        let has_multiple_contents = has_contents_array && contents.len() > 1;
        tile.has_multiple_contents = has_multiple_contents;

        // In the 1.0 schema, content is stored in tile.content instead of
        // tile.contents.
        let content_header: Option<&ContentHeader> =
            if has_contents_array && !has_multiple_contents {
                contents.first()
            } else {
                header.content.as_ref()
            };

        tile.transform = match &header.transform {
            Some(array) => Matrix4::unpack_new(array, 0),
            None => Matrix4::IDENTITY,
        };

        let parent_transform = match parent {
            Some(parent) => &parent.computed_transform,
            None => tileset_model_matrix,
        };
        tile.computed_transform =
            Matrix4::multiply_new(parent_transform, &tile.transform);

        let parent_initial_transform = match parent {
            Some(parent) => &parent.initial_transform,
            None => &Matrix4::IDENTITY,
        };
        tile.initial_transform =
            Matrix4::multiply_new(parent_initial_transform, &tile.transform);

        tile.bounding_volume = Some(create_bounding_volume(
            header.bounding_volume.as_ref(),
            &tile.computed_transform,
            &tile.initial_transform,
        )?);

        if let Some(content_header) = content_header {
            if let Some(volume_header) = &content_header.bounding_volume {
                // Non-leaf tiles may have a content bounding volume, a
                // tight-fit bounding volume around only the features in the
                // tile.
                tile.content_bounding_volume = Some(create_bounding_volume(
                    Some(volume_header),
                    &tile.computed_transform,
                    &tile.initial_transform,
                )?);
            }
        }

        if let Some(volume_header) = &header.viewer_request_volume {
            tile.viewer_request_volume = Some(create_bounding_volume(
                Some(volume_header),
                &tile.computed_transform,
                &tile.initial_transform,
            )?);
        }

        // The error, in meters, introduced if this tile is rendered and its
        // children are not.
        match header.geometric_error {
            Some(geometric_error) => tile.geometric_error = geometric_error,
            None => {
                tile.geometric_error = match parent {
                    Some(parent) => parent.geometric_error,
                    None => tileset_geometric_error,
                };
                // CesiumJS raises the "geometricErrorUndefined"
                // deprecation warning here; the Rust port logs nothing.
                // DEVIATION: oneTimeWarning is not surfaced.
            }
        }
        tile.update_geometric_error_scale();

        if let Some(refine) = &header.refine {
            // CesiumJS warns on lowercase refine values; both are accepted.
            tile.refine = if refine.to_uppercase() == "REPLACE" {
                Cesium3DTileRefine::Replace
            } else {
                Cesium3DTileRefine::Add
            };
        } else if let Some(parent) = parent {
            // Inherit from parent tile if omitted.
            tile.refine = parent.refine;
        } else {
            tile.refine = Cesium3DTileRefine::Replace;
        }

        // Content URI handling (mirrors the constructor branches; external
        // resource fetching is deferred to the async pipeline).
        if has_multiple_contents {
            tile.content_state = Cesium3DTileContentState::Unloaded;
        } else if let Some(content_header) = content_header {
            let mut content_header_uri = content_header.uri.clone();
            if content_header_uri.is_none() && content_header.url.is_some() {
                // "content.url" is deprecated, use "content.uri" instead.
                content_header_uri = content_header.url.clone();
            }
            match content_header_uri.as_deref() {
                Some("") => {
                    // An empty string creates a circular dependency; treat
                    // the tile as having empty content.
                    tile.has_empty_content = true;
                    tile.content_state = Cesium3DTileContentState::Ready;
                }
                Some(uri) => {
                    tile.content_state = Cesium3DTileContentState::Unloaded;
                    tile.content_uri = Some(uri.to_string());
                }
                None => {
                    tile.has_empty_content = true;
                    tile.content_state = Cesium3DTileContentState::Ready;
                }
            }
        } else {
            tile.has_empty_content = true;
            tile.content_state = Cesium3DTileContentState::Ready;
        }

        if let Some(expire) = &header.expire {
            tile.expire_duration = expire.duration.unwrap_or(0.0);
            tile.expire_date = expire
                .date
                .as_deref()
                .and_then(JulianDate::from_iso8601);
        }

        tile.center = tile
            .bounding_volume
            .as_ref()
            .map(|volume| volume.bounding_sphere().center)
            .unwrap_or(Cartesian3::ZERO);

        Ok(tile)
    }

    /// Returns the geometric error scale.
    pub fn geometric_error_scale(&self) -> f64 {
        self.geometric_error_scale
    }

    /// Updates the geometric error scale based on vertical exaggeration.
    ///
    /// Mirrors `updateGeometricErrorScale()`.
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

    /// Returns whether the tile has unloaded renderable content, i.e.
    /// content that should eventually be loaded.
    ///
    /// Mirrors the `hasUnloadedRenderableContent` getter.
    pub fn has_unloaded_renderable_content(&self) -> bool {
        !(self.has_empty_content
            || self.has_tileset_content
            || self.has_implicit_content)
            && self.content_state == Cesium3DTileContentState::Unloaded
    }
}

impl Default for Cesium3DTile {
    fn default() -> Self { Self::new() }
}

/// Creates a bounding volume from a tile's bounding volume header.
///
/// Mirrors `Cesium3DTile.prototype.createBoundingVolume` (metadata
/// semantic overrides and vertical exaggeration excluded: exaggeration
/// defaults to 1.0 at construction time).
///
/// # Errors
/// Returns `RuntimeError("boundingVolume must be defined")` when the
/// header is missing, or `RuntimeError("boundingVolume must contain a
/// sphere, region, or box")` when it has none of the three forms.
pub fn create_bounding_volume(
    bounding_volume_header: Option<&BoundingVolumeHeader>,
    transform: &Matrix4,
    initial_transform: &Matrix4,
) -> Result<TileBoundingVolume, RuntimeError> {
    let bounding_volume_header = bounding_volume_header.ok_or_else(|| {
        RuntimeError::new(Some("boundingVolume must be defined"))
    })?;

    if let Some(box_) = &bounding_volume_header.box_ {
        return Ok(create_box(box_, transform));
    }
    if let Some(region) = &bounding_volume_header.region {
        return Ok(create_region(region, transform, initial_transform));
    }
    if let Some(sphere) = &bounding_volume_header.sphere {
        return Ok(create_sphere(sphere, transform));
    }
    Err(RuntimeError::new(Some(
        "boundingVolume must contain a sphere, region, or box",
    )))
}

/// DEVIATION: cesium-core `Matrix4::multiply_by_point` currently uses the
/// wrong element indices for the z row (3/7/11/15 instead of 2/6/10/14)
/// and applies a perspective division; this local affine variant mirrors
/// CesiumJS `Matrix4.multiplyByPoint` for affine transformations until the
/// Core implementation is fixed.
fn multiply_by_point_affine(matrix: &Matrix4, point: &Cartesian3) -> Cartesian3 {
    let e = &matrix.elements;
    Cartesian3::new(
        e[0] * point.x + e[4] * point.y + e[8] * point.z + e[12],
        e[1] * point.x + e[5] * point.y + e[9] * point.z + e[13],
        e[2] * point.x + e[6] * point.y + e[10] * point.z + e[14],
    )
}

/// Creates an oriented bounding box volume from the 12-element `box`
/// array, mirroring the private `createBox(box, transform)` helper.
pub fn create_box(box_data: &[f64], transform: &Matrix4) -> TileBoundingVolume {
    let mut center = Cartesian3::from_elements_new(box_data[0], box_data[1], box_data[2]);
    let mut half_axes = Matrix3::from_array_new(box_data, 3);

    // Find the transformed center and halfAxes
    center = multiply_by_point_affine(transform, &center);
    let rotation_scale = Matrix4::get_matrix3_new(transform);
    half_axes = Matrix3::multiply_new(&rotation_scale, &half_axes);

    TileBoundingVolume::new_box(center, half_axes)
}

/// Creates a region volume from the six-element `region` array, mirroring
/// the private `createRegion(region, transform, initialTransform)` helper.
///
/// DEVIATION: CesiumJS converts the region into an oriented bounding box
/// via `createBoxFromTransformedRegion` when `transform` differs from the
/// initial transform; that path depends on the Core API
/// `OrientedBoundingBox.fromRectangle`, which is not yet ported, so the
/// Rust port always produces a region volume.
pub fn create_region(
    region: &[f64],
    _transform: &Matrix4,
    _initial_transform: &Matrix4,
) -> TileBoundingVolume {
    let rectangle = Rectangle::unpack(region, Some(0));
    TileBoundingVolume::new_region(rectangle, region[4], region[5])
}

/// Creates a bounding sphere volume from the four-element `sphere` array,
/// mirroring the private `createSphere(sphere, transform)` helper.
pub fn create_sphere(sphere: &[f64], transform: &Matrix4) -> TileBoundingVolume {
    let mut center = Cartesian3::from_elements_new(sphere[0], sphere[1], sphere[2]);
    let mut radius = sphere[3];

    // Find the transformed center and radius
    center = multiply_by_point_affine(transform, &center);
    let scale = Matrix4::get_scale_new(transform);
    let uniform_scale = Cartesian3::maximum_component(&scale);
    radius *= uniform_scale;

    TileBoundingVolume::new_sphere(center, radius)
}

/// Computes the screen space error of a tile in a perspective projection.
///
/// Mirrors the perspective branch of
/// `Cesium3DTile.prototype.getScreenSpaceError`:
/// `error = (geometricError * viewportHeight) / (distance * sseDenominator)`,
/// clamped at `EPSILON7` distance and divided by `pixelRatio`.
///
/// Returns 0.0 when `geometric_error` is 0 (leaf tiles).
#[must_use]
pub fn screen_space_error(
    geometric_error: f64,
    distance_to_camera: f64,
    viewport_height: f64,
    sse_denominator: f64,
    pixel_ratio: f64,
) -> f64 {
    if geometric_error == 0.0 {
        // Leaf tiles do not have any error so save the computation
        return 0.0;
    }
    // Avoid divide by zero when viewer is inside the tile
    let distance = distance_to_camera.max(CesiumMath::EPSILON7);
    let mut error = (geometric_error * viewport_height) / (distance * sse_denominator);
    error /= pixel_ratio;
    error
}

/// Computes the screen space error of a tile in a 2D / orthographic
/// projection.
///
/// Mirrors the orthographic branch of
/// `Cesium3DTile.prototype.getScreenSpaceError`:
/// `pixelSize = max(top - bottom, right - left) / max(width, height)`,
/// `error = geometricError / pixelSize / pixelRatio`.
#[must_use]
pub fn screen_space_error_orthographic(
    geometric_error: f64,
    frustum_width: f64,
    frustum_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    pixel_ratio: f64,
) -> f64 {
    if geometric_error == 0.0 {
        return 0.0;
    }
    let pixel_size = frustum_height.max(frustum_width)
        / viewport_width.max(viewport_height);
    let mut error = geometric_error / pixel_size;
    error /= pixel_ratio;
    error
}
