//! 着色器管线的领域模型。
//!
//! 映射到 CesiumJS `Renderer/ShaderProgram.js`、`Renderer/ShaderSource.js`、
//! `Renderer/ShaderBuilder.js`、`Renderer/ShaderCache.js`、
//! `Renderer/ShaderFunction.js`、`Renderer/ShaderStruct.js`。
//!
//! 这些是表示着色器编译管线的纯领域模型。
//! 实际的 GPU 编译由 Bevy 渲染适配器处理。

use std::collections::HashMap;

/// 着色器阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

/// 带元数据的 GLSL 着色器源代码。
///
/// 映射到 CesiumJS `Renderer/ShaderSource.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderSource {
    /// GLSL 源代码。
    pub sources: Vec<String>,
    /// 着色器阶段。
    pub stage: ShaderStage,
    /// 这是否为一个内置着色器。
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

    /// 将多个源代码合并为一个。
    pub fn combined_source(&self) -> String {
        self.sources.join("\n")
    }

    /// 追加源代码。
    pub fn append(&mut self, source: &str) {
        self.sources.push(source.to_string());
    }
}

/// 着色器中的一个 uniform 声明。
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderUniform {
    pub name: String,
    pub glsl_type: String,
    pub count: usize,
}

/// 着色器中的一个结构体声明。
///
/// 映射到 CesiumJS `Renderer/ShaderStruct.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderStruct {
    pub name: String,
    pub fields: Vec<ShaderUniform>,
}

/// 着色器中的一个函数声明。
///
/// 映射到 CesiumJS `Renderer/ShaderFunction.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderFunction {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<ShaderUniform>,
    pub body: String,
}

/// 用于逐步构建着色器的着色器构建器。
///
/// 映射到 CesiumJS `Renderer/ShaderBuilder.js`
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

    /// 添加一个 uniform 声明。
    pub fn add_uniform(&mut self, name: &str, glsl_type: &str) -> &mut Self {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
        self
    }

    /// 添加一个 uniform 数组声明。
    pub fn add_uniform_array(&mut self, name: &str, glsl_type: &str, count: usize) -> &mut Self {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count,
        });
        self
    }

    /// 添加一个结构体声明。
    pub fn add_struct(&mut self, name: &str, fields: Vec<ShaderUniform>) -> &mut Self {
        self.structs.push(ShaderStruct {
            name: name.to_string(),
            fields,
        });
        self
    }

    /// 添加一个函数声明。
    pub fn add_function(&mut self, func: ShaderFunction) -> &mut Self {
        self.functions.push(func);
        self
    }

    /// 添加一个预处理器 define。
    pub fn add_define(&mut self, name: &str, value: &str) -> &mut Self {
        self.defines.insert(name.to_string(), value.to_string());
        self
    }

    /// 追加顶点着色器源代码。
    pub fn append_vertex(&mut self, source: &str) -> &mut Self {
        self.vertex_source.append(source);
        self
    }

    /// 追加片元着色器源代码。
    pub fn append_fragment(&mut self, source: &str) -> &mut Self {
        self.fragment_source.append(source);
        self
    }

    /// 构建最终的顶点着色器源代码。
    pub fn build_vertex_source(&self) -> String {
        let mut result = String::new();

        // 预处理定义
        for (name, value) in &self.defines {
            result.push_str(&format!("#define {} {}\n", name, value));
        }

        // 结构体
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

        // Uniform
        for u in &self.uniforms {
            if u.count > 1 {
                result.push_str(&format!("uniform {} {}[{}];\n", u.glsl_type, u.name, u.count));
            } else {
                result.push_str(&format!("uniform {} {};\n", u.glsl_type, u.name));
            }
        }

        // 函数
        for f in &self.functions {
            let params: Vec<String> = f.parameters.iter()
                .map(|p| format!("{} {}", p.glsl_type, p.name))
                .collect();
            result.push_str(&format!("{} {}({}) {{\n", f.return_type, f.name, params.join(", ")));
            result.push_str(&f.body);
            result.push_str("\n}\n");
        }

        // 主源代码
        result.push_str(&self.vertex_source.combined_source());
        result
    }

    /// 构建最终的片元着色器源代码。
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

/// 一个已编译的着色器程序（领域表示）。
///
/// 映射到 CesiumJS `Renderer/ShaderProgram.js`
#[derive(Debug, Clone)]
pub struct ShaderProgram {
    /// 唯一标识符。
    pub id: u64,
    /// 顶点着色器源代码。
    pub vertex_shader: ShaderSource,
    /// 片元着色器源代码。
    pub fragment_shader: ShaderSource,
    /// Uniform 声明。
    pub uniforms: Vec<ShaderUniform>,
    /// 属性声明。
    pub attributes: Vec<ShaderUniform>,
    /// 该程序是否就绪。
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

    /// 添加一个 uniform 声明。
    pub fn add_uniform(&mut self, name: &str, glsl_type: &str) {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
    }

    /// 添加一个属性声明。
    pub fn add_attribute(&mut self, name: &str, glsl_type: &str) {
        self.attributes.push(ShaderUniform {
            name: name.to_string(),
            glsl_type: glsl_type.to_string(),
            count: 1,
        });
    }

    /// 将该程序标记为就绪。
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }
}

/// 用于复用已编译程序的着色器缓存。
///
/// 映射到 CesiumJS `Renderer/ShaderCache.js`
#[derive(Debug, Default)]
pub struct ShaderCache {
    programs: HashMap<u64, ShaderProgram>,
    next_id: u64,
}

impl ShaderCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取或创建一个着色器程序。
    pub fn get_or_create(
        &mut self,
        vertex: ShaderSource,
        fragment: ShaderSource,
    ) -> u64 {
        // 基于哈希的简单去重
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

    /// 按 ID 获取一个程序。
    pub fn get(&self, id: u64) -> Option<&ShaderProgram> {
        self.programs.values().find(|p| p.id == id)
    }

    /// 已缓存程序的数量。
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
        assert_eq!(id1, id2); // 相同源代码 → 相同 ID
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
