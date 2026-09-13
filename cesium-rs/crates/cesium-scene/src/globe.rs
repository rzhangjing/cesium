//! Ported from `packages/engine/Source/Scene/Globe.js`.
//!
//! The globe rendered in the scene, including its terrain and imagery layers.

use std::cmp::Ordering;
use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::Event;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::map_projection::MapProjection;
use cesium_core::math::js_min;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::ray::Ray;
use cesium_core::rectangle::Rectangle;

use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::DrawCommand;
use cesium_renderer::framebuffer::Framebuffer;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_core::index_datatype::IndexDatatype;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::pass::Pass;
use cesium_shaders::wgsl;

use crate::frame_state::FrameState;
use crate::globe_surface_shader_set::GlobeSurfaceShaderSet;
use crate::globe_surface_tile_provider::GlobeSurfaceTileProvider;
use crate::globe_terrain_fetcher::GlobeTerrainFetcher;
use crate::globe_translucency::GlobeTranslucency;
use crate::imagery_layer_collection::ImageryLayerCollection;
use crate::quadtree_primitive::QuadtreePrimitive;
use crate::quadtree_tile::{
    QuadtreeTile, TileKey, CHILD_NORTHEAST, CHILD_NORTHWEST, CHILD_SOUTHEAST, CHILD_SOUTHWEST,
};
use crate::scene_mode::SceneMode;
use crate::shadow_mode::ShadowMode;

/// The globe rendered in the scene, including its terrain and imagery layers.
/// Access the globe using `Scene::globe`.
pub struct Globe {
    // ---- Core references ----
    ellipsoid: Ellipsoid,
    imagery_layer_collection: ImageryLayerCollection,
    surface_shader_set: GlobeSurfaceShaderSet,
    surface: QuadtreePrimitive,
    surface_tile_provider: GlobeSurfaceTileProvider,
    terrain_provider_changed: Event,
    translucency: GlobeTranslucency,

    // ---- Visual properties ----
    underground_color: Color,
    underground_color_alpha_by_distance: NearFarScalar,

    // ---- Public properties (mirroring CesiumJS public fields) ----
    /// Determines if the globe will be shown.
    pub show: bool,
    /// The maximum screen-space error used to drive level-of-detail refinement.
    pub maximum_screen_space_error: f64,
    /// The size of the terrain tile cache.
    pub tile_cache_size: i32,
    /// Number of loading descendant tiles considered "too many".
    pub loading_descendant_limit: i32,
    /// Whether ancestors of rendered tiles should be preloaded.
    pub preload_ancestors: bool,
    /// Whether siblings of rendered tiles should be preloaded.
    pub preload_siblings: bool,
    /// Enable lighting the globe with the scene's light source.
    pub enable_lighting: bool,
    /// A multiplier to adjust terrain lambert lighting.
    pub lambert_diffuse_multiplier: f64,
    /// Enable dynamic lighting effects on atmosphere and fog.
    pub dynamic_atmosphere_lighting: bool,
    /// Whether dynamic atmosphere lighting uses the sun direction.
    pub dynamic_atmosphere_lighting_from_sun: bool,
    /// Enable the ground atmosphere.
    pub show_ground_atmosphere: bool,
    /// The intensity of the light for computing ground atmosphere color.
    pub atmosphere_light_intensity: f64,
    /// Rayleigh scattering coefficient for ground atmosphere.
    pub atmosphere_rayleigh_coefficient: Cartesian3,
    /// Mie scattering coefficient for ground atmosphere.
    pub atmosphere_mie_coefficient: Cartesian3,
    /// Rayleigh scale height in meters.
    pub atmosphere_rayleigh_scale_height: f64,
    /// Mie scale height in meters.
    pub atmosphere_mie_scale_height: f64,
    /// Anisotropy of the medium for Mie scattering.
    pub atmosphere_mie_anisotropy: f64,
    /// Hue shift for atmosphere.
    pub atmosphere_hue_shift: f64,
    /// Saturation shift for atmosphere.
    pub atmosphere_saturation_shift: f64,
    /// Brightness shift for atmosphere.
    pub atmosphere_brightness_shift: f64,
    /// The color to highlight terrain fill tiles.
    pub fill_highlight_color: Option<Color>,
    /// Distance at which lighting fades out.
    pub lighting_fade_out_distance: f64,
    /// Distance at which lighting fades in.
    pub lighting_fade_in_distance: f64,
    /// Distance at which night lighting fades out.
    pub night_fade_out_distance: f64,
    /// Distance at which night lighting fades in.
    pub night_fade_in_distance: f64,
    /// Whether the water effect is shown.
    pub show_water_effect: bool,
    /// Whether to show terrain skirts.
    pub show_skirts: bool,
    /// Whether back face culling is enabled.
    pub back_face_culling: bool,
    /// Vertex shadow darkness.
    pub vertex_shadow_darkness: f64,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the globe has been destroyed.
    is_destroyed: bool,

