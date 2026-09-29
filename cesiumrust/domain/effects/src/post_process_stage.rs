//! 后处理阶段系统。
//!
//! 映射到 CesiumJS：
//! - `Scene/PostProcessStage.js` —— 单个后处理阶段
//! - `Scene/PostProcessStageCollection.js` —— 有序集合
//! - `Scene/PostProcessStageLibrary.js` —— 内置阶段（FXAA、AO、Bloom）
//!
//! 领域层——纯 Rust，f64 精度。

use std::collections::HashMap;

// ─── PostProcessStage ───────────────────────────────────────────────────────

/// 如何采样输入颜色 texture。
///
/// 映射到 CesiumJS `PostProcessStageSampleMode`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleMode {
    /// 最近邻采样。
    #[default]
    Nearest,
    /// 线性插值采样。
    Linear,
}

/// 后处理输出的像素格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// RGBA 8 位。
    #[default]
    Rgba8,
    /// RGBA 16 位浮点。
    Rgba16F,
    /// RGBA 32 位浮点。
    Rgba32F,
}

/// 后处理阶段的一个 uniform 值。
#[derive(Debug, Clone, PartialEq)]
pub enum UniformValue {
    /// 浮点标量。
    Float(f64),
    /// 2D 向量。
    Vec2([f64; 2]),
    /// 3D 向量。
    Vec3([f64; 3]),
    /// 4D 向量。
    Vec4([f64; 4]),
    /// 整数。
    Int(i32),
    /// 布尔。
    Bool(bool),
    /// texture 引用（URI 或名称）。
    Texture(String),
}

/// 单个后处理阶段。
///
/// 映射到 CesiumJS `PostProcessStage`。
#[derive(Debug, Clone)]
pub struct PostProcessStage {
    /// 本阶段的唯一名称。
    pub name: String,
    /// 本阶段是否启用。
    pub enabled: bool,
    /// fragment shader 源码（GLSL/WGSL）。
    pub fragment_shader: String,
    /// shader 的 uniform 值。
    pub uniforms: HashMap<String, UniformValue>,
    /// texture 缩放 (0.0, 1.0]——缩放输出 texture 的尺寸。
    pub texture_scale: f64,
    /// 是否强制 texture 尺寸为 2 的幂。
    pub force_power_of_two: bool,
    /// 如何采样输入颜色 texture。
    pub sample_mode: SampleMode,
    /// 输出像素格式。
    pub pixel_format: PixelFormat,
    /// 清除颜色 [R, G, B, A]。
    pub clear_color: [f64; 4],
    /// 本阶段是否就绪（shader 已编译，texture 已分配）。
    pub ready: bool,
}

impl PostProcessStage {
    /// 创建一个带 fragment shader 的新后处理阶段。
    pub fn new(name: impl Into<String>, fragment_shader: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            fragment_shader: fragment_shader.into(),
            uniforms: HashMap::new(),
            texture_scale: 1.0,
            force_power_of_two: false,
            sample_mode: SampleMode::Nearest,
            pixel_format: PixelFormat::Rgba8,
            clear_color: [0.0, 0.0, 0.0, 0.0],
            ready: false,
        }
    }

    /// 设置一个 uniform 值。
    pub fn set_uniform(&mut self, name: impl Into<String>, value: UniformValue) {
        self.uniforms.insert(name.into(), value);
    }

    /// 获取一个 uniform 值。
    pub fn get_uniform(&self, name: &str) -> Option<&UniformValue> {
        self.uniforms.get(name)
    }

    /// 给定视口尺寸，计算输出 texture 的尺寸。
    pub fn output_dimensions(&self, viewport_width: u32, viewport_height: u32) -> (u32, u32) {
        let mut w = (viewport_width as f64 * self.texture_scale) as u32;
        let mut h = (viewport_height as f64 * self.texture_scale) as u32;

        if self.force_power_of_two {
            let min_dim = w.min(h);
            let pot = min_dim.next_power_of_two();
            w = pot;
            h = pot;
        }

        (w.max(1), h.max(1))
    }
}

// ─── PostProcessStageComposite ──────────────────────────────────────────────

/// 多个后处理阶段作为一体执行的复合体。
///
/// 映射到 CesiumJS `PostProcessStageComposite`。
#[derive(Debug, Clone)]
pub struct PostProcessStageComposite {
    /// 唯一名称。
    pub name: String,
    /// 复合体是否启用。
    pub enabled: bool,
    /// 本复合体中的各阶段（按顺序执行）。
    pub stages: Vec<PostProcessStage>,
    /// 是否并行执行各阶段（输入 = 同一 texture），否则顺序执行。
    pub parallel: bool,
}

