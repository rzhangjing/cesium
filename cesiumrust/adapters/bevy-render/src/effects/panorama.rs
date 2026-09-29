//! M6.3 — Panorama / SkyBox 渲染节点（cubemap + equirectangular）。
//!
//! 将 cesiumrust panorama 绘制实现为一个 `Core3d` [`ViewNode`]，遵循
//! [`super::fxaa`] 和 [`super::graph`] 所确立的 M5-E0 渲染图内部模式。
//!
//! # 蓝图（上游真相源，`packages/engine/Source/`）
//! - `Scene/SkyBox.js`（164 行）——**完全**委托给 `CubeMapPanorama`
//!   （L39-43，L100 注释 "Delegate completely"）。
//! - `Scene/CubeMapPanorama.js`（352 行）——`pass: Pass.ENVIRONMENT`（L105-106，
//!   注释 "render before everything else"），一个 2×2×2 `BoxGeometry` 缩放到
//!   `czm_entireFrustum.y`，`depthTest: {enabled: false}`，`depthMask: false`，
//!   `blending: ALPHA_BLEND`，以及 L232 `if (!defined(this._cubeMap)) return undefined;`。
//! - `Scene/EquirectangularPanorama.js`（266 行）——`DEFAULT_RADIUS = 100000.0` m，
//!   `SphereGeometry`，带 `repeat: new Cartesian2(-repeatHorizontal, repeatVertical)`（L117）
//!   的 Fabric `Image` 材质，以及
//!   `MaterialAppearance({ closed: true, translucent: false, renderState: { cull: { enabled: false } } })`。
//! - `Shaders/SkyBoxVS.glsl`、`Shaders/SkyBoxFS.glsl`、`Shaders/CubeMapPanoramaVS.glsl`。
//! - `Renderer/AutomaticUniforms.js` L329/L341（`czm_viewRotation` 是一个 **mat3**），
//!   L1064（`czm_entireFrustum` 是一个 **vec2** `(near, far)`）。
//!
//! 领域半位于 `cesium_effects::panorama`（`domain/effects/src/panorama.rs`），
//! 承载全部 f64 几何以及上游顶点 shader 的 CPU 参考。
//! 本文件仅在 uniform 边界收窄到 f32。
//!
//! # 门控
//! [`ENV_ENABLE_PANORAMA`]（`CESIUM_ENABLE_PANORAMA`），**默认 OFF**。门控由
//! *application* 层求值，由它决定是否调用 [`register_panorama_node`]；
//! [`panorama_gate_enabled`] 是供测试以及任何必须询问的适配器使用的
//! 适配层本地镜像。门控 OFF 时节点从不被加入图，
//! 没有实体携带 [`CesiumPanorama`]，八个 v0 基线保持不变（PSNR = infinity）。
//!
//! 此 env 名在此重复而非从 `application/cesium-app/src/feature_flags.rs` 导入，
//! 因为适配层不能依赖应用层（DDD）——与
//! [`crate::atmosphere::sky_dome::ENV_ENABLE_SKYDOME`] 和
//! `effects::graph::ENV_ENABLE_POSTPROCESS` 同一约定。
//!
//! # 渲染顺序——节点归属何处，以及为何
//!
//! 上游在 `Pass.ENVIRONMENT` 中绘制 panorama，即*先于其他一切*，
//! 并禁用深度测试与深度写入。Bevy 无法逐字表达这一点：
//!
//! * `MainOpaquePass3dNode`
//!   （`bevy_core_pipeline-0.15.3/src/core_3d/main_opaque_pass_3d_node.rs` L66）经
//!   `ViewTarget::get_color_attachment()` 取其颜色 attachment，其
//!   `ColorAttachment::get_attachment`
//!   （`bevy_render-0.15.3/src/texture/texture_attachment.rs` L62-74）在**首次**调用时发出
//!   `LoadOp::Clear`，此后每次发 `LoadOp::Load`。
//!   `DepthAttachment::get_attachment`（同文件，L102-106）行为完全相同。
//!   因此，放在 `Node3d::MainOpaquePass` *之前* 的节点，其输出会被 clear 擦除，
//!   而放在其前的节点根本无法绘制，因为还没有 pass 打开。
//! * 在深度测试关闭的情况下*整个主 pass 之后*绘制会覆盖到地球上，
//!   这与所希望的恰恰相反。
//!
//! 所以节点放在 **`Node3d::MainOpaquePass` 与 `Node3d::MainTransmissivePass` 之间**，
//! 配 `depth_compare = GreaterEqual` 和 `frag_depth = 0.0`（Bevy 用反向 Z，
//! 所以 `0.0` 是远平面）。不透明 pass 已经占据了每个有几何的像素，
//! 并把其余留在被清除的远深度，所以本节点**恰好**只写天空像素。
//! 最终 framebuffer 与上游的"先画 skybox，其余覆盖其上"逐位相同，
//! 因为上游 `ALPHA_BLEND` 在 `a = czm_morphTime = 1.0` 下就是一次直接
//! 覆盖（参见 `shaders/panorama.wgsl` 中的 DEVIATION 4）。
//!
//! Bevy 自己也得出同样结论：`MainOpaquePass3dNode` L113-127 把内置 skybox 绘制为
//! 一个全屏三角形，位于不透明和 alpha-mask 阶段*之后*，配的正是这一深度状态
//!（`bevy_core_pipeline-0.15.3/src/skybox/mod.rs` L201-216：
//! `depth_write_enabled: false, depth_compare: GreaterEqual`）。
//!
//! ## 相对于 starfield 和 sky dome
//! 两者都是以世界原点为中心的 `Transparent3d` 实体——
//! `application/cesium-app/src/starfield.rs` L114/L162-173（`radius = 50.0`、
//! `AlphaMode::Blend`、`unlit: true`、`cull_mode: None`、单次 draw call）和
//! `atmosphere/sky_dome.rs`（`SKY_DOME_RADIUS = 40.0`、`AlphaMode::Premultiplied`、
//! `SKY_DOME_DEPTH_BIAS = 1000.0` 把它固定在 starfield 之后、`cull_mode =
//! Some(Face::Front)`）。`Transparent3d` 运行于 `MainTransparentPass3dNode`，它位于
//! `Node3d::MainTransmissivePass` **之后**。因此把 panorama 放在该节点之前，
//! 逐像素地得到：
//!
//! ```text
//!   1. MainOpaquePass        globe writes colour + depth
//!   2. CesiumPanoramaLabel   panorama fills the remaining far-depth pixels
//!                            (SKYBOX: no depth write; BUBBLE: real depth write)
//!   3. MainTransmissivePass  (unused by cesiumrust)
//!   4. MainTransparentPass   starfield (r = 50, Blend) then sky dome
//!                            (r = 40, Premultiplied, depth_bias 1000)
//! ```
//!
//! 这正是上游的顺序：`Pass.ENVIRONMENT` panorama 先，然后 primitives，
//! 然后是星箱（`SkyBox` 本身就是一个 `CubeMapPanorama`）。sky dome 的透射率
//! 仍会消光 starfield，而 starfield 仍会在 panorama 之上混合，
//! 因为两个透明实体的深度状态和排序顺序都未被触碰。在 `BUBBLE` 放置中
//! panorama 额外写入真实深度，所以它正确地遮挡 starfield 和 dome——
//! 等价于上游 `translucent: false` 的不透明球。
//!
//! **此处不创建任何边。** [`register_panorama_node`] 只添加节点；
//! `effects::graph::register_render_graph` 拥有单一线性 `Core3d` 链
//!（Daniel H2，上游 CesiumJS 一致性），task #81 接线边。参见 [`insertion_hint`]。
//!
//! # 偏差
//! 记录于 `docs/deviations.md#dev-025`；shader 侧的那些列在
//! `shaders/panorama.wgsl` 的头里。
//! 1. 全屏三角形而非远平面缩放的 box 几何（Bevy 用无限反向投影，
//!    所以远平面在无穷远）。
//! 2. 以深度测试替代 `Pass.ENVIRONMENT`（见上）。
//! 3. 无 `czm_gammaCorrect`——sRGB 纹理格式 + sRGB framebuffer 在硬件中完成。
//! 4. 无 `czm_morphTime` alpha——cesiumrust 没有 2D/Columbus-View 变形。
//! 5. 非 HDR 颜色目标格式是 `TextureFormat::bevy_default()`
//!    （`Bgra8UnormSrgb`），而非 `Rgba8UnormSrgb`。这是
//!    `ViewTarget` 主纹理的格式；`super::fxaa` 用 `Rgba8UnormSrgb` 是因为它
//!    改为写入一个 `create_post_process_texture` 中间纹理。两者都是 sRGB，
//!    所以项目红线（"sRGB 颜色纹理使用 sRGB 格式"）成立——
//!    而 panorama *asset* 本身创建为 `Rgba8UnormSrgb`。
//! 6. Cube-map **放置**对两种纹理布局都适用，反之亦然，因为
//!    `mode`（放置）和 `source`（布局）是独立的 uniform。上游
//!    硬连线 `CubeMapPanorama` = (skybox, cube) 和 `EquirectangularPanorama`
//!    = (bubble, equirect)。