    // ---- Atmosphere (lazy GPU resources) ----
    /// Atmosphere shader program (Fresnel-based glow).
    atmosphere_shader: Option<Arc<ShaderProgram>>,
    /// Atmosphere sphere vertex array.
    atmosphere_vertex_array: Option<Arc<VertexArray>>,
    /// Atmosphere sphere index buffer vertex count.
    atmosphere_index_count: u32,
}

impl Globe {
    /// Creates a new Globe.
    pub fn new(ellipsoid: Option<Ellipsoid>) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        // TODO (B4-5): the terrain provider (Robin's cesium-core
        // cesium_terrain_provider/custom_heightmap work) plugs in here once
        // available; the ellipsoid terrain mesh generator is wired through
        // `globe_tile_geometry` for now.
        let imagery_layer_collection = ImageryLayerCollection::new();
        let surface_shader_set = GlobeSurfaceShaderSet::new();
        let surface_tile_provider = GlobeSurfaceTileProvider::new();
        let surface = QuadtreePrimitive::new();
        let translucency = GlobeTranslucency::new();

        let max_radius = ellipsoid.maximum_radius();

        Self {
            ellipsoid,
            imagery_layer_collection,
            surface_shader_set,
            surface,
            surface_tile_provider,
            terrain_provider_changed: Event::new(),
            translucency,
            underground_color: Color::new(0.0, 0.0, 0.0, 1.0),
            underground_color_alpha_by_distance: NearFarScalar::new(
                max_radius / 1000.0,
                0.0,
                max_radius / 5.0,
                1.0,
            ),
            show: true,
            maximum_screen_space_error: 2.0,
            tile_cache_size: 100,
            loading_descendant_limit: 20,
            preload_ancestors: true,
            preload_siblings: false,
            enable_lighting: false,
            lambert_diffuse_multiplier: 0.9,
            dynamic_atmosphere_lighting: true,
            dynamic_atmosphere_lighting_from_sun: false,
            show_ground_atmosphere: ellipsoid == Ellipsoid::WGS84,
            atmosphere_light_intensity: 10.0,
            atmosphere_rayleigh_coefficient: Cartesian3::new(5.5e-6, 13.0e-6, 28.4e-6),
            atmosphere_mie_coefficient: Cartesian3::new(21e-6, 21e-6, 21e-6),
            atmosphere_rayleigh_scale_height: 10000.0,
            atmosphere_mie_scale_height: 3200.0,
            atmosphere_mie_anisotropy: 0.999,
            atmosphere_hue_shift: 0.0,
            atmosphere_saturation_shift: 0.0,
            atmosphere_brightness_shift: 0.0,
            fill_highlight_color: None,
            lighting_fade_out_distance: 1.0e7,
            lighting_fade_in_distance: 1.0e7,
            night_fade_out_distance: 1.0e7,
            night_fade_in_distance: 1.0e7,
            show_water_effect: true,
            show_skirts: true,
            back_face_culling: true,
            vertex_shadow_darkness: 0.6,
            shadows: ShadowMode::Disabled,
            is_destroyed: false,
            atmosphere_shader: None,
            atmosphere_vertex_array: None,
            atmosphere_index_count: 0,
        }
    }

    // ---- Getters ----

    /// Gets the ellipsoid describing the shape of this globe.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the collection of image layers rendered on this globe.
    pub fn imagery_layers(&self) -> &ImageryLayerCollection {
        &self.imagery_layer_collection
    }

    /// Gets a mutable reference to the collection of image layers rendered
    /// on this globe.
    pub fn imagery_layers_mut(&mut self) -> &mut ImageryLayerCollection {
        &mut self.imagery_layer_collection
    }

    /// Diagnostic hook: the quadtree surface primitive (traversal results,
    /// SSE bookkeeping). Used by the globe smoke tests to assert the LOD
    /// invariants.
    pub fn surface(&self) -> &QuadtreePrimitive {
        &self.surface
    }

    /// Diagnostic hook: the surface tile provider (terrain tile states,
    /// upsample bookkeeping). Used by the terrain smoke tests.
    pub fn surface_tile_provider(&self) -> &GlobeSurfaceTileProvider {
        &self.surface_tile_provider
    }

    /// Returns `true` when the tile load queue is empty, `false` otherwise.
    /// When the load queue is empty, all terrain and imagery for the current
    /// view have been loaded.
    ///
    /// Mirrors the CesiumJS `Globe#tilesLoaded` getter (Globe.js L422-433),
    /// which ANDs the three QuadtreePrimitive queues. `Scene#render` uses it
    /// to decide whether another frame has to be requested.
    ///
    /// DEVIATION: CesiumJS returns `true` when `this._surface` is `undefined`;
    /// the port always owns a surface, so the guard has no counterpart.
    pub fn tiles_loaded(&self) -> bool {
        self.surface.tiles_loaded()
    }

    /// Installs (or clears) the terrain fetcher (B4-5). When `None`, the
    /// globe renders the ellipsoid terrain grid (CesiumJS's placeholder
    /// while no terrain provider is installed).
    pub fn set_terrain_fetcher(&mut self, fetcher: Option<Box<dyn GlobeTerrainFetcher>>) {
        self.surface_tile_provider.set_terrain_fetcher(fetcher);
        self.terrain_provider_changed.raise_event(&());
    }

    /// The installed terrain fetcher, if any.
    pub fn terrain_fetcher(&self) -> Option<&dyn GlobeTerrainFetcher> {
        self.surface_tile_provider.terrain_fetcher()
    }

    /// Gets the event raised when the terrain provider is changed.
    pub fn terrain_provider_changed(&self) -> &Event {
        &self.terrain_provider_changed
    }

    /// Gets the globe translucency properties.
    pub fn translucency(&self) -> &GlobeTranslucency {
        &self.translucency
    }

    /// Gets or sets the underground color.
    pub fn underground_color(&self) -> &Color {
        &self.underground_color
    }

    /// Sets the underground color.
    pub fn set_underground_color(&mut self, color: Color) {
        self.underground_color = color;
    }

    /// Gets the underground color alpha by distance.
    pub fn underground_color_alpha_by_distance(&self) -> &NearFarScalar {
        &self.underground_color_alpha_by_distance
    }

    /// Sets the underground color alpha by distance.
    pub fn set_underground_color_alpha_by_distance(&mut self, value: NearFarScalar) {
        debug_assert!(value.far >= value.near, "far distance must be greater than near distance");
        self.underground_color_alpha_by_distance = value;
    }

    // ---- Frame lifecycle ----

    /// Updates the globe for the current frame.
    pub fn update(&mut self, frame_state: &FrameState) {
        if !self.show {
            return;
        }
        if frame_state.passes.main {
            self.surface.update(frame_state);
        }
    }

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, frame_state: &FrameState) {
        if !frame_state.passes.main {
            return;
        }

        // Propagate globe properties to the surface tile provider
        self.surface.set_maximum_screen_space_error(self.maximum_screen_space_error);
        self.surface.set_tile_cache_size(self.tile_cache_size);
        self.surface.set_loading_descendant_limit(self.loading_descendant_limit);
        self.surface.set_preload_ancestors(self.preload_ancestors);
        self.surface.set_preload_siblings(self.preload_siblings);

        self.surface_tile_provider.set_enable_lighting(self.enable_lighting);
        self.surface_tile_provider.set_dynamic_atmosphere_lighting(self.dynamic_atmosphere_lighting);
        self.surface_tile_provider.set_show_ground_atmosphere(self.show_ground_atmosphere);
        self.surface_tile_provider.set_atmosphere_light_intensity(self.atmosphere_light_intensity);
        self.surface_tile_provider.set_shadows(self.shadows);
        self.surface_tile_provider.set_show_skirts(self.show_skirts);
        self.surface_tile_provider.set_back_face_culling(self.back_face_culling);
        self.surface_tile_provider.set_vertex_shadow_darkness(self.vertex_shadow_darkness);
        self.surface_tile_provider.set_underground_color(self.underground_color.clone());
        self.surface_tile_provider.set_lambert_diffuse_multiplier(self.lambert_diffuse_multiplier);
        self.surface_tile_provider.set_tile_cache_size(self.tile_cache_size);

        // Traversal ceiling: CesiumJS drives quadtree refinement through
        // `terrainProvider.getLevelMaximumGeometricError(level)` — imagery
        // providers contribute per-tile clamped `request_level` but never
        // cap the traversal itself (`QuadtreePrimitive` has no imagery-
        // derived maximum_level; see `Globe.js` / `GlobeSurfaceTileProvider
        // .getLevelMaximumGeometricError`). The port mirrors that: the
        // ceiling is the terrain fetcher's own maximumLevel, or `None`
        // (unbounded) when the terrain has no declared limit (e.g.
        // `EllipsoidTerrainProvider`). Each imagery provider independently
        // clamps out-of-range requests back to its own maximum level in
        // `compose_tile_imagery` (`request_level.min(maximum)`), so a
        // shallow base map simply reuses its deepest tile while deeper
        // layers still fill in.
        let maximum_level = self
            .terrain_fetcher()
            .and_then(|fetcher| fetcher.maximum_level())
            .map(|level| level as i32);
        self.surface.set_maximum_level(maximum_level);

        self.surface.begin_frame(frame_state);
    }

    /// Renders the globe: one terrain+imagery draw per selected quadtree
    /// tile into `framebuffer` (the globe offscreen pass with depth).
    ///
    /// DEVIATION (B4-3): CesiumJS issues the tile draw commands through the
    /// `frameState.commandList` inside `QuadtreePrimitive.render`; the wgpu
    /// port hands them to the collecting [`Context`] directly.
    pub fn render(
        &mut self,
        frame_state: &FrameState,
        context: &mut Context,
        framebuffer: Option<std::sync::Arc<Framebuffer>>,
    ) {
        if !self.show {
            return;
        }
        self.surface.render(frame_state);

        // NOTE: Atmosphere rendering is currently disabled because the
        // atmosphere sphere (1.02× globe) is closer to the camera than the
        // globe surface, causing it to occlude the tiles. A proper fix
        // requires either rendering the atmosphere into a separate pass
        // or integrating the Fresnel effect into the globe tile shader.
        // if self.show_ground_atmosphere {
        //     self.render_atmosphere(context, framebuffer.clone());
        // }

        // B4-5: drive the selected tiles' terrain toward Ready/NoData
        // (ancestors first) before the tile draws pick their geometry.
        let tiles = self.surface.tiles_to_render();
        self.surface_tile_provider
            .prepare_terrain(tiles, frame_state.frame_number);
        for tile in self.surface.tiles_to_render() {
            self.surface_tile_provider.render_tile(
                tile,
                &self.imagery_layer_collection,
                &self.ellipsoid,
                context,
                framebuffer.clone(),
                frame_state.frame_number,
            );
        }
    }

    /// Renders the atmospheric glow sphere into the globe framebuffer.
    fn render_atmosphere(
        &mut self,
        context: &mut Context,
        framebuffer: Option<Arc<Framebuffer>>,
    ) {
        // Lazily create the atmosphere shader program.
        if self.atmosphere_shader.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::ATMOSPHERE_SHADER,
                wgsl::ATMOSPHERE_SHADER,
                "globe_atmosphere".to_string(),
            ) {
                Ok(program) => self.atmosphere_shader = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("atmosphere shader compilation failed: {error}");
                    return;
                }
            }
        }

        // Lazily create the atmosphere sphere mesh (radius = 1.02 × max_radius).
        if self.atmosphere_vertex_array.is_none() {
            let radius = self.ellipsoid.maximum_radius() as f32 * 1.02;
            let (positions, indices) = create_sphere_mesh(radius, 48, 24);
            let to_bytes = |values: &[f32]| -> Vec<u8> {
                values.iter().flat_map(|v| v.to_le_bytes()).collect()
            };
            let position_buffer = context.create_vertex_buffer(
                Some(&to_bytes(&positions)),
                None,
                BufferUsage::StaticDraw,
            );
            let index_buffer = context.create_index_buffer(
                Some(&indices.iter().flat_map(|i| (*i as u32).to_le_bytes()).collect::<Vec<u8>>()),
                None,
                BufferUsage::StaticDraw,
                IndexDatatype::UnsignedInt,
            );
            let attributes = vec![VertexAttribute {
                index: 0,
                buffer: position_buffer,
                components_per_attribute: 4,
                component_datatype: wgpu::VertexFormat::Float32x4,
                normalize: false,
                stride_in_bytes: 16,
                offset_in_bytes: 0,
            }];
            self.atmosphere_index_count = indices.len() as u32;
            self.atmosphere_vertex_array =
                Some(Arc::new(VertexArray::new(attributes, Some(index_buffer))));
        }

        // Alpha blending, depth test on, depth writes off.
        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = true;
        render_state.depth_mask = false; // don't write depth
        render_state.blending.enabled = true;
        render_state.blending.equation_rgb = BlendEquation::FuncAdd;
        render_state.blending.equation_alpha = BlendEquation::FuncAdd;
        render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
        render_state.blending.function_source_alpha = BlendingFactor::One;
        render_state.blending.function_destination_rgb = BlendingFactor::OneMinusSrcAlpha;
        render_state.blending.function_destination_alpha = BlendingFactor::OneMinusSrcAlpha;
        // Disable culling so we see the inside of the sphere from any angle.
        render_state.cull.enabled = false;

        let mut command = DrawCommand::new();
        command.primitive_type = WebGLConstants::TRIANGLES;
        command.vertex_array = self.atmosphere_vertex_array.clone();
        command.count = Some(self.atmosphere_index_count);
        command.offset = 0;
        command.shader_program = self.atmosphere_shader.clone();
        command.render_state = render_state;
        command.framebuffer = framebuffer;
        command.pass = Some(Pass::Globe as u32);
        command.owner = Some("GlobeAtmosphere".to_string());

        context.draw(command);
    }

    /// Called at the end of each frame.
    pub fn end_frame(&mut self, frame_state: &FrameState) {
        if !self.show {
            return;
        }
        if frame_state.passes.main {
            self.surface.end_frame(frame_state);
        }
    }

    // ---- Picking ----

    /// Finds an intersection between a ray and the globe surface that was
    /// rendered. The ray must be given in world coordinates.
    ///
    /// Mirrors `Globe.prototype.pickWorldCoordinates`. The returned position
    /// is in projected coordinates for 2D and Columbus View.
    ///
    /// DEVIATION: the JS takes the `Scene` and reads only `scene.mode` and
    /// `scene.mapProjection`; the port takes those two directly because
    /// `Scene` owns the `Globe`, so a `&Scene` could not be handed to a
    /// `&mut self` method.
    pub fn pick_world_coordinates(
        &mut self,
        ray: &Ray,
        mode: SceneMode,
        projection: Option<&dyn MapProjection>,
        cull_back_faces: Option<bool>,
    ) -> Option<Cartesian3> {
        // `cullBackFaces = cullBackFaces ?? true`
        let cull_back_faces = cull_back_faces.unwrap_or(true);

        // `const sphereIntersections = scratchArray; sphereIntersections.length = 0;`
        //
        // The JS pushes the `GlobeSurfaceTile`s and reads their
        // `pickBoundingSphere` back in the sort comparator; the port carries
        // the sphere alongside the tile key.
        let mut sphere_intersections: Vec<(TileKey, BoundingSphere)> = Vec::new();

        // `for (const tile of this._surface._tilesRenderedThisFrame)`. Copied
        // out up front: `_tilesRenderedThisFrame` holds keys, not live tiles
        // (the tree is owned by `root_tiles`).
        let rendered = self.surface.tiles_rendered_this_frame().to_vec();
        let tiling_scheme = self.surface.tiling_scheme();
        for key in rendered {
            // `tile.rectangle`, recomputed the way `QuadtreeTile`'s
            // constructor does it.
            let mut rectangle = Rectangle::default();
            tiling_scheme.tile_xy_to_rectangle(key.x, key.y, key.level, &mut rectangle);

            // The `None` arm is the JS `continue` (3D with no rendered mesh).
            let Some(bounding_volume) = self.surface_tile_provider.compute_pick_bounding_sphere(
                key,
                &rectangle,
                mode,
                projection,
            ) else {
                continue;
            };

            // `IntersectionTests.raySphere(ray, boundingVolume,
            //   scratchSphereIntersectionResult)`
            if IntersectionTests::ray_sphere(ray, &bounding_volume).is_some() {
                sphere_intersections.push((key, bounding_volume));
            }
        }

        // `sphereIntersections.sort(createComparePickTileFunction(ray.origin))`,
        // whose comparator is `BoundingSphere.distanceSquaredTo(a.pickBoundingSphere,
        // rayOrigin) - BoundingSphere.distanceSquaredTo(b.pickBoundingSphere, rayOrigin)`.
        // Both `Array.prototype.sort` and `sort_by` are stable, so equal
        // distances keep the rendered-this-frame order.
        sphere_intersections.sort_by(|a, b| {
            let a_dist = BoundingSphere::distance_squared_to(&a.1, &ray.origin);
            let b_dist = BoundingSphere::distance_squared_to(&b.1, &ray.origin);
            // A NaN difference is treated as equal by `Array.prototype.sort`.
            a_dist.partial_cmp(&b_dist).unwrap_or(Ordering::Equal)
        });

        for (key, _) in &sphere_intersections {
            // `GlobeSurfaceTile.pick`: `if (!defined(this.renderedMesh))
            // return undefined;` then `this.renderedMesh.pick(ray,
            // cullBackFaces, mode, projection)`.
            let Some(mesh) = self.surface_tile_provider.rendered_mesh_mut(*key) else {
                continue;
            };
            // The `None` ellipsoid is the JS's `Ellipsoid.default`, which
            // `TerrainMesh.computeTransform` uses unconditionally.
            let intersection = mesh.pick(ray, cull_back_faces, Some(mode), projection, None);
            // `if (defined(intersection)) break;`
            if intersection.is_some() {
                return intersection;
            }
        }

        None
    }

    /// Finds an intersection between a ray and the globe surface that was
    /// rendered. The ray must be given in world coordinates.
    ///
    /// Mirrors `Globe.prototype.pick`; see the DEVIATION note on
    /// [`Globe::pick_world_coordinates`] about the `scene` argument.
    ///
    /// ```ignore
    /// // find intersection of ray through a pixel and the globe
    /// let ray = viewer.camera.get_pick_ray(window_coordinates);
    /// let intersection = globe.pick(&ray, mode, Some(projection));
    /// ```
    pub fn pick(
        &mut self,
        ray: &Ray,
        mode: SceneMode,
        projection: Option<&dyn MapProjection>,
    ) -> Option<Cartesian3> {
        // `result = this.pickWorldCoordinates(ray, scene, true, result);`
        let result = self.pick_world_coordinates(ray, mode, projection, Some(true))?;

        if mode == SceneMode::Scene3D {
            return Some(result);
        }

        // `Cartesian3.fromElements(result.y, result.z, result.x, result)` —
        // aliased in the JS, but every component is read before any is
        // written, so building a new value is equivalent.
        let projected = Cartesian3::from_elements_new(result.y, result.z, result.x);
        // `scene.mapProjection.unproject(result, cartoScratch)`. Unreachable
        // with `None`: the 2D/Columbus View branch above already needed the
        // projection to build the pick bounding spheres.
        let projection = projection
            .expect("mapProjection is required to pick the globe in 2D or Columbus View");
        let carto = projection.unproject(&projected);
        let mut cartesian = Cartesian3::default();
        self.ellipsoid.cartographic_to_cartesian(&carto, &mut cartesian);
        Some(cartesian)
    }

    /// Gets the height of the surface at a given cartographic.
    ///
    /// Mirrors `Globe.prototype.getHeight`; `None` covers every JS
    /// `return undefined`.
    pub fn get_height(&mut self, cartographic: &Cartographic) -> Option<f64> {
        // `this._surface._levelZeroTiles`. The JS returns early when it is
        // undefined; the port builds the level-zero tiles in the
        // `QuadtreePrimitive` constructor, so they always exist.
        let level_zero_tiles = self.surface.root_tiles();

        let mut index = 0;
        while index < level_zero_tiles.len()
            && !Rectangle::contains(&level_zero_tiles[index].rectangle, cartographic)
        {
            index += 1;
        }
        // JS: `if (i >= length) return undefined;` — the loop leaves `i` at
        // `length` when no tile matched.
        if index >= level_zero_tiles.len() {
            return None;
        }

        // Descend to the deepest tile with a rendered mesh. The JS reads the
        // *private* `_southwestChild`/... fields rather than the lazy public
        // getters, so it only follows children the quadtree traversal has
        // already materialised; `QuadtreeTile::children`, empty until
        // `ensure_children`, is the same thing.
        let mut tile_with_mesh = level_zero_tiles[index].key();
        let mut tile: Option<&QuadtreeTile> = Some(&level_zero_tiles[index]);
        while let Some(current) = tile {
            let next =
                tile_if_contains_cartographic(current.children.get(CHILD_SOUTHWEST), cartographic)
                    .or_else(|| {
                        tile_if_contains_cartographic(
                            current.children.get(CHILD_SOUTHEAST),
                            cartographic,
                        )
                    })
                    .or_else(|| {
                        tile_if_contains_cartographic(
                            current.children.get(CHILD_NORTHWEST),
                            cartographic,
                        )
                    })
                    // The JS's last operand is `tile._northeastChild` with no
                    // containment test.
                    .or_else(|| current.children.get(CHILD_NORTHEAST));
            tile = next;

            if let Some(next) = next {
                let key = next.key();
                // `defined(tile.data) && defined(tile.data.renderedMesh)`
                if self.surface_tile_provider.rendered_mesh(key).is_some() {
                    tile_with_mesh = key;
                }
            }
        }

        // "This tile was either rendered or culled." A culled tile is fine —
        // getting a height to place a billboard on terrain while the camera
        // looks at that billboard is the motivating case — but it must have a
        // valid mesh.
        let key = tile_with_mesh;
        if self.surface_tile_provider.rendered_mesh(key).is_none() {
            // Tile was not rendered (culled).
            return None;
        }

        let ellipsoid = self.surface.tiling_scheme().ellipsoid();

        // `cartesian` has to be on the ellipsoid surface for
        // `ellipsoid.geodeticSurfaceNormal`.
        let mut cartesian = Cartesian3::default();
        Cartesian3::from_radians(
            cartographic.longitude,
            cartographic.latitude,
            Some(0.0),
            Some(ellipsoid.radii_squared()),
            &mut cartesian,
        );

        let mut ray = Ray::default();
        // `ellipsoid.geodeticSurfaceNormal(cartesian, ray.direction)` — the
        // JS's `surfaceNormal` *is* `ray.direction`.
        ellipsoid.geodetic_surface_normal(&cartesian, &mut ray.direction);

        // Try to find the intersection point between the surface normal and
        // the z-axis. 11500.0 is the minimum height of the terrain set; the
        // JS notes this should come from the terrain provider.
        if !ellipsoid.get_surface_normal_intersection_with_z_axis(
            &cartesian,
            Some(11500.0),
            &mut ray.origin,
        ) {
            // Theoretically — not with Earth datums — the intersection point
            // can be outside the ellipsoid, so try another value.
            //
            // DEVIATION: the JS reads
            // `tile.data.tileBoundingRegion.minimumHeight`; the port has no
            // `tileBoundingRegion`, and `updateTileBoundingRegion` copies that
            // height straight off the mesh, so read the mesh instead.
            let minimum_height = self
                .surface_tile_provider
                .rendered_mesh(key)
                .map(|mesh| mesh.minimum_height);
            // `Math.min(minimumHeight ?? 0.0, -11500.0)` — `js_min`, not
            // `f64::min`, for the NaN rule.
            let magnitude = js_min(minimum_height.unwrap_or(0.0), -11500.0);

            // Multiply by the *positive* value of the magnitude.
            let mut vector_to_minimum_point = Cartesian3::default();
            Cartesian3::multiply_by_scalar(
                &ray.direction,
                magnitude.abs() + 1.0,
                &mut vector_to_minimum_point,
            );
            Cartesian3::subtract(&cartesian, &vector_to_minimum_point, &mut ray.origin);
        }

        // Globe height is the same at a given cartographic regardless of the
        // scene mode, but the ray is constructed via a surface normal (which
        // assumes 3D), so pick in 3D mode.
        //
        // DEVIATION: the JS passes `tilingScheme.projection`; `TerrainMesh`
        // never reads it on the 3D path — `computeTransform` uses
        // `Ellipsoid.default` and `getVertexPosition` returns before touching
        // the projection — so `None` is equivalent.
        let Some(mesh) = self.surface_tile_provider.rendered_mesh_mut(key) else {
            return None;
        };
        let intersection = mesh.pick(&ray, false, Some(SceneMode::Scene3D), None, None)?;

        let mut height_cartographic = Cartographic::default();
        if !ellipsoid.cartesian_to_cartographic(&intersection, &mut height_cartographic) {
            // The JS dereferences the `undefined` and throws; the port reports
            // "not found" (see `Ellipsoid::cartesian_to_cartographic`).
            return None;
        }
        Some(height_cartographic.height)
    }

    // ---- Lifecycle ----

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for Globe {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Mirrors the module-level `tileIfContainsCartographic(tile, cartographic)`
/// in `Globe.js`: `defined(tile) && Rectangle.contains(tile.rectangle,
/// cartographic) ? tile : undefined`.
fn tile_if_contains_cartographic<'a>(
    tile: Option<&'a QuadtreeTile>,
    cartographic: &Cartographic,
) -> Option<&'a QuadtreeTile> {
    match tile {
        Some(tile) if Rectangle::contains(&tile.rectangle, cartographic) => Some(tile),
        _ => None,
    }
}

