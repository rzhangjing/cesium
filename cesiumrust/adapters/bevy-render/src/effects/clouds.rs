//! M6.6：cesiumrust **云** 适配器——屏幕空间积云合成节点。
//!
//! 将积云（clouds）能力接入 M5-E 渲染图基础设施（`graph.rs`）：
//! 一组椭体积云以屏幕空间 ray-marched 体噪声合成，配合逐云 billboard。
//! 对应 [`super::clipping_planes`] / [`super::ibl`] 模式：
//! 本模块注册节点 / 资源 / 系统，**但从不创建图边**——
//! `graph.rs::wire_m6_edges` 中的单一线性 `Core3d` 链拥有这些边，
//! 所以不会形成菱形。将云节点接入该链是
//! 集成任务（阶段三 FIX-INTEG）。
//!
//! # 范围交付物（阶段二 FIX-CLOUD-FULL）
//! 本适配器完整实现 **`clouds.wgsl` 屏幕空间合成**
//! 路径：[`CesiumClouds`] 组件、[`CloudsUniform`] f64 → f32 GPU
//! 边界、[`CloudsPipeline`] bind-group layout、3D 噪声纹理、
//! [`CloudsNode`] `ViewNode`，以及三段式 `register_clouds_node*`。三个
//! WGSL shader（`clouds.wgsl` / `cloud_noise.wgsl` / `cloud_billboard.wgsl`）
//! 各自获得一个 naga 解析 + 校验 + binding 覆盖防线测试，且
//! [`CloudsUniform::from_domain`] 与领域 f64 CPU 参考
//!（`cesium-effects::cloud`）交叉校验。
//!
//! **billboard**（`cloud_billboard.wgsl`）和 **GPU 噪声生成器**
//!（`cloud_noise.wgsl`）渲染 pass 在此*未*接入图：它们的
//! `RenderPipelineDescriptor` / 实例缓冲 / compute dispatch 属于
//! 阶段三 FIX-INTEG 与真实 GPU 取证。CPU 参考 [`cesium_effects::cloud::NoiseVolume`]
//! 提供合成路径所采样的 3D 纹理，所以屏幕空间节点
//! 无需 compute 生成器即可自足。参见
//! `docs/deviations.md#dev-032`。
//!
//! # SPIKE 回报——真正的 `texture_3d`
//! 早期方案将 128³ 体数据打包进一个 2D atlas 并手写三线性插值
//!（逐体素 UV 展开 + 分段线性混合）。M6.6 SPIKE
//! 确认了 wgpu/naga 的 3D 纹理支持，所以 `clouds.wgsl` 用单个硬件三线性
//! `textureSampleLevel(…, vec3, 0.0)` 采样一个真正的 `texture_3d`，
//! 且本适配器通过 `initial_data` 将 [`NoiseVolume::to_rgba8_bytes`] 直接上传进
//! `TextureDimension::D3`——atlas 索引的花式变换就此消解。
//!
//! # 门控（单一真相源）
//! 门控名由应用层注册表
//! `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_CLOUDS` /
//! `clouds_enabled()`）拥有；下方的 [`ENV_ENABLE_CLOUDS`] 是一个字节一致的镜像，
//! 由 crate 依赖方向（`cesium-app` → `cesium-bevy-render`）强制。默认
//! OFF ⇒ 图插件从不调用 `register_clouds_node` ⇒ 不存在
//! `Core3d` 边 ⇒ 节点从不运行 ⇒ dynamic_globe v0 基线保持
//! 像素中性（PSNR = ∞）。shader 的 `count <= 0` 提前退出会原样返回
//! 源颜色，作为第二重保险。
//!
//! # 已遵守的红线
//! - 领域保持度量 **f64**；度量→渲染单位的转换
//!   （`/ METERS_PER_RENDER_UNIT`，`METERS_PER_RENDER_UNIT = 6378137`）以及
//!   `0.82·maximumSize` 椭球收缩仅发生在
//!   [`CloudsUniform::from_domain`]（GPU 边界）。
//! - `clouds.wgsl` 将 Gardner 求和、`dot(n,p) + w` 和行进
//!   累积保持为各自独立的 IEEE 舍入（无 FMA 融合）。
//! - `mod` 是 WGSL 保留字；shader 改用 `fract` / `%`。
//! - glam fast-math 全仓禁用（此处不依赖非 IEEE 浮点）。
//!
//! # 设计要点
//! - 云数据模型：每个积云是一个带位置/尺度/覆盖率/各向异性的椭球。
//! - 集合把椭球打包为 GPU uniform/实例缓冲，屏幕空间节点按相机 ray-march。
//! - 体噪声用真正的 `texture_3d` 硬件三线性采样（替代 2D atlas 手写插值）。
//! - f64 CPU 参考实现位于领域层，供本模块测试交叉校验。

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DepthPrepass, ViewPrepassTextures},
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_3d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::cloud::{
    CloudCollection, NoiseVolume, BEER_LAMBERT_EXTINCTION, CLOUD_LIGHT_DIR, ELLIPSOID_SCALE_FACTOR,
    HG_PHASE_G, NOISE_TEXTURE_DIMENSIONS, RAYMARCH_STEPS_DEFAULT,
};

