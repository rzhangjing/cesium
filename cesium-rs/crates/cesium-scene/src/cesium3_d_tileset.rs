//! Ported from `packages/engine/Source/Scene/Cesium3DTileset.js`.
//!
//! A 3D Tiles tileset, containing a hierarchy of tiles with geometric content.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::matrix4::Matrix4;
use cesium_core::resource::ResourceBackend;
use cesium_core::runtime_error::RuntimeError;

use crate::cesium3_d_tile::{Cesium3DTile, Cesium3DTileHeader};
use crate::cesium3_d_tileset_statistics::Cesium3DTilesetStatistics;
use crate::file_resource_backend::FileResourceBackend;
use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// The `asset` property of a tileset.json.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TilesetAsset {
    /// The 3D Tiles version (e.g. "1.0" or "1.1").
    #[serde(default)]
    pub version: String,
    /// The application-specific version of the tileset content.
    #[serde(default, rename = "tilesetVersion", skip_serializing_if = "Option::is_none")]
    pub tileset_version: Option<String>,
}

/// The parsed tileset.json root object.
///
/// Mirrors the JSON object consumed by
/// `Cesium3DTileset.prototype._loadTilesetJson`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TilesetHeader {
    /// Metadata about the entire tileset.
    #[serde(default)]
    pub asset: TilesetAsset,
    /// The error, in meters, determined from the level of detail.
    #[serde(default, rename = "geometricError")]
    pub geometric_error: Option<f64>,
    /// A metadata object about the tileset.
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    /// The root tile.
    #[serde(default)]
    pub root: Option<Cesium3DTileHeader>,
    /// Extensions used by the tileset.
    #[serde(default, rename = "extensionsUsed")]
    pub extensions_used: Vec<String>,
    /// Extensions required by the tileset.
    #[serde(default, rename = "extensionsRequired")]
    pub extensions_required: Vec<String>,
}

/// Initialization options for [`Cesium3DTileset`], mirroring the subset of
/// `Cesium3DTileset.ConstructorOptions` consumed by the CPU port.
///
/// Every field is optional; `None` keeps the constructor default. The JS
/// exposes many more (GPU/traversal-tuning) options that the port does not
/// yet model.
#[derive(Debug, Clone, Default)]
pub struct Cesium3DTilesetOptions {
    /// Whether the tileset is shown (JS `show`, default `true`).
    pub show: Option<bool>,
    /// The model matrix applied to the tileset root (JS `modelMatrix`,
    /// default `Matrix4.IDENTITY`).
    pub model_matrix: Option<Matrix4>,
    /// The shadow mode (JS `shadows`, default `ShadowMode.ENABLED`).
    pub shadows: Option<ShadowMode>,
    /// The maximum screen-space error driving LOD (JS
    /// `maximumScreenSpaceError`, default 16).
    pub maximum_screen_space_error: Option<f64>,
    /// The maximum memory usage in MB (JS legacy `maximumMemoryUsage`,
    /// default 512).
    pub maximum_memory_usage: Option<f64>,
    /// Whether to preload ancestors of visible tiles (JS
    /// `preloadWhenHidden`, default `false`).
    pub preload_when_visible: Option<bool>,
}

impl Cesium3DTilesetOptions {
    /// Creates empty options (all constructor defaults).
    pub fn new() -> Self {
        Self::default()
    }
}

/// A 3D Tiles tileset, containing a hierarchy of tiles with geometric content.
///
/// This is the main entry point for loading and rendering 3D Tiles data.
/// It manages the tile hierarchy, traversal, selection, and rendering pipeline.
///
/// DEVIATION: CesiumJS stores tiles as a pointer graph; the Rust port owns
/// all tiles in a flat vector ([`Cesium3DTileset::tiles`]) and references
/// them by index ([`Cesium3DTileset::root`]).
pub struct Cesium3DTileset {
    /// The URL of the tileset JSON.
    pub url: Option<String>,
    /// Whether this tileset is shown.
    pub show: bool,
    /// The maximum screen-space error used to drive level-of-detail.
    pub maximum_screen_space_error: f64,
    /// The maximum number of tiles to load simultaneously.
    pub maximum_number_of_loaded_tiles: i32,
    /// The maximum memory usage in MB (0 = unlimited).
    pub maximum_memory_usage: f64,
    /// Whether to preload ancestors of visible tiles.
    pub preload_when_visible: bool,
    /// The model matrix applied to the entire tileset.
    pub model_matrix: Matrix4,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// Whether the tileset has been destroyed.
    is_destroyed: bool,
    /// Whether all tiles are loaded.
    all_tiles_loaded: bool,
    /// Statistics about the tileset.
    statistics: Cesium3DTilesetStatistics,
    /// The bounding sphere of the entire tileset.
    bounding_sphere: BoundingSphere,
    /// The error, in meters, of the tileset root (tileset.json
    /// `geometricError`).
    geometric_error: f64,
    /// The tiles of the tileset, indexed by handle.
    tiles: Vec<Cesium3DTile>,
    /// The root tile index into [`Self::tiles`].
    root: Option<usize>,
}

