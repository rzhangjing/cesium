//! Shader pipeline domain models.
//!
//! Maps to CesiumJS `Renderer/ShaderProgram.js`, `Renderer/ShaderSource.js`,
//! `Renderer/ShaderBuilder.js`, `Renderer/ShaderCache.js`,
//! `Renderer/ShaderFunction.js`, `Renderer/ShaderStruct.js`.
//!
//! These are pure domain models representing the shader compilation pipeline.
//! The actual GPU compilation is handled by the Bevy render adapter.

use std::collections::HashMap;

/// Shader stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

/// A GLSL shader source with metadata.
///
/// Maps to CesiumJS `Renderer/ShaderSource.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderSource {
    /// GLSL source code.
    pub sources: Vec<String>,
    /// Shader stage.
    pub stage: ShaderStage,
    /// Whether this is a built-in shader.
    pub is_builtin: bool,
}

impl Default for ShaderSource {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            stage: ShaderStage::Vertex,
            is_builtin: false,
        }
    }
}

impl ShaderSource {
    pub fn new(source: &str, stage: ShaderStage) -> Self {
        Self {
            sources: vec![source.to_string()],
            stage,
            is_builtin: false,
        }
    }

    pub fn builtin(source: &str, stage: ShaderStage) -> Self {
        Self {
            sources: vec![source.to_string()],
            stage,
            is_builtin: true,
        }
    }

    /// Combine multiple sources into one.
    pub fn combined_source(&self) -> String {
        self.sources.join("\n")
    }

    /// Append source code.
    pub fn append(&mut self, source: &str) {
        self.sources.push(source.to_string());
    }
}

/// A uniform declaration in a shader.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderUniform {
    pub name: String,
    pub glsl_type: String,
    pub count: usize,
}

/// A struct declaration in a shader.
///
/// Maps to CesiumJS `Renderer/ShaderStruct.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderStruct {
    pub name: String,
    pub fields: Vec<ShaderUniform>,
}

/// A function declaration in a shader.
///
/// Maps to CesiumJS `Renderer/ShaderFunction.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderFunction {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<ShaderUniform>,
    pub body: String,
}

/// A shader builder for constructing shaders incrementally.
///
/// Maps to CesiumJS `Renderer/ShaderBuilder.js`
#[derive(Debug, Clone, Default)]
pub struct ShaderBuilder {
    pub vertex_source: ShaderSource,
    pub fragment_source: ShaderSource,
    pub uniforms: Vec<ShaderUniform>,
    pub structs: Vec<ShaderStruct>,
    pub functions: Vec<ShaderFunction>,
    pub defines: HashMap<String, String>,
}

impl ShaderBuilder {
    pub fn new() -> Self {
        Self {
            vertex_source: ShaderSource::new("", ShaderStage::Vertex),
            fragment_source: ShaderSource::new("", ShaderStage::Fragment),
            ..Default::default()
        }
    }

    /// Add a uniform declaration.
    pub fn add_uniform(&mut self, name: &str, glsl_type: &str) -> &mut Self {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
        self
    }