use crate::resources::METERS_PER_RENDER_UNIT;
use super::graph::gate_from_env_value;

// ─── Shader handle ──────────────────────────────────────────────────────────

/// 内嵌 `clouds.wgsl` 屏幕空间合成 shader 的 handle（由
/// [`CloudsNode`] 驱动）。
pub const CLOUDS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_C10D_0006_0006);

/// 内嵌 `cloud_billboard.wgsl` billboard pass 的 handle。注册为一个
/// 资源，所以源码被校验 + 可用，但其渲染 pipeline 尚
/// 未被驱动——billboard 实例化属于阶段三 FIX-INTEG
///（`docs/deviations.md#dev-032`）。
pub const CLOUD_BILLBOARD_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_C10D_0006_0007);

/// 内嵌 `cloud_noise.wgsl` compute 生成器的 handle。与
/// [`CLOUD_BILLBOARD_SHADER_HANDLE`] 状态相同：已注册，尚未 dispatch（本范围内
/// CPU [`NoiseVolume`] 为合成路径供数据）。
pub const CLOUD_NOISE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_C10D_0006_0008);

/// 上传到 GPU uniform 数组的云的最大数量。必须等于 `clouds.wgsl` 中的
/// `array<vec4<f32>, 8>` 大小和 `MAX_CLOUDS`（由
/// `wgsl_max_clouds_matches_rust_const` 断言）。
pub const MAX_CLOUDS: usize = 8;

/// 上传到 GPU 的噪声体边长（对应领域中的
/// `NOISE_TEXTURE_DIMENSIONS` 和 `clouds.wgsl` 中的 `info.y`）。
pub const CLOUDS_NOISE_DIMENSIONS: usize = NOISE_TEXTURE_DIMENSIONS;

// ─── 门控（应用层注册表的镜像——参见模块 doc） ────────────────

/// 门控 M6.6 云节点的环境变量。应用层注册表
/// 拥有者的*镜像* `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_CLOUDS` /
/// `clouds_enabled()`）；为 `pub` 以便
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` 能
/// 跨 crate 边界断言字节相等。默认 OFF。
pub const ENV_ENABLE_CLOUDS: &str = "CESIUM_ENABLE_CLOUDS";

/// 当云门控启用时返回 `true`。复用单一权威的
/// truthy 解析器（`gate_from_env_value`，全 crate 的
/// `{1, true, yes, on}` 集），所以它与其他所有 cesium 门控一致。
#[inline]
pub fn clouds_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_CLOUDS).ok())
}

// ─── 渲染图 label（本地——阶段三将其接入 Core3d 链） ───────

/// cesium 云节点在 `Core3d` 中的节点 label。本地定义，以便本
/// 模块不必编辑 `graph.rs` 中共享的 label 枚举；阶段三 FIX-INTEG
/// 创建边（推荐位置：主不透明 pass 之后，读取
/// HDR 场景颜色 + 深度前置 pass，在 AO / tonemapping 之前）。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumCloudsLabel;

// ─── 着色模式 ────────────────────────────────────────────────────────────

/// [`params.z`] 选择 `clouds.wgsl` 两条着色路径中的哪一条。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CloudsShadingMode {
    /// 忠实的单云绘制：单条光线/椭球相交，以 Gardner 正弦纹理 + Worley-FBM
    /// 侵蚀着色。这正是忠实模式所渲染的内容。
    #[default]
    Faithful,
    /// 加法式的物理体积行进（Beer-Lambert + Henyey-Greenstein，
    /// g = 0.6）。非上游——该路径引入的偏差记录为
    /// `docs/deviations.md#dev-032`。
    Volumetric,
}

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 为一个视图携带活跃 [`CloudCollection`] 的组件。
///
/// 放在相机上（像 [`super::clipping_planes::CesiumClippingPlanes`]）以
/// 驱动屏幕空间节点。经 `ExtractComponentPlugin` 提取到 render world。
/// 当 `enabled == false` 或集合为空 / 隐藏时节点提前
/// return（零 GPU 开销，像素中性）。`Default` 为派生：
/// `enabled = false`（保守）。
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumClouds {
    /// 该视图上云节点的主开关。
    pub enabled: bool,
    /// 领域云集合（度量 f64）。
    pub collection: CloudCollection,
    /// 合成所使用的着色路径。
    pub shading: CloudsShadingMode,
}