impl Cesium3DTileset {
    /// Creates a new Cesium3DTileset.
    pub fn new() -> Self {
        Self {
            url: None,
            show: true,
            maximum_screen_space_error: 16.0,
            maximum_number_of_loaded_tiles: 0,
            maximum_memory_usage: 512.0,
            preload_when_visible: false,
            model_matrix: Matrix4::IDENTITY,
            shadows: ShadowMode::Enabled,
            is_destroyed: false,
            all_tiles_loaded: false,
            statistics: Cesium3DTilesetStatistics::default(),
            bounding_sphere: BoundingSphere::default(),
            geometric_error: 0.0,
            tiles: Vec::new(),
            root: None,
        }
    }

    /// Creates a tileset from a tileset JSON URL, mirroring the JS static
    /// entry point `Cesium3DTileset.fromUrl(url, options)`.
    ///
    /// The JSON is fetched through a [`FileResourceBackend`] (offline
    /// `file://` reads, matching the project's no-network policy), the
    /// constructor `options` are applied, and the tile hierarchy is built
    /// through [`Self::load_tileset_json`] (the JS `_loadTilesetJson`
    /// chain). The JS returns a promise that resolves once the tileset is
    /// ready; the port resolves the whole chain synchronously.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the URL cannot be fetched or the
    /// tileset JSON is invalid.
    pub fn from_url(
        url: &str,
        options: Option<Cesium3DTilesetOptions>,
    ) -> Result<Self, RuntimeError> {
        Self::from_url_with_backend(url, options, &FileResourceBackend::new())
    }

    /// Same as [`Self::from_url`] with an injected [`ResourceBackend`], so
    /// tests (and alternative backends) can drive the load path without
    /// touching the filesystem layout expected by [`FileResourceBackend`].
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the URL cannot be fetched or the
    /// tileset JSON is invalid.
    pub fn from_url_with_backend<B: ResourceBackend>(
        url: &str,
        options: Option<Cesium3DTilesetOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        //>> DeveloperError: url is required.
        debug_assert!(!url.is_empty(), "url is required.");

        let mut tileset = Self::new();

        // Apply the constructor options (mirrors the `options.*` branch of
        // the JS constructor body).
        if let Some(options) = options {
            if let Some(show) = options.show {
                tileset.show = show;
            }
            if let Some(model_matrix) = options.model_matrix {
                tileset.model_matrix = model_matrix;
            }
            if let Some(shadows) = options.shadows {
                tileset.shadows = shadows;
            }
            if let Some(maximum_screen_space_error) = options.maximum_screen_space_error {
                tileset.maximum_screen_space_error = maximum_screen_space_error;
            }
            if let Some(maximum_memory_usage) = options.maximum_memory_usage {
                tileset.maximum_memory_usage = maximum_memory_usage;
            }
            if let Some(preload_when_visible) = options.preload_when_visible {
                tileset.preload_when_visible = preload_when_visible;
            }
        }

        tileset.url = Some(url.to_owned());

        // Mirrors `Resource.fetchJson` → `_loadTilesetJson` (the JS
        // `fromUrl` promise chain); the offline fetch resolves entirely
        // through synchronous steps, so a no-op-waker poll loop converges.
        let json = block_on_sync(backend.fetch_text(url, &HashMap::new()))
            .map_err(|error| RuntimeError::new(Some(&error.to_string())))?;
        tileset.load_tileset_json(&json)?;

        Ok(tileset)
    }

    /// Parses a tileset.json string and builds the tile hierarchy.
    ///
    /// Rust analogue of `Cesium3DTileset.prototype._loadTilesetJson` (CPU
    /// portion): stores the tileset-level `geometricError`, creates the
    /// root tile and recursively creates all children. Network fetches of
    /// external tileset JSONs referenced by tile content are deferred.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the JSON is invalid or a tile
    /// header is missing its `boundingVolume`.
    pub fn load_tileset_json(&mut self, json: &str) -> Result<(), RuntimeError> {
        let header: TilesetHeader = serde_json::from_str(json).map_err(|e| {
            RuntimeError::new(Some(&format!("Failed to load tileset JSON: {e}")))
        })?;
        self.create_tile_hierarchy(&header)
    }

