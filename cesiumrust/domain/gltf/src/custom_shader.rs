//! Custom shader system for glTF models and 3D Tiles.
//!
//! Maps to CesiumJS:
//! - `Scene/Model/CustomShader.js`
//! - `Scene/Model/CustomShaderMode.js`
//! - `Scene/Model/CustomShaderTranslucencyMode.js`
//! - `Scene/Model/UniformType.js`
//! - `Scene/Model/VaryingType.js`
//!
//! The CustomShader system allows users to inject custom GLSL code into the
//! model rendering pipeline, modifying vertex positions and fragment material
//! properties.

use std::collections::HashMap;

/// Custom shader mode determining how fragment shader code is applied.
///
/// Maps to CesiumJS `CustomShaderMode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomShaderMode {
    /// Modify the material after the material pipeline stage.
    /// The custom fragment shader has access to the computed material
    /// and can modify it.
    #[default]
    ModifyMaterial,
    /// Replace the material entirely with the custom shader output.
    ReplaceMaterial,
}

/// Translucency mode for custom shaders.
///
/// Maps to CesiumJS `CustomShaderTranslucencyMode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomShaderTranslucencyMode {
    /// Inherit translucency from the model's material settings.
    #[default]
    Inherit,
    /// Force opaque rendering.
    Opaque,
    /// Force translucent rendering.
    Translucent,
}

/// GLSL uniform types for custom shaders.
///
/// Maps to CesiumJS `UniformType`
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
    /// Returns the GLSL type string.
    pub fn glsl_type(&self) -> &'static str {
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

    /// Returns the number of components for this type.
    pub fn component_count(&self) -> usize {
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

    /// Returns true if this is a sampler type.
    pub fn is_sampler(&self) -> bool {
        matches!(self, Self::Sampler2D)
    }
}

/// GLSL varying types for custom shaders.
///
/// Maps to CesiumJS `VaryingType`
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
    /// Returns the GLSL type string.
    pub fn glsl_type(&self) -> &'static str {
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

/// A uniform value that can be set on a custom shader.
#[derive(Debug, Clone, PartialEq)]
pub enum UniformValue {
    /// A single float.
    Float(f64),
    /// A vec2.
    Vec2([f64; 2]),
    /// A vec3.
    Vec3([f64; 3]),
    /// A vec4.
    Vec4([f64; 4]),
    /// A single integer.
    Int(i32),
    /// An ivec2.
    IntVec2([i32; 2]),
    /// An ivec3.
    IntVec3([i32; 3]),
    /// An ivec4.
    IntVec4([i32; 4]),
    /// A boolean.
    Bool(bool),
    /// A mat3 (column-major).
    Mat3([f64; 9]),
    /// A mat4 (column-major).
    Mat4([f64; 16]),
    /// A texture uniform (URL or resource path).
    Texture(String),
}

/// A uniform declaration with type and initial value.
///
/// Maps to CesiumJS `UniformSpecifier`
#[derive(Debug, Clone)]
pub struct UniformDeclaration {
    /// The GLSL type of the uniform.
    pub uniform_type: UniformType,
    /// The initial value.
    pub value: UniformValue,
}

/// Variables used in custom shader code (for optimization).
///
/// Maps to CesiumJS `VariableSet`
#[derive(Debug, Clone, Default)]
pub struct UsedVariables {
    /// Attribute variables used (e.g., positionMC, normalEC).
    pub attribute_set: Vec<String>,
    /// Feature ID variables used.
    pub feature_id_set: Vec<String>,
    /// Metadata variables used.
    pub metadata_set: Vec<String>,
    /// Material variables used (fragment shader only).
    pub material_set: Vec<String>,
}

/// A user-defined GLSL shader for models and 3D Tiles.
///
/// Maps to CesiumJS `Scene/Model/CustomShader.js`
///
/// # Example
/// ```ignore
/// let shader = CustomShader::new(
///     CustomShaderMode::ModifyMaterial,
///     Some("void vertexMain(VertexInput vsInput, inout czm_modelVertexOutput vsOutput) { vsOutput.positionMC += 0.1 * vsInput.attributes.normalMC; }".to_string()),
///     Some("void fragmentMain(FragmentInput fsInput, inout czm_modelMaterial material) { material.diffuse = vec3(1.0, 0.0, 0.0); }".to_string()),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct CustomShader {
    /// How the custom shader interacts with the fragment shader.
    pub mode: CustomShaderMode,
    /// Translucency mode.
    pub translucency_mode: CustomShaderTranslucencyMode,
    /// User-defined uniforms.
    pub uniforms: HashMap<String, UniformDeclaration>,
    /// User-defined varyings.
    pub varyings: HashMap<String, VaryingType>,
    /// Custom vertex shader GLSL code.
    pub vertex_shader_text: Option<String>,
    /// Custom fragment shader GLSL code.
    pub fragment_shader_text: Option<String>,
    /// Variables used in the vertex shader (parsed from code).
    pub used_variables_vertex: UsedVariables,
    /// Variables used in the fragment shader (parsed from code).
    pub used_variables_fragment: UsedVariables,
}