impl CesiumClouds {
    /// 一个便捷构造函数，创建启用的视图集合（忠实着色）。
    pub fn new(collection: CloudCollection) -> Self {
        Self {
            enabled: true,
            collection,
            shading: CloudsShadingMode::Faithful,
        }
    }

    /// 带显式着色模式。
    pub fn with_shading(mut self, shading: CloudsShadingMode) -> Self {
        self.shading = shading;
        self
    }

    /// 云是否应当真正渲染（组件 + 集合一致）。门控
    /// 本身在注册时检查，不在此处。
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled && self.collection.show && !self.collection.is_empty()
    }
}

/// 云节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraCloudsPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// 持有打包后云数据的逐视图 GPU uniform 缓冲。
#[derive(Component)]
pub struct ViewCloudsUniform {
    pub buffer: UniformBuffer<CloudsUniform>,
}

// ─── GPU uniform（f32 边界） ──────────────────────────────────────────────

/// GPU 面向的云 uniform。**仅 f32**——领域集合保持度量
/// f64；[`CloudsUniform::from_domain`] 在此边界执行唯一的度量 → 渲染单位
/// 转换 + `0.82·maximumSize` 收缩（红线）。
///
/// 布局必须与 `shaders/clouds.wgsl` 中的 `struct CloudsData` 匹配（encase std140）。
///
/// 该 struct 住在带 `#![allow(dead_code)]` 的私有 `clouds_uniform`
/// 模块中（`clipping_planes.rs` 约定）：encase 的
/// `ShaderType` derive 会生成一个模块级 helper，死代码分析会标记它，
/// 尽管每个字段都通过 `write_buffer` 上传。字段值在
/// `from_domain_*` 单元测试中被断言。
pub use clouds_uniform::CloudsUniform;

mod clouds_uniform {
    #![allow(dead_code)]
    use super::MAX_CLOUDS;
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU 云 uniform；布局匹配 `shaders/clouds.wgsl` 中的
    /// `struct CloudsData`（encase std140）。
    #[derive(ShaderType, Clone, Debug)]
    pub struct CloudsUniform {
        /// `xyz` = 椭球中心（RENDER UNITS），`w` = 激活标志（1 = 着色）。
        pub centers: [Vec4; MAX_CLOUDS],
        /// `xyz` = 椭球缩放（RENDER UNITS，已是 `0.82·maximumSize`），
        /// `w` = slice。
        pub scales: [Vec4; MAX_CLOUDS],
        /// `rgba` 云颜色（`v_color`）。
        pub colors: [Vec4; MAX_CLOUDS],
        /// `x` = u_noiseDetail，`y` = raymarch 步数，`z` = 模式（0 忠实 /
        /// 1 体积），`w` = Beer-Lambert 消光。
        pub params: Vec4,
        /// `xyz` = 相机世界位置（未用：shader 读取
        /// `view.world_from_view[3].xyz`），`w` = 亮度。
        pub camera: Vec4,
        /// `xyz` = 光方向（shader 归一化），`w` = Henyey-Greenstein g。
        pub light: Vec4,
        /// `x` = 活跃云数，`y` = 噪声体边长（128），`zw` = 填充。
        pub info: Vec4,
    }
}

impl Default for CloudsUniform {
    /// 默认均匀体：所有云的 center/scale/color 置零，params.z 为忠实模式。
    fn default() -> Self {
        Self {
            centers: [Vec4::ZERO; MAX_CLOUDS],
            scales: [Vec4::ZERO; MAX_CLOUDS],
            colors: [Vec4::ZERO; MAX_CLOUDS],
            // 默认 params.z = 0（忠实）；camera.w = 1.0，所以一次
            // 无云的意外激活是纯透传（info.x = 0）。
            params: Vec4::new(0.0, RAYMARCH_STEPS_DEFAULT as f32, 0.0, BEER_LAMBERT_EXTINCTION as f32),
            camera: Vec4::new(0.0, 0.0, 0.0, 1.0),
            // light.xyz 默认为（未归一化的）CLOUD_LIGHT_DIR；shader
            // 归一化。light.w = HG g。
            light: Vec4::new(
                CLOUD_LIGHT_DIR.x as f32,
                CLOUD_LIGHT_DIR.y as f32,
                CLOUD_LIGHT_DIR.z as f32,
                HG_PHASE_G as f32,
            ),
            // info.x = 0 ⇒ shader 原样返回源颜色。
            info: Vec4::new(0.0, CLOUDS_NOISE_DIMENSIONS as f32, 0.0, 0.0),
        }
    }
}

