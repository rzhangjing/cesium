//! Ported from `packages/engine/Source/Renderer/AutomaticUniforms.js`.
//!
//! Automatic uniforms that are set every frame by the renderer.
//!
//! DEVIATION (B2.5): CesiumJS uploads each `czm_*` uniform individually via
//! `gl.uniform*` calls before every draw. The wgpu port packs the
//! smoke-critical automatic uniforms into one `CesiumAutomaticUniforms`
//! uniform buffer (group(0) binding(0) of the hand-written WGSL shaders, see
//! `cesium_shaders::wgsl`) and writes one slot per draw from a per-frame ring
//! buffer with dynamic offsets.

use cesium_core::matrix4::Matrix4;

use crate::uniform_state::UniformState;

/// Byte layout of the `CesiumAutomaticUniforms` WGSL struct (column-major
/// mat4x4&lt;f32&gt; blocks, then a vec4). Must match `globe_vs.wgsl` exactly.
pub const AUTOMATIC_UNIFORMS_SIZE: usize = 5 * 64 + 16;

/// Dynamic-offset alignment required by wgpu for uniform buffers.
const DYNAMIC_OFFSET_ALIGNMENT: u64 = 256;

/// Automatic uniforms set every frame by the renderer.
///
/// These include model-view-projection matrices, camera position,
/// time values, and other commonly-needed values.
#[derive(Debug, Clone)]
pub struct AutomaticUniforms {
    /// The model-view-projection matrix.
    pub czm_modelViewProjection: Matrix4,
    /// The model-view matrix.
    pub czm_modelView: Matrix4,
    /// The projection matrix.
    pub czm_projection: Matrix4,
    /// The model matrix.
    pub czm_model: Matrix4,
    /// The view matrix.
    pub czm_view: Matrix4,
    /// The viewport rectangle (x, y, width, height).
    pub czm_viewport: [f32; 4],
    /// The camera eye height.
    pub czm_eyeHeight: f32,
}

impl AutomaticUniforms {
    /// Creates a new set of automatic uniforms with default values.
    pub fn new() -> Self {
        Self {
            czm_modelViewProjection: Matrix4::IDENTITY,
            czm_modelView: Matrix4::IDENTITY,
            czm_projection: Matrix4::IDENTITY,
            czm_model: Matrix4::IDENTITY,
            czm_view: Matrix4::IDENTITY,
            czm_viewport: [0.0, 0.0, 1.0, 1.0],
            czm_eyeHeight: 0.0,
        }
    }

    /// Snapshots the automatic uniforms from the current [`UniformState`]
    /// (mirrors CesiumJS's per-draw `uniformState` evaluation).
    pub fn from_uniform_state(state: &mut UniformState) -> Self {
        let model = state.model().clone();
        let view = state.view().clone();
        let projection = state.projection().clone();
        let model_view = state.model_view().clone();
        let model_view_projection = state.model_view_projection().clone();
        let viewport = state.viewport();
        Self {
            czm_modelViewProjection: model_view_projection,
            czm_modelView: model_view,
            czm_projection: projection,
            czm_model: model,
            czm_view: view,
            czm_viewport: [
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
            ],
            czm_eyeHeight: state.camera_position().z as f32,
        }
    }

    /// Serializes into the exact WGSL struct byte layout (336 bytes).
    pub fn to_bytes(&self) -> [u8; AUTOMATIC_UNIFORMS_SIZE] {
        let mut bytes = [0u8; AUTOMATIC_UNIFORMS_SIZE];
        write_matrix(&mut bytes[0..64], &self.czm_modelViewProjection);
        write_matrix(&mut bytes[64..128], &self.czm_modelView);
        write_matrix(&mut bytes[128..192], &self.czm_projection);
        write_matrix(&mut bytes[192..256], &self.czm_view);
        write_matrix(&mut bytes[256..320], &self.czm_model);
        for (i, component) in self.czm_viewport.iter().enumerate() {
            bytes[320 + i * 4..324 + i * 4].copy_from_slice(&component.to_le_bytes());
        }
        bytes
    }
}

impl Default for AutomaticUniforms {
    fn default() -> Self { Self::new() }
}

