//! 用于 glTF 模型与 3D Tiles 的 custom shader 系统。
//!
//! 涵盖 custom shader 模式、半透明模式、uniform/varying 类型、
//! uniform 声明与从 shader 文本解析使用变量集合的能力。
//!
//! CustomShader 系统允许用户将自定义 GLSL 代码注入模型
//! 渲染流水线，修改顶点位置与片元 material 属性。

use std::collections::HashMap;

/// custom shader 模式，决定片元 shader 代码如何应用。
///
/// 片元侧行为分两种：修改既有 material，或整体替换 material。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomShaderMode {
    /// 在 material 流水线阶段之后修改 material。
    /// 自定义片元 shader 可以访问已计算出的 material
    /// 并对其进行修改。
    #[default]
    ModifyMaterial,
    /// 用自定义 shader 的输出完全替换 material。
    ReplaceMaterial,
}

/// custom shader 的半透明模式。
///
/// 决定模型最终是否进入半透明渲染队列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomShaderTranslucencyMode {
    /// 从模型的 material 设置继承半透明度。
    #[default]
    Inherit,
    /// 强制不透明渲染。
    Opaque,
    /// 强制半透明渲染。
    Translucent,
}

/// custom shader 的 GLSL uniform 类型。
///
/// 覆盖 GLSL 常见的标量/向量/矩阵/采样器类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniformType {
    /// `float`
    Float,
    /// `vec2`
    Vec2,
    /// `vec3`
    Vec3,
    /// `vec4`
    Vec4,
    /// `int`
    Int,
    /// `ivec2`
    IntVec2,
    /// `ivec3`
    IntVec3,
    /// `ivec4`
    IntVec4,
    /// `bool`
    Bool,
    /// `bvec2`
    BoolVec2,
    /// `bvec3`
    BoolVec3,
    /// `bvec4`
    BoolVec4,
    /// `mat2`
    Mat2,
    /// `mat3`
    Mat3,
    /// `mat4`
    Mat4,
    /// `sampler2D`
    Sampler2D,
}

impl UniformType {
    /// 返回 GLSL 类型字符串。
    pub fn glsl_type(&self) -> &'static str {
        // 逐变体映射到对应的 GLSL 内建类型名
        match self {
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Int => "int",
            Self::IntVec2 => "ivec2",
            Self::IntVec3 => "ivec3",
            Self::IntVec4 => "ivec4",
            Self::Bool => "bool",
            Self::BoolVec2 => "bvec2",
            Self::BoolVec3 => "bvec3",
            Self::BoolVec4 => "bvec4",
            Self::Mat2 => "mat2",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::Sampler2D => "sampler2D",
        }
    }

    /// 返回该类型的分量数量。
    pub fn component_count(&self) -> usize {
        // 矩阵按列展开计数：mat2=4、mat3=9、mat4=16
        match self {
            Self::Float | Self::Int | Self::Bool => 1,
            Self::Vec2 | Self::IntVec2 | Self::BoolVec2 => 2,
            Self::Vec3 | Self::IntVec3 | Self::BoolVec3 => 3,
            Self::Vec4 | Self::IntVec4 | Self::BoolVec4 => 4,
            Self::Mat2 => 4,
            Self::Mat3 => 9,
            Self::Mat4 => 16,
            Self::Sampler2D => 1,
        }
    }

    /// 若这是一个 sampler 类型则返回 true。
    pub fn is_sampler(&self) -> bool {
        // 仅 sampler2D 为采样器类型
        matches!(self, Self::Sampler2D)
    }
}

/// custom shader 的 GLSL varying 类型。
///
/// varying 仅在顶点与片元阶段间传递插值数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaryingType {
    /// `float`
    Float,
    /// `vec2`
    Vec2,
    /// `vec3`
    Vec3,
    /// `vec4`
    Vec4,
    /// `mat2`
    Mat2,
    /// `mat3`
    Mat3,
    /// `mat4`
    Mat4,
}

impl VaryingType {
    /// 返回 GLSL 类型字符串。
    pub fn glsl_type(&self) -> &'static str {
        // varying 不支持采样器，仅标量/向量/矩阵
        match self {
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Mat2 => "mat2",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
        }
    }
}

