//! Ported from `packages/engine/Source/Renderer/ShaderBuilder.js`.
//!
//! Dynamically builds shader programs from components.

use crate::shader_destination::ShaderDestination;
use crate::shader_function::ShaderFunction;
use crate::shader_struct::ShaderStruct;

/// Dynamically builds shader programs from components.
///
/// Allows adding uniforms, attributes, functions, and struct definitions
/// that are assembled into final GLSL source code.
pub struct ShaderBuilder {
    vertex_sources: Vec<String>,
    fragment_sources: Vec<String>,
    defines: Vec<String>,
    structs: Vec<ShaderStruct>,
    functions: Vec<ShaderFunction>,
    uniforms: Vec<(String, String)>,
    attributes: Vec<(String, String)>,
}

impl ShaderBuilder {
    /// Creates a new shader builder.
    pub fn new() -> Self {
        Self {
            vertex_sources: Vec::new(),
            fragment_sources: Vec::new(),
            defines: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            uniforms: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Adds a source string to the vertex shader.
    pub fn add_vertex_source(&mut self, source: String) {
        self.vertex_sources.push(source);
    }

    /// Adds a source string to the fragment shader.
    pub fn add_fragment_source(&mut self, source: String) {
        self.fragment_sources.push(source);
    }

    /// Adds a `#define` to both shaders.
    pub fn add_define(&mut self, define: String) {
        self.defines.push(define);
    }

    /// Adds a struct definition.
    pub fn add_struct(&mut self, s: ShaderStruct) {
        self.structs.push(s);
    }

    /// Adds a function.
    pub fn add_function(&mut self, f: ShaderFunction) {
        self.functions.push(f);
    }

    /// Adds a uniform declaration.
    pub fn add_uniform(&mut self, glsl_type: String, name: String) {
        self.uniforms.push((glsl_type, name));
    }

    /// Adds an attribute declaration.
    pub fn add_attribute(&mut self, glsl_type: String, name: String) {
        self.attributes.push((glsl_type, name));
    }

    /// Builds the vertex shader source.
    pub fn build_vertex_source(&self) -> String {
        let mut result = String::new();
        for def in &self.defines { result.push_str(&format!("#define {def}\n")); }
        for (t, n) in &self.uniforms { result.push_str(&format!("uniform {t} {n};\n")); }
        for (t, n) in &self.attributes { result.push_str(&format!("in {t} {n};\n")); }
        for s in &self.structs { result.push_str(&s.to_glsl()); }
        for f in &self.functions {
            if f.shader_destination.includes_vertex_shader() {
                result.push_str(&f.to_glsl());
            }
        }
        for src in &self.vertex_sources { result.push_str(src); result.push('\n'); }
        result
    }

    /// Builds the fragment shader source.
    pub fn build_fragment_source(&self) -> String {
        let mut result = String::new();
        for def in &self.defines { result.push_str(&format!("#define {def}\n")); }
        for (t, n) in &self.uniforms { result.push_str(&format!("uniform {t} {n};\n")); }
        for s in &self.structs { result.push_str(&s.to_glsl()); }
        for f in &self.functions {
            if f.shader_destination.includes_fragment_shader() {
                result.push_str(&f.to_glsl());
            }
        }
        for src in &self.fragment_sources { result.push_str(src); result.push('\n'); }
        result
    }
}

impl Default for ShaderBuilder {
    fn default() -> Self { Self::new() }
}
