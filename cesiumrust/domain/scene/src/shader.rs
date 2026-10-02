//! 着色器管线的领域模型。
//!
//! 涵盖着色器源码、结构体与函数声明、逐步构建器、已编译程序与复用缓存等表示。
//!
//! 这些是表示着色器编译管线的纯领域模型。
//! 实际的 GPU 编译由 Bevy 渲染适配器处理。

use std::collections::HashMap;

/// 着色器阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// 顶点着色器阶段。
    Vertex,
    /// 片元着色器阶段。
    Fragment,
    /// 计算着色器阶段。
    Compute,
}

/// 带元数据的 GLSL 着色器源代码。
///
/// 持有若干源码片段及其目标阶段，可在构建时逐段追加后合并。
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
    /// 默认为空源码、顶点阶段、非内置。
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            stage: ShaderStage::Vertex,
            is_builtin: false,
        }
    }
}

impl ShaderSource {
    /// 以单段源码与目标阶段创建非内置着色器源码。
    pub fn new(source: &str, stage: ShaderStage) -> Self {
        Self {
            sources: vec![source.to_string()],
            stage,
            is_builtin: false,
        }
    }

    /// 创建一个标记为内置的着色器源码。
    pub fn builtin(source: &str, stage: ShaderStage) -> Self {
        Self {
            sources: vec![source.to_string()],
            stage,
            is_builtin: true,
        }
    }

    /// 将多个源代码合并为一个。
    pub fn combined_source(&self) -> String {
        // 逐段以换行拼接，保持源码片段的追加顺序
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
    /// 变量名。
    pub name: String,
    /// GLSL 类型名（如 vec4）。
    pub glsl_type: String,
    /// 数组长度；1 表示标量。
    pub count: usize,
}

/// 着色器中的一个结构体声明。
///
/// 由名称与有序字段列表构成，构建时展开为 GLSL struct。
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderStruct {
    /// 结构体名。
    pub name: String,
    /// 字段列表（复用 uniform 声明结构）。
    pub fields: Vec<ShaderUniform>,
}

/// 着色器中的一个函数声明。
///
/// 捕获返回类型、参数与函数体，构建时拼装为可注入的 GLSL 函数。
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderFunction {
    /// 函数名。
    pub name: String,
    /// 返回值类型名。
    pub return_type: String,
    /// 参数列表（复用 uniform 声明结构）。
    pub parameters: Vec<ShaderUniform>,
    /// 函数体源代码。
    pub body: String,
}

/// 用于逐步构建着色器的着色器构建器。
///
/// 汇集源码片段、uniform、结构体、函数与预处理定义，最终生成完整 GLSL。
#[derive(Debug, Clone, Default)]
pub struct ShaderBuilder {
    /// 顶点阶段源码累加器。
    pub vertex_source: ShaderSource,
    /// 片元阶段源码累加器。
    pub fragment_source: ShaderSource,
    /// 待声明的 uniform 列表。
    pub uniforms: Vec<ShaderUniform>,
    /// 待声明的结构体列表。
    pub structs: Vec<ShaderStruct>,
    /// 待注入的函数列表。
    pub functions: Vec<ShaderFunction>,
    /// 预处理器 #define 名值表。
    pub defines: HashMap<String, String>,
}

impl ShaderBuilder {
    /// 创建带空顶点/片元源码的构建器。
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

        // 预处理定义：与顶点阶段共享同一套 #define
        for (name, value) in &self.defines {
            result.push_str(&format!("#define {} {}\n", name, value));
        }

        // 结构体声明（片元侧不区分数组字段，逐个展开为普通成员）
        for s in &self.structs {
            result.push_str(&format!("struct {} {{\n", s.name));
            for f in &s.fields {
                result.push_str(&format!("    {} {};\n", f.glsl_type, f.name));
            }
            result.push_str("};\n");
        }

        // Uniform 声明
        for u in &self.uniforms {
            result.push_str(&format!("uniform {} {};\n", u.glsl_type, u.name));
        }

        // 函数字段：拼装参数列表后接函数体
        for f in &self.functions {
            let params: Vec<String> = f.parameters.iter()
                .map(|p| format!("{} {}", p.glsl_type, p.name))
                .collect();
            result.push_str(&format!("{} {}({}) {{\n", f.return_type, f.name, params.join(", ")));
            result.push_str(&f.body);
            result.push_str("\n}\n");
        }

        // 主源代码：所有累加的片元源码片段合并到末尾
        result.push_str(&self.fragment_source.combined_source());
        result
    }
}

/// 一个已编译的着色器程序（领域表示）。
///
/// 绑定顶点/片元源码及其 uniform 与 attribute 声明，并记录就绪状态。
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
    /// 以给定源码新建尚未就绪的着色器程序。
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
/// 以源码哈希去重，命中则返回既有程序 ID，避免重复编译。
#[derive(Debug, Default)]
pub struct ShaderCache {
    /// 源码哈希到已编译程序的映射。
    programs: HashMap<u64, ShaderProgram>,
    /// 递增分配的程序 ID。
    next_id: u64,
}

impl ShaderCache {
    /// 创建空缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取或创建一个着色器程序。
    pub fn get_or_create(
        &mut self,
        vertex: ShaderSource,
        fragment: ShaderSource,
    ) -> u64 {
        // 基于哈希的简单去重：顶点与片元源码依次混入同一 31 进制滚动哈希
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

        // 命中缓存则直接复用既有程序 ID，避免重复创建
        if let Some(program) = self.programs.get(&hash) {
            return program.id;
        }

        // 未命中则分配新 ID、标记就绪并入库
        let id = self.next_id;
        self.next_id += 1;
        let mut program = ShaderProgram::new(id, vertex, fragment);
        program.mark_ready();
        self.programs.insert(hash, program);
        id
    }

    /// 按 ID 获取一个程序。
    pub fn get(&self, id: u64) -> Option<&ShaderProgram> {
        // 缓存以哈希为键，故按 ID 获取需线性扫描各程序值
        self.programs.values().find(|p| p.id == id)
    }

    /// 已缓存程序的数量。
    pub fn len(&self) -> usize {
        // 直接取哈希表的条目数作为去重后的程序总数
        self.programs.len()
    }

    /// 若缓存中无任何程序则返回 true。
    pub fn is_empty(&self) -> bool {
        // 与 len 保持同步，便于 clippy 的长度/判空成对约定
        self.programs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_source() {
        // 新建顶点源码默认非内置，附加一段后应含两段且合并文本保留主体
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
        // 同一源码两次请求应命中同一哈希，返回相同 ID
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

        // 顶点源码改写后哈希不同，应新建一个程序并令缓存增长到 2
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
