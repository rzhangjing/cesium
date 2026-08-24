//! Ported from `packages/engine/Source/Scene/GlobeSurfaceTileProvider.js`.
//!
//! Renders a tile of the globe surface.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cesium_terrain_provider::TerrainTileData;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::heightmap_terrain_data::CreateMeshOptions as HeightmapCreateMeshOptions;
use cesium_core::index_datatype::IndexDatatype;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::pixel_format::PixelFormat;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::framebuffer::Framebuffer;
use cesium_renderer::render_state::RenderState;
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::texture::{Texture, TextureOptions, TextureSource};
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_shaders::wgsl;

use crate::frame_state::FrameState;
use crate::globe_surface_shader_set::GlobeSurfaceShaderSet;
use crate::globe_terrain_fetcher::{GlobeTerrainFetcher, TerrainGeometryOutcome};
use crate::globe_tile_geometry::create_ellipsoid_grid;
use crate::imagery_layer_collection::ImageryLayerCollection;
use crate::imagery_provider::TileImageAvailability;
use crate::quadtree_tile::QuadtreeTile;
use crate::shadow_mode::ShadowMode;

/// GPU geometry for one quadtree tile: the ellipsoid longitude/latitude grid
/// over the tile's own rectangle.
///
/// NOTE (cesiumrust lesson): same-level geographic tiles are congruent in
/// lon/lat space but NOT on the ellipsoid — the cartographic→cartesian
/// mapping is nonlinear, so a per-level shared mesh translated by a model
/// matrix is impossible; geometry is cached per tile instead.
struct TileGeometryResources {
    vertex_array: Arc<VertexArray>,
    index_count: u32,
}

/// GPU resources for one rendered tile (composed imagery texture).
struct TileSurfaceResources {
    /// The day texture bound at group(1) binding(0) of `globe_fs.wgsl`.
    texture: Arc<Texture>,
    /// Mirrors CesiumJS `TileImagery.usingAncestorTexture`: set when a
    /// deterministic no-data tile inherited its ancestor's texture.
    using_ancestor_texture: bool,
}

/// Outcome of composing the imagery layers for one tile on the CPU.
enum ComposeOutcome {
    /// All layers contributed data: upload this RGBA8 image.
    Data(Vec<u8>, u32, u32),
    /// At least one layer deterministically has no data for this tile
    /// (missing file / beyond the provider's maximum level): the tile may
    /// inherit its ancestor texture permanently.
    NoData,
    /// A layer hit a transient failure (e.g. IO error): retry on a later
    /// frame. MUST NOT be stamped as permanent no-data.
    Transient,
}

/// Terrain readiness of one quadtree tile (B4-5).
///
/// failed/placeholder discipline (cesiumrust pitfall checkpoint): `NoData`
/// is the ONLY permanent negative state and is reached exclusively through
/// deterministic absence (404 / known-unavailable). IO failures land in
/// `Transient` and are retried after a cooldown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileTerrainState {
    /// Never requested.
    Unloaded,
    /// The terrain mesh (real or upsampled) is ready.
    Ready,
    /// Deterministic absence; ancestor geometry may be inherited forever.
    NoData,
    /// Transient failure; retry once the cooldown frame has passed.
    Transient,
}

/// Per-tile terrain bookkeeping.
struct TileTerrainEntry {
    state: TileTerrainState,
    /// The tile's terrain data with its mesh created (heightmap path).
    data: Option<TerrainTileData>,
    /// Mirrors CesiumJS `tile.upsampledFrom`: the data was created by
    /// upsampling an ancestor (deterministic no-data inheritance).
    upsampled_from: Option<(i32, i32, i32)>,
    /// Frame number before which a `Transient` tile is not retried.
    retry_after_frame: u64,
}

/// Cooldown (in frames) before a transient terrain failure is retried.
const TERRAIN_RETRY_COOLDOWN_FRAMES: u64 = 30;

/// Renders a tile of the globe surface.
///
/// This is the workhorse of the globe rendering system — for each visible quadtree tile,
/// it assembles terrain geometry, imagery textures, and shader uniforms, then issues
/// draw commands.
///
/// DEVIATION (B4-3): CesiumJS assembles `TerrainMesh` data from the terrain
/// provider and reprojects every imagery layer into per-tile textures through
/// `ImageryLayerTextureCache`. This batch renders the ellipsoid terrain with
/// a simplified longitude/latitude grid and composites the imagery layers
/// bottom-up on the CPU into a single day texture per tile (TEXONLY globe
/// shader). The terrain provider upgrade (B4-5) swaps the mesh generator;
/// the WebMercator reprojection stays CPU-side for now.
pub struct GlobeSurfaceTileProvider {
    // ---- Configuration ----
    surface_shader_set: GlobeSurfaceShaderSet,

    // ---- Terrain ----
    enable_lighting: bool,
    lambert_diffuse_multiplier: f64,
    show_skirts: bool,
    back_face_culling: bool,
    vertex_shadow_darkness: f64,

    // ---- Atmosphere ----
    dynamic_atmosphere_lighting: bool,
    dynamic_atmosphere_lighting_from_sun: bool,
    show_ground_atmosphere: bool,
    atmosphere_light_intensity: f64,
    atmosphere_rayleigh_coefficient: Cartesian3,
    atmosphere_mie_coefficient: Cartesian3,
    atmosphere_rayleigh_scale_height: f64,
    atmosphere_mie_scale_height: f64,
    atmosphere_mie_anisotropy: f64,
    hue_shift: f64,
    saturation_shift: f64,
    brightness_shift: f64,