use std::fmt::Write as _;

use bevy::core_pipeline::core_3d::{
    graph::{Core3d, Node3d},
    CORE_3D_DEPTH_FORMAT,
};
use bevy::ecs::query::QueryItem;
use bevy::image::BevyDefault;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_cube, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BindingResource,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
        DepthStencilState, Extent3d, FilterMode, FragmentState, MultisampleState, PipelineCache,
        PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor,
        Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor, Shader, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines,
        StencilFaceState,
        StencilState, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension, UniformBuffer, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
    view::{
        ExtractedView, Msaa, ViewDepthTexture, ViewTarget, ViewUniform, ViewUniformOffset,
        ViewUniforms,
    },
    Render, RenderApp, RenderSet,
};
use cesium_effects::panorama::{
    CubeMapPanorama, EquirectangularPanorama, PanoramaPlacement, PanoramaSource,
    PANORAMA_METERS_PER_RENDER_UNIT,
};
use glam::{DMat3, DMat4, DVec3};

// ─── 门控 ────────────────────────────────────────────────────────────────────

/// 门控 panorama 注册的 env 变量。**默认 OFF。**
///
/// **单一真相源（task #81）**：此名的拥有者是应用层注册表
/// `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_PANORAMA` 和
/// `panorama_enabled()` accessor，列在 `RESERVED_FLAGS` 中）。此 const 是一个*镜像*，
/// 仅因 `cesium-app` 依赖 `cesium-bevy-render`（绝不反向）而存在，
/// 所以本 crate 不能导入注册表。它是 `pub`，以便
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` 能
/// 跨 crate 边界断言字节相等；注册本身由
/// `effects::graph::M6WaveARenderGraphPlugin` 驱动，它每 plugin 阶段读一次
/// [`panorama_gate_enabled`]。
pub const ENV_ENABLE_PANORAMA: &str = "CESIUM_ENABLE_PANORAMA";

/// [`ENV_ENABLE_PANORAMA`] 的适配层本地求值。
///
/// 真值集即 `crate::pipeline::fetch::gate_from_env_value` 的那套
///（`"1"|"true"|"yes"|"on"`，trim + 转小写），与
/// `feature_flags::env_flag` 字节一致。
#[inline]
pub fn panorama_gate_enabled() -> bool {
    crate::pipeline::fetch::gate_from_env_value(std::env::var(ENV_ENABLE_PANORAMA).ok())
}

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `shaders/panorama.wgsl` 的唯一 handle。
///
/// 遵循 `super::fxaa::FXAA_SHADER_HANDLE`（`0xCE51_E1E1_F4AA_0012`）的
/// `CE51`（"CESI"）前缀约定；`9A4E_0A70` 中间段是 `PAN` + `ORAMA` 的谐音，
/// 结尾的 `0063` 是 M6.3 里程碑编号的十六进制。
pub const PANORAMA_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_9A4E_0A70_0063);

// ─── Shader 常量镜像 ─────────────────────────────────────────────────

/// `shaders/panorama.wgsl` 的 `MODE_SKYBOX`。[`PanoramaPlacement::Skybox`] 的 wire 值。
pub const MODE_SKYBOX: u32 = PanoramaPlacement::Skybox.as_u32();

/// `shaders/panorama.wgsl` 的 `MODE_BUBBLE`。[`PanoramaPlacement::Bubble`] 的 wire 值。
pub const MODE_BUBBLE: u32 = PanoramaPlacement::Bubble.as_u32();

/// `shaders/panorama.wgsl` 的 `SOURCE_CUBEMAP`。[`PanoramaSource::CubeMap`] 的 wire 值。
pub const SOURCE_CUBEMAP: u32 = PanoramaSource::CubeMap.as_u32();

/// `shaders/panorama.wgsl` 的 `SOURCE_EQUIRECTANGULAR`。[`PanoramaSource::Equirectangular`]
/// 的 wire 值。
pub const SOURCE_EQUIRECTANGULAR: u32 = PanoramaSource::Equirectangular.as_u32();

/// `shaders/panorama.wgsl` 的
/// `const DEGENERATE_DIRECTION_SQUARED_EPSILON: f32 = 1.0e-24;` 的 f32 镜像。
///
/// 刻意是一个独立字面量而非
/// `cesium_effects::panorama::DEGENERATE_DIRECTION_SQUARED_EPSILON as f32`：f64
/// 常量从十进制正确舍入一次，而对其强制转换会二次舍入，所以两者
/// 可能相差一个 ULP。
/// [`tests::the_wgsl_literals_match_the_rust_mirrors_bit_for_bit`] 直接从 shader 源码
/// 解析该字面量并比较 `to_bits()`，从而在无双重舍入的情况下闭环校验。
pub const PANORAMA_DEGENERATE_DIRECTION_SQUARED_EPSILON_F32: f32 = 1.0e-24;

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 在相机实体上启用 cesiumrust panorama 绘制的标记组件。
///
/// 经 [`ExtractComponentPlugin`] 提取到 render world。当 `enabled == false` 时
/// [`PanoramaNode`] 提前 return，且 [`prepare_panorama_pipelines`] 完全跳过
/// 这些视图，所以一个禁用的 panorama 在 GPU 上零开销。
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumPanorama {
    /// 该相机的主开关。
    pub enabled: bool,
    /// 无限、以相机为中心的 skybox，或有限的锚定球。
    pub placement: PanoramaPlacement,
    /// Cube-map 或 2:1 equirectangular 纹理。
    pub source: PanoramaSource,
    /// panorama 颜色图像。将其创建为 `Rgba8UnormSrgb`（项目对 sRGB 颜色纹理的
    /// 红线），以便硬件执行上游 `czm_gammaCorrect` 手工完成的 sRGB→linear 解码。
    pub image: Handle<Image>,
    /// 辐射度乘子。`1.0` 为中性；上游无对应物。
    pub brightness: f32,
    /// **world → local** panorama transform（领域 transform 的逆，其平移以
    /// render 单位表示）。方向以 `w = 0.0` 乘上它，所以在 `Skybox` 放置中只有它的
    /// 旋转有意义；bubble 中心改为在 [`Self::center`] 中移动。
    ///
    /// 与 Bevy 的 `SkyboxUniforms.transform` 同约定
    ///（`bevy_core_pipeline-0.15.3/src/skybox/mod.rs` L127-129）。
    pub transform: Mat4,
    /// bubble 中心，以世界 render 单位表示。`Skybox` 放置中不用。
    pub center: Vec3,
    /// bubble 半径，以 render 单位表示。`Skybox` 放置中不用。
    pub radius: f32,
    /// 采样器 repeat，`(-repeat_horizontal, repeat_vertical)`——上游
    /// `EquirectangularPanorama.js` L117。必须与 `AddressMode::Repeat` 配对，
    /// 由 [`PanoramaPipeline::from_world`] 提供。
    pub repeat: Vec2,
}

impl Default for CesiumPanorama {
    fn default() -> Self {
        Self {
            enabled: false,
            placement: PanoramaPlacement::Skybox,
            source: PanoramaSource::CubeMap,
            image: Handle::default(),
            brightness: 1.0,
            transform: Mat4::IDENTITY,
            center: Vec3::ZERO,
            radius: 0.0,
            repeat: Vec2::ONE,
        }
    }
}

impl CesiumPanorama {
    /// 从领域 [`EquirectangularPanorama`] 构建组件。
    ///
    /// 这是 f64 领域几何收窄到 f32 的**唯一**位置：本函数中每个
    /// `as f32` 都位于 uniform 边界上，遵循项目红线。`image` 由调用方
    /// 提供，因为领域层只持有一个 URL 字符串，对 Bevy asset handle 一无所知。
    pub fn from_domain_equirectangular(
        panorama: &EquirectangularPanorama,
        image: Handle<Image>,
        brightness: f32,
    ) -> Self {
        // 上游由一个位置加 heading/pitch/roll 组合出 `transform`
        //（`EquirectangularPanorama.js` L46-61），即一个刚体 transform：一个正交归一
        // 3x3 加上以米为单位的平移。只有平移需要重新缩放。
        let mut world_from_local = panorama.transform;
        world_from_local.w_axis.x /= PANORAMA_METERS_PER_RENDER_UNIT;
        world_from_local.w_axis.y /= PANORAMA_METERS_PER_RENDER_UNIT;
        world_from_local.w_axis.z /= PANORAMA_METERS_PER_RENDER_UNIT;

        // `DMat4::inverse()` 对奇异输入（例如零缩放）返回一个填满 NaN/inf 的矩阵
        // 而非报错，那会悄无声息地污染 uniform 并使 panorama 变白。以
        // 有限、良态的行列式做守卫；失败时回退到单位矩阵（不对 panorama
        // 做变换地渲染）并发出 warn，而非传播 NaN。
        let det = world_from_local.determinant();
        let world_from_local_inv = if det.is_finite() && det.abs() > 1.0e-12 {
            world_from_local.inverse()
        } else {
            bevy::log::warn!(
                "panorama: singular / non-finite transform (det={det}); falling back to identity"
            );
            DMat4::IDENTITY
        };

        Self {
            enabled: panorama.show,
            placement: panorama.placement(),
            source: panorama.source(),
            image,
            brightness,
            transform: f32_mat4(world_from_local_inv),
            center: f32_vec3(panorama.center_render_units()),
            radius: panorama.radius_render_units() as f32,
            repeat: f32_vec2_from_dvec2(panorama.texture_repeat()),
        }
    }

    /// 从领域 [`CubeMapPanorama`] 构建组件。
    ///
    /// 上游的 cube-map transform 是一个 **`Matrix3`**——skybox 有朝向
    /// 但无位置——所以只搬运 [`CubeMapPanorama::orientation`]，
    /// [`Self::center`] / [`Self::radius`] 保持为零。
    pub fn from_domain_cubemap(
        panorama: &CubeMapPanorama,
        image: Handle<Image>,
        brightness: f32,
    ) -> Self {
        Self {
            enabled: panorama.show,
            placement: panorama.placement(),
            source: panorama.source(),
            image,
            brightness,
            transform: Mat4::from_mat3(f32_mat3(panorama.orientation().inverse())),
            center: Vec3::ZERO,
            radius: 0.0,
            repeat: Vec2::ONE,
        }
    }
}

/// 逐视图缓存的 pipeline id，对应 `super::fxaa::CameraFxaaPipeline`。
#[derive(Component)]
pub struct CameraPanoramaPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// 逐视图 bind group 加上其后端的 uniform 缓冲。
///
/// 刻意把缓冲与 bind group 一起存储：`UniformBuffer::binding` 交出一个借用
/// GPU 缓冲的 `BufferBinding`，而在 render pass 消费 bind group 之前 drop 掉
/// `UniformBuffer` 会释放对它的最后一个 Rust 侧 handle。
#[derive(Component)]
pub struct CameraPanoramaBindGroup {
    pub bind_group: BindGroup,
    pub uniforms: UniformBuffer<PanoramaUniforms>,
}

// ─── Uniforms ────────────────────────────────────────────────────────────────

/// `shaders/panorama.wgsl` 的 `struct PanoramaUniforms`——112 字节。
///
/// 字段顺序和 padding 就是契约；shader 侧的布局表在该文件的头里。
/// `center` 是 `Vec3`（align 16，size 12），所以 `Mat4` 前需要 `_pad_c`。
///
/// 该 struct 位于一个带 `#![allow(dead_code)]` 的私有 `panorama_uniform` 模块中
///——即 `sky_dome.rs` / `clipping_planes.rs` / `ibl.rs` 的约定：encase
/// `ShaderType` derive 会发出一个模块级 `check` helper，dead-code pass 会标记它，
/// 尽管每个字段都通过 `write_buffer` 上传。偏移由
/// [`tests::the_uniform_layout_matches_the_wgsl_struct_field_for_field`] 逐字段
/// 对照 WGSL 文本钉死。
pub use panorama_uniform::PanoramaUniforms;

