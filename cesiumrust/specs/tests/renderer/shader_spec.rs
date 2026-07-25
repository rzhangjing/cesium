//! Renderer/ShaderProgramSpec.js, ShaderSourceSpec.js, ShaderBuilderSpec.js,
//! ShaderCacheSpec.js, ShaderFunctionSpec.js, ShaderStructSpec.js
//! → Rust integration tests

use cesium_scene::{
    ShaderStage, ShaderSource, ShaderUniform, ShaderStruct, ShaderFunction,
    ShaderBuilder, ShaderProgram, ShaderCache,
};

// === ShaderSource ===

#[test]
fn test_shader_source_new() {
    let src = ShaderSource::new("void main() {}", ShaderStage::Vertex);
    assert_eq!(src.stage, ShaderStage::Vertex);
    assert!(!src.is_builtin);
    assert_eq!(src.sources.len(), 1);
}

#[test]
fn test_shader_source_builtin() {
    let src = ShaderSource::builtin("void main() {}", ShaderStage::Fragment);
    assert!(src.is_builtin);
    assert_eq!(src.stage, ShaderStage::Fragment);
}

#[test]
fn test_shader_source_append() {
    let mut src = ShaderSource::new("line1", ShaderStage::Vertex);
    src.append("line2");
    assert_eq!(src.sources.len(), 2);
    let combined = src.combined_source();
    assert!(combined.contains("line1"));
    assert!(combined.contains("line2"));
}

#[test]
fn test_shader_source_combined() {
    let mut src = ShaderSource::new("a", ShaderStage::Vertex);
    src.append("b");
    src.append("c");
    assert_eq!(src.combined_source(), "a\nb\nc");
}

// === ShaderBuilder ===

#[test]
fn test_shader_builder_new() {
    let builder = ShaderBuilder::new();
    assert!(builder.uniforms.is_empty());
    assert!(builder.structs.is_empty());
    assert!(builder.functions.is_empty());
}

#[test]
fn test_shader_builder_add_uniform() {
    let mut builder = ShaderBuilder::new();
    builder.add_uniform("u_color", "vec4");
    assert_eq!(builder.uniforms.len(), 1);
    assert_eq!(builder.uniforms[0].name, "u_color");
    assert_eq!(builder.uniforms[0].glsl_type, "vec4");
}

#[test]
fn test_shader_builder_add_uniform_array() {
    let mut builder = ShaderBuilder::new();
    builder.add_uniform_array("u_lights", "vec3", 8);
    assert_eq!(builder.uniforms[0].count, 8);
    let vs = builder.build_vertex_source();
    assert!(vs.contains("uniform vec3 u_lights[8];"));
}

#[test]
fn test_shader_builder_add_define() {
    let mut builder = ShaderBuilder::new();
    builder.add_define("HAS_TEXTURE", "1");
    let vs = builder.build_vertex_source();
    assert!(vs.contains("#define HAS_TEXTURE 1"));
}

#[test]
fn test_shader_builder_add_struct() {
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
fn test_shader_builder_add_function() {
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
fn test_shader_builder_vertex_fragment() {
    let mut builder = ShaderBuilder::new();
    builder
        .add_uniform("u_color", "vec4")
        .append_vertex("gl_Position = vec4(0.0);")
        .append_fragment("gl_FragColor = u_color;");

    let vs = builder.build_vertex_source();
    assert!(vs.contains("uniform vec4 u_color;"));
    assert!(vs.contains("gl_Position"));

    let fs = builder.build_fragment_source();
    assert!(fs.contains("uniform vec4 u_color;"));
    assert!(fs.contains("gl_FragColor"));
}

// === ShaderProgram ===

#[test]
fn test_shader_program_new() {
    let prog = ShaderProgram::new(
        0,
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    assert_eq!(prog.id, 0);
    assert!(!prog.ready);
    assert!(prog.uniforms.is_empty());
    assert!(prog.attributes.is_empty());
}

#[test]
fn test_shader_program_add_uniform_attribute() {
    let mut prog = ShaderProgram::new(
        1,
        ShaderSource::new("", ShaderStage::Vertex),
        ShaderSource::new("", ShaderStage::Fragment),
    );
    prog.add_uniform("u_color", "vec4");
    prog.add_attribute("a_position", "vec3");
    assert_eq!(prog.uniforms.len(), 1);
    assert_eq!(prog.attributes.len(), 1);
}

#[test]
fn test_shader_program_mark_ready() {
    let mut prog = ShaderProgram::new(
        2,
        ShaderSource::new("", ShaderStage::Vertex),
        ShaderSource::new("", ShaderStage::Fragment),
    );
    assert!(!prog.ready);
    prog.mark_ready();
    assert!(prog.ready);
}

// === ShaderCache ===

#[test]
fn test_shader_cache_new() {
    let cache = ShaderCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_shader_cache_dedup() {
    let mut cache = ShaderCache::new();
    let id1 = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    let id2 = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    assert_eq!(id1, id2);
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_shader_cache_different_sources() {
    let mut cache = ShaderCache::new();
    let id1 = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    let id2 = cache.get_or_create(
        ShaderSource::new("void main() { gl_Position = vec4(1.0); }", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    assert_ne!(id1, id2);
    assert_eq!(cache.len(), 2);
}

#[test]
fn test_shader_cache_get() {
    let mut cache = ShaderCache::new();
    let id = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    assert!(cache.get(id).is_some());
    assert!(cache.get(999).is_none());
}

// === ShaderStage ===

#[test]
fn test_shader_stage_variants() {
    assert_ne!(ShaderStage::Vertex, ShaderStage::Fragment);
    assert_ne!(ShaderStage::Fragment, ShaderStage::Compute);
}