/// 可在 custom shader 上设置的 uniform 值。
#[derive(Debug, Clone, PartialEq)]
pub enum UniformValue {
    /// 单个 float。
    Float(f64),
    /// 一个 vec2。
    Vec2([f64; 2]),
    /// 一个 vec3。
    Vec3([f64; 3]),
    /// 一个 vec4。
    Vec4([f64; 4]),
    /// 单个整数。
    Int(i32),
    /// 一个 ivec2。
    IntVec2([i32; 2]),
    /// 一个 ivec3。
    IntVec3([i32; 3]),
    /// 一个 ivec4。
    IntVec4([i32; 4]),
    /// 一个布尔值。
    Bool(bool),
    /// 一个 mat3（列主序）。
    Mat3([f64; 9]),
    /// 一个 mat4（列主序）。
    Mat4([f64; 16]),
    /// 一个 texture uniform（URL 或资源路径）。
    Texture(String),
}

/// 一个带有类型与初始值的 uniform 声明。
///
/// 将类型与初始值打包，供构造期一次性登记。
#[derive(Debug, Clone)]
pub struct UniformDeclaration {
    /// 该 uniform 的 GLSL 类型。
    pub uniform_type: UniformType,
    /// 初始值。
    pub value: UniformValue,
}

/// custom shader 代码中使用的变量（用于优化）。
///
/// 渲染后端据此只注入实际用到的属性/metadata。
#[derive(Debug, Clone, Default)]
pub struct UsedVariables {
    /// 使用的 attribute 变量（例如 positionMC、normalEC）。
    pub attribute_set: Vec<String>,
    /// 使用的 feature ID 变量。
    pub feature_id_set: Vec<String>,
    /// 使用的 metadata 变量。
    pub metadata_set: Vec<String>,
    /// 使用的 material 变量（仅片元 shader）。
    pub material_set: Vec<String>,
}

/// 用于模型与 3D Tiles 的用户自定义 GLSL shader。
///
/// 允许向顶点/片元阶段注入代码以改写位置或 material。
///
/// # 示例
/// ```ignore
/// let shader = CustomShader::new(
///     CustomShaderMode::ModifyMaterial,
///     Some("void vertexMain(VertexInput vsInput, inout czm_modelVertexOutput vsOutput) { vsOutput.positionMC += 0.1 * vsInput.attributes.normalMC; }".to_string()),
///     Some("void fragmentMain(FragmentInput fsInput, inout czm_modelMaterial material) { material.diffuse = vec3(1.0, 0.0, 0.0); }".to_string()),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct CustomShader {
    /// custom shader 与片元 shader 的交互方式。
    pub mode: CustomShaderMode,
    /// 半透明模式。
    pub translucency_mode: CustomShaderTranslucencyMode,
    /// 用户定义的 uniforms。
    pub uniforms: HashMap<String, UniformDeclaration>,
    /// 用户定义的 varyings。
    pub varyings: HashMap<String, VaryingType>,
    /// 自定义顶点 shader 的 GLSL 代码。
    pub vertex_shader_text: Option<String>,
    /// 自定义片元 shader 的 GLSL 代码。
    pub fragment_shader_text: Option<String>,
    /// 顶点 shader 中使用的变量（从代码解析）。
    pub used_variables_vertex: UsedVariables,
    /// 片元 shader 中使用的变量（从代码解析）。
    pub used_variables_fragment: UsedVariables,
}

impl Default for CustomShader {
    /// 默认 custom shader：ModifyMaterial 模式、无 uniform/varying、无 shader 文本。
    fn default() -> Self {
        Self {
            mode: CustomShaderMode::ModifyMaterial,
            translucency_mode: CustomShaderTranslucencyMode::Inherit,
            uniforms: HashMap::new(),
            varyings: HashMap::new(),
            vertex_shader_text: None,
            fragment_shader_text: None,
            used_variables_vertex: UsedVariables::default(),
            used_variables_fragment: UsedVariables::default(),
        }
    }
}

impl CustomShader {
    /// 创建一个具有给定模式与 shader 文本的新 custom shader。
    pub fn new(
        mode: CustomShaderMode,
        vertex_shader_text: Option<String>,
        fragment_shader_text: Option<String>,
    ) -> Self {
        // 先以传入字段覆盖默认值，其余保持 Default
        let mut shader = Self {
            mode,
            vertex_shader_text,
            fragment_shader_text,
            ..Default::default()
        };
        shader.find_used_variables();
        shader
    }