    // ---- Lighting fade ----
    lighting_fade_out_distance: f64,
    lighting_fade_in_distance: f64,
    night_fade_out_distance: f64,
    night_fade_in_distance: f64,

    // ---- Water ----
    has_water_mask: bool,
    show_water_effect: bool,
    zoomed_out_ocean_specular_intensity: f64,

    // ---- Underground coloring ----
    underground_color: Color,
    underground_color_alpha_by_distance: NearFarScalar,

    // ---- Shadows ----
    shadows: ShadowMode,

    // ---- Fill highlight ----
    fill_highlight_color: Option<Color>,

    // ---- Base color (when no imagery) ----
    base_color: Color,

    // ---- Render resources (B4-3) ----
    /// The globe TEXONLY shader program (lazy).
    shader_program: Option<Arc<ShaderProgram>>,
    /// Per-tile ellipsoid grid geometry cache, keyed by `(level, x, y)`.
    /// Bounded by the traversal's maximum level (the imagery provider
    /// ceiling), so the cache cannot grow unboundedly.
    geometry_cache: HashMap<(i32, i32, i32), Arc<TileGeometryResources>>,
    /// Per-tile resources, keyed by `(level, x, y)`.
    tile_resources: HashMap<(i32, i32, i32), TileSurfaceResources>,
    /// Insertion order for LRU-style eviction against `tile_cache_size`.
    tile_resource_order: VecDeque<(i32, i32, i32)>,
    /// Tile cache size propagated from `Globe.tile_cache_size`.
    tile_cache_size: i32,
    /// 1×1 base-color texture (lazy), used before imagery is available.
    base_color_texture: Option<Arc<Texture>>,

    // ---- Terrain (B4-5) ----
    /// The terrain tile fetcher (`None` = ellipsoid terrain path).
    terrain_fetcher: Option<Box<dyn GlobeTerrainFetcher>>,
    /// Owned tiling scheme matching the terrain provider (used by
    /// `createMesh` / `upsample`).
    terrain_tiling_scheme: Option<Box<dyn TilingScheme>>,
    /// Per-tile terrain state, keyed by `(level, x, y)`.
    terrain_tiles: HashMap<(i32, i32, i32), TileTerrainEntry>,
    /// GPU geometry cache for terrain meshes, keyed by `(level, x, y)`.
    /// Separate from the ellipsoid grid cache: a tile flips between the two
    /// sources while its terrain is transient, and stale entries must never
    /// leak across the flip.
    terrain_geometry_cache: HashMap<(i32, i32, i32), Arc<TileGeometryResources>>,
}

impl GlobeSurfaceTileProvider {
    /// Creates a new GlobeSurfaceTileProvider.
    pub fn new() -> Self {
        Self {
            surface_shader_set: GlobeSurfaceShaderSet::new(),
            enable_lighting: false,
            lambert_diffuse_multiplier: 0.9,
            show_skirts: true,
            back_face_culling: true,
            vertex_shadow_darkness: 0.6,
            dynamic_atmosphere_lighting: true,
            dynamic_atmosphere_lighting_from_sun: false,
            show_ground_atmosphere: true,
            atmosphere_light_intensity: 10.0,
            atmosphere_rayleigh_coefficient: Cartesian3::new(5.5e-6, 13.0e-6, 28.4e-6),
            atmosphere_mie_coefficient: Cartesian3::new(21e-6, 21e-6, 21e-6),
            atmosphere_rayleigh_scale_height: 10000.0,
            atmosphere_mie_scale_height: 3200.0,
            atmosphere_mie_anisotropy: 0.999,
            hue_shift: 0.0,
            saturation_shift: 0.0,
            brightness_shift: 0.0,
            lighting_fade_out_distance: 1.0e7,
            lighting_fade_in_distance: 1.0e7,
            night_fade_out_distance: 1.0e7,
            night_fade_in_distance: 1.0e7,
            has_water_mask: false,
            show_water_effect: true,
            zoomed_out_ocean_specular_intensity: 0.4,
            underground_color: Color::new(0.0, 0.0, 0.0, 1.0),
            underground_color_alpha_by_distance: NearFarScalar::default(),
            shadows: ShadowMode::Disabled,
            fill_highlight_color: None,
            base_color: Color::new(0.0, 0.1, 0.3, 1.0), // dark blue-ish ocean
            shader_program: None,
            geometry_cache: HashMap::new(),
            tile_resources: HashMap::new(),
            tile_resource_order: VecDeque::new(),
            tile_cache_size: 100,
            base_color_texture: None,
            terrain_fetcher: None,
            terrain_tiling_scheme: None,
            terrain_tiles: HashMap::new(),
            terrain_geometry_cache: HashMap::new(),
        }
    }

    // ---- Setters (called by Globe::begin_frame to propagate properties) ----