impl PostProcessStageComposite {
    /// 创建一个新的复合体。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            stages: Vec::new(),
            parallel: false,
        }
    }

    /// 向复合体添加一个阶段。
    pub fn add_stage(&mut self, stage: PostProcessStage) {
        self.stages.push(stage);
    }

    /// 返回阶段的数量。
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// 返回复合体是否为空。
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 返回是否所有阶段都已就绪。
    pub fn is_ready(&self) -> bool {
        self.stages.iter().all(|s| s.ready)
    }
}

// ─── 内置阶段工厂 ───────────────────────────────────────────────

/// 创建一个 FXAA（快速近似抗锯齿）阶段。
///
/// 映射到 CesiumJS `PostProcessStageLibrary.createFXAAStage()`。
///
/// # M5-E1 实现说明
/// 运行时的 shader 是自实现的 WGSL，位于
/// `adapters/bevy-render/shaders/fxaa.wgsl`——一个 FXAA 3.11 的翻译，
/// **仅含 quality preset 12**（`FXAA_QUALITY_PS=5`、`P0=1.0, P1=1.5, P2=2.0,
/// P3=4.0, P4=12.0`），绿色通道作为亮度 + 提前退出。
///
/// 蓝图：`cesium-rs/crates/cesium-shaders/shaders/FXAA3_11.glsl` L102-108
/// （preset 12 的 define）+ L261-650（核心算法）；接口封装
/// `packages/engine/Source/Shaders/PostProcessStages/FXAA.glsl` L1-21。
///
/// 下方的三个 quality 参数在 CesiumJS GLSL（FXAA.glsl L5-7）和我们的
/// WGSL 中均为**编译期 `const`**——而非运行时 uniform。它们在此
/// 以 `f64` 记录，供领域侧自省 / 半位断言使用。
/// 参见 `docs/deviations.md#dev-017`。
pub fn create_fxaa_stage() -> PostProcessStage {
    let mut stage = PostProcessStage::new(
        "czm_fxaa",
        "// FXAA 3.11, quality preset 12 (self-implemented WGSL).\n\
         // Runtime shader: adapters/bevy-render/shaders/fxaa.wgsl\n\
         // const QUALITY_PS=5; P0=1.0 P1=1.5 P2=2.0 P3=4.0 P4=12.0 (FXAA3_11.glsl L102-108)\n\
         // const SUBPIX_QUALITY=0.5; EDGE_THRESHOLD=0.125; EDGE_THRESHOLD_MIN=0.0833 (FXAA.glsl L5-7)\n\
         // green-as-luma + early-exit; 5 unrolled edge-search steps; alpha preserved.",
    );
    stage.enabled = false; // 默认禁用（通过 CESIUM_ENABLE_POSTPROCESS 门控启用）
    stage.sample_mode = SampleMode::Linear;
    // CesiumJS FXAA.glsl L5-7 的 quality 参数（上游为编译期常量；
    // 在此以 f64 镜像，以便领域侧描述子可自省/可测试）。
    stage.set_uniform("fxaaQualitySubpix", UniformValue::Float(0.5));
    stage.set_uniform("fxaaQualityEdgeThreshold", UniformValue::Float(0.125));
    stage.set_uniform("fxaaQualityEdgeThresholdMin", UniformValue::Float(0.0833));
    stage
}

/// 创建一个 Bloom 复合阶段。
///
/// 映射到 CesiumJS `PostProcessStageLibrary.createBloomStage()`。
pub fn create_bloom_composite() -> PostProcessStageComposite {
    let mut composite = PostProcessStageComposite::new("czm_bloom");
    composite.enabled = false;

    // 亮部提取 pass：提取明亮像素
    let mut bright_pass = PostProcessStage::new(
        "czm_bloom_brightness",
        "// Brightness threshold pass",
    );
    bright_pass.set_uniform("contrast", UniformValue::Float(128.0));
    bright_pass.set_uniform("brightness", UniformValue::Float(-0.3));
    bright_pass.set_uniform("glowOnly", UniformValue::Bool(false));
    composite.add_stage(bright_pass);

    // 模糊 pass：高斯模糊
    let mut blur_pass = PostProcessStage::new("czm_bloom_blur", "// Gaussian blur pass");
    blur_pass.set_uniform("delta", UniformValue::Float(1.0));
    blur_pass.set_uniform("sigma", UniformValue::Float(3.8));
    blur_pass.set_uniform("stepSize", UniformValue::Float(1.5));
    composite.add_stage(blur_pass);

    composite
}