mod panorama_uniform {
    #![allow(dead_code)]
    use bevy::prelude::{Mat4, Vec2, Vec3};
    use bevy::render::render_resource::ShaderType;

    /// GPU panorama uniform；布局匹配 `shaders/panorama.wgsl` 中的
    /// `struct PanoramaUniforms`（encase std140，112 字节）。
    #[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
    pub struct PanoramaUniforms {
        /// [`super::MODE_SKYBOX`] 或 [`super::MODE_BUBBLE`]。
        pub mode: u32,
        /// [`super::SOURCE_CUBEMAP`] 或 [`super::SOURCE_EQUIRECTANGULAR`]。
        pub source: u32,
        /// 辐射度乘子。
        pub brightness: f32,
        /// bubble 半径，以 render 单位表示。
        pub radius: f32,
        /// `(-repeat_horizontal, repeat_vertical)`.
        pub repeat: Vec2,
        /// 将 `center` 对齐到 16 字节。
        pub _pad_b: Vec2,
        /// bubble 中心，以世界 render 单位表示。
        pub center: Vec3,
        /// 将 `transform` 对齐到 16 字节。
        pub _pad_c: u32,
        /// world → local 的 panorama 变换。
        pub transform: Mat4,
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：bind group layout、采样器，以及让一个 layout 服务全部
/// 四种 mode × source 组合的两个占位纹理 view。
#[derive(Resource)]
pub struct PanoramaPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub sampler: GpuSampler,
    /// 当活跃 source 为 equirectangular 时绑定进 `texture_cube` 槽。
    pub placeholder_cube_view: TextureView,
    /// 当活跃 source 为 cube map 时绑定进 `texture_2d` 槽。
    pub placeholder_flat_view: TextureView,
    /// 在该资源的整个生命周期内保持占位 cube 的 GPU 缓冲存活。
    placeholder_cube: Texture,
    /// 保持占位平面纹理的 GPU 缓冲存活。
    placeholder_flat: Texture,
}

impl PanoramaPipeline {
    /// 每个面 1×1 一个 texel，`Rgba8UnormSrgb`（sRGB 格式，所以满足 cube 槽的
    /// `Float { filterable: true }` sample 类型）。
    fn placeholder_cube(device: &RenderDevice) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("cesium_panorama_placeholder_cube"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("cesium_panorama_placeholder_cube_view"),
            dimension: Some(TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });
        (texture, view)
    }

    /// 1×1 一个 texel，`Rgba8UnormSrgb`（项目红线对 sRGB panorama 颜色纹理
    /// 所要求的格式）。
    fn placeholder_flat(device: &RenderDevice) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("cesium_panorama_placeholder_flat"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("cesium_panorama_placeholder_flat_view"),
            ..Default::default()
        });
        (texture, view)
    }
}

impl FromWorld for PanoramaPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_panorama_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_cube(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    // view uniform 也被 fragment 阶段读取（bubble 深度 + 光线重建），
                    // 且 Bevy 以动态方式绑定它。
                    uniform_buffer::<ViewUniform>(true)
                        .visibility(ShaderStages::VERTEX_FRAGMENT),
                    // **不**动态。`prepare_panorama_bind_groups` 每相机写入一个普通
                    // `UniformBuffer`（`uniform_buffer.binding()`，无动态 offset），
                    // 所以 `set_bind_group` 恰好提供一个动态 offset——view uniform 的那个。
                    // 把此 binding 声明为动态让 wgpu 期望 2 个 offset 并在
                    // `RenderPass::end` 处校验失败（"BindGroup with
                    // 'cesium_panorama_bind_group' label 0 expects 2 dynamic
                    // offsets. However 1 dynamic offset were provided."），由首次
                    // 带 `CESIUM_ENABLE_PANORAMA=1` 的真实 GPU 运行发现（task #81）。
                    // 与同级节点一致：`ibl.rs` L307 和 `clipping_planes.rs` L293
                    // 都用 `(false)`。
                    uniform_buffer::<PanoramaUniforms>(false),
                ),
            ),
        );

        // `AddressMode::Repeat` 是强制的：`PanoramaUniforms::repeat` 携带上游的
        // 负水平分量，所以采样的 u 对每个方向都为负，必须恰如 GL_REPEAT 那样回绕。
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_panorama_sampler"),
            address_mode_u: bevy::render::render_resource::AddressMode::Repeat,
            address_mode_v: bevy::render::render_resource::AddressMode::Repeat,
            address_mode_w: bevy::render::render_resource::AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let (placeholder_cube, placeholder_cube_view) = Self::placeholder_cube(render_device);
        let (placeholder_flat, placeholder_flat_view) = Self::placeholder_flat(render_device);

        let pipeline = Self {
            bind_group_layout,
            sampler,
            placeholder_cube_view,
            placeholder_flat_view,
            placeholder_cube,
            placeholder_flat,
        };

        // 这两个 `Texture` 字段纯粹是为了保持 `placeholder_*_view` 背后的 GPU 对象
        // 存活：`wgpu::TextureView` 只持有 `Arc<C>` 和 `Box<Data>`
        //（wgpu-23.0.1/src/api/texture_view.rs L12-15）而**不**持有对其 `Texture` 的
        // 引用，而 drop 一个 `Texture` 会摧毁其后端资源——所以没有它们 view 就会悬垂。
        // 没有别的东西读这些字段，所以在此读它们把那条 keep-alive 不变式变成一个被校验
        // 的事实而非一个 `#[allow(dead_code)]`，且一个形状错误的占位（它会被绑定进
        // 一个其 sample 类型不满足的槽）会在每个 debug 构建和测试运行中响亮地失败。
        debug_assert_eq!(
            pipeline.placeholder_cube.depth_or_array_layers(),
            6,
            "the cube placeholder must have one layer per face"
        );
        debug_assert_eq!(
            pipeline.placeholder_flat.depth_or_array_layers(),
            1,
            "the equirectangular placeholder must be a single layer"
        );
        debug_assert_eq!(pipeline.placeholder_cube.width(), 1);
        debug_assert_eq!(pipeline.placeholder_flat.width(), 1);

        pipeline
    }
}

