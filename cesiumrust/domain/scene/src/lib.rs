//! cesium-scene: Scene graph and rendering pipeline domain models
//!
//! Maps to CesiumJS:
//! - `Scene/Scene.js`
//! - `Scene/Primitive.js`
//! - `Renderer/DrawCommand.js`
//! - `Scene/Pass.js`
//!
//! # Features
//! - Scene graph node hierarchy with transforms
//! - Frustum culling and visibility determination
//! - Draw command generation and render pass management
//! - Frame statistics tracking

pub mod scene_graph;
pub mod culling;
pub mod draw_command;
pub mod shader;
pub mod render_state;
pub mod debug_inspector;
pub mod axis;
pub mod attribute_type;
pub mod metadata_component_type;
pub mod job_scheduler;
pub mod implicit_availability_bitstream;

pub use scene_graph::{SceneGraph, SceneNode, NodeId, RenderableContent};
pub use culling::{
    CullingContext, CullResult, VisibilityResult,
    cull_scene, sort_front_to_back, sort_back_to_front, filter_visible,
};
pub use draw_command::{
    DrawCommand, RenderCommandList, RenderPass, BlendState, DepthState, FrameStatistics,
};
pub use shader::{
    ShaderStage, ShaderSource, ShaderUniform, ShaderStruct, ShaderFunction,
    ShaderBuilder, ShaderProgram, ShaderCache,
};
pub use render_state::{
    CullFace, StencilOp, StencilState, PolygonOffsetState, ScissorState,
    RenderState, DepthFunc, ClearCommand, ComputeCommand, ComputeUniformValue,
    PassState, PixelFormat, PixelDatatype, TextureFilter, TextureWrap,
    Texture, Framebuffer, TextureAtlas, TextureAtlasEntry, BufferUsage, GpuBuffer,
};
pub use debug_inspector::{
    DebugInspector, HighlightMode, TileDebugInfo, FrameDebugStats,
    PerformanceOverlay, TilesetInspector,
};
