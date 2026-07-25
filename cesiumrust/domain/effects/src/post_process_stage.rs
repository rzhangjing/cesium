//! Post-process stage system.
//!
//! Maps to CesiumJS:
//! - `Scene/PostProcessStage.js` — individual post-process stage
//! - `Scene/PostProcessStageCollection.js` — ordered collection
//! - `Scene/PostProcessStageLibrary.js` — built-in stages (FXAA, AO, Bloom)
//!
//! Domain layer — pure Rust, f64 precision.

use std::collections::HashMap;

// ─── PostProcessStage ───────────────────────────────────────────────────────

/// How to sample the input color texture.
///
/// Maps to CesiumJS `PostProcessStageSampleMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleMode {
    /// Nearest-neighbor sampling.
    #[default]
    Nearest,
    /// Linear interpolation sampling.
    Linear,
}

/// Pixel format for post-process output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// RGBA 8-bit.
    #[default]
    Rgba8,
    /// RGBA 16-bit float.
    Rgba16F,
    /// RGBA 32-bit float.
    Rgba32F,
}

/// A uniform value for a post-process stage.
#[derive(Debug, Clone, PartialEq)]
pub enum UniformValue {
    /// Float scalar.
    Float(f64),
    /// 2D vector.
    Vec2([f64; 2]),
    /// 3D vector.
    Vec3([f64; 3]),
    /// 4D vector.
    Vec4([f64; 4]),
    /// Integer.
    Int(i32),
    /// Boolean.
    Bool(bool),
    /// Texture reference (URI or name).
    Texture(String),
}

/// A single post-process stage.
///
/// Maps to CesiumJS `PostProcessStage`.
#[derive(Debug, Clone)]
pub struct PostProcessStage {
    /// Unique name of this stage.
    pub name: String,
    /// Whether this stage is enabled.
    pub enabled: bool,
    /// The fragment shader source (GLSL/WGSL).
    pub fragment_shader: String,
    /// Uniform values for the shader.
    pub uniforms: HashMap<String, UniformValue>,
    /// Texture scale (0.0, 1.0] — scales the output texture dimensions.
    pub texture_scale: f64,
    /// Whether to force power-of-two texture dimensions.
    pub force_power_of_two: bool,
    /// How to sample the input color texture.
    pub sample_mode: SampleMode,
    /// Output pixel format.
    pub pixel_format: PixelFormat,
    /// Clear color [R, G, B, A].
    pub clear_color: [f64; 4],
    /// Whether this stage is ready (shader compiled, textures allocated).
    pub ready: bool,
}

impl PostProcessStage {
    /// Creates a new post-process stage with a fragment shader.
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

    /// Sets a uniform value.
    pub fn set_uniform(&mut self, name: impl Into<String>, value: UniformValue) {
        self.uniforms.insert(name.into(), value);
    }

    /// Gets a uniform value.
    pub fn get_uniform(&self, name: &str) -> Option<&UniformValue> {
        self.uniforms.get(name)
    }

    /// Computes the output texture dimensions given viewport size.
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

/// A composite of multiple post-process stages that execute as a unit.
///
/// Maps to CesiumJS `PostProcessStageComposite`.
#[derive(Debug, Clone)]
pub struct PostProcessStageComposite {
    /// Unique name.
    pub name: String,
    /// Whether the composite is enabled.
    pub enabled: bool,
    /// The stages in this composite (executed in order).
    pub stages: Vec<PostProcessStage>,
    /// Whether to execute stages in parallel (input = same texture) or sequentially.
    pub parallel: bool,
}

impl PostProcessStageComposite {
    /// Creates a new composite.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            stages: Vec::new(),
            parallel: false,
        }
    }

    /// Adds a stage to the composite.
    pub fn add_stage(&mut self, stage: PostProcessStage) {
        self.stages.push(stage);
    }

    /// Returns the number of stages.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Returns whether the composite is empty.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Returns whether all stages are ready.
    pub fn is_ready(&self) -> bool {
        self.stages.iter().all(|s| s.ready)
    }
}

// ─── Built-in Stage Factories ───────────────────────────────────────────────

/// Creates an FXAA (Fast Approximate Anti-Aliasing) stage.
///
/// Maps to CesiumJS `PostProcessStageLibrary.createFXAAStage()`.
pub fn create_fxaa_stage() -> PostProcessStage {
    let mut stage = PostProcessStage::new(
        "czm_fxaa",
        "// FXAA fragment shader\nuniform sampler2D colorTexture;\nin vec2 v_textureCoordinates;\nvoid main() {\n    out_FragColor = fxaa(colorTexture, v_textureCoordinates);\n}",
    );
    stage.enabled = false; // Disabled by default
    stage.sample_mode = SampleMode::Linear;
    stage
}