/// 创建一个环境光遮蔽（Ambient Occlusion）复合阶段。
///
/// 映射到 CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage()`
/// （`PostProcessStageLibrary.js` L496）/ `isAmbientOcclusionSupported`（L599）。
///
/// # M5-E2 实现说明
/// 运行时的 shader 是自实现的 WGSL，位于
/// `adapters/bevy-render/shaders/ao.wgsl`——一个**半球 16 样本 SSAO**
/// 核（`fragment_generate`）+ 一个 **4×4 box blur + modulate** pass
/// （`fragment_blur_modulate`），由 Bevy 的 `DepthPrepass` + `NormalPrepass` 供数。
///
/// 蓝图（语义）：`packages/engine/Source/Shaders/PostProcessStages/
/// AmbientOcclusionGenerate.glsl` L1-144（HBAO ray-march）+
/// `AmbientOcclusionModulate.glsl` L1-11。结构/API 参考：
/// `bevy_pbr-0.15.3/src/ssao/{mod.rs,ssao.wgsl}`。
///
/// 偏差：CesiumJS AO 是一个 HBAO ray-march（directionCount × stepCount）；
/// cesiumrust 按 M5-E2 计划实现半球核 SSAO 家族。
/// 因此下方的 `directionCount` / `stepCount` uniform 为领域侧的
/// 半位/自省而保留，但在运行时仅为**参考信息**（
/// WGSL 使用固定的 16-tap 半球核）。AO 参数
/// （intensity=3.0、sample_radius=0.5、sample_count=16、bias=0.001、
/// length_cap=0.26）在此为 f64，在 `ao.wgsl` 中投影为 f32 `const`。
/// 参见 `docs/deviations.md#dev-018`。
pub fn create_ambient_occlusion_composite() -> PostProcessStageComposite {
    let mut composite = PostProcessStageComposite::new("czm_ambient_occlusion");
    composite.enabled = false;

    // AO 生成 pass（半球 16 样本核 → AO factor texture）。
    let mut ao_pass = PostProcessStage::new(
        "czm_ambient_occlusion_generate",
        "// SSAO hemisphere 16-sample kernel (self-implemented WGSL).\n\
         // Runtime shader: adapters/bevy-render/shaders/ao.wgsl @fragment_generate\n\
         // const SAMPLE_COUNT=16; AO_INTENSITY=3.0; AO_RADIUS=0.5; AO_BIAS=0.001; AO_LENGTH_CAP=0.26\n\
         // inputs: DepthPrepass + NormalPrepass (view_from_clip reconstruct, view_from_world normal).\n\
         // per-pixel TBN + noise decorrelation; range-check + bias; ao = pow(1 - occ/16, intensity).",
    );
    ao_pass.set_uniform("intensity", UniformValue::Float(3.0));
    ao_pass.set_uniform("bias", UniformValue::Float(0.1));
    ao_pass.set_uniform("lengthCap", UniformValue::Float(0.26));
    ao_pass.set_uniform("directionCount", UniformValue::Int(8));
    ao_pass.set_uniform("stepCount", UniformValue::Int(32));
    ao_pass.set_uniform("ambientOcclusionOnly", UniformValue::Bool(false));
    composite.add_stage(ao_pass);

    // 模糊 + 调制 pass（对 AO factor 做 4×4 box blur，乘入颜色）。
    let blur_pass = PostProcessStage::new(
        "czm_ambient_occlusion_blur",
        "// 4x4 box blur + modulate (self-implemented WGSL).\n\
         // Runtime shader: adapters/bevy-render/shaders/ao.wgsl @fragment_blur_modulate\n\
         // 16-tap box blur of the AO factor, then colour.rgb *= ao (AmbientOcclusionModulate.glsl L1-11).",
    );
    composite.add_stage(blur_pass);

    composite
}

/// 创建一个自动曝光阶段。
///
/// 映射到 CesiumJS `PostProcessStageLibrary.createAutoExposureStage()`。
pub fn create_auto_exposure_stage() -> PostProcessStage {
    let mut stage = PostProcessStage::new("czm_auto_exposure", "// Auto exposure histogram");
    stage.enabled = false;
    stage
}

// ─── Tonemapper ─────────────────────────────────────────────────────────────