    /// Builds the tile hierarchy from a parsed [`TilesetHeader`].
    ///
    /// Mirrors `_createTileTree(tilesetJson)`.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when a tile header is missing its
    /// `boundingVolume` or the bounding volume is malformed.
    pub fn create_tile_hierarchy(&mut self, tileset_header: &TilesetHeader) -> Result<(), RuntimeError> {
        self.geometric_error = tileset_header.geometric_error.unwrap_or(0.0);

        let Some(root_header) = &tileset_header.root else {
            self.root = None;
            return Ok(());
        };

        self.tiles.clear();
        let root_index = self.create_tile_recursively(root_header, None)?;
        self.root = Some(root_index);

        // Mirrors `statistics.numberOfTilesTotal` bookkeeping: number of
        // tiles in the tileset JSON.
        self.statistics.number_of_tiles_total = self.tiles.len() as i32;

        // The tileset bounding sphere is derived from the root tile's
        // bounding volume (mirrors `_root.boundingVolume.boundingSphere`).
        if let Some(volume) = &self.tiles[root_index].bounding_volume {
            self.bounding_sphere = volume.bounding_sphere();
        }

        Ok(())
    }

    fn create_tile_recursively(
        &mut self,
        header: &Cesium3DTileHeader,
        parent_index: Option<usize>,
    ) -> Result<usize, RuntimeError> {
        let parent_context = parent_index.map(|index| self.tiles[index].parent_context());
        let mut tile = Cesium3DTile::from_header(
            header,
            parent_context.as_ref(),
            &self.model_matrix,
            self.geometric_error,
        )?;
        tile.parent = parent_index;
        tile.depth = match parent_index {
            Some(index) => self.tiles[index].depth + 1,
            None => 0,
        };

        let child_headers: Vec<Cesium3DTileHeader> = header.children.clone();
        let tile_index = self.tiles.len();
        self.tiles.push(tile);

        if let Some(parent_index) = parent_index {
            self.tiles[parent_index].children.push(tile_index);
        }

        for child_header in &child_headers {
            self.create_tile_recursively(child_header, Some(tile_index))?;
        }

        Ok(tile_index)
    }

    /// The index of the root tile in [`Self::tiles`], if the tileset has
    /// been loaded.
    ///
    /// Mirrors the readonly `root` property.
    pub fn root(&self) -> Option<usize> { self.root }

    /// The tiles of the tileset.
    pub fn tiles(&self) -> &[Cesium3DTile] { &self.tiles }

    /// The tiles of the tileset (mutable).
    pub fn tiles_mut(&mut self) -> &mut Vec<Cesium3DTile> { &mut self.tiles }

    /// The error, in meters, of the tileset (tileset.json
    /// `geometricError`).
    ///
    /// Mirrors the private `_geometricError` used as the root fallback.
    pub fn geometric_error(&self) -> f64 { self.geometric_error }

    /// Returns whether all tiles are loaded.
    pub fn all_tiles_loaded(&self) -> bool { self.all_tiles_loaded }

    /// Returns the tileset statistics.
    pub fn statistics(&self) -> &Cesium3DTilesetStatistics { &self.statistics }

    /// Returns the tileset statistics (mutable).
    pub fn statistics_mut(&mut self) -> &mut Cesium3DTilesetStatistics { &mut self.statistics }

    /// Returns the bounding sphere of the entire tileset.
    pub fn bounding_sphere(&self) -> &BoundingSphere { &self.bounding_sphere }

    /// Updates the tileset for the current frame.
    ///
    /// DEVIATION: the full traversal/selection/render pipeline depends on
    /// the GPU pass and is wired up with the renderer track; this CPU port
    /// keeps the entry point.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Cesium3DTileset {
    fn default() -> Self { Self::new() }
}

/// Drives a future to completion on the current thread without an executor.
///
/// The offline fetch chain resolves entirely through synchronous steps
/// (local file reads), so a no-op-waker poll loop always converges; the
/// loop is capped defensively against unexpected pending futures. Mirrors
/// the helper of the same name in `globe_terrain_fetcher.rs`.
fn block_on_sync<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut context = Context::from_waker(Waker::noop());
    let mut future = pin!(future);
    for _ in 0..64 {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => {}
        }
    }
    panic!("block_on_sync: future did not resolve within 64 polls")
}