/// Creates a Bloom composite stage.
///
/// Maps to CesiumJS `PostProcessStageLibrary.createBloomStage()`.
pub fn create_bloom_composite() -> PostProcessStageComposite {
    let mut composite = PostProcessStageComposite::new("czm_bloom");
    composite.enabled = false;

    // Bright pass: extract bright pixels
    let mut bright_pass = PostProcessStage::new(
        "czm_bloom_brightness",
        "// Brightness threshold pass",
    );
    bright_pass.set_uniform("contrast", UniformValue::Float(128.0));
    bright_pass.set_uniform("brightness", UniformValue::Float(-0.3));
    bright_pass.set_uniform("glowOnly", UniformValue::Bool(false));
    composite.add_stage(bright_pass);

    // Blur pass: Gaussian blur
    let mut blur_pass = PostProcessStage::new("czm_bloom_blur", "// Gaussian blur pass");
    blur_pass.set_uniform("delta", UniformValue::Float(1.0));
    blur_pass.set_uniform("sigma", UniformValue::Float(3.8));
    blur_pass.set_uniform("stepSize", UniformValue::Float(1.5));
    composite.add_stage(blur_pass);

    composite
}

/// Creates an Ambient Occlusion composite stage (HBAO).
///
/// Maps to CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage()`.
pub fn create_ambient_occlusion_composite() -> PostProcessStageComposite {
    let mut composite = PostProcessStageComposite::new("czm_ambient_occlusion");
    composite.enabled = false;

    // AO generation pass
    let mut ao_pass = PostProcessStage::new("czm_ambient_occlusion_generate", "// HBAO pass");
    ao_pass.set_uniform("intensity", UniformValue::Float(3.0));
    ao_pass.set_uniform("bias", UniformValue::Float(0.1));
    ao_pass.set_uniform("lengthCap", UniformValue::Float(0.26));
    ao_pass.set_uniform("directionCount", UniformValue::Int(8));
    ao_pass.set_uniform("stepCount", UniformValue::Int(32));
    ao_pass.set_uniform("ambientOcclusionOnly", UniformValue::Bool(false));
    composite.add_stage(ao_pass);

    // Blur pass for AO
    let blur_pass = PostProcessStage::new("czm_ambient_occlusion_blur", "// AO blur pass");
    composite.add_stage(blur_pass);

    composite
}

/// Creates an auto-exposure stage.
///
/// Maps to CesiumJS `PostProcessStageLibrary.createAutoExposureStage()`.
pub fn create_auto_exposure_stage() -> PostProcessStage {
    let mut stage = PostProcessStage::new("czm_auto_exposure", "// Auto exposure histogram");
    stage.enabled = false;
    stage
}

// ─── Tonemapper ─────────────────────────────────────────────────────────────

/// Tonemapper selection for HDR → LDR conversion.
///
/// Maps to CesiumJS `Tonemapper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    /// PBR Neutral tonemapper (CesiumJS default).
    #[default]
    PbrNeutral,
    /// ACES Filmic tonemapper.
    AcesFilmic,
    /// Reinhard tonemapper.
    Reinhard,
    /// No tonemapping.
    None,
}

impl Tonemapper {
    /// Returns the shader function name for this tonemapper.
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

/// A collection of post-process stages executed in order.
///
/// Maps to CesiumJS `PostProcessStageCollection`.
///
/// Execution order:
/// 1. Ambient Occlusion (if enabled)
/// 2. Bloom (if enabled)
/// 3. User stages (in add order)
/// 4. Tonemapping (if enabled)
/// 5. FXAA (if enabled)
#[derive(Debug, Clone)]
pub struct PostProcessStageCollection {
    /// Built-in FXAA stage.
    pub fxaa: PostProcessStage,
    /// Built-in Ambient Occlusion composite.
    pub ambient_occlusion: PostProcessStageComposite,
    /// Built-in Bloom composite.
    pub bloom: PostProcessStageComposite,
    /// Built-in auto-exposure stage.
    pub auto_exposure: PostProcessStage,
    /// Whether auto-exposure is enabled.
    pub auto_exposure_enabled: bool,
    /// Manual exposure value (when auto-exposure is disabled).
    pub exposure: f64,
    /// The tonemapper to use.
    pub tonemapper: Tonemapper,
    /// Whether tonemapping is enabled.
    pub tonemapping_enabled: bool,
    /// User-added stages.
    stages: Vec<PostProcessStage>,
    /// Stage names for lookup.
    stage_names: HashMap<String, usize>,
}

impl Default for PostProcessStageCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessStageCollection {
    /// Creates a new collection with built-in stages.
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

    /// Adds a user stage to the collection.
    ///
    /// Returns the index of the added stage.
    pub fn add(&mut self, stage: PostProcessStage) -> usize {
        let index = self.stages.len();
        self.stage_names.insert(stage.name.clone(), index);
        self.stages.push(stage);
        index
    }