impl Default for CustomShader {
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
    /// Creates a new custom shader with the given mode and shader text.
    pub fn new(
        mode: CustomShaderMode,
        vertex_shader_text: Option<String>,
        fragment_shader_text: Option<String>,
    ) -> Self {
        let mut shader = Self {
            mode,
            vertex_shader_text,
            fragment_shader_text,
            ..Default::default()
        };
        shader.find_used_variables();
        shader
    }

    /// Adds a uniform declaration.
    pub fn with_uniform(
        mut self,
        name: &str,
        uniform_type: UniformType,
        value: UniformValue,
    ) -> Self {
        self.uniforms.insert(
            name.to_string(),
            UniformDeclaration {
                uniform_type,
                value,
            },
        );
        self
    }

    /// Adds a varying declaration.
    pub fn with_varying(mut self, name: &str, varying_type: VaryingType) -> Self {
        self.varyings.insert(name.to_string(), varying_type);
        self
    }

    /// Sets the translucency mode.
    pub fn with_translucency_mode(
        mut self,
        mode: CustomShaderTranslucencyMode,
    ) -> Self {
        self.translucency_mode = mode;
        self
    }

    /// Updates a uniform value.
    ///
    /// Maps to CesiumJS `CustomShader.prototype.setUniform`
    pub fn set_uniform(&mut self, name: &str, value: UniformValue) -> Result<(), ShaderError> {
        if let Some(decl) = self.uniforms.get_mut(name) {
            decl.value = value;
            Ok(())
        } else {
            Err(ShaderError::UniformNotDeclared(name.to_string()))
        }
    }

    /// Parses used variables from shader text.
    ///
    /// Maps to CesiumJS `findUsedVariables`
    fn find_used_variables(&mut self) {
        if let Some(ref vs_text) = self.vertex_shader_text {
            self.used_variables_vertex = parse_variables(vs_text);
        }
        if let Some(ref fs_text) = self.fragment_shader_text {
            self.used_variables_fragment = parse_variables(fs_text);
        }
    }

