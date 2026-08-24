//! Ported from `packages/engine/Source/Scene/PointPrimitiveCollection.js`.
//!
//! A collection of point primitives.
//!
//! M3/S3 materialization: the CesiumJS batch pipeline is ported — points are
//! CPU-expanded into screen-aligned pixel quads (the wgpu equivalent of the
//! JS `gl_PointSize` path) sharing the billboard WGSL pair and the shared
//! quad-batch upload, then issued as one [`DrawCommand`].
//!
//! DEVIATION: CesiumJS separates `pixelSize` and `scale` on
//! `PointPrimitive`; the cesium-rs `PointPrimitive` carries a single `scale`
//! field documented as the point size in pixels, which this collection uses
//! directly as the quad pixel size. Outline rendering (`outlineColor` /
//! `outlineWidth`) is accepted for API parity but not yet batched.

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::texture::Texture;
use cesium_shaders::wgsl;

use crate::billboard_collection::{
    build_billboard_batch, upload_quad_batch, ResolvedBillboard,
};
use crate::frame_state::FrameState;
use crate::point_primitive::PointPrimitive;
use crate::primitive_collection::ScenePrimitive;
use crate::texture_atlas::TextureAtlas;

/// GPU resources of the current point batch.
struct BatchResources {
    vertex_array: Arc<cesium_renderer::vertex_array::VertexArray>,
    index_count: u32,
    atlas_texture: Arc<Texture>,
    bounding_sphere: BoundingSphere,
}

/// A collection of point primitives for efficient rendering of many points.
///
/// Mirrors CesiumJS `PointPrimitiveCollection` (813 lines).
pub struct PointPrimitiveCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The point primitives in this collection.
    points: Vec<PointPrimitive>,
    /// A 1×1 white-texture atlas (points are flat colored; mirrors the JS
    /// collection owning a texture atlas).
    atlas: TextureAtlas,
    /// Whether the batch buffers are stale.
    dirty: bool,
    /// The batched GPU resources (rebuilt when dirty).
    batch: Option<BatchResources>,
    /// The billboard batch WGSL program (lazy).
    shader_program: Option<Arc<ShaderProgram>>,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

/// The atlas id of the flat white texel the point quads sample.
const WHITE_IMAGE_ID: &str = "__cesium_point_white";

impl PointPrimitiveCollection {
    /// Creates a new PointPrimitiveCollection.
    pub fn new() -> Self {
        Self {
            show: true,
            points: Vec::new(),
            atlas: TextureAtlas::new(),
            dirty: true,
            batch: None,
            shader_program: None,
            is_destroyed: false,
        }
    }

    /// Adds a point to the collection and returns its index (CesiumJS
    /// returns the point; the Rust port moves it in and returns the index).
    pub fn add(&mut self, point: PointPrimitive) -> usize {
        self.dirty = true;
        let index = self.points.len();
        self.points.push(point);
        index
    }

    /// Removes the point at the given index, returning whether it was
    /// present (mirrors the JS boolean `remove` contract).
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.points.len() {
            self.points.remove(index);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes all points from the collection (mirrors the JS `removeAll`).
    pub fn remove_all(&mut self) {
        self.points.clear();
        self.dirty = true;
    }

    /// Gets a point by index (mirrors the JS `get`).
    pub fn get(&self, index: usize) -> Option<&PointPrimitive> {
        self.points.get(index)
    }

    /// Gets a mutable reference to a point by index (marks the batch dirty).
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PointPrimitive> {
        if let Some(point) = self.points.get_mut(index) {
            self.dirty = true;
            Some(point)
        } else {
            None
        }
    }

    /// Returns the number of points (mirrors the JS `length` property).
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Updates the collection for the current frame.
    ///
    /// Mirrors CesiumJS `PointPrimitiveCollection#update`: rebuild the
    /// batch when dirty, then append the draw command to the frame's
    /// command list.
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show || self.points.is_empty() || !frame_state.passes.main {
            return;
        }

        if self.dirty || self.batch.is_none() {
            self.create_batch(context);
        }

        let (vertex_array, index_count, atlas_texture, bounding_sphere) = match self.batch.as_ref() {
            Some(batch) => (
                batch.vertex_array.clone(),
                batch.index_count,
                batch.atlas_texture.clone(),
                batch.bounding_sphere.clone(),
            ),
            None => return,
        };
        let shader_program = match self.shader_program.clone() {
            Some(program) => program,
            None => return,
        };

        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = true;
        render_state.depth_mask = false;
        render_state.blending.enabled = true;
        render_state.blending.equation_rgb = BlendEquation::FuncAdd;
        render_state.blending.equation_alpha = BlendEquation::FuncAdd;
        render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
        render_state.blending.function_source_alpha = BlendingFactor::One;
        render_state.blending.function_destination_rgb = BlendingFactor::OneMinusSrcAlpha;
        render_state.blending.function_destination_alpha = BlendingFactor::OneMinusSrcAlpha;
        render_state.cull.enabled = false;

        let mut command = DrawCommand::new();
        command.bounding_volume = Some(bounding_sphere);
        command.primitive_type = WebGLConstants::TRIANGLES;
        command.vertex_array = Some(vertex_array);
        command.count = Some(index_count);
        command.offset = 0;
        command.shader_program = Some(shader_program);
        command.uniform_overrides = vec![(
            "u_atlas".to_string(),
            UniformValue::Texture(atlas_texture),
        )];
        command.render_state = render_state;
        command.framebuffer = None;
        command.pass = Some(Pass::Translucent as u32);
        command.owner = Some("PointPrimitiveCollection".to_string());
        context.draw(command);
    }