/// 特化 key。`placement` 是 key 的一部分，因为它是唯一改变 pipeline 状态的轴：
/// `Bubble` 写深度（有限球必须遮挡其后的透明绘制），而 `Skybox` 不写。
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct PanoramaPipelineKey {
    pub hdr: bool,
    pub samples: u32,
    pub depth_format: TextureFormat,
    pub placement: PanoramaPlacement,
}

impl SpecializedRenderPipeline for PanoramaPipeline {
    type Key = PanoramaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cesium_panorama_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: PANORAMA_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "panorama_vertex".into(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: key.depth_format,
                // BUBBLE 写入真实深度，所以 starfield（r = 50）和 sky dome
                //（r = 40）被一个有限 panorama 正确遮挡。SKYBOX 必须不写：它位于
                // 远平面，无论如何都遮挡不了任何东西，且不写入能让深度缓冲为随后的
                // 透明 pass 保持原样。
                depth_write_enabled: key.placement == PanoramaPlacement::Bubble,
                // 反向 Z：被清除的深度是 0.0（远平面），所以 GreaterEqual 只在不透明
                // pass 留下天空的地方放行 panorama。与 Bevy 自身的 skybox
                //（skybox/mod.rs L204）以及 `atmosphere/sky_dome.rs` 的 premultiplied dome 一致。
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: PANORAMA_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "panorama_fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        // ViewTarget 主纹理的格式——见偏差 5。
                        TextureFormat::bevy_default()
                    },
                    // `None` == REPLACE。上游的 `ALPHA_BLEND` 在
                    // `a = czm_morphTime = 1.0` 下退化得恰好就是此。
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

// ─── 渲染图 label ──────────────────────────────────────────────────────

/// panorama 绘制的 `Core3d` 节点 label。
///
/// 在此定义而非作为 `CesiumPostProcessLabel` 变体，因为那个 enum 位于
/// `effects/graph.rs`，由 M6 Wave A 集成任务（#81）拥有。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumPanoramaLabel;

/// task #81 必须添加的边，为集成报告完整拼出。
///
/// `effects::graph::insert_node_in_core3d(render_app, CesiumPanoramaLabel,
/// Node3d::MainOpaquePass, Node3d::MainTransmissivePass)`——关于为何是这个槽位
/// 而非其他，参见模块文档。
pub fn insertion_hint() -> String {
    let mut hint = String::new();
    let _ = write!(
        hint,
        "insert_node_in_core3d(render_app, CesiumPanoramaLabel, {:?}, {:?})",
        Node3d::MainOpaquePass,
        Node3d::MainTransmissivePass,
    );
    hint
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// panorama 绘制。运行于 `Node3d::MainOpaquePass` 与 `Node3d::MainTransmissivePass`
/// 之间；参见模块文档。
///
/// 设计上无状态：与 `super::fxaa::FxaaNode` 不同，没有东西需要缓存，
/// 因为 bind group 在 [`prepare_panorama_bind_groups`] 中逐帧重建
///（其内容取决于当前的 image 和 view uniform），且 pipeline 按 id 查找。
#[derive(Default)]
pub struct PanoramaNode;

impl ViewNode for PanoramaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static CameraPanoramaPipeline,
        &'static CameraPanoramaBindGroup,
        &'static ViewUniformOffset,
        &'static CesiumPanorama,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, depth, pipeline_handle, bind_group, view_uniform_offset, panorama): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !panorama.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        // 两个 attachment 都是本帧第二或更后的使用者，所以
        // `ColorAttachment::get_attachment` / `DepthAttachment::get_attachment`
        // 返回 `LoadOp::Load`（bevy_render-0.15.3/src/texture/texture_attachment.rs
        // L62-74, L102-106）。不透明 pass 已经完成了清除。
        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_panorama_pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group.bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个启用的视图准备特化的 panorama pipeline。
/// 运行于 `Render` 的 `RenderSet::Prepare`。
pub fn prepare_panorama_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PanoramaPipeline>>,
    pipeline: Res<PanoramaPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa, &CesiumPanorama)>,
) {
    for (entity, view, msaa, panorama) in &views {
        if !panorama.enabled {
            continue;
        }

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            PanoramaPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
                depth_format: CORE_3D_DEPTH_FORMAT,
                placement: panorama.placement,
            },
        );

        commands.entity(entity).insert(CameraPanoramaPipeline { pipeline_id });
    }
}

/// 构建逐视图 bind group。运行于 `Render` 的 `RenderSet::PrepareBindGroups`
///（在 `write_view_uniforms` 之后），与 Bevy 自身的 `prepare_skybox_bind_groups`
/// 所用的同一集合。
///
/// image 尚未常驻的视图会被**跳过**，这是上游一致性：cube map 仍在加载时
/// `CubeMapPanorama.js` L232 返回 `undefined`——完全不发出 draw command。
/// 被跳过的视图没有 [`CameraPanoramaBindGroup`]，所以 [`PanoramaNode`] 的 query
/// 不匹配它，什么都不绘制。
pub fn prepare_panorama_bind_groups(
    mut commands: Commands,
    pipeline: Res<PanoramaPipeline>,
    view_uniforms: Res<ViewUniforms>,
    images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &CesiumPanorama)>,
) {
    for (entity, panorama) in &views {
        if !panorama.enabled {
            continue;
        }

        let Some(gpu_image) = images.get(&panorama.image) else {
            continue;
        };

        // 活跃 source 决定真实 image 进入哪个槽；另一个槽取一个 1×1 占位，
        // 从而一个 bind group layout 服务全部四种 mode × source 组合。一个常驻
        // image 若其形状与其槽不匹配，则被视为"尚未加载"而非强行绑定——wgpu 会在
        // bind-group 创建时拒绝把 `D2` view 放进 `texture_cube` 槽，那会在 render
        // 线程上造成硬 panic。
        //
        // `GpuImage` 不携带 `Image::texture_view_dimension`，所以 cube 判据是
        // array-layer 数：一个 wgpu cube 纹理是恰好六层的 2D array 纹理，而一个普通
        // equirectangular image 只有一层。此处保守是上游一致性，而非 workaround——
        // `CubeMapPanorama.js` L232 同样在 cube map 完全常驻之前不发出任何 draw command。
        let dimension_matches = match panorama.source {
            PanoramaSource::CubeMap => gpu_image.texture.depth_or_array_layers() == 6,
            PanoramaSource::Equirectangular => gpu_image.texture.depth_or_array_layers() == 1,
        };
        if !dimension_matches {
            continue;
        }

        let Some(view_binding) = view_uniforms.uniforms.binding() else {
            continue;
        };

        let uniforms = PanoramaUniforms {
            mode: panorama.placement.as_u32(),
            source: panorama.source.as_u32(),
            brightness: panorama.brightness,
            radius: panorama.radius,
            repeat: panorama.repeat,
            _pad_b: Vec2::ZERO,
            center: panorama.center,
            _pad_c: 0,
            transform: panorama.transform,
        };
        let mut uniform_buffer = UniformBuffer::from(uniforms);
        uniform_buffer.write_buffer(&render_device, &render_queue);
        let Some(uniforms_binding) = uniform_buffer.binding() else {
            continue;
        };

        let (cube_view, flat_view): (&TextureView, &TextureView) = match panorama.source {
            PanoramaSource::CubeMap => (&gpu_image.texture_view, &pipeline.placeholder_flat_view),
            PanoramaSource::Equirectangular => {
                (&pipeline.placeholder_cube_view, &gpu_image.texture_view)
            }
        };

        let bind_group = render_device.create_bind_group(
            Some("cesium_panorama_bind_group"),
            &pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                BindingResource::TextureView(cube_view),
                BindingResource::TextureView(flat_view),
                BindingResource::Sampler(&pipeline.sampler),
                view_binding,
                uniforms_binding,
            )),
        );

        commands.entity(entity).insert(CameraPanoramaBindGroup {
            bind_group,
            uniforms: uniform_buffer,
        });
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 把 panorama 节点注册进 `RenderApp`（shader + extract + 节点 + 系统）。
///
/// 仅当 [`ENV_ENABLE_PANORAMA`] 为真时由 cesium-app 的 `main.rs` 调用——
/// 与 `atmosphere::CesiumAtmospherePlugin` 和 `register_fxaa_node` 同一形态。
///
/// 本函数注册节点**但不创建图边**：
/// `effects::graph::register_m6_render_graph` 拥有单一线性 `Core3d`
/// 链（Daniel H2 / Lee M6.3 菱形告警）。task #81 完成接线——实际
/// 产出的形态参见 [`insertion_hint`]。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_panorama_node_main_world` from `Plugin::build` and `register_panorama_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_panorama_node(app: &mut App) {
    register_panorama_node_main_world(app);
    // 无头 `MinimalPlugins` 没有 `RenderApp`——优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_panorama_node_render_world(render_app);
    }
}