    /// Add a uniform array declaration.
    pub fn add_uniform_array(&mut self, name: &str, glsl_type: &str, count: usize) -> &mut Self {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count,
        });
        self
    }

    /// Add a struct declaration.
    pub fn add_struct(&mut self, name: &str, fields: Vec<ShaderUniform>) -> &mut Self {
        self.structs.push(ShaderStruct {
            name: name.to_string(),
            fields,
        });
        self
    }

    /// Add a function declaration.
    pub fn add_function(&mut self, func: ShaderFunction) -> &mut Self {
        self.functions.push(func);
        self
    }

    /// Add a preprocessor define.
    pub fn add_define(&mut self, name: &str, value: &str) -> &mut Self {
        self.defines.insert(name.to_string(), value.to_string());
        self
    }

    /// Append vertex shader source.
    pub fn append_vertex(&mut self, source: &str) -> &mut Self {
        self.vertex_source.append(source);
        self
    }

    /// Append fragment shader source.
    pub fn append_fragment(&mut self, source: &str) -> &mut Self {
        self.fragment_source.append(source);
        self
    }

    /// Build the final vertex shader source.
    pub fn build_vertex_source(&self) -> String {
        let mut result = String::new();

        // Defines
        for (name, value) in &self.defines {
            result.push_str(&format!("#define {} {}\n", name, value));
        }

        // Structs
        for s in &self.structs {
            result.push_str(&format!("struct {} {{\n", s.name));
            for f in &s.fields {
                if f.count > 1 {
                    result.push_str(&format!("    {} {}[{}];\n", f.glsl_type, f.name, f.count));
                } else {
                    result.push_str(&format!("    {} {};\n", f.glsl_type, f.name));
                }
            }
            result.push_str("};\n");
        }

        // Uniforms
        for u in &self.uniforms {
            if u.count > 1 {
                result.push_str(&format!("uniform {} {}[{}];\n", u.glsl_type, u.name, u.count));
            } else {
                result.push_str(&format!("uniform {} {};\n", u.glsl_type, u.name));
            }
        }

        // Functions
        for f in &self.functions {
            let params: Vec<String> = f.parameters.iter()
                .map(|p| format!("{} {}", p.glsl_type, p.name))
                .collect();
            result.push_str(&format!("{} {}({}) {{\n", f.return_type, f.name, params.join(", ")));
            result.push_str(&f.body);
            result.push_str("\n}\n");
        }

        // Main source
        result.push_str(&self.vertex_source.combined_source());
        result
    }

    /// Build the final fragment shader source.
    pub fn build_fragment_source(&self) -> String {
        let mut result = String::new();

        for (name, value) in &self.defines {
            result.push_str(&format!("#define {} {}\n", name, value));
        }

        for s in &self.structs {
            result.push_str(&format!("struct {} {{\n", s.name));
            for f in &s.fields {
                result.push_str(&format!("    {} {};\n", f.glsl_type, f.name));
            }
            result.push_str("};\n");
        }

        for u in &self.uniforms {
            result.push_str(&format!("uniform {} {};\n", u.glsl_type, u.name));
        }

        for f in &self.functions {
            let params: Vec<String> = f.parameters.iter()
                .map(|p| format!("{} {}", p.glsl_type, p.name))
                .collect();
            result.push_str(&format!("{} {}({}) {{\n", f.return_type, f.name, params.join(", ")));
            result.push_str(&f.body);
            result.push_str("\n}\n");
        }

        result.push_str(&self.fragment_source.combined_source());
        result
    }
}

/// A compiled shader program (domain representation).
///
/// Maps to CesiumJS `Renderer/ShaderProgram.js`
#[derive(Debug, Clone)]
pub struct ShaderProgram {
    /// Unique identifier.
    pub id: u64,
    /// Vertex shader source.
    pub vertex_shader: ShaderSource,
    /// Fragment shader source.
    pub fragment_shader: ShaderSource,
    /// Uniform declarations.
    pub uniforms: Vec<ShaderUniform>,
    /// Attribute declarations.
    pub attributes: Vec<ShaderUniform>,
    /// Whether the program is ready.
    pub ready: bool,
}

impl ShaderProgram {
    pub fn new(id: u64, vertex: ShaderSource, fragment: ShaderSource) -> Self {
        Self {
            id,
            vertex_shader: vertex,
            fragment_shader: fragment,
            uniforms: Vec::new(),
            attributes: Vec::new(),
            ready: false,
        }
    }

    /// Add a uniform declaration.
    pub fn add_uniform(&mut self, name: &str, glsl_type: &str) {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
    }

    /// Add an attribute declaration.
    pub fn add_attribute(&mut self, name: &str, glsl_type: &str) {
        self.attributes.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
    }

    /// Mark the program as ready.
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }
}

/// Shader cache for reusing compiled programs.
///
/// Maps to CesiumJS `Renderer/ShaderCache.js`
#[derive(Debug, Default)]
pub struct ShaderCache {
    programs: HashMap<u64, ShaderProgram>,
    next_id: u64,
}

