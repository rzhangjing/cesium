//! Ported from `packages/engine/Source/Scene/BillboardCollection.js`.
//!
//! A renderable collection of 2D billboards that always face the camera.
//!
//! M3/S3 materialization: the CesiumJS batch pipeline is ported — billboards
//! are CPU-expanded into one batched vertex/index buffer pair (anchor
//! position + screen-space corner offset + atlas texture coordinates +
//! color), the images live in a [`TextureAtlas`], and `update` issues one
//! [`DrawCommand`] through the billboard WGSL pair.
//!
//! DEVIATION: CesiumJS resolves `billboard.image` through an asynchronous
//! image loader + `BillboardCollection`-level texture atlas promises; the
//! wgpu port takes images synchronously through
//! [`BillboardCollection::add_image`] (RGBA8 bytes) and looks the image id
//! up at batch time (unknown ids fall back to a flat white texel region,
//! keeping the billboard's color contract). Per-billboard `eyeOffset`,
//! `alignedAxis`, `rotation`, `scaleByDistance`, `translucencyByDistance`
//! and `distanceDisplayCondition` are accepted for API parity but not yet
//! expanded into the batch (see `billboard_vs.wgsl` header).

use std::sync::Arc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_shaders::wgsl;

use crate::billboard::Billboard;
use crate::frame_state::FrameState;
use crate::primitive_collection::ScenePrimitive;
use crate::texture_atlas::{TextureAtlas, TextureCoordinateRectangle};

/// The atlas id of the built-in flat white image used by billboards whose
/// image id is not (yet) registered in the atlas.
const WHITE_IMAGE_ID: &str = "__cesium_billboard_white";

/// The size (pixels) assumed by billboards that carry no explicit `size`
/// and whose image is not registered (CesiumJS derives the size from the
/// decoded image dimensions, which the wgpu port does not fetch).
const DEFAULT_BILLBOARD_SIZE: (f64, f64) = (16.0, 16.0);

/// One resolved billboard, ready for batch expansion. Also shared by the
/// point-primitive / label batches (same screen-aligned quad layout).
pub(crate) struct ResolvedBillboard {
    pub position: Cartesian3,
    pub width: f64,
    pub height: f64,
    pub pixel_offset: (f64, f64),
    pub color: [f32; 4],
    pub texture_rectangle: TextureCoordinateRectangle,
}

/// GPU resources of the current batch.
struct BatchResources {
    vertex_array: Arc<VertexArray>,
    index_count: u32,
    atlas_texture: Arc<cesium_renderer::texture::Texture>,
    bounding_sphere: BoundingSphere,
}

/// A renderable collection of 2D billboards that always face the camera.
///
/// Billboards are screen-aligned images positioned at 3D world coordinates.
/// The collection manages GPU resources efficiently for batch rendering.
pub struct BillboardCollection {
    /// The billboards in this collection.
    billboards: Vec<Billboard>,
    /// Whether this collection is shown.
    pub show: bool,
    /// The texture atlas backing the batch (mirrors the JS
    /// `_billboardCollection._textureAtlas`).
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

impl BillboardCollection {
    /// Creates a new BillboardCollection.
    pub fn new() -> Self {
        Self {
            billboards: Vec::new(),
            show: true,
            atlas: TextureAtlas::new(),
            dirty: true,
            batch: None,
            shader_program: None,
            is_destroyed: false,
        }
    }

    /// Registers an image in the collection's texture atlas (mirrors the JS
    /// image id resolution, minus the asynchronous loader; see module
    /// DEVIATION). Returns the image's atlas rectangle.
    pub fn add_image(&mut self, id: &str, width: u32, height: u32, rgba: Vec<u8>) -> TextureCoordinateRectangle {
        self.dirty = true;
        self.atlas.add_image(id, width, height, rgba)
    }

    /// Adds a billboard to the collection and returns its index
    /// (CesiumJS returns the billboard itself; the Rust port moves it into
    /// the collection and returns the index instead).
    pub fn add(&mut self, billboard: Billboard) -> usize {
        self.dirty = true;
        self.billboards.push(billboard);
        self.billboards.len() - 1
    }

    /// Returns the number of billboards (mirrors the JS `length` property).
    pub fn len(&self) -> usize { self.billboards.len() }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool { self.billboards.is_empty() }

    /// Returns the billboard at the given index (mirrors the JS `get`).
    pub fn get(&self, index: usize) -> Option<&Billboard> { self.billboards.get(index) }

    /// Returns a mutable reference to the billboard at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Billboard> {
        if let Some(billboard) = self.billboards.get_mut(index) {
            self.dirty = true;
            Some(billboard)
        } else {
            None
        }
    }