/// 用于 HDR → LDR 转换的色调映射器选择。
///
/// 映射到 CesiumJS `Tonemapper`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    /// PBR Neutral 色调映射器（CesiumJS 默认）。
    #[default]
    PbrNeutral,
    /// ACES Filmic 色调映射器。
    AcesFilmic,
    /// Reinhard 色调映射器。
    Reinhard,
    /// 不做色调映射。
    None,
}

impl Tonemapper {
    /// 返回本色调映射器对应的 shader 函数名。
    pub fn shader_function(&self) -> &'static str {
        match self {
            Self::PbrNeutral => "czm_pbrNeutralTonemap",
            Self::AcesFilmic => "czm_acesFilmicTonemap",
            Self::Reinhard => "czm_reinhardTonemap",
            Self::None => "czm_noTonemap",
        }
    }
}

// ─── PostProcessStageCollection ─────────────────────────────────────────────

/// 按顺序执行的后处理阶段集合。
///
/// 映射到 CesiumJS `PostProcessStageCollection`。
///
/// 执行顺序：
/// 1. 环境光遮蔽（若启用）
/// 2. Bloom（若启用）
/// 3. 用户阶段（按添加顺序）
/// 4. 色调映射（若启用）
/// 5. FXAA（若启用）
#[derive(Debug, Clone)]
pub struct PostProcessStageCollection {
    /// 内置 FXAA 阶段。
    pub fxaa: PostProcessStage,
    /// 内置环境光遮蔽复合体。
    pub ambient_occlusion: PostProcessStageComposite,
    /// 内置 Bloom 复合体。
    pub bloom: PostProcessStageComposite,
    /// 内置自动曝光阶段。
    pub auto_exposure: PostProcessStage,
    /// 自动曝光是否启用。
    pub auto_exposure_enabled: bool,
    /// 手动曝光值（当自动曝光禁用时）。
    pub exposure: f64,
    /// 使用的色调映射器。
    pub tonemapper: Tonemapper,
    /// 色调映射是否启用。
    pub tonemapping_enabled: bool,
    /// 用户添加的阶段。
    stages: Vec<PostProcessStage>,
    /// 供查找使用的阶段名称。
    stage_names: HashMap<String, usize>,
}

impl Default for PostProcessStageCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessStageCollection {
    /// 创建一个带内置阶段的新集合。
    pub fn new() -> Self {
        Self {
            fxaa: create_fxaa_stage(),
            ambient_occlusion: create_ambient_occlusion_composite(),
            bloom: create_bloom_composite(),
            auto_exposure: create_auto_exposure_stage(),
            auto_exposure_enabled: false,
            exposure: 1.0,
            tonemapper: Tonemapper::PbrNeutral,
            tonemapping_enabled: false,
            stages: Vec::new(),
            stage_names: HashMap::new(),
        }
    }

    /// 向集合添加一个用户阶段。
    ///
    /// 返回被添加阶段的索引。
    pub fn add(&mut self, stage: PostProcessStage) -> usize {
        let index = self.stages.len();
        self.stage_names.insert(stage.name.clone(), index);
        self.stages.push(stage);
        index
    }

    /// 按名称移除一个阶段。
    ///
    /// 若找到，返回被移除的阶段。
    pub fn remove(&mut self, name: &str) -> Option<PostProcessStage> {
        if let Some(&index) = self.stage_names.get(name) {
            self.stage_names.remove(name);
            // 移除后重建索引
            let removed = self.stages.remove(index);
            self.rebuild_indices();
            Some(removed)
        } else {
            None
        }
    }

    /// 按名称获取一个阶段。
    pub fn get_by_name(&self, name: &str) -> Option<&PostProcessStage> {
        self.stage_names.get(name).map(|&i| &self.stages[i])
    }

    /// 按名称获取一个可变阶段。
    pub fn get_by_name_mut(&mut self, name: &str) -> Option<&mut PostProcessStage> {
        self.stage_names.get(name).copied().map(|i| &mut self.stages[i])
    }

    /// 返回用户阶段的数量。
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// 返回是否没有用户阶段。
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 返回是否有任一阶段既就绪又启用。
    pub fn is_ready(&self) -> bool {
        let built_in_ready = (self.fxaa.ready && self.fxaa.enabled)
            || (self.ambient_occlusion.enabled && self.ambient_occlusion.is_ready())
            || (self.bloom.enabled && self.bloom.is_ready())
            || self.tonemapping_enabled;

        built_in_ready || self.stages.iter().any(|s| s.ready && s.enabled)
    }

