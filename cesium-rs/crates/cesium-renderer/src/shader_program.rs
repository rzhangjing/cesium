//! Ported from `packages/engine/Source/Renderer/ShaderProgram.js`.
//!
//! A compiled and linked shader program. In the Rust/wgpu port, this wraps
//! `wgpu::ShaderModule` creation and manages the uniform map (attributes,
//! uniforms, samplers) that mirrors the CesiumJS `ShaderProgram` uniform
//! interface.
//!
//! DEVIATION (B2.2): CesiumJS introspects uniforms/attributes with
//! `gl.getActiveUniform` / `gl.getActiveAttrib` after linking. The wgpu port
//! performs the equivalent reflection on the naga IR of the WGSL source
//! (`naga::front::wgsl::parse_str` + entry-point/global-variable walk),
//! replacing the previous fragile string parsing. GLSL inputs are no longer
//! silently fed to wgpu as WGSL; they are rejected with an explicit error
//! (see `ShaderError::UnsupportedLanguage`). Per `docs/shader-strategy.md`
//! GLSL shaders either go through the naga GLSL frontend (sampler-less
//! shaders) or are replaced by hand-written WGSL (`cesium_shaders::wgsl`).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

/// Errors produced while creating or reflecting a shader program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderError {
    /// The WGSL source failed to parse with naga.
    Parse(String),
    /// GLSL was passed to a path that only accepts WGSL.
    ///
    /// DEVIATION: CesiumJS only ever speaks GLSL. The wgpu port accepts WGSL
    /// directly; GLSL must be translated upstream (naga glsl-in) or replaced
    /// by a hand-written WGSL variant.
    UnsupportedLanguage { stage: &'static str },
    /// No `@vertex` / `@fragment` entry point was found in the module.
    MissingEntryPoint { stage: &'static str },
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderError::Parse(message) => write!(f, "shader parse error: {message}"),
            ShaderError::UnsupportedLanguage { stage } => write!(
                f,
                "{stage} shader is GLSL; ShaderProgram accepts WGSL only — translate with \
                 naga glsl-in or use a hand-written WGSL variant (docs/shader-strategy.md)"
            ),
            ShaderError::MissingEntryPoint { stage } => {
                write!(f, "no @{stage} entry point found in WGSL module")
            }
        }
    }
}

impl std::error::Error for ShaderError {}

/// Which shader language the program sources are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderLanguage {
    /// GLSL sources (rejected at module creation time — see `ShaderError`).
    Glsl,
    /// WGSL sources (the only language accepted by the wgpu backend).
    Wgsl,
}

/// The kind of a reflected global resource binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// `var<uniform>` buffer (e.g. automatic uniform blocks, materials).
    UniformBuffer,
    /// Sampled texture (`texture_2d<f32>` etc.).
    Texture,
    /// Sampler (`sampler` / `sampler_comparison`).
    Sampler,
    /// Storage texture (`texture_storage_*`).
    StorageTexture,
    /// Storage buffer (`var<storage>`).
    StorageBuffer,
}

/// A reflected global resource binding (group/binding pair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingInfo {
    /// The global variable name (e.g. `czm`, `u_texture`).
    pub name: String,
    /// The `@group` index.
    pub group: u32,
    /// The `@binding` index.
    pub binding: u32,
    /// The resource kind.
    pub kind: BindingKind,
    /// Byte size of the buffer type (uniform/storage), 0 for textures/samplers.
    pub byte_size: u32,
    /// Whether the buffer binding requires dynamic offsets (uniform buffers
    /// only, used by the per-frame automatic-uniform ring buffer).
    pub has_dynamic_offset: bool,
}

/// A vertex attribute in a shader program.
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    /// The name of the attribute.
    pub name: String,
    /// The GL type of the attribute (WebGL constant, when known).
    pub gl_type: Option<u32>,
    /// The `@location` index.
    pub location: u32,
    /// The wgpu vertex format derived from the naga IR type.
    pub format: Option<wgpu::VertexFormat>,
}