    /// Validates built-in variable usage.
    ///
    /// Maps to CesiumJS `validateBuiltinVariables`
    pub fn validate(&self) -> Result<(), ShaderError> {
        // Check vertex shader for fragment-only variables
        let vs_attrs = &self.used_variables_vertex.attribute_set;
        for name in vs_attrs {
            if name == "position" || name == "normal" || name == "tangent" || name == "bitangent" {
                return Err(ShaderError::AmbiguousVariable {
                    name: name.clone(),
                    shader: "vertex".to_string(),
                    suggestion: format!("{}MC", name),
                });
            }
            if name == "positionWC" || name == "positionEC" {
                return Err(ShaderError::WrongShaderVariable {
                    name: name.clone(),
                    found_in: "vertex".to_string(),
                    suggestion: "positionMC".to_string(),
                });
            }
            if name == "normalEC" || name == "tangentEC" || name == "bitangentEC" {
                let mc_name = name.replace("EC", "MC");
                return Err(ShaderError::WrongShaderVariable {
                    name: name.clone(),
                    found_in: "vertex".to_string(),
                    suggestion: mc_name,
                });
            }
        }

        // Check fragment shader for vertex-only variables
        let fs_attrs = &self.used_variables_fragment.attribute_set;
        for name in fs_attrs {
            if name == "position" || name == "normal" || name == "tangent" || name == "bitangent" {
                return Err(ShaderError::AmbiguousVariable {
                    name: name.clone(),
                    shader: "fragment".to_string(),
                    suggestion: format!("{}EC", name),
                });
            }
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

    /// Generates GLSL uniform declarations.
    pub fn generate_uniform_declarations(&self) -> String {
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

    /// Generates GLSL varying declarations.
    pub fn generate_varying_declarations(&self) -> String {
        let mut result = String::new();
        for (name, vtype) in &self.varyings {
            result.push_str(&format!("varying {} {};\n", vtype.glsl_type(), name));
        }
        result
    }
}

/// Errors that can occur in custom shader processing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ShaderError {
    /// Uniform not declared in the constructor.
    #[error("Uniform '{0}' must be declared in the CustomShader constructor")]
    UniformNotDeclared(String),

    /// Ambiguous variable name (missing coordinate suffix).
    #[error("'{name}' is ambiguous in the {shader} shader. Did you mean '{suggestion}'?")]
    AmbiguousVariable {
        /// The ambiguous name.
        name: String,
        /// Which shader it was found in.
        shader: String,
        /// The suggested correct name.
        suggestion: String,
    },

    /// Variable used in wrong shader stage.
    #[error("'{name}' is not available in the {found_in} shader. Did you mean '{suggestion}'?")]
    WrongShaderVariable {
        /// The variable name.
        name: String,
        /// Which shader it was found in.
        found_in: String,
        /// The suggested correct name.
        suggestion: String,
    },
}

/// Parses used variables from shader text.
///
/// Extracts variable names from patterns like:
/// - `vsInput.attributes.positionMC` → attribute "positionMC"
/// - `fsInput.featureIds.featureId_0` → feature ID "featureId_0"
/// - `vsInput.metadata.height` → metadata "height"
/// - `material.diffuse` → material "diffuse"
fn parse_variables(shader_text: &str) -> UsedVariables {
    let mut vars = UsedVariables::default();

    // Parse attribute references: [vf]sInput.attributes.(\w+)
    extract_matches(shader_text, ".attributes.", &mut vars.attribute_set);

    // Parse feature ID references: [vf]sInput.featureIds.(\w+)
    extract_matches(shader_text, ".featureIds.", &mut vars.feature_id_set);

    // Parse metadata references: [vf]sInput.metadata.(\w+) or .metadataClass. or .metadataStatistics.
    extract_matches(shader_text, ".metadata.", &mut vars.metadata_set);
    extract_matches(shader_text, ".metadataClass.", &mut vars.metadata_set);
    extract_matches(shader_text, ".metadataStatistics.", &mut vars.metadata_set);

    // Parse material references: material.(\w+)
    extract_matches(shader_text, "material.", &mut vars.material_set);

    // De-duplicate
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

/// Extracts variable names following a pattern prefix.
fn extract_matches(text: &str, pattern: &str, output: &mut Vec<String>) {
    let mut search_start = 0;
    while let Some(pos) = text[search_start..].find(pattern) {
        let abs_pos = search_start + pos + pattern.len();
        // Extract the identifier after the pattern
        let remaining = &text[abs_pos..];
        let ident: String = remaining
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !ident.is_empty() {
            output.push(ident);
        }
        search_start = abs_pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_type_glsl() {
        assert_eq!(UniformType::Float.glsl_type(), "float");
        assert_eq!(UniformType::Vec3.glsl_type(), "vec3");
        assert_eq!(UniformType::Mat4.glsl_type(), "mat4");
        assert_eq!(UniformType::Sampler2D.glsl_type(), "sampler2D");
        assert_eq!(UniformType::Bool.glsl_type(), "bool");
        assert_eq!(UniformType::IntVec4.glsl_type(), "ivec4");
    }

    #[test]
    fn test_uniform_type_components() {
        assert_eq!(UniformType::Float.component_count(), 1);
        assert_eq!(UniformType::Vec2.component_count(), 2);
        assert_eq!(UniformType::Vec3.component_count(), 3);
        assert_eq!(UniformType::Vec4.component_count(), 4);
        assert_eq!(UniformType::Mat3.component_count(), 9);
        assert_eq!(UniformType::Mat4.component_count(), 16);
    }

    #[test]
    fn test_uniform_type_is_sampler() {
        assert!(UniformType::Sampler2D.is_sampler());
        assert!(!UniformType::Float.is_sampler());
        assert!(!UniformType::Mat4.is_sampler());
    }

    #[test]
    fn test_varying_type_glsl() {
        assert_eq!(VaryingType::Float.glsl_type(), "float");
        assert_eq!(VaryingType::Vec2.glsl_type(), "vec2");
        assert_eq!(VaryingType::Mat4.glsl_type(), "mat4");
    }

    #[test]
    fn test_custom_shader_default() {
        let shader = CustomShader::default();
        assert_eq!(shader.mode, CustomShaderMode::ModifyMaterial);
        assert_eq!(shader.translucency_mode, CustomShaderTranslucencyMode::Inherit);
        assert!(shader.uniforms.is_empty());
        assert!(shader.varyings.is_empty());
    }

    #[test]
    fn test_custom_shader_builder() {
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
        let shader = CustomShader::default()
            .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0))
            .with_uniform("u_color", UniformType::Vec4, UniformValue::Vec4([1.0; 4]));

        let decl = shader.generate_uniform_declarations();
        assert!(decl.contains("uniform float u_time;"));
        assert!(decl.contains("uniform vec4 u_color;"));
    }

    #[test]
    fn test_generate_varying_declarations() {
        let shader = CustomShader::default()
            .with_varying("v_color", VaryingType::Vec3)
            .with_varying("v_uv", VaryingType::Vec2);

        let decl = shader.generate_varying_declarations();
        assert!(decl.contains("varying vec3 v_color;"));
        assert!(decl.contains("varying vec2 v_uv;"));
    }

    #[test]
    fn test_custom_shader_mode_default() {
        assert_eq!(CustomShaderMode::default(), CustomShaderMode::ModifyMaterial);
    }

    #[test]
    fn test_translucency_mode_default() {
        assert_eq!(
            CustomShaderTranslucencyMode::default(),
            CustomShaderTranslucencyMode::Inherit
        );
    }
}
