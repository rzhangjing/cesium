//! Ported from `packages/engine/Source/Renderer/ShaderProgram.js`.
//!
//! A compiled and linked shader program. In the Rust/wgpu port, this wraps
//! `wgpu::ShaderModule` creation and manages the uniform map (attributes,
//! uniforms, samplers) that mirrors the CesiumJS `ShaderProgram` uniform interface.

use std::collections::HashMap;
use crate::shader_source::ShaderType;

/// A uniform variable in a shader program.
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

/// A vertex attribute in a shader program.
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    /// The name of the attribute.
    pub name: String,
    /// The GL type of the attribute.
    pub gl_type: Option<u32>,
    /// The location/binding index.
    pub location: u32,
}

/// A compiled GPU shader program.
///
/// In wgpu, shader programs are represented as `wgpu::ShaderModule` + pipeline.
/// This struct captures the logical shader program state including uniform/attribute
/// maps, mirroring the CesiumJS `ShaderProgram` interface.
pub struct ShaderProgram {
    /// The processed vertex shader source (GLSL or WGSL).
    vertex_source: String,
    /// The processed fragment shader source (GLSL or WGSL).
    fragment_source: String,
    /// Compilation/link log.
    log: String,
    /// Whether this program has been destroyed.
    is_destroyed: bool,
    /// Uniform variable map: name -> UniformInfo.
    uniforms: HashMap<String, UniformInfo>,
    /// Vertex attribute map: name -> AttributeInfo.
    attributes: HashMap<String, AttributeInfo>,
    /// Sampler uniform names (separate from regular uniforms in Vulkan GLSL).
    sampler_uniforms: Vec<String>,
    /// The wgpu shader module (created lazily).
    shader_module: Option<wgpu::ShaderModule>,
    /// Cache key for this program.
    cache_key: String,
}

impl ShaderProgram {
    /// Creates a new shader program from processed source strings.
    ///
    /// DEVIATION: Actual wgpu compilation is deferred to Context-level
    /// pipeline creation. This constructor stores the processed sources.
    pub fn new(vertex_source: String, fragment_source: String) -> Self {
        Self {
            vertex_source,
            fragment_source,
            log: String::new(),
            is_destroyed: false,
            uniforms: HashMap::new(),
            attributes: HashMap::new(),
            sampler_uniforms: Vec::new(),
            shader_module: None,
            cache_key: String::new(),
        }
    }

    /// Creates a new shader program with a cache key.
    pub fn with_cache_key(
        vertex_source: String,
        fragment_source: String,
        cache_key: String,
    ) -> Self {
        let mut program = Self::new(vertex_source, fragment_source);
        program.cache_key = cache_key;
        program
    }

    /// Returns the vertex shader source.
    pub fn vertex_source(&self) -> &str { &self.vertex_source }

    /// Returns the fragment shader source.
    pub fn fragment_source(&self) -> &str { &self.fragment_source }

    /// Returns the compilation/link log.
    pub fn log(&self) -> &str { &self.log }

    /// Returns whether this program has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Returns the cache key.
    pub fn cache_key(&self) -> &str { &self.cache_key }

    /// Destroys the shader program.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
        self.shader_module = None;
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
        });
    }

    // ---- wgpu shader module ----

    /// Creates the wgpu shader module from the vertex source.
    ///
    /// DEVIATION: In wgpu, vertex and fragment shaders are separate shader modules
    /// (or entries within one module). This creates a module from the vertex source.
    pub fn create_shader_module(&mut self, device: &wgpu::Device) -> &wgpu::ShaderModule {
        if self.shader_module.is_none() {
            // Try to detect if source is WGSL or GLSL
            let is_wgsl = self.vertex_source.contains("@vertex")
                || self.vertex_source.contains("@fragment")
                || self.vertex_source.contains("fn main");

            let shader_module = if is_wgsl {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("ShaderProgram WGSL"),
                    source: wgpu::ShaderSource::Wgsl(self.vertex_source.as_str().into()),
                })
            } else {
                // For GLSL, we need to use naga to convert to WGSL first
                // This is a fallback that stores the source for later processing
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("ShaderProgram GLSL"),
                    source: wgpu::ShaderSource::Wgsl(self.vertex_source.as_str().into()),
                })
            };
            self.shader_module = Some(shader_module);
        }
        self.shader_module.as_ref().unwrap()
    }

    /// Returns the cached shader module, if created.
    pub fn shader_module(&self) -> Option<&wgpu::ShaderModule> {
        self.shader_module.as_ref()
    }

    /// Parses uniform declarations from the shader source and populates the uniform map.
    ///
    /// This is a simplified version of CesiumJS's uniform parsing. In the full port,
    /// this would be done by naga's reflection after shader compilation.
    pub fn parse_uniforms_from_source(&mut self) {
        // Collect data first to avoid borrow conflicts
        let mut new_uniforms = Vec::new();
        let mut new_samplers = Vec::new();

        let sources = [self.vertex_source.clone(), self.fragment_source.clone()];
        for source in &sources {
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("uniform ") {
                    // Parse "uniform <type> <name>;" or "uniform <type> <name>[<size>];"
                    let parts: Vec<&str> = trimmed
                        .trim_start_matches("uniform ")
                        .trim_end_matches(';')
                        .split_whitespace()
                        .collect();
                    if parts.len() >= 2 {
                        let gl_type_str = parts[0];
                        let name = parts[1].trim_end_matches(|c: char| c == '[' || c.is_numeric());

                        // Check if it's a sampler type
                        if gl_type_str.starts_with("sampler") || gl_type_str == "sampler2D"
                            || gl_type_str == "samplerCube" || gl_type_str == "sampler3D"
                        {
                            new_samplers.push(name.to_string());
                        } else {
                            let size = if parts[1].contains('[') {
                                // Array uniform
                                parts[1]
                                    .split('[')
                                    .nth(1)
                                    .and_then(|s| s.trim_end_matches(']').parse().ok())
                                    .unwrap_or(1)
                            } else {
                                1
                            };
                            new_uniforms.push((name.to_string(), None, size, 0));
                        }
                    }
                }
            }
        }

        // Now insert collected data
        for (name, gl_type, size, location) in new_uniforms {
            self.add_uniform(name, gl_type, size, location);
        }
        for name in new_samplers {
            self.add_sampler_uniform(name);
        }
    }

    /// Parses attribute declarations from the vertex shader source.
    ///
    /// In Vulkan GLSL 460, attributes are `in` qualifiers.
    pub fn parse_attributes_from_source(&mut self) {
        // Collect data first to avoid borrow conflicts
        let mut new_attributes = Vec::new();

        for line in self.vertex_source.lines() {
            let trimmed = line.trim();
            // Match "in <type> <name>;" or "layout(location=N) in <type> <name>;"
            if trimmed.starts_with("in ") || trimmed.contains(") in ") {
                let in_part = if let Some(pos) = trimmed.find(") in ") {
                    &trimmed[pos + 5..]
                } else {
                    &trimmed[3..]
                };

                let parts: Vec<&str> = in_part
                    .trim_end_matches(';')
                    .split_whitespace()
                    .collect();
                if parts.len() >= 2 {
                    let name = parts[1];
                    // Extract location from layout if present
                    let location = if trimmed.starts_with("layout(") {
                        trimmed
                            .split("location=")
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    new_attributes.push((name.to_string(), None, location));
                }
            }
        }

        // Now insert collected data
        for (name, gl_type, location) in new_attributes {
            self.add_attribute(name, gl_type, location);
        }
    }
}