    /// Resolves every shown point into a pixel quad and uploads the batch
    /// (mirrors the JS `_createPointBatch` chain).
    fn create_batch(&mut self, context: &mut Context) {
        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::BILLBOARD_VS,
                wgsl::BILLBOARD_FS,
                "point_primitive_batch".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("point primitive shader compilation failed: {error}");
                    return;
                }
            }
        }

        let rectangle = if let Some(rectangle) = self.atlas.rectangle_of(WHITE_IMAGE_ID) {
            rectangle
        } else {
            self.atlas.add_image(WHITE_IMAGE_ID, 1, 1, vec![255, 255, 255, 255])
        };

        let resolved: Vec<ResolvedBillboard> = self
            .points
            .iter()
            .filter(|point| point.show)
            .map(|point| ResolvedBillboard {
                position: point.position,
                width: point.scale,
                height: point.scale,
                pixel_offset: (point.pixel_offset.x, point.pixel_offset.y),
                color: [
                    point.color.red as f32,
                    point.color.green as f32,
                    point.color.blue as f32,
                    point.color.alpha as f32,
                ],
                texture_rectangle: rectangle,
            })
            .collect();

        let (positions, corners, texture_coordinates, colors, indices, anchors) =
            build_billboard_batch(&resolved);
        if indices.is_empty() {
            self.batch = None;
            self.dirty = false;
            return;
        }

        let atlas_texture = match self.atlas.texture(context) {
            Some(texture) => texture,
            None => return,
        };
        let vertex_array = match upload_quad_batch(
            context,
            &positions,
            &corners,
            &texture_coordinates,
            &colors,
            &indices,
        ) {
            Some(vertex_array) => vertex_array,
            None => return,
        };

        self.batch = Some(BatchResources {
            vertex_array,
            index_count: indices.len() as u32,
            atlas_texture,
            bounding_sphere: BoundingSphere::from_points(&anchors, None),
        });
        self.dirty = false;
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.points.clear();
        self.atlas.destroy();
        self.batch = None;
        self.is_destroyed = true;
    }
}

impl Default for PointPrimitiveCollection {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for PointPrimitiveCollection {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        PointPrimitiveCollection::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { PointPrimitiveCollection::is_destroyed(self) }
    fn destroy(&mut self) { PointPrimitiveCollection::destroy(self); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::color::Color;
    use crate::texture_atlas::TextureCoordinateRectangle;

    /// Mirrors PointPrimitiveCollectionSpec: "adds a point primitive".
    #[test]
    fn adds_a_point() {
        let mut collection = PointPrimitiveCollection::new();
        let index = collection.add(PointPrimitive::new());
        assert_eq!(index, 0);
        assert_eq!(collection.len(), 1);
        assert!(collection.get(0).is_some());
    }

    /// Mirrors PointPrimitiveCollectionSpec: "removes a point primitive".
    #[test]
    fn removes_a_point() {
        let mut collection = PointPrimitiveCollection::new();
        collection.add(PointPrimitive::new());
        assert!(collection.remove(0));
        assert_eq!(collection.len(), 0);
        assert!(!collection.remove(0));
    }

    /// Mirrors PointPrimitiveCollectionSpec: "removes all point primitives".
    #[test]
    fn removes_all_points() {
        let mut collection = PointPrimitiveCollection::new();
        collection.add(PointPrimitive::new());
        collection.add(PointPrimitive::new());
        collection.remove_all();
        assert!(collection.is_empty());
    }

    /// Mirrors PointPrimitiveCollectionSpec: "destroys".
    #[test]
    fn destroys() {
        let mut collection = PointPrimitiveCollection::new();
        assert!(!collection.is_destroyed());
        collection.destroy();
        assert!(collection.is_destroyed());
    }

    /// Points resolve into pixel quads of their `scale` size (the JS
    /// gl_PointSize contract mapped to screen-aligned quads).
    #[test]
    fn points_resolve_to_pixel_quads() {
        let rectangle = TextureCoordinateRectangle {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 1.0,
            y_max: 1.0,
        };
        let mut point = PointPrimitive::new();
        point.position = Cartesian3::new(7.0, 8.0, 9.0);
        point.scale = 10.0;
        point.color = Color::new(1.0, 0.0, 0.0, 1.0);
        let resolved = vec![ResolvedBillboard {
            position: point.position,
            width: point.scale,
            height: point.scale,
            pixel_offset: (point.pixel_offset.x, point.pixel_offset.y),
            color: [
                point.color.red as f32,
                point.color.green as f32,
                point.color.blue as f32,
                point.color.alpha as f32,
            ],
            texture_rectangle: rectangle,
        }];
        let (positions, corners, _, colors, indices, anchors) =
            build_billboard_batch(&resolved);
        assert_eq!(positions.len(), 12);
        assert_eq!(indices.len(), 6);
        assert_eq!(corners[0], -5.0); // half of the 10px size
        assert_eq!(&colors[0..3], &[1.0, 0.0, 0.0]);
        assert_eq!(anchors[0], point.position);
    }

    /// GPU path: `update` must upload the batch and issue a DrawCommand.
    ///
    /// UNLOCK CONDITION: requires a live wgpu device/queue (GPU smoke
    /// harness like `tests/globe_smoke.rs`).
    #[ignore = "requires a wgpu device (GPU smoke harness)"]
    #[test]
    fn update_creates_batch_and_draw_command() {
        // Exercised by the GPU smoke harness once the collection joins it.
    }
}