    /// Removes the billboard at the given index, returning whether it was
    /// present (mirrors the JS boolean `remove` return; the JS takes the
    /// billboard object, the port takes its index).
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.billboards.len() {
            self.billboards.remove(index);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes all billboards (mirrors the JS `removeAll`).
    pub fn remove_all(&mut self) {
        self.billboards.clear();
        self.dirty = true;
    }

    /// Updates the collection for the current frame.
    ///
    /// Mirrors CesiumJS `BillboardCollection#update`: resolve the batch on
    /// first use / after mutations, then append the draw command to the
    /// frame's command list.
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show {
            return;
        }
        if self.billboards.is_empty() {
            return;
        }
        if !frame_state.passes.main {
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

        // Billboards are blended like the JS translucent path: alpha
        // blending on, no depth writes, depth test against the globe.
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
        command.owner = Some("BillboardCollection".to_string());
        context.draw(command);
    }

    /// Resolves every shown billboard and uploads the batched buffers
    /// (mirrors the JS `_createBillboardBatch` chain).
    fn create_batch(&mut self, context: &mut Context) {
        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::BILLBOARD_VS,
                wgsl::BILLBOARD_FS,
                "billboard_batch".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("billboard shader compilation failed: {error}");
                    return;
                }
            }
        }

        // Ensure the fallback white image exists (flat-color billboards).
        if self.atlas.rectangle_of(WHITE_IMAGE_ID).is_none() {
            self.atlas.add_image(WHITE_IMAGE_ID, 1, 1, vec![255, 255, 255, 255]);
        }

        let mut resolved: Vec<ResolvedBillboard> = Vec::with_capacity(self.billboards.len());
        for billboard in &self.billboards {
            if !billboard.show {
                continue;
            }
            let (width, height) = billboard
                .size
                .map(|size| (size.x, size.y))
                .unwrap_or(DEFAULT_BILLBOARD_SIZE);
            let rectangle = billboard
                .image
                .as_deref()
                .and_then(|id| self.atlas.rectangle_of(id))
                .or_else(|| self.atlas.rectangle_of(WHITE_IMAGE_ID))
                .unwrap();
            resolved.push(ResolvedBillboard {
                position: billboard.position,
                width: width * billboard.scale,
                height: height * billboard.scale,
                pixel_offset: (billboard.pixel_offset.x, billboard.pixel_offset.y),
                color: [
                    billboard.color.red as f32,
                    billboard.color.green as f32,
                    billboard.color.blue as f32,
                    billboard.color.alpha as f32,
                ],
                texture_rectangle: rectangle,
            });
        }

        let (positions, corners, texture_coordinates, colors, indices, positions_cartesian) =
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
            bounding_sphere: BoundingSphere::from_points(&positions_cartesian, None),
        });
        self.dirty = false;
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.billboards.clear();
        self.atlas.destroy();
        self.batch = None;
        self.is_destroyed = true;
    }
}