/// A uniform variable in a shader program.
///
/// DEVIATION: for WGSL programs, "uniforms" are the global `var<uniform>`
/// buffers reflected from the naga IR; the CesiumJS-style per-scalar uniform
/// list is approximated by one entry per uniform buffer.
#[derive(Debug, Clone)]
pub struct UniformInfo {
    /// The name of the uniform.
    pub name: String,
    /// The GL type of the uniform (e.g., "float", "vec3", "mat4").
    pub gl_type: Option<u32>,
    /// The size of the uniform (for arrays).
    pub size: u32,
    /// The location/binding index.
    pub location: u32,
}

/// A compiled GPU shader program.
///
/// In wgpu, a program is a pair of `wgpu::ShaderModule`s (vertex + fragment)
/// plus the reflection data needed to build pipelines and bind groups.
/// Modules are created lazily (via `OnceLock`) so that `Arc<ShaderProgram>`
/// can be shared between the shader cache and draw commands.
#[derive(Debug)]
pub struct ShaderProgram {
    /// The processed vertex shader source (WGSL, or legacy GLSL).
    vertex_source: String,
    /// The processed fragment shader source (WGSL, or legacy GLSL).
    fragment_source: String,
    /// The shader language of both sources.
    language: ShaderLanguage,
    /// Compilation/link log.
    log: String,
    /// Whether this program has been destroyed.
    is_destroyed: bool,
    /// Entry point name of the vertex stage.
    vertex_entry: String,
    /// Entry point name of the fragment stage.
    fragment_entry: String,
    /// Reflected vertex attributes (from the vertex entry-point arguments).
    attributes: HashMap<String, AttributeInfo>,
    /// Reflected uniform buffer names (one per `var<uniform>` global).
    uniforms: HashMap<String, UniformInfo>,
    /// Sampler uniform names (kept for CesiumJS interface parity).
    sampler_uniforms: Vec<String>,
    /// All reflected resource bindings, merged across both stages.
    bindings: Vec<BindingInfo>,
    /// The vertex wgpu shader module (created lazily).
    vertex_module: OnceLock<wgpu::ShaderModule>,
    /// The fragment wgpu shader module (created lazily).
    fragment_module: OnceLock<wgpu::ShaderModule>,
    /// Cache key for this program.
    cache_key: String,
}

impl ShaderProgram {
    /// Creates a new shader program from GLSL source strings.
    ///
    /// DEVIATION: retained for API parity with existing call sites, but GLSL
    /// programs can no longer produce wgpu shader modules — module creation
    /// returns `ShaderError::UnsupportedLanguage` instead of silently feeding
    /// GLSL to wgpu as WGSL.
    pub fn new(vertex_source: String, fragment_source: String) -> Self {
        Self {
            vertex_source,
            fragment_source,
            language: ShaderLanguage::Glsl,
            log: String::new(),
            is_destroyed: false,
            vertex_entry: "main".to_string(),
            fragment_entry: "main".to_string(),
            attributes: HashMap::new(),
            uniforms: HashMap::new(),
            sampler_uniforms: Vec::new(),
            bindings: Vec::new(),
            vertex_module: OnceLock::new(),
            fragment_module: OnceLock::new(),
            cache_key: String::new(),
        }
    }

    /// Creates a new GLSL shader program with a cache key.
    pub fn with_cache_key(
        vertex_source: String,
        fragment_source: String,
        cache_key: String,
    ) -> Self {
        let mut program = Self::new(vertex_source, fragment_source);
        program.cache_key = cache_key;
        program
    }