/// Generates a UV sphere mesh with the given radius, sector count, and stack
/// count. Returns (positions as vec4 xyzw, indices as u16 triangles).
fn create_sphere_mesh(radius: f32, sectors: u32, stacks: u32) -> (Vec<f32>, Vec<u16>) {
    let mut positions = Vec::new();
    let mut indices = Vec::new();

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;
    let stack_step = std::f32::consts::PI / stacks as f32;

    // Generate vertices
    for i in 0..=stacks {
        let stack_angle = std::f32::consts::FRAC_PI_2 - i as f32 * stack_step;
        let xz = radius * stack_angle.cos();
        let y = radius * stack_angle.sin();

        for j in 0..=sectors {
            let sector_angle = j as f32 * sector_step;
            let x = xz * sector_angle.cos();
            let z = xz * sector_angle.sin();
            positions.push(x);
            positions.push(y);
            positions.push(z);
            positions.push(1.0);
        }
    }

    // Generate indices
    for i in 0..stacks {
        let k1 = i * (sectors + 1);
        let k2 = k1 + sectors + 1;
        for j in 0..sectors {
            if i != 0 {
                indices.push((k1 + j) as u16);
                indices.push((k2 + j) as u16);
                indices.push((k1 + j + 1) as u16);
            }
            if i != stacks - 1 {
                indices.push((k1 + j + 1) as u16);
                indices.push((k2 + j) as u16);
                indices.push((k2 + j + 1) as u16);
            }
        }
    }

    (positions, indices)
}