    /// 添加一个 uniform 声明。
    pub fn with_uniform(
        mut self,
        name: &str,
        uniform_type: UniformType,
        value: UniformValue,
    ) -> Self {
        // 记录 uniform 名 -> 类型+初始值 的声明，后续链式返回 self
        self.uniforms.insert(
            name.to_string(),
            UniformDeclaration {
                uniform_type,
                value,
            },
        );
        self
    }

    /// 添加一个 varying 声明。
    pub fn with_varying(mut self, name: &str, varying_type: VaryingType) -> Self {
        self.varyings.insert(name.to_string(), varying_type);
        self
    }

    /// 设置半透明模式。
    pub fn with_translucency_mode(
        mut self,
        mode: CustomShaderTranslucencyMode,
    ) -> Self {
        self.translucency_mode = mode;
        self
    }

    /// 更新一个 uniform 值。
    ///
    /// 仅更新已声明 uniform，未声明者返回错误。
    pub fn set_uniform(&mut self, name: &str, value: UniformValue) -> Result<(), ShaderError> {
        // 命中已声明 uniform 则覆盖其值，否则报未声明错误
        if let Some(decl) = self.uniforms.get_mut(name) {
            decl.value = value;
            Ok(())
        } else {
            Err(ShaderError::UniformNotDeclared(name.to_string()))
        }
    }

    /// 从 shader 文本解析使用的变量。
    ///
    /// 分别从顶点与片元文本各解析一次变量使用集合。
    fn find_used_variables(&mut self) {
        // 顶点与片元各自独立解析，避免跨阶段变量混淆
        if let Some(ref vs_text) = self.vertex_shader_text {
            self.used_variables_vertex = parse_variables(vs_text);
        }
        if let Some(ref fs_text) = self.fragment_shader_text {
            self.used_variables_fragment = parse_variables(fs_text);
        }
    }

    /// 校验内置变量的使用。
    ///
    /// 拦截跨阶段误用的内建坐标系变量。
    pub fn validate(&self) -> Result<(), ShaderError> {
        // 检查顶点 shader 中是否用了仅限片元的变量
        let vs_attrs = &self.used_variables_vertex.attribute_set;
        for name in vs_attrs {
            // 裸名（无坐标系后缀）在顶点阶段有歧义，建议补 MC 后缀
            if name == "position" || name == "normal" || name == "tangent" || name == "bitangent" {
                return Err(ShaderError::AmbiguousVariable {
                    name: name.clone(),
                    shader: "vertex".to_string(),
                    suggestion: format!("{}MC", name),
                });
            }
            // WC/EC 位置属世界/眼空间，顶点阶段应改用 MC
            if name == "positionWC" || name == "positionEC" {
                return Err(ShaderError::WrongShaderVariable {
                    name: name.clone(),
                    found_in: "vertex".to_string(),
                    suggestion: "positionMC".to_string(),
                });
            }
            // EC 法线/切线属片元空间，提示时转为对应 MC 名
            if name == "normalEC" || name == "tangentEC" || name == "bitangentEC" {
                let mc_name = name.replace("EC", "MC");
                return Err(ShaderError::WrongShaderVariable {
                    name: name.clone(),
                    found_in: "vertex".to_string(),
                    suggestion: mc_name,
                });
            }
        }

        // 检查片元 shader 中是否用了仅限顶点的变量
        let fs_attrs = &self.used_variables_fragment.attribute_set;
        for name in fs_attrs {
            // 裸名在片元阶段同样歧义，建议补 EC 后缀
            if name == "position" || name == "normal" || name == "tangent" || name == "bitangent" {
                return Err(ShaderError::AmbiguousVariable {
                    name: name.clone(),
                    shader: "fragment".to_string(),
                    suggestion: format!("{}EC", name),
                });
            }
            // MC 法线/切线属顶点空间，片元应改用 EC
            if name == "normalMC" || name == "tangentMC" || name == "bitangentMC" {
                let ec_name = name.replace("MC", "EC");
                return Err(ShaderError::WrongShaderVariable {
                    name: name.clone(),
                    found_in: "fragment".to_string(),
                    suggestion: ec_name,
                });
            }
        }

        Ok(())
    }

    /// 生成 GLSL uniform 声明。
    pub fn generate_uniform_declarations(&self) -> String {
        // 每个已声明 uniform 拼接一行 `uniform <类型> <名>;`
        let mut result = String::new();
        for (name, decl) in &self.uniforms {
            result.push_str(&format!(
                "uniform {} {};\n",
                decl.uniform_type.glsl_type(),
                name
            ));
        }
        result
    }