    /// Creates a shader program directly from WGSL sources.
    ///
    /// Both stages are parsed with naga and reflected: vertex entry-point
    /// arguments become `AttributeInfo` records and global resource bindings
    /// become `BindingInfo` records. This replaces the CesiumJS
    /// `gl.getActiveUniform`/`gl.getActiveAttrib` post-link introspection.
    pub fn from_wgsl(
        vertex_source: &str,
        fragment_source: &str,
        cache_key: String,
    ) -> Result<Self, ShaderError> {
        let vertex_module = parse_wgsl(vertex_source, "vertex")?;
        let fragment_module = parse_wgsl(fragment_source, "fragment")?;

        let (vertex_entry, attributes) = reflect_vertex_stage(&vertex_module)?;
        let fragment_entry = find_entry_point(&fragment_module, naga::ShaderStage::Fragment)?;
        let bindings = reflect_bindings(&vertex_module, &fragment_module);

        let mut uniforms = HashMap::new();
        let mut sampler_uniforms = Vec::new();
        for binding in &bindings {
            match binding.kind {
                BindingKind::UniformBuffer => {
                    uniforms.insert(
                        binding.name.clone(),
                        UniformInfo {
                            name: binding.name.clone(),
                            gl_type: None,
                            size: binding.byte_size,
                            location: binding.binding,
                        },
                    );
                }
                BindingKind::Sampler => {
                    sampler_uniforms.push(binding.name.clone());
                }
                _ => {}
            }
        }

        Ok(Self {
            vertex_source: vertex_source.to_string(),
            fragment_source: fragment_source.to_string(),
            language: ShaderLanguage::Wgsl,
            log: String::new(),
            is_destroyed: false,
            vertex_entry,
            fragment_entry,
            attributes,
            uniforms,
            sampler_uniforms,
            bindings,
            vertex_module: OnceLock::new(),
            fragment_module: OnceLock::new(),
            cache_key,
        })
    }

    /// Returns the vertex shader source.
    pub fn vertex_source(&self) -> &str { &self.vertex_source }

    /// Returns the fragment shader source.
    pub fn fragment_source(&self) -> &str { &self.fragment_source }

    /// Returns the shader language of this program's sources.
    pub fn language(&self) -> ShaderLanguage { self.language }

    /// Returns the compilation/link log.
    pub fn log(&self) -> &str { &self.log }

    /// Returns whether this program has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Returns the cache key.
    pub fn cache_key(&self) -> &str { &self.cache_key }

    /// Returns the vertex entry point name.
    pub fn vertex_entry(&self) -> &str { &self.vertex_entry }

    /// Returns the fragment entry point name.
    pub fn fragment_entry(&self) -> &str { &self.fragment_entry }

    /// Destroys the shader program.
    ///
    /// DEVIATION: the lazily created `wgpu::ShaderModule`s are reference
    /// counted by wgpu and cannot be explicitly destroyed; only the source
    /// state is dropped here.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }

    // ---- Uniform map ----

    /// Returns the uniform map.
    pub fn uniforms(&self) -> &HashMap<String, UniformInfo> { &self.uniforms }

    /// Gets a uniform by name.
    pub fn get_uniform(&self, name: &str) -> Option<&UniformInfo> {
        self.uniforms.get(name)
    }

    /// Adds a uniform to the map.
    pub fn add_uniform(&mut self, name: String, gl_type: Option<u32>, size: u32, location: u32) {
        self.uniforms.insert(name.clone(), UniformInfo {
            name,
            gl_type,
            size,
            location,
        });
    }

    /// Returns the sampler uniform names.
    pub fn sampler_uniforms(&self) -> &[String] { &self.sampler_uniforms }

    /// Adds a sampler uniform name.
    pub fn add_sampler_uniform(&mut self, name: String) {
        self.sampler_uniforms.push(name);
    }

    // ---- Attribute map ----

    /// Returns the attribute map.
    pub fn attributes(&self) -> &HashMap<String, AttributeInfo> { &self.attributes }

    /// Gets an attribute by name.
    pub fn get_attribute(&self, name: &str) -> Option<&AttributeInfo> {
        self.attributes.get(name)
    }