    /// 返回所有活跃阶段的执行顺序。
    ///
    /// 这决定了各阶段应被执行的顺序。
    pub fn execution_order(&self) -> Vec<StageRef> {
        let mut order = Vec::new();

        // 1. 环境光遮蔽（在所有其他阶段之前）
        if self.ambient_occlusion.enabled {
            order.push(StageRef::AmbientOcclusion);
        }

        // 2. Bloom
        if self.bloom.enabled {
            order.push(StageRef::Bloom);
        }

        // 3. 用户阶段
        for (i, stage) in self.stages.iter().enumerate() {
            if stage.enabled {
                order.push(StageRef::User(i));
            }
        }

        // 4. 色调映射
        if self.tonemapping_enabled {
            order.push(StageRef::Tonemapping);
        }

        // 5. FXAA（在所有其他阶段之后）
        if self.fxaa.enabled {
            order.push(StageRef::Fxaa);
        }

        order
    }

    fn rebuild_indices(&mut self) {
        self.stage_names.clear();
        for (i, stage) in self.stages.iter().enumerate() {
            self.stage_names.insert(stage.name.clone(), i);
        }
    }
}

/// 对执行流水线中某个阶段的引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageRef {
    /// 内置环境光遮蔽。
    AmbientOcclusion,
    /// 内置 bloom。
    Bloom,
    /// 指定索引处的用户阶段。
    User(usize),
    /// 内置色调映射。
    Tonemapping,
    /// 内置 FXAA。
    Fxaa,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── PostProcessStage 测试 ─────────────────────────────────────────

    #[test]
    fn test_stage_creation() {
        let stage = PostProcessStage::new("test", "void main() {}");
        assert_eq!(stage.name, "test");
        assert!(stage.enabled);
        assert!(!stage.ready);
        assert!((stage.texture_scale - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_stage_uniforms() {
        let mut stage = PostProcessStage::new("test", "");
        stage.set_uniform("scale", UniformValue::Float(1.5));
        stage.set_uniform("offset", UniformValue::Vec3([0.1, 0.2, 0.3]));

        assert_eq!(stage.get_uniform("scale"), Some(&UniformValue::Float(1.5)));
        assert_eq!(
            stage.get_uniform("offset"),
            Some(&UniformValue::Vec3([0.1, 0.2, 0.3]))
        );
        assert_eq!(stage.get_uniform("missing"), None);
    }

    #[test]
    fn test_stage_output_dimensions() {
        let stage = PostProcessStage {
            texture_scale: 0.5,
            ..PostProcessStage::new("test", "")
        };

        let (w, h) = stage.output_dimensions(1920, 1080);
        assert_eq!(w, 960);
        assert_eq!(h, 540);
    }

    #[test]
    fn test_stage_output_dimensions_pot() {
        let stage = PostProcessStage {
            texture_scale: 1.0,
            force_power_of_two: true,
            ..PostProcessStage::new("test", "")
        };

        let (w, h) = stage.output_dimensions(1920, 1080);
        // min(1920, 1080) = 1080, next_power_of_two(1080) = 2048
        assert_eq!(w, 2048);
        assert_eq!(h, 2048);
    }

    // ─── 复合体测试 ────────────────────────────────────────────────

    #[test]
    fn test_composite_creation() {
        let composite = PostProcessStageComposite::new("test_composite");
        assert_eq!(composite.name, "test_composite");
        assert!(composite.enabled);
        assert!(composite.is_empty());
    }

    #[test]
    fn test_composite_add_stages() {
        let mut composite = PostProcessStageComposite::new("test");
        composite.add_stage(PostProcessStage::new("s1", ""));
        composite.add_stage(PostProcessStage::new("s2", ""));

        assert_eq!(composite.len(), 2);
        assert!(!composite.is_empty());
    }

    #[test]
    fn test_composite_ready() {
        let mut composite = PostProcessStageComposite::new("test");
        let mut s1 = PostProcessStage::new("s1", "");
        s1.ready = true;
        let s2 = PostProcessStage::new("s2", "");

        composite.add_stage(s1);
        composite.add_stage(s2);

        assert!(!composite.is_ready()); // s2 未就绪

        composite.stages[1].ready = true;
        assert!(composite.is_ready());
    }

    // ─── 内置阶段测试 ───────────────────────────────────────────

    #[test]
    fn test_fxaa_stage() {
        let fxaa = create_fxaa_stage();
        assert_eq!(fxaa.name, "czm_fxaa");
        assert!(!fxaa.enabled); // 默认禁用
        assert_eq!(fxaa.sample_mode, SampleMode::Linear);
    }

    #[test]
    fn test_bloom_composite() {
        let bloom = create_bloom_composite();
        assert_eq!(bloom.name, "czm_bloom");
        assert!(!bloom.enabled);
        assert_eq!(bloom.len(), 2); // 亮度 + 模糊
    }

    #[test]
    fn test_ao_composite() {
        let ao = create_ambient_occlusion_composite();
        assert_eq!(ao.name, "czm_ambient_occlusion");
        assert!(!ao.enabled);
        assert_eq!(ao.len(), 2); // 生成 + 模糊

        // 检查 AO uniform
        let gen = &ao.stages[0];
        assert_eq!(gen.get_uniform("intensity"), Some(&UniformValue::Float(3.0)));
        assert_eq!(gen.get_uniform("directionCount"), Some(&UniformValue::Int(8)));
    }

    // ─── Tonemapper 测试 ───────────────────────────────────────────────

    #[test]
    fn test_tonemapper_shader_functions() {
        assert_eq!(Tonemapper::PbrNeutral.shader_function(), "czm_pbrNeutralTonemap");
        assert_eq!(Tonemapper::AcesFilmic.shader_function(), "czm_acesFilmicTonemap");
        assert_eq!(Tonemapper::Reinhard.shader_function(), "czm_reinhardTonemap");
        assert_eq!(Tonemapper::None.shader_function(), "czm_noTonemap");
    }

    // ─── Collection 测试 ───────────────────────────────────────────────

    #[test]
    fn test_collection_creation() {
        let collection = PostProcessStageCollection::new();
        assert!(!collection.fxaa.enabled);
        assert!(!collection.ambient_occlusion.enabled);
        assert!(!collection.bloom.enabled);
        assert!(!collection.tonemapping_enabled);
        assert!((collection.exposure - 1.0).abs() < 1e-10);
        assert_eq!(collection.tonemapper, Tonemapper::PbrNeutral);
    }

    #[test]
    fn test_collection_add_remove() {
        let mut collection = PostProcessStageCollection::new();

        let stage = PostProcessStage::new("my_stage", "void main() {}");
        let idx = collection.add(stage);
        assert_eq!(idx, 0);
        assert_eq!(collection.len(), 1);

        // 按名称获取
        assert!(collection.get_by_name("my_stage").is_some());
        assert!(collection.get_by_name("nonexistent").is_none());

        // 移除
        let removed = collection.remove("my_stage");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 0);
    }

    #[test]
    fn test_collection_execution_order_empty() {
        let collection = PostProcessStageCollection::new();
        let order = collection.execution_order();
        assert!(order.is_empty()); // 未启用任何阶段
    }

    #[test]
    fn test_collection_execution_order_full() {
        let mut collection = PostProcessStageCollection::new();
        collection.ambient_occlusion.enabled = true;
        collection.bloom.enabled = true;
        collection.tonemapping_enabled = true;
        collection.fxaa.enabled = true;

        let mut user_stage = PostProcessStage::new("user", "");
        user_stage.enabled = true;
        collection.add(user_stage);

        let order = collection.execution_order();

        assert_eq!(order.len(), 5);
        assert_eq!(order[0], StageRef::AmbientOcclusion);
        assert_eq!(order[1], StageRef::Bloom);
        assert_eq!(order[2], StageRef::User(0));
        assert_eq!(order[3], StageRef::Tonemapping);
        assert_eq!(order[4], StageRef::Fxaa);
    }

    #[test]
    fn test_collection_execution_order_disabled_user() {
        let mut collection = PostProcessStageCollection::new();
        collection.tonemapping_enabled = true;

        let mut disabled_stage = PostProcessStage::new("disabled", "");
        disabled_stage.enabled = false;
        collection.add(disabled_stage);

        let order = collection.execution_order();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], StageRef::Tonemapping);
    }

    #[test]
    fn test_collection_is_ready() {
        let mut collection = PostProcessStageCollection::new();
        assert!(!collection.is_ready());

        collection.tonemapping_enabled = true;
        assert!(collection.is_ready());
    }

    #[test]
    fn test_collection_get_by_name_mut() {
        let mut collection = PostProcessStageCollection::new();
        collection.add(PostProcessStage::new("test", ""));

        if let Some(stage) = collection.get_by_name_mut("test") {
            stage.enabled = false;
        }

        assert!(!collection.get_by_name("test").unwrap().enabled);
    }
}