/// [`register_panorama_node`] 在 `Plugin::build` 时执行的半：所有活在
/// **主** world 中的东西（WGSL shader 资产 + `ExtractComponentPlugin`）。
///
/// 由 task #81 拆出——见 `docs/deviations.md#dev-029`。`PanoramaPipeline` 的
/// `FromWorld`（L471）读取 `RenderDevice`，而 Bevy 只在 `RenderPlugin::finish`
///（`bevy_render/src/lib.rs` L399-430）把它插入 render world，
/// 所以下面的 render-world 半必须从 plugin 的 `finish` 运行——从 `build`
/// 调用它会以 "RenderDevice does not exist in the World" panic。
pub fn register_panorama_node_main_world(app: &mut App) {
    // 无头安全：当 `Assets<Shader>` 缺失时是 no-op，而非
    // `load_internal_asset!` panic（docs/deviations.md#dev-005）。
    crate::shader_registry::try_load_internal_shader(
        app,
        PANORAMA_SHADER_HANDLE,
        include_str!("../../shaders/panorama.wgsl"),
        "shaders/panorama.wgsl",
    );

    app.add_plugins(ExtractComponentPlugin::<CesiumPanorama>::default());
}

/// [`register_panorama_node`] 在 `Plugin::finish` 时执行的半：render-world
/// pipeline 资源 + `Core3d` 节点。
pub fn register_panorama_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半从 `build` 到达，或一个裸 render world）。见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<PanoramaPipeline>()
        .init_resource::<SpecializedRenderPipelines<PanoramaPipeline>>()
        .add_systems(
            Render,
            (
                prepare_panorama_pipelines.in_set(RenderSet::Prepare),
                prepare_panorama_bind_groups.in_set(RenderSet::PrepareBindGroups),
            ),
        )
        .add_render_graph_node::<ViewNodeRunner<PanoramaNode>>(Core3d, CesiumPanoramaLabel);

    // 注意：边由 `effects::graph::wire_m6_edges`（task #81）创建——见
    // `insertion_hint()`。
}

// ─── f64 → f32 边界 helper ──────────────────────────────────────────────

/// 把领域 `DMat3` 收窄为 uniform 缓冲所需的 `Mat3`。
///
/// 列主序，与 glam 的 `from_cols_array` 和 WGSL 的 `mat3x3` 布局一致。
pub fn f32_mat3(value: DMat3) -> Mat3 {
    Mat3::from_cols_array(&[
        value.x_axis.x as f32,
        value.x_axis.y as f32,
        value.x_axis.z as f32,
        value.y_axis.x as f32,
        value.y_axis.y as f32,
        value.y_axis.z as f32,
        value.z_axis.x as f32,
        value.z_axis.y as f32,
        value.z_axis.z as f32,
    ])
}

/// 把领域 `DMat4` 收窄为 uniform 缓冲所需的 `Mat4`。
pub fn f32_mat4(value: DMat4) -> Mat4 {
    Mat4::from_cols_array(&[
        value.x_axis.x as f32,
        value.x_axis.y as f32,
        value.x_axis.z as f32,
        value.x_axis.w as f32,
        value.y_axis.x as f32,
        value.y_axis.y as f32,
        value.y_axis.z as f32,
        value.y_axis.w as f32,
        value.z_axis.x as f32,
        value.z_axis.y as f32,
        value.z_axis.z as f32,
        value.z_axis.w as f32,
        value.w_axis.x as f32,
        value.w_axis.y as f32,
        value.w_axis.z as f32,
        value.w_axis.w as f32,
    ])
}

/// 把领域 `DVec3` 收窄为 render-unit `Vec3`。
pub fn f32_vec3(value: DVec3) -> Vec3 {
    Vec3::new(value.x as f32, value.y as f32, value.z as f32)
}

