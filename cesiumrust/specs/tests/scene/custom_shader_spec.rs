//! Scene/Model/CustomShader → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/Model/CustomShader.js
//! - Scene/Model/CustomShaderMode.js
//! - Scene/Model/CustomShaderTranslucencyMode.js
//! - Scene/Model/UniformType.js
//! - Scene/Model/VaryingType.js
//!
//! A-class tests: UniformType/VaryingType glsl_type/component_count,
//! CustomShader creation/uniforms/varyings/validate/parse_variables/generate_declarations.

use cesium_gltf::custom_shader::{
    CustomShader, CustomShaderMode, CustomShaderTranslucencyMode, ShaderError,
    UniformType, UniformValue, VaryingType,
};

// === UniformType ===

#[test]
fn uniform_type_glsl_type() {
    assert_eq!(UniformType::Float.glsl_type(), "float");
    assert_eq!(UniformType::Vec2.glsl_type(), "vec2");
    assert_eq!(UniformType::Vec3.glsl_type(), "vec3");
    assert_eq!(UniformType::Vec4.glsl_type(), "vec4");
    assert_eq!(UniformType::Int.glsl_type(), "int");
    assert_eq!(UniformType::Mat3.glsl_type(), "mat3");
    assert_eq!(UniformType::Mat4.glsl_type(), "mat4");
    assert_eq!(UniformType::Sampler2D.glsl_type(), "sampler2D");
}

#[test]
fn uniform_type_component_count() {
    assert_eq!(UniformType::Float.component_count(), 1);
    assert_eq!(UniformType::Vec2.component_count(), 2);
    assert_eq!(UniformType::Vec3.component_count(), 3);
    assert_eq!(UniformType::Vec4.component_count(), 4);
    assert_eq!(UniformType::Mat2.component_count(), 4);
    assert_eq!(UniformType::Mat3.component_count(), 9);
    assert_eq!(UniformType::Mat4.component_count(), 16);
    assert_eq!(UniformType::Sampler2D.component_count(), 1);
}

#[test]
fn uniform_type_is_sampler() {
    assert!(UniformType::Sampler2D.is_sampler());
    assert!(!UniformType::Float.is_sampler());
    assert!(!UniformType::Mat4.is_sampler());
}

// === VaryingType ===

#[test]
fn varying_type_glsl_type() {
    assert_eq!(VaryingType::Float.glsl_type(), "float");
    assert_eq!(VaryingType::Vec2.glsl_type(), "vec2");
    assert_eq!(VaryingType::Vec3.glsl_type(), "vec3");
    assert_eq!(VaryingType::Vec4.glsl_type(), "vec4");
    assert_eq!(VaryingType::Mat3.glsl_type(), "mat3");
    assert_eq!(VaryingType::Mat4.glsl_type(), "mat4");
}

// === CustomShader creation ===

#[test]
fn custom_shader_default() {
    let shader = CustomShader::default();
    assert_eq!(shader.mode, CustomShaderMode::ModifyMaterial);
    assert_eq!(shader.translucency_mode, CustomShaderTranslucencyMode::Inherit);
    assert!(shader.uniforms.is_empty());
    assert!(shader.varyings.is_empty());
    assert!(shader.vertex_shader_text.is_none());
    assert!(shader.fragment_shader_text.is_none());
}

#[test]
fn custom_shader_new_with_text() {
    let shader = CustomShader::new(
        CustomShaderMode::ReplaceMaterial,
        Some("void vertexMain() {}".to_string()),
        Some("void fragmentMain() {}".to_string()),
    );
    assert_eq!(shader.mode, CustomShaderMode::ReplaceMaterial);
    assert!(shader.vertex_shader_text.is_some());
    assert!(shader.fragment_shader_text.is_some());
}

#[test]
fn custom_shader_with_uniform() {
    let shader = CustomShader::default()
        .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0))
        .with_uniform("u_color", UniformType::Vec3, UniformValue::Vec3([1.0, 0.0, 0.0]));
    assert_eq!(shader.uniforms.len(), 2);
    assert!(shader.uniforms.contains_key("u_time"));
    assert!(shader.uniforms.contains_key("u_color"));
}

#[test]
fn custom_shader_with_varying() {
    let shader = CustomShader::default()
        .with_varying("v_height", VaryingType::Float)
        .with_varying("v_normal", VaryingType::Vec3);
    assert_eq!(shader.varyings.len(), 2);
    assert_eq!(shader.varyings["v_height"], VaryingType::Float);
    assert_eq!(shader.varyings["v_normal"], VaryingType::Vec3);
}