impl CloudsUniform {
    /// 将一个领域 [`CloudCollection`] 打包进 GPU uniform。
    ///
    /// - 每朵可见云的度量 `position` 除以
    ///   [`METERS_PER_RENDER_UNIT`] 以进入渲染单位世界空间（即
    ///   `world_from_view` 所住的同一空间）；其 `maximum_size` 按
    ///   [`ELLIPSOID_SCALE_FACTOR`]（0.82，上游 `drawCloud` 所用的椭球半径）
    ///   收缩，并同样除以渲染单位。
    /// - `centers[i].w = 1.0` 标记该槽激活；数量被限幅到
    ///   [`MAX_CLOUDS`]。
    /// - `camera.w`（亮度）将逐云的 `brightness` 归并为一个
    ///   全局值（第一朵可见云）——`clouds.wgsl` 只有一个亮度槽。
    ///   这就是记录在案的偏差 `docs/deviations.md#dev-032`。
    /// - 当集合隐藏或为空时 `info.x = 0` ⇒ 纯透传
    ///   （像素中性）。
    pub fn from_domain(collection: &CloudCollection, shading: CloudsShadingMode) -> Self {
        let mut centers = [Vec4::ZERO; MAX_CLOUDS];
        let mut scales = [Vec4::ZERO; MAX_CLOUDS];
        let mut colors = [Vec4::ZERO; MAX_CLOUDS];

        let mut count = 0usize;
        let mut brightness = 1.0_f64;
        let mut first_seen = false;

        for cloud in collection.visible_clouds() {
            if count >= MAX_CLOUDS {
                break;
            }
            if !first_seen {
                brightness = cloud.brightness;
                first_seen = true;
            }
            centers[count] = Vec4::new(
                (cloud.position.x / METERS_PER_RENDER_UNIT) as f32,
                (cloud.position.y / METERS_PER_RENDER_UNIT) as f32,
                (cloud.position.z / METERS_PER_RENDER_UNIT) as f32,
                1.0, // 激活标志
            );
            // 0.82·maximumSize（渲染单位），slice 存于 .w。
            scales[count] = Vec4::new(
                (cloud.maximum_size.x * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                (cloud.maximum_size.y * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                (cloud.maximum_size.z * ELLIPSOID_SCALE_FACTOR / METERS_PER_RENDER_UNIT) as f32,
                cloud.slice as f32,
            );
            colors[count] = Vec4::new(
                cloud.color[0] as f32,
                cloud.color[1] as f32,
                cloud.color[2] as f32,
                cloud.color[3] as f32,
            );
            count += 1;
        }

        let mode_z: f32 = match shading {
            CloudsShadingMode::Faithful => 0.0,
            CloudsShadingMode::Volumetric => 1.0,
        };

        let active = collection.show && count > 0;
        let params = Vec4::new(
            collection.noise_detail as f32,
            RAYMARCH_STEPS_DEFAULT as f32,
            mode_z,
            BEER_LAMBERT_EXTINCTION as f32,
        );
        let info = Vec4::new(
            if active { count as f32 } else { 0.0 },
            CLOUDS_NOISE_DIMENSIONS as f32,
            0.0,
            0.0,
        );

        Self {
            centers,
            scales,
            colors,
            params,
            camera: Vec4::new(0.0, 0.0, 0.0, brightness as f32),
            light: Vec4::new(
                CLOUD_LIGHT_DIR.x as f32,
                CLOUD_LIGHT_DIR.y as f32,
                CLOUD_LIGHT_DIR.z as f32,
                HG_PHASE_G as f32,
            ),
            info,
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：云节点的 bind group layout + 采样器。
///
/// Group 0 bindings（必须匹配 `clouds.wgsl` + binding 覆盖测试）：
/// - 0：深度前置 pass（`texture_depth_2d`）——遮挡 + 光线重建
/// - 1：3D 噪声体（`texture_3d<f32>`）——Worley-FBM 侵蚀通道
/// - 2：颜色源（`texture_2d<f32>`）——后处理输入
/// - 3：线性采样器（Filtering——颜色 + 三线性噪声）
/// - 4：点采样器（NonFiltering——深度）
/// - 5：`ViewUniform`（动态偏移）——`world_from_view` / `view_from_clip`
/// - 6：`CloudsUniform`——打包后的云数据
#[derive(Resource)]
pub struct CloudsPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
}

impl FromWorld for CloudsPipeline {
    /// 从 render-world 构建云合成 pass 的设备资源：为场景颜色、深度、
    /// 3D 噪声纹理、云均匀体与 view uniform 创建 bind group 布局。
    ///
    /// # 参数
    /// - `render_world`：提供 `RenderDevice` 的渲染世界。
    ///
    /// # 返回
    /// 装配好的 [`CloudsPipeline`] 资源。
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_clouds_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_3d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<CloudsUniform>(false),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clouds_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // 2D 颜色源用 Clamp-to-edge；3D 噪声的 Repeat 由 shader 在此
        // 绑定的采样器设定。本范围内单个 Filtering 采样器同时服务
        // 颜色 + 噪声（噪声环绕依赖 shader 的
        // `fract` 重居中，参见 `sample_noise`）。
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clouds_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── 3D 噪声纹理 ────────────────────────────────────────────────────────

/// Render-world 资源：128³ Worley-FBM 噪声体，作为真正的
/// `texture_3d` 上传（SPIKE 回报）。在
/// [`initialize_clouds_noise`] 中从领域 CPU [`NoiseVolume`] 参考创建一次，所以
/// 合成路径无需 `cloud_noise.wgsl` compute
/// 生成器即可自足（推迟到阶段三 / GPU）。
#[derive(Resource)]
pub struct CloudsNoiseTexture {
    /// 拥有 GPU 纹理（某些后端中仅有 view 可能悬空）。
    #[allow(dead_code)]
    texture: Texture,
    /// 已绑定的 3D 纹理视图（供 bind group 引用）。
    view: TextureView,
}

impl CloudsNoiseTexture {
    /// 已绑定的 3D 纹理视图。
    #[inline]
    pub fn view(&self) -> &TextureView {
        &self.view
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// 屏幕空间云合成 `ViewNode`。
///
/// 从深度前置 pass + `view_from_clip` 重建世界光线，行进
/// （最多 8 个）云椭球，并将它们从前到后叠加合成到
/// HDR 场景颜色上。当组件未激活，或深度前置 pass / pipeline / 噪声纹理
/// 不可用时提前 return（像素中性）。
#[derive(Default)]
pub struct CloudsNode;

impl ViewNode for CloudsNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraCloudsPipeline,
        &'static CesiumClouds,
        &'static ViewCloudsUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    /// 运行屏幕空间云合成 pass：若未激活则直接返回；否则从深度 prepass
    /// 重建世界光线，逐个 ray-march 云椭球并从前到后叠加到 HDR 场景色。
    ///
    /// # 参数
    /// - `_graph`：渲染图上下文（本节点无子 pass）。
    /// - `render_context`：当前 pass 的 GPU 命令记录器。
    /// - `target`：视图的渲染目标（后处理读写）。
    /// - `pipeline_handle`：该视图缓存的云合成 pipeline ID。
    /// - `clouds`：相机级云开关；`clouds_uniform`：本帧云参数。
    /// - `prepass`：深度 prepass 纹理；`view_uniform_offset`：本视图偏移。
    /// - `world`：提供 pipeline/noise/texture 资源的 render-world。
    ///
    /// # 返回
    /// 成功提交命令则为 `Ok(())`；前置资源未就绪时返回错误。
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, clouds, clouds_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !clouds.is_active() {
            return Ok(());
        }

        // 合成读取深度前置 pass 以做遮挡 + 光线重建。
        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let clouds_pipeline = world.resource::<CloudsPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let noise = world.resource::<CloudsNoiseTexture>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(clouds_binding) = clouds_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let bind_group = render_device.create_bind_group(
            Some("cesium_clouds_bind_group"),
            &clouds_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                noise.view(),
                source,
                &clouds_pipeline.linear_sampler,
                &clouds_pipeline.point_sampler,
                view_uniform_binding.clone(),
                clouds_binding,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_clouds_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        // Binding 5（ViewUniform）是动态偏移；提供其偏移（Ryan C2
        // 教训：`&[]` 偏移列表会使动态缓冲校验失败）。
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 从 CPU [`NoiseVolume`] 参考创建 128³ 噪声纹理一次。
/// 运行在 `Render`、`RenderSet::Prepare`（通过资源守卫幂等）。该
/// 体使用默认的生产 detail（16.0）；逐集合的 detail /
/// offset 重生成是阶段三 / GPU 的改进（参见 `#dev-032`）。
pub fn initialize_clouds_noise(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    existing: Option<Res<CloudsNoiseTexture>>,
) {
    if existing.is_some() {
        return;
    }

    let volume = NoiseVolume::generate(
        CLOUDS_NOISE_DIMENSIONS,
        // 默认生产噪声 detail（匹配 CloudCollection::default()）。
        16.0,
        glam::DVec3::ZERO,
    );
    let bytes = volume.to_rgba8_bytes();
    let dim = CLOUDS_NOISE_DIMENSIONS as u32;

    let texture = render_device.create_texture_with_data(
        &render_queue,
        &TextureDescriptor {
            label: Some("cesium_clouds_noise"),
            size: Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: dim,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            // 线性数据（worley 通道），绝非 sRGB。
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        // z 最慢 / 行主序匹配 NoiseVolume::to_rgba8_bytes 的顺序。
        wgpu::util::TextureDataOrder::default(),
        &bytes,
    );
    let view = texture.create_view(&Default::default());

    commands.insert_resource(CloudsNoiseTexture { texture, view });
}

/// 为每个活跃视图准备云 pipeline + 逐视图 uniform 缓冲。
/// 运行在 `Render`、`RenderSet::Prepare`（在渲染图执行之前）。
pub fn prepare_clouds(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    clouds_pipeline: Res<CloudsPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumClouds)>,
) {
    for (entity, view, clouds) in &views {
        if !clouds.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_clouds_pipeline".into()),
            layout: vec![clouds_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: CLOUDS_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: destination_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });

        // 将领域集合（度量 f64）打包进 GPU uniform（f32，
        // 渲染单位位置/缩放）并上传。
        let mut buffer = UniformBuffer::from(CloudsUniform::from_domain(
            &clouds.collection,
            clouds.shading,
        ));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraCloudsPipeline { pipeline_id },
            ViewCloudsUniform { buffer },
        ));
    }
}

// ─── 主 world 系统 ──────────────────────────────────────────────────────

/// 确保驱动一个活跃云集合的相机携带 [`DepthPrepass`]
///（该节点的遮挡 / 光线重建输入）。适配层启用，以便
/// 应用层相机束保持不变（与 `setup_clipping_prepass`
/// 同一纪律）。仅插入（绝不移除——DEF-033 守卫住在
/// `ao.rs`）。仅当云门控 ON 时注册。
pub fn setup_clouds_prepass(mut commands: Commands, cameras: Query<(Entity, &CesiumClouds)>) {
    for (entity, clouds) in &cameras {
        if clouds.is_active() {
            commands.entity(entity).insert(DepthPrepass);
        }
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 将云节点注册进 `RenderApp`（shader + extract + 节点 + 系统）。
///
/// 当 [`clouds_gate_enabled()`] 为 true 时由
/// `effects::graph::M6WaveARenderGraphPlugin`（阶段三 FIX-INTEG）调用；
/// `Core3d` 边由 `wire_m6_edges` 创建
///（本函数注册节点但从不接边，所以 `graph.rs` 中
/// 共享的线性链仍是唯一拥有者）。
///
/// 无头安好：没有 `RenderApp`（MinimalPlugins）时降级为 no-op。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_clouds_node_main_world` from `Plugin::build` and `register_clouds_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_clouds_node(app: &mut App) {
    register_clouds_node_main_world(app);
    // 无头 `MinimalPlugins` 没有 `RenderApp`——优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_clouds_node_render_world(render_app);
    }
}

/// [`register_clouds_node`] 的 `Plugin::build` 时前半：**主** world 的 WGSL
/// shader 资源 + `ExtractComponentPlugin` + `setup_clouds_prepass` 系统。
///
/// 按 DEV-029 / §5.1 纪律拆分——pipeline 的 `FromWorld` 读取
/// `RenderDevice`，而 Bevy 只在 `RenderPlugin::finish` 中把它插入
/// render world，所以 render-world 半必须从 `finish` 运行。
pub fn register_clouds_node_main_world(app: &mut App) {
    // 注册三个云 WGSL shader（经 shader_registry 无头安好）。
    // 本范围内只有 `clouds.wgsl` 由 pipeline 驱动；另外两个被
    // 注册以便其源码可用 + 已校验（阶段三接它们）。
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUDS_SHADER_HANDLE,
        include_str!("../../shaders/clouds.wgsl"),
        "shaders/clouds.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUD_BILLBOARD_SHADER_HANDLE,
        include_str!("../../shaders/cloud_billboard.wgsl"),
        "shaders/cloud_billboard.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        CLOUD_NOISE_SHADER_HANDLE,
        include_str!("../../shaders/cloud_noise.wgsl"),
        "shaders/cloud_noise.wgsl",
    );

    // ExtractComponentPlugin：每帧 main → render world（ExtractSchedule）。
    app.add_plugins(ExtractComponentPlugin::<CesiumClouds>::default());

    // 主 world：为驱动活跃集合的相机附加 DepthPrepass。
    app.add_systems(Update, setup_clouds_prepass);
}

/// [`register_clouds_node`] 的 `Plugin::finish` 时后半：render-world
/// pipeline 资源 + 噪声纹理 + `Render` 系统 + `Core3d` 节点。
pub fn register_clouds_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（从 `build` 到达 finish 半，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<CloudsPipeline>()
        .add_systems(
            Render,
            (
                initialize_clouds_noise,
                prepare_clouds.in_set(RenderSet::Prepare),
            )
                .chain(),
        )
        .add_render_graph_node::<ViewNodeRunner<CloudsNode>>(Core3d, CesiumCloudsLabel);
    // NOTE：边由 `effects::graph::wire_m6_edges`（阶段三 FIX-INTEG）创建，
    // 它是共享 `Core3d` 链的唯一拥有者。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::cloud::{CumulusCloud, ELLIPSOID_SCALE_FACTOR as DOM_SCALE_FACTOR};
    use glam::DVec3;

    #[test]
    fn clouds_component_default_disabled() {
        let c = CesiumClouds::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn clouds_gate_const_is_byte_stable() {
        // 门控环境变量是注册表的一个*镜像*（在 feature_flags 中跨 crate
        // 断言）；在此钉住，所以一次本地重命名会变红。
        assert_eq!(ENV_ENABLE_CLOUDS, "CESIUM_ENABLE_CLOUDS");
        assert_ne!(ENV_ENABLE_CLOUDS, "CESIUM_ENABLE_OIT");
        // 规范的 env 名：已完全大写（小写重命名
        // 会破坏 feature_flags 中断言的注册表镜像契约）。
        assert_eq!(ENV_ENABLE_CLOUDS, ENV_ENABLE_CLOUDS.to_uppercase().as_str());
        // 复用权威的 truthy 解析器。
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn clouds_shader_handles_unique() {
        assert_ne!(CLOUDS_SHADER_HANDLE, CLOUD_BILLBOARD_SHADER_HANDLE);
        assert_ne!(CLOUDS_SHADER_HANDLE, CLOUD_NOISE_SHADER_HANDLE);
        assert_ne!(
            CLOUDS_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
        assert_ne!(CLOUDS_SHADER_HANDLE, super::super::oit::OIT_ACCUMULATE_SHADER_HANDLE);
        assert_ne!(
            CLOUDS_SHADER_HANDLE,
            super::super::clipping_planes::CLIPPING_SHADER_HANDLE
        );
    }

    #[test]
    fn clouds_headless_graceful() {
        // 没有 RenderApp（无头）→ 注册不得 panic。
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_clouds_node(&mut app);
    }

    // ─── Uniform 打包：度量→渲染单位边界（红线） ────────

    #[test]
    fn from_domain_empty_is_pixel_neutral() {
        // 空集合 ⇒ info.x = 0 ⇒ shader 透传源。
        let collection = CloudCollection::new();
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert_eq!(u.info.x, 0.0, "no clouds ⇒ count 0 ⇒ pixel-neutral");
        assert_eq!(u.params.z, 0.0, "default mode is faithful (0)");
    }

    #[test]
    fn from_domain_divides_position_and_scales_into_render_units() {
        // 一朵云沿 +X 方向 6_378_137 m，maximumSize 为一个渲染单位立方体。
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::new(
            DVec3::new(METERS_PER_RENDER_UNIT, 0.0, 0.0),
            DVec3::new(METERS_PER_RENDER_UNIT, METERS_PER_RENDER_UNIT, METERS_PER_RENDER_UNIT),
        ));
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);

        assert!((u.info.x - 1.0).abs() < 1e-6, "one active cloud");
        // 位置 ÷ 6378137 ⇒ X 上 1 渲染单位。
        assert!((u.centers[0].x - 1.0).abs() < 1e-5, "centre.x = 1 render unit");
        assert!((u.centers[0].w - 1.0).abs() < 1e-6, "active flag set");
        // 缩放 = 0.82 · maximumSize ÷ MPU ⇒ X 上 0.82 渲染单位。
        assert!(
            (u.scales[0].x - DOM_SCALE_FACTOR as f32).abs() < 1e-5,
            "scale.x = 0.82 (ELLIPSOID_SCALE_FACTOR) render units"
        );
    }

    #[test]
    fn from_domain_copies_physical_constants_to_uniform() {
        // params.w 消光、light.w HG g、info.y 噪声边长反映领域值。
        let collection = CloudCollection::new();
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Volumetric);
        assert!((u.params.w - BEER_LAMBERT_EXTINCTION as f32).abs() < 1e-6);
        assert!((u.light.w - HG_PHASE_G as f32).abs() < 1e-6);
        assert!((u.info.y - NOISE_TEXTURE_DIMENSIONS as f32).abs() < 1e-6);
        assert!((u.params.y - RAYMARCH_STEPS_DEFAULT as f32).abs() < 1e-6);
        assert_eq!(u.params.z, 1.0, "volumetric mode ⇒ params.z = 1");
    }

    #[test]
    fn from_domain_caps_at_max_clouds() {
        let mut collection = CloudCollection::new();
        for i in 0..(MAX_CLOUDS + 5) {
            collection.add(CumulusCloud::new(
                DVec3::new(i as f64, 0.0, 0.0),
                DVec3::new(100.0, 100.0, 100.0),
            ));
        }
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert_eq!(u.info.x as usize, MAX_CLOUDS, "count clamped to MAX_CLOUDS");
        // 全部 8 个槽激活；第 9 个及以后被丢弃（仍为 ZERO）。
        assert_eq!(u.centers[MAX_CLOUDS - 1].w, 1.0);
    }

    #[test]
    fn from_domain_takes_brightness_from_first_cloud() {
        let mut collection = CloudCollection::new();
        let mut cloud = CumulusCloud::new(DVec3::ZERO, DVec3::new(20.0, 12.0, 8.0));
        cloud.brightness = 0.4;
        collection.add(cloud);
        let u = CloudsUniform::from_domain(&collection, CloudsShadingMode::Faithful);
        assert!((u.camera.w - 0.4).abs() < 1e-6, "camera.w = first cloud brightness");
    }

    // ─── Naga 防线：解析 + 校验 + binding 覆盖 ───────────────

    /// 用于 naga 无法解析的 `#import` 指令的 stub。
    const CLOUD_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct View {
    view_from_clip: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    viewport: vec4<f32>,
    frustum: vec4<f32>,
}
";

    /// 剥离 `#import` 行并在前面加回 stub struct，以便 naga（没有
    /// Bevy prelude 来解析 `#import`）能解析该模块。
    fn cloud_stubbed_wgsl(path: &str) -> String {
        let mut source = String::from(CLOUD_WGSL_IMPORT_STUBS);
        for line in path.lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// 通过遍历入口点的表达式树（以及传递性地，被调用的函数），
    /// 收集从该入口点可达的 `(group, binding)` 对。
    fn used_bindings(
        module: &naga::Module,
        entry_name: &str,
    ) -> std::collections::BTreeSet<(u32, u32)> {
        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == entry_name)
            .unwrap_or_else(|| panic!("missing entry point {entry_name}"));
        let mut used = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            used.insert((binding.group, binding.binding));
                        }
                    }
                    naga::Expression::CallResult(func_handle) if visited.insert(func_handle) => {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }
        used
    }

    fn validate(source: &str, label: &str) -> naga::Module {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{label} does not parse:\n{}", error.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{label} does not validate: {error}"));
        module
    }

    #[test]
    fn clouds_wgsl_parses_and_type_checks_under_naga() {
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/clouds.wgsl"));
        let module = validate(&source, "clouds.wgsl");
        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "clouds.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn clouds_wgsl_bindings_covered_by_layout() {
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/clouds.wgsl"));
        let module = validate(&source, "clouds.wgsl");
        // CloudsPipeline layout: group(0) bindings 0..=6.
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6)]
                .into_iter()
                .collect();
        let used = used_bindings(&module, "fragment");
        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "clouds.wgsl uses bindings {missing:?} absent from CloudsPipeline layout"
        );
    }

    #[test]
    fn cloud_noise_wgsl_parses_and_type_checks_under_naga() {
        // cloud_noise.wgsl 没有 `#import` 指令；原样校验它。
        let source = include_str!("../../shaders/cloud_noise.wgsl");
        let module = validate(source, "cloud_noise.wgsl");
        let has_compute = module
            .entry_points
            .iter()
            .any(|e| e.stage == naga::ShaderStage::Compute);
        assert!(
            has_compute,
            "cloud_noise.wgsl must expose a compute entry point"
        );
    }

    #[test]
    fn cloud_billboard_wgsl_parses_and_type_checks_under_naga() {
        // cloud_billboard.wgsl 使用 `#import bevy_render::view::View`；为它做 stub。
        let source = cloud_stubbed_wgsl(include_str!("../../shaders/cloud_billboard.wgsl"));
        let module = validate(&source, "cloud_billboard.wgsl");
        let mut has_vertex = false;
        let mut has_fragment = false;
        for e in module.entry_points.iter() {
            match e.stage {
                naga::ShaderStage::Vertex => has_vertex = true,
                naga::ShaderStage::Fragment => has_fragment = true,
                _ => {}
            }
        }
        assert!(
            has_vertex && has_fragment,
            "cloud_billboard.wgsl must expose a vertex + fragment entry point"
        );
    }

    #[test]
    fn wgsl_max_clouds_matches_rust_const() {
        // WGSL 的 `MAX_CLOUDS` + `array<vec4<f32>, 8>` uniform 大小必须
        // 与 Rust 的 `MAX_CLOUDS` 保持同步（一次静默的偏移会导致
        // GPU 上丢弃或越界读云）。
        let wgsl = include_str!("../../shaders/clouds.wgsl");
        assert!(
            wgsl.contains(&format!("const MAX_CLOUDS: i32 = {MAX_CLOUDS};")),
            "clouds.wgsl MAX_CLOUDS must equal Rust MAX_CLOUDS = {MAX_CLOUDS}"
        );
        assert!(
            wgsl.contains("array<vec4<f32>, 8>"),
            "clouds.wgsl uniform arrays must be sized 8 to match MAX_CLOUDS"
        );
    }

    #[test]
    fn domain_physical_constants_mirror_wgsl() {
        // shader 硬编码的领域 f64 常量必须匹配，所以合成所依赖的
        // 领域↔GPU 一致性不会静默漂移。
        assert!((ELLIPSOID_SCALE_FACTOR - 0.82).abs() < 1e-12);
        assert!((HG_PHASE_G - 0.6).abs() < 1e-12);
        assert_eq!(NOISE_TEXTURE_DIMENSIONS, 128);
        assert!((BEER_LAMBERT_EXTINCTION - 0.1).abs() < 1e-12);
    }
}