    /// Adds an attribute to the map.
    pub fn add_attribute(&mut self, name: String, gl_type: Option<u32>, location: u32) {
        self.attributes.insert(name.clone(), AttributeInfo {
            name,
            gl_type,
            location,
            format: None,
        });
    }

    // ---- Reflected resource bindings ----

    /// Returns all reflected resource bindings (both stages merged).
    pub fn bindings(&self) -> &[BindingInfo] { &self.bindings }

    /// Builds the merged `wgpu::BindGroupLayoutEntry` list for this program.
    ///
    /// Bindings present in both stages are merged with the shader-stage
    /// visibility ORed together, matching what `create_pipeline_layout`
    /// requires.
    pub fn bind_group_layout_entries(&self) -> Vec<(u32, Vec<wgpu::BindGroupLayoutEntry>)> {
        let mut groups: HashMap<u32, HashMap<u32, wgpu::BindGroupLayoutEntry>> = HashMap::new();
        for info in &self.bindings {
            let entry = wgpu::BindGroupLayoutEntry {
                binding: info.binding,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: binding_type(info),
                count: None,
            };
            groups
                .entry(info.group)
                .or_default()
                .entry(info.binding)
                .and_modify(|existing| {
                    existing.visibility |= entry.visibility;
                })
                .or_insert(entry);
        }
        let mut result: Vec<(u32, Vec<wgpu::BindGroupLayoutEntry>)> = groups
            .into_iter()
            .map(|(group, entries)| {
                let mut entries: Vec<_> = entries.into_values().collect();
                entries.sort_by_key(|e| e.binding);
                (group, entries)
            })
            .collect();
        result.sort_by_key(|(group, _)| *group);
        result
    }

    /// A stable hash of this program (cache key ⊕ sources), used as part of
    /// the pipeline cache key in `Context`.
    pub fn pipeline_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        if !self.cache_key.is_empty() {
            self.cache_key.hash(&mut hasher);
        } else {
            self.vertex_source.hash(&mut hasher);
            self.fragment_source.hash(&mut hasher);
        }
        hasher.finish()
    }

    // ---- wgpu shader modules ----

    /// Creates (or returns the cached) vertex wgpu shader module.
    ///
    /// DEVIATION: In wgpu, vertex and fragment shaders are separate shader
    /// modules (or entries within one module). CesiumJS links both into one
    /// GL program; here they are created independently.
    pub fn create_vertex_shader_module(
        &self,
        device: &wgpu::Device,
    ) -> Result<&wgpu::ShaderModule, ShaderError> {
        if self.language != ShaderLanguage::Wgsl {
            return Err(ShaderError::UnsupportedLanguage { stage: "vertex" });
        }
        Ok(self.vertex_module.get_or_init(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ShaderProgram vertex (WGSL)"),
                source: wgpu::ShaderSource::Wgsl(self.vertex_source.as_str().into()),
            })
        }))
    }

    /// Creates (or returns the cached) fragment wgpu shader module.
    pub fn create_fragment_shader_module(
        &self,
        device: &wgpu::Device,
    ) -> Result<&wgpu::ShaderModule, ShaderError> {
        if self.language != ShaderLanguage::Wgsl {
            return Err(ShaderError::UnsupportedLanguage { stage: "fragment" });
        }
        Ok(self.fragment_module.get_or_init(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ShaderProgram fragment (WGSL)"),
                source: wgpu::ShaderSource::Wgsl(self.fragment_source.as_str().into()),
            })
        }))
    }
}

/// Parses WGSL with naga, rejecting GLSL explicitly.
fn parse_wgsl(source: &str, stage: &'static str) -> Result<naga::Module, ShaderError> {
    // Cheap heuristic: GLSL sources start with a #version directive or
    // contain GLSL-only keywords. They must never reach wgpu as WGSL.
    let trimmed = source.trim_start();
    if trimmed.starts_with("#version") || trimmed.starts_with("precision ") {
        return Err(ShaderError::UnsupportedLanguage { stage });
    }
    naga::front::wgsl::parse_str(source)
        .map_err(|e| ShaderError::Parse(format!("{stage}: {e}")))
}