/// Writes a `Matrix4` (column-major f64) as a WGSL `mat4x4<f32>` (16 f32,
/// column-major, 64 bytes).
fn write_matrix(destination: &mut [u8], matrix: &Matrix4) {
    for (i, value) in matrix.elements.iter().enumerate() {
        let float = *value as f32;
        destination[i * 4..i * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
}

/// Per-frame ring buffer for `CesiumAutomaticUniforms` slots.
///
/// One slot is allocated per draw that consumes automatic uniforms; the
/// matching bind group uses a dynamic offset so a single bind group (and a
/// single buffer) serves the whole frame.
///
/// DEVIATION: CesiumJS has no equivalent — GL uniforms are pushed with
/// `gl.uniform*` per draw. This is the wgpu-side realization of the same
/// automatic-uniform mechanism.
pub struct AutomaticUniformRing {
    buffer: wgpu::Buffer,
    slot_size: u64,
    capacity_slots: u64,
    next_offset: u64,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl AutomaticUniformRing {
    /// Creates the ring buffer, its bind group layout (group(0) binding(0),
    /// uniform buffer with dynamic offset) and the frame-shared bind group.
    pub fn new(device: &wgpu::Device, capacity_slots: u64) -> Self {
        let slot_size = (AUTOMATIC_UNIFORMS_SIZE as u64 + DYNAMIC_OFFSET_ALIGNMENT - 1)
            / DYNAMIC_OFFSET_ALIGNMENT
            * DYNAMIC_OFFSET_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CesiumAutomaticUniforms ring"),
            size: slot_size * capacity_slots,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CesiumAutomaticUniforms BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CesiumAutomaticUniforms BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(AUTOMATIC_UNIFORMS_SIZE as u64),
                }),
            }],
        });
        Self {
            buffer,
            slot_size,
            capacity_slots,
            next_offset: 0,
            bind_group_layout,
            bind_group,
        }
    }

    /// The bind group layout used at group(0) in pipeline layouts.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// The frame-shared bind group; pair with the dynamic slot offset.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Resets the ring for a new frame.
    pub fn begin_frame(&mut self) {
        self.next_offset = 0;
    }

    /// Allocates one slot, writes the current automatic uniforms into it and
    /// returns the dynamic bind-group offset. Returns `None` when the ring is
    /// exhausted for the frame.
    pub fn allocate(
        &mut self,
        queue: &wgpu::Queue,
        uniforms: &AutomaticUniforms,
    ) -> Option<u32> {
        if self.next_offset + self.slot_size > self.slot_size * self.capacity_slots {
            return None;
        }
        let offset = self.next_offset;
        self.next_offset += self.slot_size;
        queue.write_buffer(&self.buffer, offset, &uniforms.to_bytes());
        Some(offset as u32)
    }

    /// Returns the number of slots used this frame.
    pub fn slots_used(&self) -> u64 {
        self.next_offset / self.slot_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_layout_matches_wgsl_struct() {
        let mut uniforms = AutomaticUniforms::new();
        uniforms.czm_viewport = [0.0, 0.0, 800.0, 600.0];
        let bytes = uniforms.to_bytes();
        assert_eq!(bytes.len(), AUTOMATIC_UNIFORMS_SIZE);

        // Identity model matrix at offset 256: column-major, m[0]=m[5]=m[10]=m[15]=1.
        let read_f32 = |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!(read_f32(256), 1.0);
        assert_eq!(read_f32(256 + 20), 1.0);
        assert_eq!(read_f32(256 + 40), 1.0);
        assert_eq!(read_f32(256 + 60), 1.0);

        // Viewport at offset 320.
        assert_eq!(read_f32(320), 0.0);
        assert_eq!(read_f32(328), 800.0);
        assert_eq!(read_f32(332), 600.0);
    }

    #[test]
    fn matrix_columns_are_written_in_order() {
        // Column-major: element[1] is the second row of the first column.
        let mut matrix = Matrix4::IDENTITY;
        matrix.elements[1] = 2.0;
        let mut bytes = [0u8; 64];
        write_matrix(&mut bytes, &matrix);
        let value = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(value, 2.0);
    }
}