    /// 生成 GLSL varying 声明。
    pub fn generate_varying_declarations(&self) -> String {
        // 每个 varying 拼接一行 `varying <类型> <名>;`
        let mut result = String::new();
        for (name, vtype) in &self.varyings {
            result.push_str(&format!("varying {} {};\n", vtype.glsl_type(), name));
        }
        result
    }
}

/// custom shader 处理中可能出现的错误。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ShaderError {
    /// 构造函数中未声明的 uniform。
    #[error("Uniform '{0}' must be declared in the CustomShader constructor")]
    UniformNotDeclared(String),

    /// 歧义的变量名（缺少坐标系后缀）。
    #[error("'{name}' is ambiguous in the {shader} shader. Did you mean '{suggestion}'?")]
    AmbiguousVariable {
        /// 歧义的名称。
        name: String,
        /// 在哪个 shader 中找到。
        shader: String,
        /// 建议的正确名称。
        suggestion: String,
    },

    /// 变量用在了错误的 shader 阶段。
    #[error("'{name}' is not available in the {found_in} shader. Did you mean '{suggestion}'?")]
    WrongShaderVariable {
        /// 变量名。
        name: String,
        /// 在哪个 shader 中找到。
        found_in: String,
        /// 建议的正确名称。
        suggestion: String,
    },
}

/// 从 shader 文本解析使用的变量。
///
/// 从如下模式中提取变量名：
/// - `vsInput.attributes.positionMC` → attribute "positionMC"
/// - `fsInput.featureIds.featureId_0` → feature ID "featureId_0"
/// - `vsInput.metadata.height` → metadata "height"
/// - `material.diffuse` → material "diffuse"
fn parse_variables(shader_text: &str) -> UsedVariables {
    // 逐个模式扫描 shader 文本，分类收集 attribute/featureId/metadata/material 变量
    let mut vars = UsedVariables::default();

    // 解析 attribute 引用：[vf]sInput.attributes.(\w+)
    extract_matches(shader_text, ".attributes.", &mut vars.attribute_set);

    // 解析 feature ID 引用：[vf]sInput.featureIds.(\w+)
    extract_matches(shader_text, ".featureIds.", &mut vars.feature_id_set);

    // 解析 metadata 引用：[vf]sInput.metadata.(\w+) 或 .metadataClass. 或 .metadataStatistics.
    // metadataClass/metadataStatistics 与 metadata 归入同一集合
    extract_matches(shader_text, ".metadata.", &mut vars.metadata_set);
    extract_matches(shader_text, ".metadataClass.", &mut vars.metadata_set);
    extract_matches(shader_text, ".metadataStatistics.", &mut vars.metadata_set);

    // 解析 material 引用：material.(\w+)
    extract_matches(shader_text, "material.", &mut vars.material_set);

    // 去重：先排序再 dedup，保证集合稳定且无重复
    vars.attribute_set.sort();
    vars.attribute_set.dedup();
    vars.feature_id_set.sort();
    vars.feature_id_set.dedup();
    vars.metadata_set.sort();
    vars.metadata_set.dedup();
    vars.material_set.sort();
    vars.material_set.dedup();

    vars
}

/// 提取位于某个模式前缀之后的变量名。
fn extract_matches(text: &str, pattern: &str, output: &mut Vec<String>) {
    // 反复定位下一个模式出现位置，直至文本末尾
    let mut search_start = 0;
    while let Some(pos) = text[search_start..].find(pattern) {
        let abs_pos = search_start + pos + pattern.len();
        // 取出模式前缀之后的连续标识符字符（字母/数字/下划线）
        let remaining = &text[abs_pos..];
        let ident: String = remaining
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // 忽略模式后紧跟非标识符字符的空匹配
        if !ident.is_empty() {
            output.push(ident);
        }
        // 将游标推进到本次模式末尾，继续向后查找
        search_start = abs_pos;
    }
}