/// Finds the entry point name for the given stage.
fn find_entry_point(module: &naga::Module, stage: naga::ShaderStage) -> Result<String, ShaderError> {
    module
        .entry_points
        .iter()
        .find(|ep| ep.stage == stage)
        .map(|ep| ep.name.clone())
        .ok_or(ShaderError::MissingEntryPoint {
            stage: match stage {
                naga::ShaderStage::Vertex => "vertex",
                naga::ShaderStage::Fragment => "fragment",
                _ => "compute",
            },
        })
}

/// Reflects the vertex stage: entry point name + input attributes.
fn reflect_vertex_stage(
    module: &naga::Module,
) -> Result<(String, HashMap<String, AttributeInfo>), ShaderError> {
    let entry = module
        .entry_points
        .iter()
        .find(|ep| ep.stage == naga::ShaderStage::Vertex)
        .ok_or(ShaderError::MissingEntryPoint { stage: "vertex" })?;

    let mut attributes = HashMap::new();
    for argument in &entry.function.arguments {
        let location = match &argument.binding {
            Some(naga::Binding::Location { location, .. }) => *location,
            _ => continue,
        };
        let format = vertex_format_from_type(module, argument.ty);
        let name = argument.name.clone().unwrap_or_else(|| format!("location{location}"));
        attributes.insert(
            name.clone(),
            AttributeInfo {
                name,
                gl_type: None,
                location,
                format,
            },
        );
    }
    Ok((entry.name.clone(), attributes))
}

/// Reflects global resource bindings from both stages, merging duplicates.
fn reflect_bindings(vertex: &naga::Module, fragment: &naga::Module) -> Vec<BindingInfo> {
    let mut merged: HashMap<(u32, u32), BindingInfo> = HashMap::new();
    for module in [vertex, fragment] {
        for (_, global) in module.global_variables.iter() {
            let resource = match &global.binding {
                Some(resource) => resource,
                None => continue,
            };
            let (kind, byte_size) = match global.space {
                naga::ir::AddressSpace::Uniform => {
                    let size = type_byte_size(module, global.ty);
                    (BindingKind::UniformBuffer, size)
                }
                naga::ir::AddressSpace::Storage { .. } => {
                    let size = type_byte_size(module, global.ty);
                    (BindingKind::StorageBuffer, size)
                }
                _ => match &module.types[global.ty].inner {
                    naga::ir::TypeInner::Image { .. } => {
                        match module.types[global.ty].inner {
                            naga::ir::TypeInner::Image { class: naga::ir::ImageClass::Storage { .. }, .. } => {
                                (BindingKind::StorageTexture, 0)
                            }
                            _ => (BindingKind::Texture, 0),
                        }
                    }
                    naga::ir::TypeInner::Sampler { .. } => (BindingKind::Sampler, 0),
                    _ => continue,
                },
            };
            let info = BindingInfo {
                name: global.name.clone().unwrap_or_default(),
                group: resource.group,
                binding: resource.binding,
                kind,
                byte_size,
                has_dynamic_offset: kind == BindingKind::UniformBuffer,
            };
            merged.insert((resource.group, resource.binding), info);
        }
    }
    let mut result: Vec<BindingInfo> = merged.into_values().collect();
    result.sort_by_key(|info| (info.group, info.binding));
    result
}

/// Maps a naga IR type to the corresponding `wgpu::BindingType`.
fn binding_type(info: &BindingInfo) -> wgpu::BindingType {
    match info.kind {
        BindingKind::UniformBuffer => wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: info.has_dynamic_offset,
            min_binding_size: None,
        },
        BindingKind::StorageBuffer => wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        BindingKind::Texture => wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        BindingKind::StorageTexture => wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        BindingKind::Sampler => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    }
}

