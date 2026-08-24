//! Ported from `packages/engine/Source/Scene/LabelCollection.js`.
//!
//! A collection of labels.
//!
//! M3/S3 materialization: the CesiumJS batch pipeline is ported for the
//! collection semantics and the draw path — labels resolve into
//! screen-aligned flat-color rectangles through the shared billboard quad
//! batch + WGSL pair and are issued as one [`DrawCommand`].
//!
//! DEVIATION: CesiumJS rasterizes text glyphs through a canvas-based
//! `GlyphAtlas` and renders textured quads per glyph. The wgpu port has no
//! text rasterizer; each label renders as one flat-color rectangle sized by
//! a deterministic text-measure approximation (`char count × pixel size`),
//! keeping the layout/anchor/color contract renderable and spec-testable.
//! Outline rendering (`style` OUTLINE variants) is accepted for API parity
//! but not yet batched.

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::matrix4::Matrix4;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::texture::Texture;
use cesium_renderer::vertex_array::VertexArray;
use cesium_shaders::wgsl;

use crate::billboard_collection::{
    build_billboard_batch, upload_quad_batch, ResolvedBillboard,
};
use crate::frame_state::FrameState;
use crate::label::Label;
use crate::primitive_collection::ScenePrimitive;
use crate::texture_atlas::TextureAtlas;

/// GPU resources of the current label batch.
struct BatchResources {
    vertex_array: Arc<VertexArray>,
    index_count: u32,
    atlas_texture: Arc<Texture>,
    bounding_sphere: BoundingSphere,
}

/// The atlas id of the flat white texel the label rectangles sample.
const WHITE_IMAGE_ID: &str = "__cesium_label_white";

/// A collection of labels for efficient rendering of many text labels.
///
/// Mirrors CesiumJS `LabelCollection` (984 lines).
pub struct LabelCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The model matrix for this collection.
    pub model_matrix: cesium_core::matrix4::Matrix4,
    /// Whether to enable depth testing for labels.
    pub depth_test_enabled: bool,
    /// The labels in this collection.
    labels: Vec<Label>,
    /// A 1×1 white-texture atlas (labels are flat colored rectangles in the
    /// wgpu port; mirrors the JS collection owning a glyph atlas).
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

impl LabelCollection {
    /// Creates a new LabelCollection.
    pub fn new() -> Self {
        Self {
            show: true,
            model_matrix: cesium_core::matrix4::Matrix4::IDENTITY.clone(),
            depth_test_enabled: true,
            labels: Vec::new(),
            atlas: TextureAtlas::new(),
            dirty: true,
            batch: None,
            shader_program: None,
            is_destroyed: false,
        }
    }

    /// Adds a label to the collection and returns its index (CesiumJS
    /// returns the label; the Rust port moves it in and returns the index).
    pub fn add(&mut self, label: Label) -> usize {
        self.dirty = true;
        let index = self.labels.len();
        self.labels.push(label);
        index
    }