// 单元测试：覆盖类型映射、构造器链、变量解析与跨阶段校验。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_type_glsl() {
        // 校验各 uniform 类型到 GLSL 名称的映射
        assert_eq!(UniformType::Float.glsl_type(), "float");
        assert_eq!(UniformType::Vec3.glsl_type(), "vec3");
        assert_eq!(UniformType::Mat4.glsl_type(), "mat4");
        assert_eq!(UniformType::Sampler2D.glsl_type(), "sampler2D");
        assert_eq!(UniformType::Bool.glsl_type(), "bool");
        assert_eq!(UniformType::IntVec4.glsl_type(), "ivec4");
    }

    #[test]
    fn test_uniform_type_components() {
        // 校验各类型的分量数量（含矩阵列展开）
        assert_eq!(UniformType::Float.component_count(), 1);
        assert_eq!(UniformType::Vec2.component_count(), 2);
        assert_eq!(UniformType::Vec3.component_count(), 3);
        assert_eq!(UniformType::Vec4.component_count(), 4);
        assert_eq!(UniformType::Mat3.component_count(), 9);
        assert_eq!(UniformType::Mat4.component_count(), 16);
    }

    #[test]
    fn test_uniform_type_is_sampler() {
        // 仅 sampler2D 应被视为采样器
        // 其余标量/矩阵类型均不应命中 is_sampler
        assert!(UniformType::Sampler2D.is_sampler());
        assert!(!UniformType::Float.is_sampler());
        assert!(!UniformType::Mat4.is_sampler());
    }

    #[test]
    fn test_varying_type_glsl() {
        // 校验 varying 类型的 GLSL 名称
        assert_eq!(VaryingType::Float.glsl_type(), "float");
        assert_eq!(VaryingType::Vec2.glsl_type(), "vec2");
        assert_eq!(VaryingType::Mat4.glsl_type(), "mat4");
    }

    #[test]
    fn test_custom_shader_default() {
        // 默认实例应为 ModifyMaterial + Inherit 且无 uniform/varying
        let shader = CustomShader::default();
        assert_eq!(shader.mode, CustomShaderMode::ModifyMaterial);
        assert_eq!(shader.translucency_mode, CustomShaderTranslucencyMode::Inherit);
        assert!(shader.uniforms.is_empty());
        assert!(shader.varyings.is_empty());
    }

    #[test]
    fn test_custom_shader_builder() {
        // 链式构造器应正确登记 uniform 与 varying
        let shader = CustomShader::new(
            CustomShaderMode::ReplaceMaterial,
            Some("void vertexMain() {}".to_string()),
            Some("void fragmentMain() {}".to_string()),
        )
        .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0))
        .with_uniform(
            "u_color",
            UniformType::Vec3,
            UniformValue::Vec3([1.0, 0.0, 0.0]),
        )
        .with_varying("v_selectedColor", VaryingType::Vec3)
        .with_translucency_mode(CustomShaderTranslucencyMode::Opaque);

        assert_eq!(shader.mode, CustomShaderMode::ReplaceMaterial);
        assert_eq!(shader.translucency_mode, CustomShaderTranslucencyMode::Opaque);
        assert_eq!(shader.uniforms.len(), 2);
        assert_eq!(shader.varyings.len(), 1);
    }

    #[test]
    fn test_set_uniform() {
        // 已声明 uniform 可更新，未声明者报错
        let mut shader = CustomShader::default()
            .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0));

        assert!(shader.set_uniform("u_time", UniformValue::Float(1.5)).is_ok());
        assert_eq!(
            shader.uniforms["u_time"].value,
            UniformValue::Float(1.5)
        );

        assert!(shader.set_uniform("u_unknown", UniformValue::Float(0.0)).is_err());
    }

    #[test]
    fn test_parse_attribute_variables() {
        // 从 vs/fs 文本提取 attributes 与 material 变量集合
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            Some(
                "void vertexMain(VertexInput vsInput, inout czm_modelVertexOutput vsOutput) { \
                    vsOutput.positionMC += vsInput.attributes.normalMC * 0.1; \
                    vsOutput.positionMC += vsInput.attributes.positionMC; \
                }".to_string(),
            ),
            Some(
                "void fragmentMain(FragmentInput fsInput, inout czm_modelMaterial material) { \
                    material.diffuse = fsInput.attributes.color_0.rgb; \
                }".to_string(),
            ),
        );

        assert!(shader.used_variables_vertex.attribute_set.contains(&"normalMC".to_string()));
        assert!(shader.used_variables_vertex.attribute_set.contains(&"positionMC".to_string()));
        assert!(shader.used_variables_fragment.attribute_set.contains(&"color_0".to_string()));
        assert!(shader.used_variables_fragment.material_set.contains(&"diffuse".to_string()));
    }

    #[test]
    fn test_parse_feature_id_variables() {
        // featureIds 前缀应归入 feature_id_set
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            None,
            Some(
                "void fragmentMain(FragmentInput fsInput, inout czm_modelMaterial material) { \
                    float id = fsInput.featureIds.featureId_0; \
                }".to_string(),
            ),
        );

        assert!(shader.used_variables_fragment.feature_id_set.contains(&"featureId_0".to_string()));
    }

    #[test]
    fn test_parse_metadata_variables() {
        // metadata 前缀应归入 metadata_set
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            None,
            Some(
                "void fragmentMain(FragmentInput fsInput, inout czm_modelMaterial material) { \
                    float h = fsInput.metadata.height; \
                }".to_string(),
            ),
        );

        assert!(shader.used_variables_fragment.metadata_set.contains(&"height".to_string()));
    }

    #[test]
    fn test_validate_ambiguous_vertex() {
        // 顶点裸名 position 应报歧义并建议 positionMC
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            Some(
                "void vertexMain() { vec3 p = vsInput.attributes.position; }".to_string(),
            ),
            None,
        );

        let result = shader.validate();
        assert!(result.is_err());
        if let Err(ShaderError::AmbiguousVariable { name, suggestion, .. }) = result {
            assert_eq!(name, "position");
            assert_eq!(suggestion, "positionMC");
        }
    }

    #[test]
    fn test_validate_wrong_shader_stage() {
        // 顶点使用 positionWC 属跨阶段误用
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            Some(
                "void vertexMain() { vec3 p = vsInput.attributes.positionWC; }".to_string(),
            ),
            None,
        );

        let result = shader.validate();
        assert!(result.is_err());
        if let Err(ShaderError::WrongShaderVariable { name, suggestion, .. }) = result {
            assert_eq!(name, "positionWC");
            assert_eq!(suggestion, "positionMC");
        }
    }

    #[test]
    fn test_validate_fragment_mc_variable() {
        // 片元使用 normalMC 应建议改为 normalEC
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            None,
            Some(
                "void fragmentMain() { vec3 n = fsInput.attributes.normalMC; }".to_string(),
            ),
        );

        let result = shader.validate();
        assert!(result.is_err());
        if let Err(ShaderError::WrongShaderVariable { name, suggestion, .. }) = result {
            assert_eq!(name, "normalMC");
            assert_eq!(suggestion, "normalEC");
        }
    }

    #[test]
    fn test_validate_valid_shader() {
        // 各阶段变量均合法时校验通过
        let shader = CustomShader::new(
            CustomShaderMode::ModifyMaterial,
            Some(
                "void vertexMain() { vec3 p = vsInput.attributes.positionMC; }".to_string(),
            ),
            Some(
                "void fragmentMain() { vec3 n = fsInput.attributes.normalEC; }".to_string(),
            ),
        );

        assert!(shader.validate().is_ok());
    }

    #[test]
    fn test_generate_uniform_declarations() {
        // 生成的声明文本应包含每个 uniform 行
        let shader = CustomShader::default()
            .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0))
            .with_uniform("u_color", UniformType::Vec4, UniformValue::Vec4([1.0; 4]));

        let decl = shader.generate_uniform_declarations();
        assert!(decl.contains("uniform float u_time;"));
        assert!(decl.contains("uniform vec4 u_color;"));
    }

    #[test]
    fn test_generate_varying_declarations() {
        // 生成的声明文本应包含每个 varying 行
        let shader = CustomShader::default()
            .with_varying("v_color", VaryingType::Vec3)
            .with_varying("v_uv", VaryingType::Vec2);

        let decl = shader.generate_varying_declarations();
        assert!(decl.contains("varying vec3 v_color;"));
        assert!(decl.contains("varying vec2 v_uv;"));
    }

    #[test]
    fn test_custom_shader_mode_default() {
        // CustomShaderMode 默认应为 ModifyMaterial
        assert_eq!(CustomShaderMode::default(), CustomShaderMode::ModifyMaterial);
    }

    #[test]
    fn test_translucency_mode_default() {
        // 半透明模式默认应为 Inherit
        assert_eq!(
            CustomShaderTranslucencyMode::default(),
            CustomShaderTranslucencyMode::Inherit
        );
    }
}