    pub fn set_enable_lighting(&mut self, value: bool) { self.enable_lighting = value; }
    pub fn set_dynamic_atmosphere_lighting(&mut self, value: bool) { self.dynamic_atmosphere_lighting = value; }
    pub fn set_show_ground_atmosphere(&mut self, value: bool) { self.show_ground_atmosphere = value; }
    pub fn set_atmosphere_light_intensity(&mut self, value: f64) { self.atmosphere_light_intensity = value; }
    pub fn set_shadows(&mut self, value: ShadowMode) { self.shadows = value; }
    pub fn set_show_skirts(&mut self, value: bool) { self.show_skirts = value; }
    pub fn set_back_face_culling(&mut self, value: bool) { self.back_face_culling = value; }
    pub fn set_vertex_shadow_darkness(&mut self, value: f64) { self.vertex_shadow_darkness = value; }
    pub fn set_underground_color(&mut self, value: Color) { self.underground_color = value; }
    pub fn set_lambert_diffuse_multiplier(&mut self, value: f64) { self.lambert_diffuse_multiplier = value; }
    pub fn set_lighting_fade_out_distance(&mut self, value: f64) { self.lighting_fade_out_distance = value; }
    pub fn set_lighting_fade_in_distance(&mut self, value: f64) { self.lighting_fade_in_distance = value; }
    pub fn set_has_water_mask(&mut self, value: bool) { self.has_water_mask = value; }
    pub fn set_show_water_effect(&mut self, value: bool) { self.show_water_effect = value; }
    pub fn set_fill_highlight_color(&mut self, value: Option<Color>) { self.fill_highlight_color = value; }
    /// Sets the tile cache size (propagated from `Globe.tile_cache_size`).
    pub fn set_tile_cache_size(&mut self, value: i32) { self.tile_cache_size = value; }

    /// Installs (or clears) the terrain fetcher (B4-5). When `None`, tiles
    /// fall back to the ellipsoid terrain grid.
    pub fn set_terrain_fetcher(&mut self, fetcher: Option<Box<dyn GlobeTerrainFetcher>>) {
        self.terrain_tiling_scheme = fetcher.as_ref().map(|f| f.make_tiling_scheme());
        self.terrain_fetcher = fetcher;
        // New provider: every cached terrain outcome/mesh belongs to the old
        // one.
        self.terrain_tiles.clear();
        self.terrain_geometry_cache.clear();
    }

    /// The installed terrain fetcher, if any.
    pub fn terrain_fetcher(&self) -> Option<&dyn GlobeTerrainFetcher> {
        self.terrain_fetcher.as_deref()
    }

    /// Diagnostic hook: terrain state of a tile (tests assert the
    /// failed/placeholder and upsample invariants through this).
    pub fn terrain_tile_state(&self, level: i32, x: i32, y: i32) -> Option<TileTerrainState> {
        self.terrain_tiles.get(&(level, x, y)).map(|entry| entry.state)
    }

    /// Diagnostic hook: whether the tile's terrain was created by upsampling
    /// an ancestor (CesiumJS `tile.upsampledFrom`).
    pub fn terrain_upsampled_from(
        &self,
        level: i32,
        x: i32,
        y: i32,
    ) -> Option<Option<(i32, i32, i32)>> {
        self.terrain_tiles
            .get(&(level, x, y))
            .map(|entry| entry.upsampled_from)
    }

    // ---- Getters ----

    /// Gets the base color used when no imagery is available.
    pub fn base_color(&self) -> &Color { &self.base_color }
    /// Sets the base color.
    pub fn set_base_color(&mut self, value: Color) { self.base_color = value; }
    /// Whether lighting is enabled.
    pub fn enable_lighting(&self) -> bool { self.enable_lighting }
    /// Whether ground atmosphere is shown.
    pub fn show_ground_atmosphere(&self) -> bool { self.show_ground_atmosphere }
    /// The shadow mode.
    pub fn shadows(&self) -> ShadowMode { self.shadows }
    /// Number of cached per-tile GPU resource entries (test/diagnostic hook).
    pub fn tile_resource_count(&self) -> usize { self.tile_resources.len() }