    /// Removes a stage by name.
    ///
    /// Returns the removed stage, if found.
    pub fn remove(&mut self, name: &str) -> Option<PostProcessStage> {
        if let Some(&index) = self.stage_names.get(name) {
            self.stage_names.remove(name);
            // Rebuild indices after removal
            let removed = self.stages.remove(index);
            self.rebuild_indices();
            Some(removed)
        } else {
            None
        }
    }

    /// Gets a stage by name.
    pub fn get_by_name(&self, name: &str) -> Option<&PostProcessStage> {
        self.stage_names.get(name).map(|&i| &self.stages[i])
    }

    /// Gets a mutable stage by name.
    pub fn get_by_name_mut(&mut self, name: &str) -> Option<&mut PostProcessStage> {
        self.stage_names.get(name).copied().map(|i| &mut self.stages[i])
    }

    /// Returns the number of user stages.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Returns whether there are no user stages.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Returns whether any stage is ready and enabled.
    pub fn is_ready(&self) -> bool {
        let built_in_ready = (self.fxaa.ready && self.fxaa.enabled)
            || (self.ambient_occlusion.enabled && self.ambient_occlusion.is_ready())
            || (self.bloom.enabled && self.bloom.is_ready())
            || self.tonemapping_enabled;

        built_in_ready || self.stages.iter().any(|s| s.ready && s.enabled)
    }

    /// Returns the execution order of all active stages.
    ///
    /// This determines the order in which stages should be executed.
    pub fn execution_order(&self) -> Vec<StageRef> {
        let mut order = Vec::new();

        // 1. Ambient Occlusion (before all others)
        if self.ambient_occlusion.enabled {
            order.push(StageRef::AmbientOcclusion);
        }

        // 2. Bloom
        if self.bloom.enabled {
            order.push(StageRef::Bloom);
        }

        // 3. User stages
        for (i, stage) in self.stages.iter().enumerate() {
            if stage.enabled {
                order.push(StageRef::User(i));
            }
        }

        // 4. Tonemapping
        if self.tonemapping_enabled {
            order.push(StageRef::Tonemapping);
        }

        // 5. FXAA (after all others)
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

/// Reference to a stage in the execution pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageRef {
    /// Built-in ambient occlusion.
    AmbientOcclusion,
    /// Built-in bloom.
    Bloom,
    /// User stage at index.
    User(usize),
    /// Built-in tonemapping.
    Tonemapping,
    /// Built-in FXAA.
    Fxaa,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── PostProcessStage tests ─────────────────────────────────────────

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

    // ─── Composite tests ────────────────────────────────────────────────

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

        assert!(!composite.is_ready()); // s2 not ready

        composite.stages[1].ready = true;
        assert!(composite.is_ready());
    }

    // ─── Built-in stage tests ───────────────────────────────────────────

    #[test]
    fn test_fxaa_stage() {
        let fxaa = create_fxaa_stage();
        assert_eq!(fxaa.name, "czm_fxaa");
        assert!(!fxaa.enabled); // Disabled by default
        assert_eq!(fxaa.sample_mode, SampleMode::Linear);
    }

    #[test]
    fn test_bloom_composite() {
        let bloom = create_bloom_composite();
        assert_eq!(bloom.name, "czm_bloom");
        assert!(!bloom.enabled);
        assert_eq!(bloom.len(), 2); // brightness + blur
    }

    #[test]
    fn test_ao_composite() {
        let ao = create_ambient_occlusion_composite();
        assert_eq!(ao.name, "czm_ambient_occlusion");
        assert!(!ao.enabled);
        assert_eq!(ao.len(), 2); // generate + blur

        // Check AO uniforms
        let gen = &ao.stages[0];
        assert_eq!(gen.get_uniform("intensity"), Some(&UniformValue::Float(3.0)));
        assert_eq!(gen.get_uniform("directionCount"), Some(&UniformValue::Int(8)));
    }

    // ─── Tonemapper tests ───────────────────────────────────────────────

    #[test]
    fn test_tonemapper_shader_functions() {
        assert_eq!(Tonemapper::PbrNeutral.shader_function(), "czm_pbrNeutralTonemap");
        assert_eq!(Tonemapper::AcesFilmic.shader_function(), "czm_acesFilmicTonemap");
        assert_eq!(Tonemapper::Reinhard.shader_function(), "czm_reinhardTonemap");
        assert_eq!(Tonemapper::None.shader_function(), "czm_noTonemap");
    }

    // ─── Collection tests ───────────────────────────────────────────────

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

        // Get by name
        assert!(collection.get_by_name("my_stage").is_some());
        assert!(collection.get_by_name("nonexistent").is_none());

        // Remove
        let removed = collection.remove("my_stage");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 0);
    }

    #[test]
    fn test_collection_execution_order_empty() {
        let collection = PostProcessStageCollection::new();
        let order = collection.execution_order();
        assert!(order.is_empty()); // Nothing enabled
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