impl ShaderCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a shader program.
    pub fn get_or_create(
        &mut self,
        vertex: ShaderSource,
        fragment: ShaderSource,
    ) -> u64 {
        // Simple hash-based dedup
        let hash = {
            let v = vertex.combined_source();
            let f = fragment.combined_source();
            let mut h: u64 = 0;
            for b in v.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            for b in f.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            h
        };

        if let Some(program) = self.programs.get(&hash) {
            return program.id;
        }

        let id = self.next_id;
        self.next_id += 1;
        let mut program = ShaderProgram::new(id, vertex, fragment);
        program.mark_ready();
        self.programs.insert(hash, program);
        id
    }

    /// Get a program by ID.
    pub fn get(&self, id: u64) -> Option<&ShaderProgram> {
        self.programs.values().find(|p| p.id == id)
    }

    /// Number of cached programs.
    pub fn len(&self) -> usize {
        self.programs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_source() {
        let mut src = ShaderSource::new("void main() {}", ShaderStage::Vertex);
        assert_eq!(src.stage, ShaderStage::Vertex);
        assert!(!src.is_builtin);
        src.append("// extra");
        assert_eq!(src.sources.len(), 2);
        assert!(src.combined_source().contains("void main()"));
    }

    #[test]
    fn test_shader_builder() {
        let mut builder = ShaderBuilder::new();
        builder
            .add_uniform("u_color", "vec4")
            .add_define("HAS_TEXTURE", "1")
            .append_vertex("gl_Position = vec4(0.0);")
            .append_fragment("gl_FragColor = u_color;");

        let vs = builder.build_vertex_source();
        assert!(vs.contains("#define HAS_TEXTURE 1"));
        assert!(vs.contains("uniform vec4 u_color;"));
        assert!(vs.contains("gl_Position"));

        let fs = builder.build_fragment_source();
        assert!(fs.contains("uniform vec4 u_color;"));
        assert!(fs.contains("gl_FragColor"));
    }

    #[test]
    fn test_shader_builder_struct() {
        let mut builder = ShaderBuilder::new();
        builder.add_struct("Material", vec![
            ShaderUniform { name: "diffuse".to_string(), glsl_type: "vec3".to_string(), count: 1 },
            ShaderUniform { name: "alpha".to_string(), glsl_type: "float".to_string(), count: 1 },
        ]);

        let vs = builder.build_vertex_source();
        assert!(vs.contains("struct Material {"));
        assert!(vs.contains("vec3 diffuse;"));
        assert!(vs.contains("float alpha;"));
    }

    #[test]
    fn test_shader_builder_function() {
        let mut builder = ShaderBuilder::new();
        builder.add_function(ShaderFunction {
            name: "getAlpha".to_string(),
            return_type: "float".to_string(),
            parameters: vec![ShaderUniform {
                name: "x".to_string(),
                glsl_type: "float".to_string(),
                count: 1,
            }],
            body: "return x * 0.5;".to_string(),
        });

        let vs = builder.build_vertex_source();
        assert!(vs.contains("float getAlpha(float x) {"));
        assert!(vs.contains("return x * 0.5;"));
    }

    #[test]
    fn test_shader_program() {
        let mut prog = ShaderProgram::new(
            0,
            ShaderSource::new("void main() {}", ShaderStage::Vertex),
            ShaderSource::new("void main() {}", ShaderStage::Fragment),
        );
        prog.add_uniform("u_color", "vec4");
        prog.add_attribute("a_position", "vec3");
        assert!(!prog.ready);
        prog.mark_ready();
        assert!(prog.ready);
        assert_eq!(prog.uniforms.len(), 1);
        assert_eq!(prog.attributes.len(), 1);
    }

    #[test]
    fn test_shader_cache() {
        let mut cache = ShaderCache::new();
        let id1 = cache.get_or_create(
            ShaderSource::new("void main() {}", ShaderStage::Vertex),
            ShaderSource::new("void main() {}", ShaderStage::Fragment),
        );
        let id2 = cache.get_or_create(
            ShaderSource::new("void main() {}", ShaderStage::Vertex),
            ShaderSource::new("void main() {}", ShaderStage::Fragment),
        );
        assert_eq!(id1, id2); // Same source → same ID
        assert_eq!(cache.len(), 1);

        let id3 = cache.get_or_create(
            ShaderSource::new("void main() { gl_Position = vec4(1.0); }", ShaderStage::Vertex),
            ShaderSource::new("void main() {}", ShaderStage::Fragment),
        );
        assert_ne!(id1, id3);
        assert_eq!(cache.len(), 2);

        assert!(cache.get(id1).is_some());
        assert!(cache.get(999).is_none());
    }
}