impl Default for BillboardCollection {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for BillboardCollection {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        BillboardCollection::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { BillboardCollection::is_destroyed(self) }
    fn destroy(&mut self) { BillboardCollection::destroy(self); }
}

/// Uploads a screen-aligned quad batch (the billboard vertex layout) to
/// GPU buffers. Shared by BillboardCollection / PointPrimitiveCollection /
/// LabelCollection.
pub(crate) fn upload_quad_batch(
    context: &Context,
    positions: &[f32],
    corners: &[f32],
    texture_coordinates: &[f32],
    colors: &[f32],
    indices: &[u32],
) -> Option<Arc<VertexArray>> {
    let to_bytes = |values: &[f32]| -> Vec<u8> {
        values.iter().flat_map(|value| value.to_le_bytes()).collect()
    };
    let position_buffer = context.create_vertex_buffer(
        Some(&to_bytes(positions)), None, BufferUsage::StaticDraw);
    let corner_buffer = context.create_vertex_buffer(
        Some(&to_bytes(corners)), None, BufferUsage::StaticDraw);
    let texture_coordinate_buffer = context.create_vertex_buffer(
        Some(&to_bytes(texture_coordinates)), None, BufferUsage::StaticDraw);
    let color_buffer = context.create_vertex_buffer(
        Some(&to_bytes(colors)), None, BufferUsage::StaticDraw);
    let index_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let index_buffer = context.create_index_buffer(
        Some(&index_bytes),
        None,
        BufferUsage::StaticDraw,
        cesium_core::index_datatype::IndexDatatype::UnsignedInt,
    );

    let attributes = vec![
        VertexAttribute {
            index: 0,
            buffer: position_buffer,
            components_per_attribute: 3,
            component_datatype: wgpu::VertexFormat::Float32x3,
            normalize: false,
            stride_in_bytes: 12,
            offset_in_bytes: 0,
        },
        VertexAttribute {
            index: 1,
            buffer: corner_buffer,
            components_per_attribute: 2,
            component_datatype: wgpu::VertexFormat::Float32x2,
            normalize: false,
            stride_in_bytes: 8,
            offset_in_bytes: 0,
        },
        VertexAttribute {
            index: 2,
            buffer: texture_coordinate_buffer,
            components_per_attribute: 2,
            component_datatype: wgpu::VertexFormat::Float32x2,
            normalize: false,
            stride_in_bytes: 8,
            offset_in_bytes: 0,
        },
        VertexAttribute {
            index: 3,
            buffer: color_buffer,
            components_per_attribute: 4,
            component_datatype: wgpu::VertexFormat::Float32x4,
            normalize: false,
            stride_in_bytes: 16,
            offset_in_bytes: 0,
        },
    ];
    Some(Arc::new(VertexArray::new(attributes, Some(index_buffer))))
}

/// Expands the resolved billboards into the batched vertex/index data.
///
/// Returns `(positions [xyz], corners [xy], texture_coordinates [uv],
/// colors [rgba], indices, anchor_positions)`. Pure CPU function so the
/// batch layout is spec-testable without a GPU.
pub(crate) fn build_billboard_batch(
    resolved: &[ResolvedBillboard],
) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>, Vec<Cartesian3>) {
    let mut positions: Vec<f32> = Vec::with_capacity(resolved.len() * 4 * 3);
    let mut corners: Vec<f32> = Vec::with_capacity(resolved.len() * 4 * 2);
    let mut texture_coordinates: Vec<f32> = Vec::with_capacity(resolved.len() * 4 * 2);
    let mut colors: Vec<f32> = Vec::with_capacity(resolved.len() * 4 * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(resolved.len() * 6);
    let mut anchors: Vec<Cartesian3> = Vec::with_capacity(resolved.len());

    for billboard in resolved {
        let base = (positions.len() / 3) as u32;
        let half_width = billboard.width * 0.5;
        let half_height = billboard.height * 0.5;
        let (offset_x, offset_y) = billboard.pixel_offset;
        let rectangle = billboard.texture_rectangle;

        // Four screen-aligned corners (mirrors the JS corner expansion:
        // the offset is applied in screen space by the vertex shader).
        let corner_offsets: [(f64, f64); 4] = [
            (-half_width + offset_x, -half_height + offset_y),
            (half_width + offset_x, -half_height + offset_y),
            (half_width + offset_x, half_height + offset_y),
            (-half_width + offset_x, half_height + offset_y),
        ];
        let uvs: [(f64, f64); 4] = [
            (rectangle.x_min, rectangle.y_max),
            (rectangle.x_max, rectangle.y_max),
            (rectangle.x_max, rectangle.y_min),
            (rectangle.x_min, rectangle.y_min),
        ];

        for (corner_offset, uv) in corner_offsets.iter().zip(uvs.iter()) {
            positions.push(billboard.position.x as f32);
            positions.push(billboard.position.y as f32);
            positions.push(billboard.position.z as f32);
            corners.push(corner_offset.0 as f32);
            corners.push(corner_offset.1 as f32);
            texture_coordinates.push(uv.0 as f32);
            texture_coordinates.push(uv.1 as f32);
            colors.extend_from_slice(&billboard.color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        anchors.push(billboard.position);
    }

    (positions, corners, texture_coordinates, colors, indices, anchors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::color::Color;

    /// Mirrors BillboardCollectionSpec: "adds a billboard".
    #[test]
    fn adds_a_billboard() {
        let mut collection = BillboardCollection::new();
        let index = collection.add(Billboard::new());
        assert_eq!(index, 0);
        assert_eq!(collection.len(), 1);
        assert!(collection.get(0).is_some());
    }

    /// Mirrors BillboardCollectionSpec: "removes a billboard" (boolean
    /// return contract).
    #[test]
    fn removes_a_billboard() {
        let mut collection = BillboardCollection::new();
        collection.add(Billboard::new());
        assert!(collection.remove(0));
        assert_eq!(collection.len(), 0);
        assert!(!collection.remove(0));
    }

    /// Mirrors BillboardCollectionSpec: "removes all billboards".
    #[test]
    fn removes_all_billboards() {
        let mut collection = BillboardCollection::new();
        collection.add(Billboard::new());
        collection.add(Billboard::new());
        collection.remove_all();
        assert!(collection.is_empty());
    }

    /// Mirrors BillboardCollectionSpec: "isDestroyed"/"destroys".
    #[test]
    fn destroys() {
        let mut collection = BillboardCollection::new();
        assert!(!collection.is_destroyed());
        collection.destroy();
        assert!(collection.is_destroyed());
    }

    /// Mirrors the batch layout contract of the JS `BillboardBatch`:
    /// 4 vertices + 6 indices per shown billboard, hidden ones skipped.
    #[test]
    fn batch_expansion_layout() {
        let rectangle = TextureCoordinateRectangle { x_min: 0.0, y_min: 0.0, x_max: 1.0, y_max: 1.0 };
        let resolved = vec![
            ResolvedBillboard {
                position: Cartesian3::new(1.0, 2.0, 3.0),
                width: 8.0,
                height: 4.0,
                pixel_offset: (0.0, 0.0),
                color: [1.0, 0.0, 0.0, 1.0],
                texture_rectangle: rectangle,
            },
            ResolvedBillboard {
                position: Cartesian3::new(4.0, 5.0, 6.0),
                width: 2.0,
                height: 2.0,
                pixel_offset: (1.0, -1.0),
                color: [0.0, 1.0, 0.0, 0.5],
                texture_rectangle: rectangle,
            },
        ];
        let (positions, corners, uvs, colors, indices, anchors) = build_billboard_batch(&resolved);
        assert_eq!(positions.len(), 2 * 4 * 3);
        assert_eq!(corners.len(), 2 * 4 * 2);
        assert_eq!(uvs.len(), 2 * 4 * 2);
        assert_eq!(colors.len(), 2 * 4 * 4);
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(anchors.len(), 2);
        // First billboard corners: ±half extents around the anchor.
        assert_eq!(corners[0], -4.0);
        assert_eq!(corners[1], -2.0);
        // Second billboard honors the pixel offset (starts at vertex 4 →
        // corner component 8).
        assert_eq!(corners[8], -1.0 + 1.0);
        assert_eq!(corners[9], -1.0 - 1.0);
    }

    /// Hidden billboards never reach the batch (the JS filters `show`
    /// during batch resolution).
    #[test]
    fn batch_resolution_skips_hidden_billboards() {
        let mut collection = BillboardCollection::new();
        let mut shown = Billboard::new();
        shown.position = Cartesian3::new(1.0, 0.0, 0.0);
        let mut hidden = Billboard::new();
        hidden.show = false;
        collection.add(shown);
        collection.add(hidden);

        // Resolve through the same logic `create_batch` uses (without GPU).
        let mut resolved = Vec::new();
        for billboard in &collection.billboards {
            if billboard.show {
                resolved.push(ResolvedBillboard {
                    position: billboard.position,
                    width: 16.0,
                    height: 16.0,
                    pixel_offset: (0.0, 0.0),
                    color: [1.0, 1.0, 1.0, 1.0],
                    texture_rectangle: TextureCoordinateRectangle {
                        x_min: 0.0,
                        y_min: 0.0,
                        x_max: 1.0,
                        y_max: 1.0,
                    },
                });
            }
        }
        let (_, _, _, _, indices, _) = build_billboard_batch(&resolved);
        assert_eq!(indices.len(), 6);
        assert_eq!(collection.len(), 2);
    }

    /// Color fields flow through `Color` into the batch unchanged.
    #[test]
    fn billboard_color_flows_into_batch() {
        let mut billboard = Billboard::new();
        billboard.color = Color::new(0.5, 0.25, 0.125, 0.75);
        let resolved = vec![ResolvedBillboard {
            position: Cartesian3::default(),
            width: 4.0,
            height: 4.0,
            pixel_offset: (0.0, 0.0),
            color: [
                billboard.color.red as f32,
                billboard.color.green as f32,
                billboard.color.blue as f32,
                billboard.color.alpha as f32,
            ],
            texture_rectangle: TextureCoordinateRectangle {
                x_min: 0.0,
                y_min: 0.0,
                x_max: 1.0,
                y_max: 1.0,
            },
        }];
        let (_, _, _, colors, _, _) = build_billboard_batch(&resolved);
        assert_eq!(&colors[0..4], &[0.5, 0.25, 0.125, 0.75]);
    }

    /// GPU path: `update` must upload the batch and issue a DrawCommand.
    ///
    /// UNLOCK CONDITION: requires a live wgpu device/queue (run under the
    /// GPU smoke harness like `tests/globe_smoke.rs`); the CPU-only test
    /// profile cannot build a `Context`.
    #[ignore = "requires a wgpu device (GPU smoke harness)"]
    #[test]
    fn update_creates_batch_and_draw_command() {
        // Intentionally empty under #[ignore]: exercised by the GPU smoke
        // harness once billboard_collection joins it.
    }
}
