//! cesium-scene：场景图与渲染流水线的领域模型
//!
//! 状态（P2 代码健康审计，2026-09-27）：已实现并由 `cesium-specs` 套件覆盖，
//! 但**尚未接入任何生产运行时路径**。其旧的 Bevy 桥接（`adapters/bevy-render/src/scene_pipeline.rs`）
//! 已在 P1-2 删除，因为黄金地球路径是 `dynamic_globe` ECS 渲染器（见
//! docs/ARCHITECTURE.md "Render Main Path"），因此目前没有任何 adapter/application
//! crate 依赖本 crate。作为 CesiumJS 功能对等性的领域模型保留，
//! 预留给未来的通用 draw-command 流水线；不要将其误读为已交付的能力。参见
//! docs/ARCHITECTURE.md "Test-only domain crates"。
//!
//! 映射到 CesiumJS：
//! - `Scene/Scene.js`
//! - `Scene/Primitive.js`
//! - `Renderer/DrawCommand.js`
//! - `Scene/Pass.js`
//!
//! # 特性
//! - 带变换的场景图节点层次结构
//! - 视景体剔除与可见性判定
//! - draw command 生成与渲染通道管理
//! - 帧统计跟踪

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