/// 把领域 `DVec2`（此处：纹理 repeat）收窄为 `Vec2`。
pub fn f32_vec2_from_dvec2(value: glam::DVec2) -> Vec2 {
    Vec2::new(value.x as f32, value.y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::graph::CesiumPostProcessLabel;
    use crate::resources::METERS_PER_RENDER_UNIT;
    use naga::valid::{Capabilities, ValidationFlags, Validator};
    use std::collections::BTreeSet;

    // ─── naga 防线（M5 Ultra Review 发现 C1 / C2）────────────────────

    /// 对 `#import bevy_render::view::View` 的替身。
    ///
    /// 对 `panorama.wgsl` 读取的每个字段都是名称与类型忠实的
    ///（`view_from_clip`、`clip_from_view`、`world_from_view`、`view_from_world`、
    /// `world_position`、`viewport`——均存在于
    /// `bevy_render-0.15.3/src/view/view.wgsl` L17-27），但*不是*偏移忠实的：
    /// naga 只做类型检查，不知道真实缓冲布局，而运行时
    /// Bevy 提供真正的 `View`。
    const PANORAMA_WGSL_IMPORT_STUBS: &str = "\
struct View {
    view_from_clip: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    world_position: vec3<f32>,
    viewport: vec4<f32>,
}
";

    /// 把 `#import` 行替换为 [`PANORAMA_WGSL_IMPORT_STUBS`] 后的 shader 源码。
    fn panorama_stubbed_wgsl() -> String {
        let mut source = String::from(PANORAMA_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/panorama.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    fn panorama_wgsl_source() -> &'static str {
        include_str!("../../shaders/panorama.wgsl")
    }

    /// 用于**多行** `contains` 断言的行尾归一化。
    ///
    /// `include_str!` 嵌入文件在磁盘上的任意行尾，而本 checkout 在
    /// Windows 上，两个文件都是 CRLF。因此用 `\n` 写的 needle 从不
    /// 匹配一个字节一致的源文件。`str::lines()` 已会剔去尾部
    /// 的 `\r`，这就是为何本模块中面向行的断言从不需要 helper——
    /// 只有跨行换行的那些才需要。
    fn normalized(source: &str) -> String {
        source.replace("\r\n", "\n")
    }

    /// 从 shader 源码中解析一个 `const NAME: f32 = LITERAL;`。
    fn wgsl_f32_const(source: &str, name: &str) -> f32 {
        let prefix = format!("const {name}: f32 = ");
        let tail = source
            .split(prefix.as_str())
            .nth(1)
            .unwrap_or_else(|| panic!("shaders/panorama.wgsl has no `const {name}: f32 = ...`"));
        let literal = tail
            .split(';')
            .next()
            .unwrap_or_else(|| panic!("unterminated `const {name}` literal"));
        literal
            .trim()
            .parse::<f32>()
            .unwrap_or_else(|error| panic!("`const {name}` literal {literal:?} is not an f32: {error}"))
    }

    /// 从 shader 源码中解析一个 `const NAME: u32 = LITERALu;`。
    fn wgsl_u32_const(source: &str, name: &str) -> u32 {
        let prefix = format!("const {name}: u32 = ");
        let tail = source
            .split(prefix.as_str())
            .nth(1)
            .unwrap_or_else(|| panic!("shaders/panorama.wgsl has no `const {name}: u32 = ...`"));
        let literal = tail
            .split(';')
            .next()
            .unwrap_or_else(|| panic!("unterminated `const {name}` literal"));
        literal
            .trim()
            .trim_end_matches('u')
            .parse::<u32>()
            .unwrap_or_else(|error| panic!("`const {name}` literal {literal:?} is not a u32: {error}"))
    }

    /// C1 类回归：新 WGSL 文件必须在 Bevy 编译所用的确切前端下
    /// **同时**通过解析**和**类型检查。
    #[test]
    fn panorama_wgsl_parses_and_type_checks_under_naga() {
        let source = panorama_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!(
                "panorama.wgsl does not parse:\n{}",
                error.emit_to_string(&source)
            )
        });

        // `ValidationFlags::all()` 包含 uniformity 分析，正是它使
        // `textureSample` 周围的 `uniforms.mode` / `uniforms.source` 分支合法——
        // 那里的非一致分支是硬错误，不是警告。
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("panorama.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![
                ("panorama_vertex", naga::ShaderStage::Vertex),
                ("panorama_fragment", naga::ShaderStage::Fragment),
            ],
            "the Rust side asks for exactly these two entry points"
        );
    }

    /// C2 类回归：entry point 实际触及的每个全局变量必须在 Rust bind group
    /// 布局中有一个槽位，否则 pipeline 会静默地绑定乱码。
    #[test]
    fn panorama_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = panorama_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("panorama.wgsl does not parse:\n{error}"));
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("panorama.wgsl does not validate");

        // 对应 `PanoramaPipeline::from_world`：cube、flat、sampler、view、uniforms。
        let layout: BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)].into_iter().collect();

        let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();

        // 遍历**整个 module**，而非只遍历 entry points。`panorama_fragment`
        // 只通过它调用的 `sample_panorama` helper 触及 `panorama_cube` /
        // `panorama_equirect` / `panorama_sampler`，而 naga 把每个函数的
        // 表达式存在它自己的 arena 里。只扫描 entry points 会把 `used`
        // 恰好少报那三个槽位，从而使下面的“无槽位被付费却未使用”
        // 断言在那些确实被使用的绑定上失败。
        let visit = |expressions: &naga::Arena<naga::Expression>,
                     used: &mut BTreeSet<(u32, u32)>| {
            for (_, expression) in expressions.iter() {
                if let naga::Expression::GlobalVariable(global) = expression {
                    if let Some(binding) = &module.global_variables[*global].binding {
                        used.insert((binding.group, binding.binding));
                    }
                }
            }
        };
        for (_, function) in module.functions.iter() {
            visit(&function.expressions, &mut used);
        }
        for entry in &module.entry_points {
            visit(&entry.function.expressions, &mut used);
        }

        let uncovered: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            uncovered.is_empty(),
            "panorama.wgsl reads bindings {uncovered:?} that PanoramaPipeline never binds \
             (C2-class regression: the pipeline would validate but sample garbage)"
        );

        // 反向同理：无槽位被付费却未使用。
        let unused: Vec<(u32, u32)> = layout.difference(&used).copied().collect();
        assert!(
            unused.is_empty(),
            "PanoramaPipeline binds slots {unused:?} that no entry point reads"
        );
    }

    /// shader 的 mode/source 字面量必须等于领域 enum 的判别值，
    /// 否则 `PanoramaUniforms` 会在 GPU 上选错分支。
    #[test]
    fn the_wgsl_mode_and_source_literals_match_the_domain_discriminants() {
        let source = panorama_wgsl_source();

        assert_eq!(wgsl_u32_const(source, "MODE_SKYBOX"), MODE_SKYBOX);
        assert_eq!(wgsl_u32_const(source, "MODE_BUBBLE"), MODE_BUBBLE);
        assert_eq!(wgsl_u32_const(source, "SOURCE_CUBEMAP"), SOURCE_CUBEMAP);
        assert_eq!(
            wgsl_u32_const(source, "SOURCE_EQUIRECTANGULAR"),
            SOURCE_EQUIRECTANGULAR
        );

        // 且它们来自领域，而非本地重新发明。
        assert_eq!(MODE_SKYBOX, PanoramaPlacement::Skybox.as_u32());
        assert_eq!(MODE_BUBBLE, PanoramaPlacement::Bubble.as_u32());
        assert_eq!(SOURCE_CUBEMAP, PanoramaSource::CubeMap.as_u32());
        assert_eq!(
            SOURCE_EQUIRECTANGULAR,
            PanoramaSource::Equirectangular.as_u32()
        );
        assert_eq!(MODE_SKYBOX, CubeMapPanorama::default().placement().as_u32());
        assert_eq!(
            MODE_BUBBLE,
            EquirectangularPanorama::default().placement().as_u32()
        );
    }

    /// 对 shader 与 Rust 共享的每个 f32 常量做位一致交叉校验。
    ///
    /// 字面量是**从 shader 源码中解析**而非从领域的 f64 常量 cast，
    /// 所以双重舍入无法掩盖不一致。
    #[test]
    fn the_wgsl_literals_match_the_rust_mirrors_bit_for_bit() {
        let source = panorama_wgsl_source();

        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_PI").to_bits(),
            std::f32::consts::PI.to_bits(),
            "the shader's PI must be the correctly-rounded f32 pi"
        );
        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_TAU").to_bits(),
            std::f32::consts::TAU.to_bits()
        );
        assert_eq!(
            wgsl_f32_const(source, "PANORAMA_HALF_PI").to_bits(),
            std::f32::consts::FRAC_PI_2.to_bits()
        );
        assert_eq!(
            wgsl_f32_const(source, "DEGENERATE_DIRECTION_SQUARED_EPSILON").to_bits(),
            PANORAMA_DEGENERATE_DIRECTION_SQUARED_EPSILON_F32.to_bits()
        );

        // 领域的 f64 epsilon 是同一十进制值，所以两个护栏拒绝同一类
        // 输入，即使它们存活于不同精度。
        assert_eq!(
            cesium_effects::panorama::DEGENERATE_DIRECTION_SQUARED_EPSILON,
            1.0e-24
        );
    }

    // ─── 门控 ────────────────────────────────────────────────────────────────

    /// 门控必须默认 OFF，以便黄金路径保持像素中性。
    ///
    /// 在 env 变量移除的情况下运行；`std::env::remove_var` 在本 toolchain 上
    /// 非 unsafe，但并*不*线程安全，所以断言写为容忍一个并行测试已将其
    /// 设置——真正重要的不变量是：未设置或空值都读为 OFF。
    #[test]
    fn an_unset_or_empty_gate_reads_as_off() {
        use crate::pipeline::fetch::gate_from_env_value;

        assert!(!gate_from_env_value(None));
        assert!(!gate_from_env_value(Some(String::new())));
        assert!(!gate_from_env_value(Some("   ".to_string())));
        assert!(!gate_from_env_value(Some("0".to_string())));
        assert!(!gate_from_env_value(Some("false".to_string())));
        assert!(!gate_from_env_value(Some("off".to_string())));
        assert!(!gate_from_env_value(Some("no".to_string())));

        assert!(gate_from_env_value(Some("1".to_string())));
        assert!(gate_from_env_value(Some("TRUE".to_string())));
        assert!(gate_from_env_value(Some(" yes ".to_string())));
        assert!(gate_from_env_value(Some("on".to_string())));

        assert_eq!(ENV_ENABLE_PANORAMA, "CESIUM_ENABLE_PANORAMA");

        // `CesiumPanorama::default()` 即使 plugin 已注册也是禁用，
        // 所以一个孤立组件不会意外打开绘制。
        assert!(!CesiumPanorama::default().enabled);
    }

    // ─── 渲染顺序 ────────────────────────────────────────────────────────

    /// 图 label 必须唯一，否则 `add_render_graph_node` 会静默覆盖现有节点的 runner。
    #[test]
    fn the_panorama_label_collides_with_no_existing_label() {
        let panorama = format!("{CesiumPanoramaLabel:?}");
        assert_eq!(panorama, "CesiumPanoramaLabel");

        for existing in [
            format!("{:?}", CesiumPostProcessLabel::PassThrough),
            format!("{:?}", CesiumPostProcessLabel::Fxaa),
            format!("{:?}", CesiumPostProcessLabel::AmbientOcclusion),
        ] {
            assert_ne!(
                panorama, existing,
                "the panorama label must not shadow a post-process label"
            );
        }

        for builtin in [
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
        ] {
            assert_ne!(
                panorama,
                format!("{builtin:?}"),
                "the panorama label must not shadow the built-in {builtin:?}"
            );
        }

        // 文档中记录的插入点，以便集成任务有一个机器可校验的
        // 字符串而非 prose。
        let hint = insertion_hint();
        assert!(hint.contains("CesiumPanoramaLabel"), "{hint}");
        assert!(hint.contains("MainOpaquePass"), "{hint}");
        assert!(hint.contains("MainTransmissivePass"), "{hint}");
    }

    /// `Skybox` 与 `Bubble` 不得共享一个 pipeline：只有 `Bubble` 写深度。
    ///
    /// 这故意是**键级**而非 descriptor 级。`PanoramaPipeline`
    /// 持有一个 `BindGroupLayout`、一个 `GpuSampler` 和两个占位 `Texture`，
    /// 没有 `RenderDevice` 都无法伪造——而 `MinimalPlugins` 下 `RenderDevice`
    /// 并不存在（M5 Ultra Review / Robin #80 的无头工作提出同一观点）。
    /// 特化一个假 pipeline 会 `panic!`，所以 descriptor 事实改由
    /// [`the_pipeline_descriptor_shape_is_pinned_by_source`] 在无设备下钉住，
    /// 它断言 `specialize` 发出的确切源码行。
    ///
    /// *能*在无头下证明、且是真正正确性要求的，是两个放置产生
    /// **不同的 key**：`SpecializedRenderPipelines` 按 key 的 `Hash + Eq`
    /// 缓存，所以相等的 key 会把一个写深度的 pipeline 交给 skybox（或把
    /// 一个盲深度的交给 bubble）从而破坏帧。
    #[test]
    fn the_pipeline_key_separates_the_placements_because_depth_write_differs() {
        let skybox = PanoramaPipelineKey {
            hdr: false,
            samples: 1,
            depth_format: CORE_3D_DEPTH_FORMAT,
            placement: PanoramaPlacement::Skybox,
        };
        let bubble = PanoramaPipelineKey {
            placement: PanoramaPlacement::Bubble,
            ..skybox
        };
        assert_ne!(
            skybox, bubble,
            "placement is a specialization axis; equal keys would share one pipeline"
        );

        // Hash 也必须分隔它们，而不仅是 Eq：`SpecializedRenderPipelines`
        // 透过一个 `HashMap` 查找。
        let hash_of = |key: &PanoramaPipelineKey| {
            let mut state = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(key, &mut state);
            std::hash::Hasher::finish(&state)
        };
        assert_ne!(
            hash_of(&skybox),
            hash_of(&bubble),
            "two placements that hash identically would collide in the pipeline cache"
        );

        // 且其余两个轴保持独立，所以 key 不会意外地退化。
        assert_ne!(
            skybox,
            PanoramaPipelineKey { hdr: true, ..skybox },
            "HDR is an independent axis (ViewTarget::TEXTURE_FORMAT_HDR vs bevy_default)"
        );
        assert_ne!(
            skybox,
            PanoramaPipelineKey { samples: 4, ..skybox },
            "MSAA sample count is an independent axis"
        );
        assert_ne!(
            skybox,
            PanoramaPipelineKey {
                // 不是 `Depth32Float`：在 Bevy 0.15 中那**就是** `CORE_3D_DEPTH_FORMAT`
                //（反向 Z 需要一个浮点深度缓冲），所以它会与 `skybox` 相等
                // 而使断言什么也测不到。
                depth_format: TextureFormat::Depth24PlusStencil8,
                ..skybox
            },
            "depth format is an independent axis"
        );
        assert_eq!(
            CORE_3D_DEPTH_FORMAT,
            TextureFormat::Depth32Float,
            "if Bevy ever changes the core_3d depth format, the assertion above has to \
             pick a different counter-example"
        );

        // key 所携带的 placement 判别值就是 shader 分支依据的同一 wire
        // 值，所以 `PanoramaPlacement` 的重排不会静默地互换两个 pipeline。
        assert_eq!(PanoramaPlacement::Skybox.as_u32(), MODE_SKYBOX);
        assert_eq!(PanoramaPlacement::Bubble.as_u32(), MODE_BUBBLE);
    }

    /// 对上述 GPU 支撑的测试在 `MinimalPlugins` 下无法触及的 descriptor 事实做无设备
    /// 钉住：特化源码文本本身。
    #[test]
    fn the_pipeline_descriptor_shape_is_pinned_by_source() {
        let source = normalized(include_str!("panorama.rs"));
        let source = source.as_str();

        assert!(
            source.contains("depth_write_enabled: key.placement == PanoramaPlacement::Bubble"),
            "depth write must stay tied to the placement axis"
        );
        assert!(
            source.contains("depth_compare: CompareFunction::GreaterEqual"),
            "reversed-Z sky selection must stay GreaterEqual"
        );
        assert!(
            source.contains("depth_format: CORE_3D_DEPTH_FORMAT"),
            "the depth format must come from the core_3d constant, not be hardcoded"
        );
        assert!(
            source.contains("entry_point: \"panorama_vertex\".into()")
                && source.contains("entry_point: \"panorama_fragment\".into()"),
            "the pipeline must ask for the two entry points the shader defines"
        );
        assert!(
            source.contains("draw(0..3, 0..1)"),
            "the fullscreen triangle must stay three vertices, one instance"
        );
        assert!(
            source.contains("TextureFormat::bevy_default()"),
            "the non-HDR colour format must match the ViewTarget main texture"
        );
        assert!(
            source.contains("AddressMode::Repeat"),
            "the sampler must wrap: upstream's negative horizontal repeat makes u negative"
        );
        assert!(
            source.contains("blend: None"),
            "upstream's ALPHA_BLEND degenerates to REPLACE at czm_morphTime == 1.0; \
             Bevy documents None as both faster and equivalent (skybox/mod.rs L232-233)"
        );
        assert!(
            source.contains("if key.hdr {\n                        ViewTarget::TEXTURE_FORMAT_HDR"),
            "the colour format must track ViewTarget, never a hardcoded TextureFormat"
        );
    }

    // ─── 领域 → 适配层映射 ────────────────────────────────────────────

    /// 缩放常量必须与适配层自己的常量一致，否则每个 panorama 半径
    /// 和中心都会错一个比例。
    #[test]
    fn panorama_meters_per_render_unit_matches_the_adapter_constant() {
        assert_eq!(PANORAMA_METERS_PER_RENDER_UNIT, 6_378_137.0);
        assert_eq!(
            PANORAMA_METERS_PER_RENDER_UNIT as f32,
            METERS_PER_RENDER_UNIT as f32,
            "domain and adapter must share the metres-per-render-unit scale"
        );
    }

    /// FIX-PANO-INVERSE：一个奇异（零缩放）的领域 transform 不得用 NaN
    /// 污染 uniform——护栏改为回退到单位阵。
    #[test]
    fn singular_transform_falls_back_to_identity_instead_of_nan() {
        let mut broken = EquirectangularPanorama::new("panorama.jpg");
        broken.transform = DMat4::from_scale(DVec3::ZERO); // 行列式 == 0
        let component =
            CesiumPanorama::from_domain_equirectangular(&broken, Handle::default(), 1.0);
        for col in component.transform.to_cols_array_2d() {
            assert!(col.iter().all(|v| v.is_finite()), "singular transform leaked NaN into the uniform");
        }
        assert_eq!(component.transform, Mat4::IDENTITY);

        // 一个良条件的刚体 transform 仍能正确求逆。
        let mut ok = EquirectangularPanorama::new("panorama.jpg");
        ok.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let ok_component = CesiumPanorama::from_domain_equirectangular(&ok, Handle::default(), 1.0);
        assert_ne!(ok_component.transform, Mat4::IDENTITY, "a real rotation must not be dropped");
    }

    /// f64 在领域里保持 f64；收窄发生在此处且仅在此处。
    #[test]
    fn the_domain_constructors_narrow_to_f32_only_at_the_uniform_boundary() {
        // Equirectangular：100 km bubble 锚定在 +Z 轴上 1 个 render unit 处。
        let anchored = EquirectangularPanorama::with_transform(
            DMat4::from_translation(DVec3::new(0.0, 0.0, PANORAMA_METERS_PER_RENDER_UNIT)),
            "panorama.jpg",
        );
        let component =
            CesiumPanorama::from_domain_equirectangular(&anchored, Handle::default(), 1.0);

        assert!(component.enabled, "`show` defaults to true upstream");
        assert_eq!(component.placement, PanoramaPlacement::Bubble);
        assert_eq!(component.source, PanoramaSource::Equirectangular);
        assert_eq!(component.center, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            (component.radius - 0.0156787).abs() < 1.0e-6,
            "100 km must be ~0.015678 render units, got {}",
            component.radius
        );
        // 上游 L117：repeat = (-repeatHorizontal, repeatVertical)。
        assert_eq!(component.repeat, Vec2::new(-1.0, 1.0));

        // `transform` 是 world→local，所以它求逆领域 transform。
        let round_trip = component.transform.inverse();
        assert!(
            (round_trip.w_axis - Vec4::new(0.0, 0.0, 1.0, 1.0)).length() < 1.0e-6,
            "{}",
            round_trip
        );

        // 一个航向旋转必须在往返中存活。
        let mut oriented = EquirectangularPanorama::new("panorama.jpg");
        oriented.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let oriented_component =
            CesiumPanorama::from_domain_equirectangular(&oriented, Handle::default(), 2.0);
        assert_eq!(oriented_component.brightness, 2.0);
        // Rz(+90°) 的 world→local 是 Rz(-90°)：+X 映射到 -Y。
        let mapped = oriented_component.transform.transform_vector3(Vec3::X);
        assert!(
            (mapped - Vec3::new(0.0, -1.0, 0.0)).length() < 1.0e-6,
            "{mapped}"
        );

        // CubeMap：只有朝向，绝无位置（上游 Matrix3）。
        let mut cube = CubeMapPanorama::new([
            "px.jpg".into(),
            "nx.jpg".into(),
            "py.jpg".into(),
            "ny.jpg".into(),
            "pz.jpg".into(),
            "nz.jpg".into(),
        ]);
        cube.transform = DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2)
            * DMat4::from_translation(DVec3::new(1.0e9, 2.0e9, 3.0e9));
        let cube_component = CesiumPanorama::from_domain_cubemap(&cube, Handle::default(), 1.0);
        assert_eq!(cube_component.placement, PanoramaPlacement::Skybox);
        assert_eq!(cube_component.source, PanoramaSource::CubeMap);
        assert_eq!(
            cube_component.center,
            Vec3::ZERO,
            "a cube-map skybox is camera-centred: the domain translation must be dropped"
        );
        assert_eq!(cube_component.radius, 0.0);
        assert_eq!(cube_component.repeat, Vec2::ONE);
        let cube_mapped = cube_component.transform.transform_vector3(Vec3::X);
        assert!(
            (cube_mapped - Vec3::new(0.0, -1.0, 0.0)).length() < 1.0e-6,
            "{cube_mapped}"
        );
        // 旋转部分在收窄后仍保持标准正交。
        let basis = cube_component.transform;
        assert!((basis.x_axis.truncate().length() - 1.0).abs() < 1.0e-6);
        assert!((basis.y_axis.truncate().length() - 1.0).abs() < 1.0e-6);
        assert!((basis.z_axis.truncate().length() - 1.0).abs() < 1.0e-6);

        // `show = false` 会传播，所以隐藏 panorama 永不抵达 GPU。
        let mut hidden = EquirectangularPanorama::new("panorama.jpg");
        hidden.show = false;
        assert!(!CesiumPanorama::from_domain_equirectangular(&hidden, Handle::default(), 1.0).enabled);
        let hidden_cube = CubeMapPanorama {
            show: false,
            ..Default::default()
        };
        assert!(!CesiumPanorama::from_domain_cubemap(&hidden_cube, Handle::default(), 1.0).enabled);
    }

    /// uniform 结构必须是 112 字节且带文档中的字段偏移，否则 shader
    /// 会读到 Rust 侧写入的不同字段。
    ///
    /// `encase` **不是**本 crate 的直接依赖（Bevy 只重新导出
    /// `ShaderType` derive，而非 `encase::internal::SizeValue`），所以此处从 WGSL
    /// 文本用 uniform 地址空间的对齐规则推出大小——无论如何这是更强的
    /// 校验，因为它验证的是 *shader* 声明的布局，而非信任 derive 宏已产出它。
    #[test]
    fn the_uniform_layout_matches_the_wgsl_struct_field_for_field() {
        let source = panorama_wgsl_source();

        let struct_body = source
            .split("struct PanoramaUniforms {")
            .nth(1)
            .expect("shaders/panorama.wgsl must define struct PanoramaUniforms");
        let struct_body = struct_body.split('}').next().expect("unterminated struct");
        let wgsl_fields: Vec<(&str, &str)> = struct_body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| {
                let (name, rest) = line
                    .split_once(':')
                    .unwrap_or_else(|| panic!("`{line}` is not a `name: type,` field"));
                let ty = rest.trim().trim_end_matches(',').trim();
                (name.trim(), ty)
            })
            .collect();

        // 字段顺序就是契约，所以先于其他一切检查它。
        assert_eq!(
            wgsl_fields
                .iter()
                .map(|(name, ty)| format!("{name}: {ty},"))
                .collect::<Vec<_>>(),
            vec![
                "mode: u32,",
                "source: u32,",
                "brightness: f32,",
                "radius: f32,",
                "repeat: vec2<f32>,",
                "_pad_b: vec2<f32>,",
                "center: vec3<f32>,",
                "_pad_c: u32,",
                "transform: mat4x4<f32>,"
            ],
            "reordering these fields silently changes every offset"
        );

        // 遍历 WGSL uniform 地址空间布局规则：每个成员从其自身对齐的
        // 下一个倍数开始，而结构体大小向上取整到最大成员对齐。
        let align_and_size = |ty: &str| -> (usize, usize) {
            match ty {
                "u32" | "i32" | "f32" => (4, 4),
                "vec2<f32>" => (8, 8),
                // vec3 像 vec4 一样对齐，但只占 12 字节。
                "vec3<f32>" => (16, 12),
                // 四个 vec4 列，每个 16 对齐。
                "mat4x4<f32>" => (16, 64),
                other => panic!("unhandled WGSL uniform type {other:?}"),
            }
        };
        let mut offset = 0usize;
        let mut struct_align = 1usize;
        let mut computed: Vec<(&str, usize)> = Vec::new();
        for (name, ty) in &wgsl_fields {
            let (align, size) = align_and_size(ty);
            struct_align = struct_align.max(align);
            offset = offset.div_ceil(align) * align;
            computed.push((name, offset));
            offset += size;
        }
        let total = offset.div_ceil(struct_align) * struct_align;

        assert_eq!(
            total, 112,
            "shaders/panorama.wgsl documents a 112-byte PanoramaUniforms"
        );

        // 模块文档的布局表所宣传的偏移，逐字段。
        let documented = [
            ("mode", 0),
            ("source", 4),
            ("brightness", 8),
            ("radius", 12),
            ("repeat", 16),
            ("_pad_b", 24),
            ("center", 32),
            ("_pad_c", 44),
            ("transform", 48),
        ];
        assert_eq!(computed.as_slice(), documented.as_slice());

        // 而 Rust 侧必须以相同顺序声明相同名称，并带能映射到 WGSL
        // 那些的名称，否则 `#[derive(ShaderType)]` 写出的缓冲与 shader 读的不同。
        let rust_source = include_str!("panorama.rs");
        let rust_body = rust_source
            .split("pub struct PanoramaUniforms {")
            .nth(1)
            .expect("this file must define pub struct PanoramaUniforms");
        // `str::lines()`（而非 `split('\n')` + `collect::<String>()`）：把切片
        // collect 回一个 `String` 会丢掉每个换行，从而将整个结构体体
        // 融合成一行，以第一个字段的 `///` doc 注释开头，随后被整体过滤。
        let rust_fields: Vec<(String, String)> = rust_body
            .lines()
            // `}` 测试前先 `trim_start`：结构体现存活于私有的
            // `panorama_uniform` module 内，所以它的闭合大括号是缩进的，
            // 否则会被当作一个字段解析。
            .take_while(|line| !line.trim_start().starts_with('}'))
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| {
                let (name, rest) = line
                    .split_once(':')
                    .unwrap_or_else(|| panic!("`{line}` is not a `name: Type,` field"));
                (
                    name.trim().trim_start_matches("pub ").to_owned(),
                    rest.trim().trim_end_matches(',').trim().to_owned(),
                )
            })
            .collect();

        let type_map = [
            ("mode", "u32"),
            ("source", "u32"),
            ("brightness", "f32"),
            ("radius", "f32"),
            ("repeat", "Vec2"),
            ("_pad_b", "Vec2"),
            ("center", "Vec3"),
            ("_pad_c", "u32"),
            ("transform", "Mat4"),
        ];
        assert_eq!(
            rust_fields,
            type_map
                .iter()
                .map(|(name, ty)| (name.to_string(), ty.to_string()))
                .collect::<Vec<_>>(),
            "the Rust uniform struct and the WGSL struct must stay field-for-field identical"
        );
    }

    // ─── 无头注册 ───────────────────────────────────────────────

    /// `register_panorama_node` 在 `MinimalPlugins`（无 `Assets<Shader>`、
    /// 无 `RenderApp`）下不得 panic——即 `docs/deviations.md#dev-005` 的
    /// `load_internal_asset!` panic 类。
    #[test]
    fn registering_under_minimal_plugins_degrades_gracefully() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_panorama_node(&mut app);

        // 没有 shader 存储，所以什么也没有插入……
        assert!(!crate::shader_registry::shader_assets_available(&app));
        // ……而 extract-component plugin 仍在，因为它是纯 ECS。
        app.update();
    }

    /// 有资产后端但无 render app 时，shader 被注册，而节点注册仍降级
    /// 而非 panic。
    #[test]
    fn registering_with_an_asset_backend_but_no_render_app_loads_the_shader() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        <App as bevy::asset::AssetApp>::init_asset::<Shader>(&mut app);

        #[allow(deprecated)]
        register_panorama_node(&mut app);

        let shaders = app.world().resource::<Assets<Shader>>();
        assert!(
            shaders.get(&PANORAMA_SHADER_HANDLE).is_some(),
            "the embedded panorama.wgsl must be registered under its weak handle"
        );
        app.update();
    }

    /// 内嵌 shader 源码确实就是磁盘上的文件，且它确实携带 pipeline
    /// 所要求的两个 entry point。
    #[test]
    fn the_embedded_shader_is_the_file_on_disk() {
        let owned = normalized(panorama_wgsl_source());
        let source = owned.as_str();
        assert!(source.contains("@vertex\nfn panorama_vertex"));
        assert!(source.contains("@fragment\nfn panorama_fragment"));
        assert!(source.contains("#import bevy_render::view::View"));

        // 五个绑定，按槽位顺序。
        for declaration in [
            "@group(0) @binding(0) var panorama_cube: texture_cube<f32>;",
            "@group(0) @binding(1) var panorama_equirect: texture_2d<f32>;",
            "@group(0) @binding(2) var panorama_sampler: sampler;",
            "@group(0) @binding(3) var<uniform> view: View;",
            "@group(0) @binding(4) var<uniform> uniforms: PanoramaUniforms;",
        ] {
            assert!(
                source.contains(declaration),
                "shaders/panorama.wgsl is missing `{declaration}`"
            );
        }

        // 上游忠实、后续不应被“清理”的那些部分。
        assert!(
            source.contains("vec3(1.0, 1.0, -1.0)"),
            "the left-handed cube correction must stay"
        );
        assert!(
            source.contains("* uniforms.repeat"),
            "the equirectangular flip lives in `repeat`, not in a hardcoded negation"
        );
        assert!(
            source.contains("discard;"),
            "a bubble ray that misses the sphere must not write colour"
        );
        assert!(
            source.contains("clamp(direction.z, -1.0, 1.0)"),
            "asin of a 1-ULP-over-length z is NaN; the clamp is mandatory"
        );
    }

    /// `PanoramaNode` 必须保持无状态：一个以纹理 view id 为键的缓存 bind group
    /// 会在 panorama image 资产重载的那一刻失效。
    #[test]
    fn the_node_is_stateless_and_early_returns_when_disabled() {
        // 结构体无字段，所以既无物可缓存也无物可失效；本测试
        // 针对未来重构钉住这一事实。
        let node = PanoramaNode;
        let _ = &node;
        assert_eq!(
            std::mem::size_of::<PanoramaNode>(),
            0,
            "PanoramaNode must stay a zero-sized, stateless ViewNode"
        );

        // 禁用的组件会被两个 prepare 系统都跳过，所以无 pipeline id
        // 也无 bind group 会到达节点的 query。
        let component = CesiumPanorama::default();
        assert!(!component.enabled);
        assert_eq!(component.placement.as_u32(), MODE_SKYBOX);
        assert_eq!(component.source.as_u32(), SOURCE_CUBEMAP);
    }
}