#[test]
fn custom_shader_with_translucency_mode() {
    let shader = CustomShader::default()
        .with_translucency_mode(CustomShaderTranslucencyMode::Opaque);
    assert_eq!(shader.translucency_mode, CustomShaderTranslucencyMode::Opaque);
}

// === setUniform ===

#[test]
fn set_uniform_existing() {
    let mut shader = CustomShader::default()
        .with_uniform("u_scale", UniformType::Float, UniformValue::Float(1.0));
    let result = shader.set_uniform("u_scale", UniformValue::Float(2.5));
    assert!(result.is_ok());
    assert_eq!(shader.uniforms["u_scale"].value, UniformValue::Float(2.5));
}

#[test]
fn set_uniform_not_declared() {
    let mut shader = CustomShader::default();
    let result = shader.set_uniform("u_undeclared", UniformValue::Float(1.0));
    assert!(matches!(result, Err(ShaderError::UniformNotDeclared(_))));
}

// === parse_variables (findUsedVariables) ===

#[test]
fn parse_attributes_from_vertex_shader() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        Some("vsInput.attributes.positionMC + vsInput.attributes.normalMC".to_string()),
        None,
    );
    assert!(shader.used_variables_vertex.attribute_set.contains(&"positionMC".to_string()));
    assert!(shader.used_variables_vertex.attribute_set.contains(&"normalMC".to_string()));
}

#[test]
fn parse_feature_ids() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        None,
        Some("fsInput.featureIds.featureId_0".to_string()),
    );
    assert!(shader.used_variables_fragment.feature_id_set.contains(&"featureId_0".to_string()));
}

#[test]
fn parse_metadata() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        None,
        Some("fsInput.metadata.height + fsInput.metadataClass.name".to_string()),
    );
    assert!(shader.used_variables_fragment.metadata_set.contains(&"height".to_string()));
    assert!(shader.used_variables_fragment.metadata_set.contains(&"name".to_string()));
}

#[test]
fn parse_material_variables() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        None,
        Some("material.diffuse = vec3(1.0); material.alpha = 0.5;".to_string()),
    );
    assert!(shader.used_variables_fragment.material_set.contains(&"diffuse".to_string()));
    assert!(shader.used_variables_fragment.material_set.contains(&"alpha".to_string()));
}

#[test]
fn parse_variables_deduplicates() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        Some("vsInput.attributes.positionMC + vsInput.attributes.positionMC".to_string()),
        None,
    );
    let count = shader.used_variables_vertex.attribute_set.iter()
        .filter(|s| s.as_str() == "positionMC")
        .count();
    assert_eq!(count, 1);
}

// === validate ===

#[test]
fn validate_ambiguous_position_in_vertex() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        Some("vsInput.attributes.position".to_string()),
        None,
    );
    let result = shader.validate();
    assert!(matches!(result, Err(ShaderError::AmbiguousVariable { .. })));
}

#[test]
fn validate_wrong_shader_positionWC_in_vertex() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        Some("vsInput.attributes.positionWC".to_string()),
        None,
    );
    let result = shader.validate();
    assert!(matches!(result, Err(ShaderError::WrongShaderVariable { .. })));
}

#[test]
fn validate_normalMC_in_fragment() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        None,
        Some("fsInput.attributes.normalMC".to_string()),
    );
    let result = shader.validate();
    assert!(matches!(result, Err(ShaderError::WrongShaderVariable { .. })));
}

#[test]
fn validate_correct_variables_pass() {
    let shader = CustomShader::new(
        CustomShaderMode::ModifyMaterial,
        Some("vsInput.attributes.positionMC".to_string()),
        Some("fsInput.attributes.normalEC".to_string()),
    );
    assert!(shader.validate().is_ok());
}

// === generate declarations ===

#[test]
fn generate_uniform_declarations() {
    let shader = CustomShader::default()
        .with_uniform("u_time", UniformType::Float, UniformValue::Float(0.0));
    let decl = shader.generate_uniform_declarations();
    assert!(decl.contains("uniform float u_time;"));
}

#[test]
fn generate_varying_declarations() {
    let shader = CustomShader::default()
        .with_varying("v_height", VaryingType::Float);
    let decl = shader.generate_varying_declarations();
    assert!(decl.contains("varying float v_height;"));
}