/// Derives a `wgpu::VertexFormat` from a naga IR type (vertex input).
fn vertex_format_from_type(module: &naga::Module, ty: naga::Handle<naga::Type>) -> Option<wgpu::VertexFormat> {
    let scalar_format = |scalar: naga::ir::Scalar, size: naga::ir::VectorSize| {
        use naga::ir::{ScalarKind, VectorSize};
        match (scalar.kind, scalar.width, size) {
            (ScalarKind::Float, 4, VectorSize::Bi) => Some(wgpu::VertexFormat::Float32x2),
            (ScalarKind::Float, 4, VectorSize::Tri) => Some(wgpu::VertexFormat::Float32x3),
            (ScalarKind::Float, 4, VectorSize::Quad) => Some(wgpu::VertexFormat::Float32x4),
            _ => None,
        }
    };
    match &module.types[ty].inner {
        naga::ir::TypeInner::Vector { size, scalar } => scalar_format(*scalar, *size),
        naga::ir::TypeInner::Scalar(scalar) => {
            scalar_format(*scalar, naga::ir::VectorSize::Bi).map(|_| match scalar.width {
                _ => wgpu::VertexFormat::Float32,
            })
        }
        naga::ir::TypeInner::Matrix { columns, rows, .. } => {
            // mat4 inputs are split into 4 vec4 slots by WGSL lowering; the
            // first location carries the first column.
            match (columns, rows) {
                (naga::ir::VectorSize::Quad, naga::ir::VectorSize::Quad) => {
                    Some(wgpu::VertexFormat::Float32x4)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Computes the byte size (span) of a naga IR type, resolving structs.
fn type_byte_size(module: &naga::Module, ty: naga::Handle<naga::Type>) -> u32 {
    match &module.types[ty].inner {
        naga::ir::TypeInner::Struct { span, .. } => *span,
        naga::ir::TypeInner::Scalar(scalar) => scalar.width as u32,
        naga::ir::TypeInner::Vector { size, scalar } => {
            scalar.width as u32 * match size {
                naga::ir::VectorSize::Bi => 2,
                naga::ir::VectorSize::Tri => 3,
                naga::ir::VectorSize::Quad => 4,
            }
        }
        naga::ir::TypeInner::Matrix { columns, rows, scalar } => {
            scalar.width as u32
                * match rows {
                    naga::ir::VectorSize::Bi => 2,
                    naga::ir::VectorSize::Tri => 3,
                    naga::ir::VectorSize::Quad => 4,
                }
                * match columns {
                    naga::ir::VectorSize::Bi => 2,
                    naga::ir::VectorSize::Tri => 3,
                    naga::ir::VectorSize::Quad => 4,
                }
        }
        naga::ir::TypeInner::Array { size, stride, .. } => {
            let count = match *size {
                naga::ir::ArraySize::Constant(n) => n.get(),
                _ => 0,
            };
            stride * count
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_shaders::wgsl as shaders;

    #[test]
    fn viewport_quad_color_program_reflects() {
        let program = ShaderProgram::from_wgsl(
            shaders::VIEWPORT_QUAD_VS,
            shaders::VIEWPORT_QUAD_COLOR_FS,
            "viewport_quad_color".to_string(),
        )
        .expect("hand-written WGSL must parse");

        assert_eq!(program.language(), ShaderLanguage::Wgsl);
        assert_eq!(program.vertex_entry(), "main");
        assert_eq!(program.fragment_entry(), "main");

        // VS inputs: position3DAndHeight @location(0),
        // textureCoordAndEncodedAttributes @location(1), both vec4<f32>.
        assert_eq!(program.attributes().len(), 2);
        let position = program.get_attribute("position3DAndHeight").unwrap();
        assert_eq!(position.location, 0);
        assert_eq!(position.format, Some(wgpu::VertexFormat::Float32x4));
        let texcoord = program.get_attribute("textureCoordAndEncodedAttributes").unwrap();
        assert_eq!(texcoord.location, 1);
        assert_eq!(texcoord.format, Some(wgpu::VertexFormat::Float32x4));

        // FS material uniform: group(1) binding(0), 16 bytes (vec4 color).
        let material = program
            .bindings()
            .iter()
            .find(|b| b.group == 1 && b.binding == 0)
            .expect("material uniform buffer binding");
        assert_eq!(material.kind, BindingKind::UniformBuffer);
        assert_eq!(material.byte_size, 16);
        assert!(material.has_dynamic_offset);
    }

    #[test]
    fn viewport_quad_texture_program_reflects_texture_and_sampler() {
        let program = ShaderProgram::from_wgsl(
            shaders::VIEWPORT_QUAD_VS,
            shaders::VIEWPORT_QUAD_TEXTURE_FS,
            "viewport_quad_texture".to_string(),
        )
        .expect("hand-written WGSL must parse");

        let texture = program
            .bindings()
            .iter()
            .find(|b| b.group == 1 && b.kind == BindingKind::Texture)
            .expect("u_texture binding");
        assert_eq!(texture.binding, 0);
        let sampler = program
            .bindings()
            .iter()
            .find(|b| b.group == 1 && b.kind == BindingKind::Sampler)
            .expect("u_sampler binding");
        assert_eq!(sampler.binding, 1);
        assert_eq!(program.sampler_uniforms(), &["u_sampler".to_string()]);
    }

    #[test]
    fn globe_program_reflects_automatic_uniform_block() {
        let program = ShaderProgram::from_wgsl(
            shaders::GLOBE_VS,
            shaders::GLOBE_FS,
            "globe_texonly".to_string(),
        )
        .expect("hand-written WGSL must parse");

        // group(0) binding(0): CesiumAutomaticUniforms, 5*64 + 16 = 336 bytes.
        let czm = program
            .bindings()
            .iter()
            .find(|b| b.group == 0 && b.binding == 0)
            .expect("automatic uniforms binding");
        assert_eq!(czm.kind, BindingKind::UniformBuffer);
        assert_eq!(czm.byte_size, cesium_shaders::wgsl::CESIUM_AUTOMATIC_UNIFORMS_SIZE as u32);

        // group(1): day texture + sampler from the fragment stage.
        assert!(program
            .bindings()
            .iter()
            .any(|b| b.group == 1 && b.kind == BindingKind::Texture));
        assert!(program
            .bindings()
            .iter()
            .any(|b| b.group == 1 && b.kind == BindingKind::Sampler));

        // Bind group layout entries are merged per group.
        let layout = program.bind_group_layout_entries();
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].0, 0);
        assert_eq!(layout[0].1.len(), 1);
        assert_eq!(layout[1].0, 1);
        assert_eq!(layout[1].1.len(), 2);
    }

    #[test]
    fn glsl_source_is_rejected_explicitly() {
        let glsl = "#version 460\nvoid main() { gl_Position = vec4(0.0); }\n";
        let error = ShaderProgram::from_wgsl(glsl, glsl, "glsl".to_string())
            .expect_err("GLSL must not be accepted as WGSL");
        assert!(matches!(
            error,
            ShaderError::UnsupportedLanguage { stage: "vertex" }
        ));

        // Legacy constructor keeps the sources but module creation fails.
        let program = ShaderProgram::new(glsl.to_string(), glsl.to_string());
        assert_eq!(program.language(), ShaderLanguage::Glsl);
    }

    #[test]
    fn invalid_wgsl_reports_parse_error() {
        let error = ShaderProgram::from_wgsl("fn main( ->", "fn main() {}", "bad".to_string())
            .expect_err("invalid WGSL must fail");
        assert!(matches!(error, ShaderError::Parse(_)));
    }
}