    /// Removes the label at the given index, returning whether it was
    /// present (mirrors the JS boolean `remove` contract).
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.labels.len() {
            self.labels.remove(index);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes all labels from the collection (mirrors the JS `removeAll`).
    pub fn remove_all(&mut self) {
        self.labels.clear();
        self.dirty = true;
    }

    /// Gets a label by index (mirrors the JS `get`).
    pub fn get(&self, index: usize) -> Option<&Label> {
        self.labels.get(index)
    }

    /// Gets a mutable reference to a label by index (marks the batch dirty).
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Label> {
        if let Some(label) = self.labels.get_mut(index) {
            self.dirty = true;
            Some(label)
        } else {
            None
        }
    }

    /// Returns the number of labels (mirrors the JS `length` property).
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Updates the collection for the current frame.
    ///
    /// Mirrors CesiumJS `LabelCollection#update`: rebuild the batch when
    /// dirty, then append the draw command to the frame's command list.
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show || self.labels.is_empty() || !frame_state.passes.main {
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
        render_state.depth_test.enabled = self.depth_test_enabled;
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
        command.owner = Some("LabelCollection".to_string());
        context.draw(command);
    }

    /// Resolves every shown label into a flat-color rectangle and uploads
    /// the batch (mirrors the JS glyph-batch chain; see module DEVIATION).
    fn create_batch(&mut self, context: &mut Context) {
        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::BILLBOARD_VS,
                wgsl::BILLBOARD_FS,
                "label_batch".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("label shader compilation failed: {error}");
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
            .labels
            .iter()
            .filter(|label| label.show && !label.text.is_empty())
            .map(|label| {
                let (width, height) = estimate_label_size(label);
                let position = if Matrix4::equals(&self.model_matrix, &Matrix4::IDENTITY) {
                    label.position
                } else {
                    Matrix4::multiply_by_point_new(&self.model_matrix, &label.position)
                };
                ResolvedBillboard {
                    position,
                    width: width * label.scale,
                    height: height * label.scale,
                    pixel_offset: (label.pixel_offset.x, label.pixel_offset.y),
                    color: [
                        label.fill_color.red as f32,
                        label.fill_color.green as f32,
                        label.fill_color.blue as f32,
                        label.fill_color.alpha as f32,
                    ],
                    texture_rectangle: rectangle,
                }
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
        self.labels.clear();
        self.atlas.destroy();
        self.batch = None;
        self.is_destroyed = true;
    }
}

impl Default for LabelCollection {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for LabelCollection {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        LabelCollection::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { LabelCollection::is_destroyed(self) }
    fn destroy(&mut self) { LabelCollection::destroy(self); }
}

/// Extracts the pixel size from a CSS-like font string (mirrors the JS
/// `Label._getFontHeight` canvas measurement; the port parses the `Npx`
/// prefix and defaults to 30 like the JS default font).
pub(crate) fn font_pixel_size(font: &str) -> f64 {
    for token in font.split_whitespace() {
        if let Some(number) = token.strip_suffix("px") {
            if let Ok(value) = number.parse::<f64>() {
                return value;
            }
        }
    }
    30.0
}

/// Approximates the rendered size of a label (see module DEVIATION: no
/// glyph rasterizer; deterministic char-count measurement instead of the JS
/// canvas measurement).
fn estimate_label_size(label: &Label) -> (f64, f64) {
    let pixel_size = font_pixel_size(&label.font);
    let width = label.text.chars().count() as f64 * pixel_size * 0.6;
    let height = pixel_size * 1.2;
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture_atlas::TextureCoordinateRectangle;

    /// Mirrors LabelCollectionSpec: "adds a label".
    #[test]
    fn adds_a_label() {
        let mut collection = LabelCollection::new();
        let index = collection.add(Label::new());
        assert_eq!(index, 0);
        assert_eq!(collection.len(), 1);
        assert!(collection.get(0).is_some());
    }

    /// Mirrors LabelCollectionSpec: "removes a label".
    #[test]
    fn removes_a_label() {
        let mut collection = LabelCollection::new();
        collection.add(Label::new());
        assert!(collection.remove(0));
        assert_eq!(collection.len(), 0);
        assert!(!collection.remove(0));
    }

    /// Mirrors LabelCollectionSpec: "removes all labels".
    #[test]
    fn removes_all_labels() {
        let mut collection = LabelCollection::new();
        collection.add(Label::new());
        collection.add(Label::new());
        collection.remove_all();
        assert!(collection.is_empty());
    }

    /// Mirrors LabelCollectionSpec: "destroys".
    #[test]
    fn destroys() {
        let mut collection = LabelCollection::new();
        assert!(!collection.is_destroyed());
        collection.destroy();
        assert!(collection.is_destroyed());
    }

    /// The font string parsing contract (JS default `30px sans-serif`).
    #[test]
    fn parses_font_pixel_size() {
        assert_eq!(font_pixel_size("30px sans-serif"), 30.0);
        assert_eq!(font_pixel_size("bold 12px monospace"), 12.0);
        assert_eq!(font_pixel_size("sans-serif"), 30.0);
    }

    /// Labels resolve into rectangles sized by the text measure
    /// approximation; empty text never reaches the batch.
    #[test]
    fn labels_resolve_to_sized_rectangles() {
        let mut label = Label::new();
        label.text = "abcd".to_string();
        label.font = "10px sans-serif".to_string();
        let (width, height) = estimate_label_size(&label);
        assert_eq!(width, 4.0 * 10.0 * 0.6);
        assert_eq!(height, 12.0);

        let rectangle = TextureCoordinateRectangle {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 1.0,
            y_max: 1.0,
        };
        let resolved = vec![ResolvedBillboard {
            position: label.position,
            width,
            height,
            pixel_offset: (0.0, 0.0),
            color: [1.0, 1.0, 1.0, 1.0],
            texture_rectangle: rectangle,
        }];
        let (_, _, _, _, indices, _) = build_billboard_batch(&resolved);
        assert_eq!(indices.len(), 6);
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