    // ---- Frame lifecycle ----

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, _frame_state: &FrameState) {
        // In full port: process tile load queues, start new loads
    }

    /// Renders a single tile: ensures the ellipsoid grid vertex buffers and
    /// the composed imagery day texture exist, then submits a globe draw
    /// command targeting `framebuffer` (the globe offscreen pass with depth).
    ///
    /// All-or-nothing readiness invariant (cesiumrust pitfall checkpoint):
    /// quadtree traversal only replaces a parent with its children when the
    /// whole child set is renderable (synchronous loading in this batch), so
    /// a tile's mesh + texture are always created together in the same frame
    /// — the geometry and the day texture are never observed half-swapped.
    pub fn render_tile(
        &mut self,
        tile: &QuadtreeTile,
        layers: &ImageryLayerCollection,
        ellipsoid: &Ellipsoid,
        context: &mut Context,
        framebuffer: Option<Arc<Framebuffer>>,
    ) {
        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::GLOBE_VS,
                wgsl::GLOBE_FS,
                "globe_texonly".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("globe shader compilation failed: {error}");
                    return;
                }
            }
        }

        let key = (tile.level, tile.x, tile.y);
        // B4-5 geometry source: the terrain mesh when the tile's terrain is
        // ready (real or upsampled), otherwise the ellipsoid grid fallback
        // (CesiumJS's ellipsoid terrain placeholder while data is in
        // flight). The imagery day texture resolves independently, so the
        // all-or-nothing invariant holds per resource kind within a frame.
        let terrain_ready = self
            .terrain_tiles
            .get(&key)
            .is_some_and(|entry| entry.state == TileTerrainState::Ready);
        let geometry = if terrain_ready {
            self.ensure_terrain_geometry(tile, ellipsoid, context)
        } else {
            self.ensure_tile_geometry(tile, ellipsoid, context)
        };
        let (texture, using_ancestor_texture, cacheable) =
            self.resolve_tile_texture(tile, layers, context);

        let vertex_array = geometry.vertex_array.clone();
        let index_count = geometry.index_count;
        if cacheable {
            if !self.tile_resources.contains_key(&key) {
                self.tile_resource_order.push_back(key);
            }
            self.tile_resources.insert(
                key,
                TileSurfaceResources {
                    texture: texture.clone(),
                    using_ancestor_texture,
                },
            );
            self.evict_tile_resources();
        }

        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = true;
        render_state.depth_test.func = cesium_renderer::render_state::DepthFunction::Less;
        render_state.cull.enabled = self.back_face_culling;

        let mut command = DrawCommand::new();
        command.primitive_type = WebGLConstants::TRIANGLES;
        command.vertex_array = Some(vertex_array);
        command.count = Some(index_count);
        command.offset = 0;
        command.shader_program = self.shader_program.clone();
        command.uniform_overrides = vec![(
            "u_dayTexture".to_string(),
            UniformValue::Texture(texture),
        )];
        command.render_state = render_state;
        command.framebuffer = framebuffer;
        command.bounding_volume = Some(tile.bounding_sphere.clone());
        command.pass = Some(cesium_renderer::pass::Pass::Globe as u32);
        command.owner = Some("GlobeSurfaceTileProvider".to_string());

        context.draw(command);
    }

    /// Called at the end of each frame.
    pub fn end_frame(&mut self, _frame_state: &FrameState) {
        // In full port: free tiles not needed this frame
    }

    // ---- Terrain loading (B4-5) ----

    /// Ensures the terrain of the tiles selected this frame is loaded,
    /// upsampled, or classified (failed/placeholder discipline).
    ///
    /// DEVIATION (B4-5): CesiumJS runs `loadTile`/`processTile` through
    /// promise chains, the `GlobeTileState` machine and time-sliced load
    /// queues; the port processes the selected tiles synchronously inside
    /// the frame (local-file fetches resolve immediately), ancestors first
    /// (level-ascending) so upsample chains settle in one pass.
    pub fn prepare_terrain(&mut self, tiles: &[QuadtreeTile], frame_number: u64) {
        if self.terrain_fetcher.is_none() {
            return;
        }
        let mut keys: Vec<(i32, i32, i32)> = tiles
            .iter()
            .map(|tile| (tile.level, tile.x, tile.y))
            .collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            self.ensure_terrain(key, frame_number);
        }
    }

    /// Drives one tile toward a terminal terrain state (Ready / NoData) or
    /// holds it in the Transient cooldown.
    fn ensure_terrain(&mut self, key: (i32, i32, i32), frame_number: u64) {
        match self.terrain_tiles.get(&key) {
            Some(entry)
                if entry.state == TileTerrainState::Ready
                    || entry.state == TileTerrainState::NoData =>
            {
                return;
            }
            Some(entry)
                if entry.state == TileTerrainState::Transient
                    && frame_number < entry.retry_after_frame =>
            {
                return; // cooldown: retry on a later frame, never give up
            }
            _ => {}
        }

        let (level, x, y) = key;

        // Known-unavailable (availability bitmaps): deterministic no-data,
        // handled through ancestor inheritance exactly like a 404.
        let known_unavailable = self
            .terrain_fetcher
            .as_ref()
            .and_then(|fetcher| fetcher.get_tile_data_available(x, y, level))
            == Some(false);
        if known_unavailable {
            self.upsample_or_no_data(key, frame_number);
            return;
        }

        let Some(fetcher) = self.terrain_fetcher.as_mut() else {
            return;
        };
        match fetcher.request_tile_geometry(x, y, level) {
            TerrainGeometryOutcome::Data(data) => {
                self.process_tile_data(key, data, frame_number)
            }
            TerrainGeometryOutcome::NoData => {
                self.upsample_or_no_data(key, frame_number);
            }
            TerrainGeometryOutcome::Transient(message) => {
                // Transient IO failure: cool down and retry. NEVER stamped
                // as permanent no-data (cesiumrust pitfall checkpoint).
                log::warn!(
                    "terrain tile {key:?} transient failure: {message}; \
                     retrying after cooldown"
                );
                self.terrain_tiles.insert(
                    key,
                    TileTerrainEntry {
                        state: TileTerrainState::Transient,
                        data: None,
                        upsampled_from: None,
                        retry_after_frame: frame_number + TERRAIN_RETRY_COOLDOWN_FRAMES,
                    },
                );
            }
        }
    }

    /// Creates the terrain mesh for arrived data (mirrors CesiumJS
    /// `processTile` → `createMesh`).
    fn process_tile_data(
        &mut self,
        key: (i32, i32, i32),
        mut data: TerrainTileData,
        frame_number: u64,
    ) {
        let (level, x, y) = key;
        let scheme = self.terrain_tiling_scheme.as_deref();
        let ready = match (&mut data, scheme) {
            (TerrainTileData::Heightmap(heightmap), Some(scheme)) => {
                // throttle = false: synchronous in-frame creation (the
                // returned ready future only releases the worker permit).
                heightmap.create_mesh(HeightmapCreateMeshOptions {
                    tiling_scheme: scheme,
                    x,
                    y,
                    level,
                    exaggeration: None,
                    exaggeration_relative_height: None,
                    throttle: Some(false),
                });
                heightmap.mesh().is_some()
            }
            (TerrainTileData::QuantizedMesh(_), _) => {
                // DEVIATION (B4-5): `QuantizedMeshTerrainData.createMesh`
                // still stubs the web-worker port in cesium-core; keep such
                // tiles in the transient placeholder class (ellipsoid grid
                // fallback, retried under cooldown) — never permanent
                // no-data — until the worker lands.
                false
            }
            (_, None) => false,
        };
        if ready {
            self.terrain_tiles.insert(
                key,
                TileTerrainEntry {
                    state: TileTerrainState::Ready,
                    data: Some(data),
                    upsampled_from: None,
                    retry_after_frame: 0,
                },
            );
        } else {
            self.terrain_tiles.insert(
                key,
                TileTerrainEntry {
                    state: TileTerrainState::Transient,
                    data: None,
                    upsampled_from: None,
                    retry_after_frame: frame_number + TERRAIN_RETRY_COOLDOWN_FRAMES,
                },
            );
        }
    }

    /// Deterministic no-data inheritance (mirrors CesiumJS `upsample`
    /// chaining through `tile.upsampledFrom`): the child receives an
    /// upsampled copy of the nearest ancestor with a mesh, so the mesh and
    /// the (ancestor-inherited) imagery texture swap together in one frame.
    fn upsample_or_no_data(&mut self, key: (i32, i32, i32), frame_number: u64) {
        let (level, x, y) = key;
        if level == 0 {
            // No ancestor to inherit from: permanent deterministic no-data.
            self.terrain_tiles.insert(
                key,
                TileTerrainEntry {
                    state: TileTerrainState::NoData,
                    data: None,
                    upsampled_from: None,
                    retry_after_frame: 0,
                },
            );
            return;
        }

        let parent_key = (level - 1, x >> 1, y >> 1);
        self.ensure_terrain(parent_key, frame_number);

        // Take the parent entry out so the upsampled child can be inserted
        // without conflicting borrows on `terrain_tiles`.
        let mut parent_entry = self.terrain_tiles.remove(&parent_key);
        let scheme = self.terrain_tiling_scheme.as_deref();
        let mut upsampled: Option<TerrainTileData> = None;
        if let (Some(parent), Some(scheme)) = (parent_entry.as_mut(), scheme) {
            if parent.state == TileTerrainState::Ready {
                upsampled = match parent.data.as_mut() {
                    Some(TerrainTileData::Heightmap(heightmap)) => heightmap
                        .upsample(
                            Some(scheme),
                            Some(parent_key.1),
                            Some(parent_key.2),
                            Some(parent_key.0),
                            Some(x),
                            Some(y),
                            Some(level),
                        )
                        .map(TerrainTileData::Heightmap),
                    // Quantized-mesh upsample is still a cesium-core stub.
                    Some(TerrainTileData::QuantizedMesh(_)) | None => None,
                };
            }
        }
        if let Some(parent) = parent_entry {
            self.terrain_tiles.insert(parent_key, parent);
        }

        match upsampled {
            Some(mut child) => {
                // Tessellate the upsampled heightmap (same frame: the mesh +
                // inherited imagery appear together — all-or-nothing).
                if let (TerrainTileData::Heightmap(heightmap), Some(scheme)) =
                    (&mut child, self.terrain_tiling_scheme.as_deref())
                {
                    heightmap.create_mesh(HeightmapCreateMeshOptions {
                        tiling_scheme: scheme,
                        x,
                        y,
                        level,
                        exaggeration: None,
                        exaggeration_relative_height: None,
                        throttle: Some(false),
                    });
                }
                self.terrain_tiles.insert(
                    key,
                    TileTerrainEntry {
                        state: TileTerrainState::Ready,
                        data: Some(child),
                        upsampled_from: Some(parent_key),
                        retry_after_frame: 0,
                    },
                );
            }
            None => {
                // Parent unusable: if it is still transient the child waits
                // too (retry, never permanent); otherwise inherit the
                // deterministic no-data.
                let parent_state = self
                    .terrain_tiles
                    .get(&parent_key)
                    .map(|entry| entry.state);
                let entry = if parent_state == Some(TileTerrainState::Transient) {
                    TileTerrainEntry {
                        state: TileTerrainState::Transient,
                        data: None,
                        upsampled_from: None,
                        retry_after_frame: frame_number + TERRAIN_RETRY_COOLDOWN_FRAMES,
                    }
                } else {
                    TileTerrainEntry {
                        state: TileTerrainState::NoData,
                        data: None,
                        upsampled_from: None,
                        retry_after_frame: 0,
                    }
                };
                self.terrain_tiles.insert(key, entry);
            }
        }
    }

    // ---- Resource management (B4-3) ----

    /// Returns (creating on first use) the ellipsoid grid geometry for the
    /// tile's own rectangle.
    fn ensure_tile_geometry(
        &mut self,
        tile: &QuadtreeTile,
        ellipsoid: &Ellipsoid,
        context: &Context,
    ) -> Arc<TileGeometryResources> {
        let key = (tile.level, tile.x, tile.y);
        if let Some(existing) = self.geometry_cache.get(&key) {
            return existing.clone();
        }
        let geometry =
            create_ellipsoid_grid(&tile.rectangle, ellipsoid, crate::globe_tile_geometry::DEFAULT_GRID_SEGMENTS);
        let resources = upload_grid_geometry(
            &geometry.positions,
            &geometry.texture_coordinates,
            &geometry.indices,
            context,
        );
        self.geometry_cache.insert(key, resources.clone());
        resources
    }

    /// Returns (creating on first use) the GPU geometry for the tile's
    /// terrain mesh (B4-5). Falls back to the ellipsoid grid when the mesh
    /// is unexpectedly absent (defensive: state said Ready).
    fn ensure_terrain_geometry(
        &mut self,
        tile: &QuadtreeTile,
        ellipsoid: &Ellipsoid,
        context: &Context,
    ) -> Arc<TileGeometryResources> {
        let key = (tile.level, tile.x, tile.y);
        if let Some(existing) = self.terrain_geometry_cache.get(&key) {
            return existing.clone();
        }
        let decoded = self
            .terrain_tiles
            .get(&key)
            .and_then(|entry| entry.data.as_ref())
            .and_then(|data| match data {
                TerrainTileData::Heightmap(heightmap) => {
                    heightmap.mesh().map(decode_terrain_mesh)
                }
                TerrainTileData::QuantizedMesh(quantized) => {
                    quantized.mesh().map(decode_terrain_mesh)
                }
            });
        let Some((positions, texture_coordinates, indices)) = decoded else {
            return self.ensure_tile_geometry(tile, ellipsoid, context);
        };
        let resources =
            upload_grid_geometry(&positions, &texture_coordinates, &indices, context);
        self.terrain_geometry_cache.insert(key, resources.clone());
        resources
    }

    /// Resolves the day texture for a tile.
    ///
    /// failed/placeholder discipline (cesiumrust pitfall checkpoint):
    /// - deterministic `NoData` (file absent / beyond the provider's maximum
    ///   level) → the imagery request level cap is lowered step by step
    ///   (ancestor imagery inheritance, mirroring CesiumJS
    ///   `TileImagery.usingAncestorTexture`) until some level yields data;
    ///   the inherited texture is cached permanently for this tile.
    /// - `Transient` (IO error) → nothing is cached; the base color is shown
    ///   for this frame only and the request is retried next frame. It is
    ///   never stamped as permanent no-data.
    ///
    /// Returns `(texture, using_ancestor_imagery, cacheable)`.
    fn resolve_tile_texture(
        &mut self,
        tile: &QuadtreeTile,
        layers: &ImageryLayerCollection,
        context: &mut Context,
    ) -> (Arc<Texture>, bool, bool) {
        let key = (tile.level, tile.x, tile.y);
        if let Some(existing) = self.tile_resources.get(&key) {
            return (
                existing.texture.clone(),
                existing.using_ancestor_texture,
                true,
            );
        }

        let mut cap = tile.level;
        loop {
            match compose_tile_imagery(tile, layers, cap) {
                ComposeOutcome::Data(pixels, width, height) => {
                    let texture = upload_tile_texture(pixels, width, height, context);
                    return (texture, cap < tile.level, true);
                }
                ComposeOutcome::NoData => {
                    if cap > 0 {
                        // Inherit ancestor imagery: retry one level up.
                        cap -= 1;
                        continue;
                    }
                    // No imagery at any level: permanent base-color result.
                    return (self.ensure_base_color_texture(context), false, true);
                }
                ComposeOutcome::Transient => {
                    // Retry next frame; show the base color meanwhile. Never
                    // cache this outcome (no permanent no-data stamping).
                    return (self.ensure_base_color_texture(context), false, false);
                }
            }
        }
    }

    /// Returns (creating on first use) the 1×1 base-color texture.
    fn ensure_base_color_texture(&mut self, context: &mut Context) -> Arc<Texture> {
        if self.base_color_texture.is_none() {
            let color = &self.base_color;
            let pixels = vec![
                (color.red * 255.0) as u8,
                (color.green * 255.0) as u8,
                (color.blue * 255.0) as u8,
                (color.alpha * 255.0) as u8,
            ];
            let texture = context.create_texture(TextureOptions {
                source: Some(TextureSource {
                    width: 1,
                    height: 1,
                    array_buffer_view: pixels.clone(),
                }),
                pixel_format: PixelFormat::Rgba,
                flip_y: false,
                ..Default::default()
            });
            texture.upload_source(
                context.queue(),
                &TextureSource {
                    width: 1,
                    height: 1,
                    array_buffer_view: pixels,
                },
            );
            self.base_color_texture = Some(Arc::new(texture));
        }
        self.base_color_texture.clone().unwrap()
    }

    /// Evicts the oldest tile resources beyond `tile_cache_size`.
    fn evict_tile_resources(&mut self) {
        let limit = (self.tile_cache_size.max(1)) as usize;
        while self.tile_resources.len() > limit {
            if let Some(oldest) = self.tile_resource_order.pop_front() {
                self.tile_resources.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

/// Decodes a [`cesium_core::terrain_mesh::TerrainMesh`] into the globe's
/// CPU geometry streams.
///
/// DEVIATION (B4-5): CesiumJS decodes the RTC-relative, optionally
/// quantized vertices on the GPU (`getPosition` codegen +
/// `u_center3D`); the port expands them on the CPU into absolute
/// `position3DAndHeight` vertices so the TEXONLY globe shader needs no
/// terrain-specific path (pipeline-key convergence).
///
/// UV v-flip — the geometry-side single decision point (cesiumrust pitfall
/// checkpoint): terrain mesh UVs follow the CesiumJS convention `v = 0 at
/// NORTH` (paired with WebGL's `UNPACK_FLIP_Y_WEBGL`), while this
/// pipeline's day textures are row-flipped at upload so that `v = 0`
/// samples SOUTH. The one `1.0 - v` here reconciles the two; no other site
/// in the terrain path may flip UVs.
fn decode_terrain_mesh(
    mesh: &cesium_core::terrain_mesh::TerrainMesh,
) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
    let stride = mesh.stride;
    let vertex_count = mesh.vertices.len() / stride;
    let mut positions: Vec<f32> = Vec::with_capacity(vertex_count * 4);
    let mut texture_coordinates: Vec<f32> = Vec::with_capacity(vertex_count * 4);
    for index in 0..vertex_count {
        let base = index * stride;
        positions.push(mesh.center.x as f32 + mesh.vertices[base]);
        positions.push(mesh.center.y as f32 + mesh.vertices[base + 1]);
        positions.push(mesh.center.z as f32 + mesh.vertices[base + 2]);
        positions.push(mesh.vertices[base + 3]); // height in the w slot
        texture_coordinates.push(mesh.vertices[base + 4]); // u
        texture_coordinates.push(1.0 - mesh.vertices[base + 5]); // v flip
        texture_coordinates.push(0.0);
        texture_coordinates.push(0.0);
    }
    (positions, texture_coordinates, mesh.indices.clone())
}

/// Uploads position/uv/index streams as the tile's GPU geometry.
///
/// DEVIATION: CesiumJS interleaves all vertex attributes in one buffer;
/// `Buffer` is move-only in this port, so the two attributes use two vertex
/// buffers (same layout-hash semantics as ViewportQuad).
fn upload_grid_geometry(
    positions: &[f32],
    texture_coordinates: &[f32],
    indices: &[u32],
    context: &Context,
) -> Arc<TileGeometryResources> {
    let to_bytes = |values: &[f32]| -> Vec<u8> {
        values.iter().flat_map(|value| value.to_le_bytes()).collect()
    };
    let position_buffer = context.create_vertex_buffer(
        Some(&to_bytes(positions)),
        None,
        BufferUsage::StaticDraw,
    );
    let texture_coordinate_buffer = context.create_vertex_buffer(
        Some(&to_bytes(texture_coordinates)),
        None,
        BufferUsage::StaticDraw,
    );
    let index_bytes: Vec<u8> = indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect();
    let index_buffer = context.create_index_buffer(
        Some(&index_bytes),
        None,
        BufferUsage::StaticDraw,
        IndexDatatype::UnsignedInt,
    );

    let attributes = vec![
        VertexAttribute {
            index: 0,
            buffer: position_buffer,
            components_per_attribute: 4,
            component_datatype: wgpu::VertexFormat::Float32x4,
            normalize: false,
            stride_in_bytes: 16,
            offset_in_bytes: 0,
        },
        VertexAttribute {
            index: 1,
            buffer: texture_coordinate_buffer,
            components_per_attribute: 4,
            component_datatype: wgpu::VertexFormat::Float32x4,
            normalize: false,
            stride_in_bytes: 16,
            offset_in_bytes: 0,
        },
    ];
    let vertex_array = Arc::new(VertexArray::new(attributes, Some(index_buffer)));
    Arc::new(TileGeometryResources {
        vertex_array,
        index_count: indices.len() as u32,
    })
}

/// Uploads the composed RGBA8 tile image as the day texture.
///
/// UV v-flip — THE single decision point (cesiumrust pitfall checkpoint):
/// wgpu texture space has its origin at the TOP-left (v = 0 is row 0),
/// whereas the globe mesh UVs follow the WebGL convention (v = 0 at the
/// SOUTH edge). The composed `pixels` are in image-space rows (row 0 =
/// NORTH, the imagery XYZ convention shared by CesiumJS imagery tiles).
/// Row-flipping here once makes `v = 0` sample the south edge, exactly like
/// CesiumJS's `UNPACK_FLIP_Y_WEBGL` texture upload. No other site in the
/// globe pipeline may flip UVs or rows.
fn upload_tile_texture(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    context: &mut Context,
) -> Arc<Texture> {
    let row_bytes = (width * 4) as usize;
    let mut flipped = vec![0u8; pixels.len()];
    for row in 0..height as usize {
        let source = ((height as usize - 1 - row) * row_bytes)..((height as usize - row) * row_bytes);
        let target = row * row_bytes..(row + 1) * row_bytes;
        flipped[target].copy_from_slice(&pixels[source]);
    }

    let texture = context.create_texture(TextureOptions {
        source: Some(TextureSource {
            width,
            height,
            array_buffer_view: flipped.clone(),
        }),
        pixel_format: PixelFormat::Rgba,
        // The flip is handled above at the single decision point.
        flip_y: false,
        ..Default::default()
    });
    texture.upload_source(
        context.queue(),
        &TextureSource {
            width,
            height,
            array_buffer_view: flipped,
        },
    );
    Arc::new(texture)
}

/// Composites the imagery layers for `tile` bottom-up into one RGBA8 image.
///
/// Mirrors CesiumJS `GlobeSurfaceTileProvider.createTileImagerySkeleton`
/// semantics simplified for the TEXONLY path: every shown layer with data
/// for the tile is alpha-blended over a transparent canvas at the base
/// layer's tile resolution. DEVIATION: CesiumJS reprojects per-layer
/// textures on the GPU; this batch samples decoded layer images on the CPU
/// (nearest) into the geographic tile grid.
fn compose_tile_imagery(
    tile: &QuadtreeTile,
    layers: &ImageryLayerCollection,
    level_cap: i32,
) -> ComposeOutcome {
    // Target resolution: the first visible layer's tile size (CesiumJS uses
    // the imagery layer's texture dimensions), defaulting to 256×256.
    let (mut width, mut height) = (256u32, 256u32);
    for index in 0..layers.length() {
        if let Some(layer) = layers.get(index) {
            if layer.show {
                if let Some(provider) = layer.provider() {
                    width = provider.tile_width().max(1);
                    height = provider.tile_height().max(1);
                }
                break;
            }
        }
    }

    // Transparent canvas (RGBA premultiplied-free; straight alpha blend).
    let mut canvas = vec![0u8; (width * height * 4) as usize];
    let mut any_data = false;
    let mut transient = false;

    for index in 0..layers.length() {
        let layer = match layers.get(index) {
            Some(layer) if layer.show => layer,
            _ => continue,
        };
        let Some(provider) = layer.provider() else {
            continue;
        };
        if !provider.is_ready() {
            continue;
        }

        // Clamp the request into the provider's level range and into the
        // ancestor-inheritance cap (CesiumJS picks the nearest available
        // level when the tile's own level has no imagery).
        let minimum = provider.minimum_level().unwrap_or(0);
        let maximum = provider.maximum_level();
        let request_level = (tile.level.max(minimum as i32).min(level_cap) as u32)
            .min(maximum.unwrap_or(u32::MAX));

        // Map the tile rectangle center into the provider's own tiling grid
        // at the request level (geographic 2×1 root scheme; the CPU
        // reprojection point for WebMercator providers — DEVIATION B4-4).
        let columns = 2u32 << request_level;
        let rows = 1u32 << request_level;
        let center = cesium_core::rectangle::Rectangle::center(&tile.rectangle);
        let u = ((center.longitude + std::f64::consts::PI)
            / (2.0 * std::f64::consts::PI))
            .clamp(0.0, 1.0);
        let v = ((std::f64::consts::FRAC_PI_2 - center.latitude)
            / std::f64::consts::PI)
            .clamp(0.0, 1.0);
        let provider_x = ((u * columns as f64) as u32).min(columns - 1);
        let provider_y = ((v * rows as f64) as u32).min(rows - 1);

        match provider.request_tile_image_availability(provider_x, provider_y, request_level) {
            TileImageAvailability::Data(encoded) => {
                let Some((image_pixels, image_width, image_height)) = decode_rgba(&encoded)
                else {
                    // Corrupt/undecodable image: treat as transient so the
                    // tile is retried rather than permanently blacked out.
                    transient = true;
                    continue;
                };
                blend_layer(
                    &mut canvas,
                    width,
                    height,
                    &image_pixels,
                    image_width,
                    image_height,
                    layer.alpha,
                );
                any_data = true;
            }
            TileImageAvailability::NoData => {
                // Deterministic absence for THIS tile: the whole composition
                // yields to ancestor inheritance.
                return ComposeOutcome::NoData;
            }
            TileImageAvailability::Transient => {
                transient = true;
            }
        }
    }

    if transient && !any_data {
        ComposeOutcome::Transient
    } else if any_data {
        ComposeOutcome::Data(canvas, width, height)
    } else {
        // No imagery layers at all: deterministic no-data (the globe shows
        // the base color / ancestor texture).
        ComposeOutcome::NoData
    }
}

/// Decodes an encoded image (PNG/JPEG/…) into straight-alpha RGBA8.
fn decode_rgba(encoded: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let image = image::load_from_memory(encoded).ok()?;
    let rgba = image.into_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    Some((rgba.into_raw(), width, height))
}

/// Alpha-blends `source` over `destination` (straight alpha, source-over),
/// sampling the source with nearest filtering into the destination grid.
fn blend_layer(
    destination: &mut [u8],
    destination_width: u32,
    destination_height: u32,
    source: &[u8],
    source_width: u32,
    source_height: u32,
    layer_alpha: f64,
) {
    for row in 0..destination_height {
        let source_row =
            ((row as f64 + 0.5) / destination_height as f64 * source_height as f64) as u32;
        let source_row = source_row.min(source_height - 1);
        for col in 0..destination_width {
            let source_col =
                ((col as f64 + 0.5) / destination_width as f64 * source_width as f64) as u32;
            let source_col = source_col.min(source_width - 1);
            let source_offset = ((source_row * source_width + source_col) * 4) as usize;
            let s = &source[source_offset..source_offset + 4];
            let alpha_source = (s[3] as f64 / 255.0) * layer_alpha.clamp(0.0, 1.0);
            if alpha_source <= 0.0 {
                continue;
            }
            let destination_offset = ((row * destination_width + col) * 4) as usize;
            let d = &mut destination[destination_offset..destination_offset + 4];
            let alpha_destination = d[3] as f64 / 255.0;
            let alpha_out = alpha_source + alpha_destination * (1.0 - alpha_source);
            if alpha_out <= 0.0 {
                continue;
            }
            for channel in 0..3 {
                let blended = (s[channel] as f64 * alpha_source
                    + d[channel] as f64 * alpha_destination * (1.0 - alpha_source))
                    / alpha_out;
                d[channel] = blended.round().clamp(0.0, 255.0) as u8;
            }
            d[3] = (alpha_out * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

impl Default for GlobeSurfaceTileProvider {
    fn default() -> Self { Self::new() }
}
