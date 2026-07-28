//! ShaderBuilder / ShaderSource / ShaderProgram / ShaderCache specs
//! Ported from CesiumJS Renderer/ShaderBuilder.js + ShaderSource.js + ShaderCache.js
//!
//! A-class tests: ShaderSource construction/append/combined, ShaderBuilder
//! uniforms/structs/functions/defines/build, ShaderProgram lifecycle, ShaderCache dedup

use cesium_scene::{
    ShaderBuilder, ShaderCache, ShaderFunction, ShaderProgram, ShaderSource, ShaderStage,
    ShaderStruct, ShaderUniform,
};

// ─── ShaderSource ──────────────────────────────────────────────────────────────

#[test]
fn shader_source_new() {
    let src = ShaderSource::new("void main() {}", ShaderStage::Vertex);
    assert_eq!(src.stage, ShaderStage::Vertex);
    assert!(!src.is_builtin);
    assert_eq!(src.sources.len(), 1);
    assert_eq!(src.combined_source(), "void main() {}");
}

#[test]
fn shader_source_builtin() {
    let src = ShaderSource::builtin("// builtin code", ShaderStage::Fragment);
    assert_eq!(src.stage, ShaderStage::Fragment);
    assert!(src.is_builtin);
}

#[test]
fn shader_source_append_and_combine() {
    let mut src = ShaderSource::new("line1", ShaderStage::Vertex);
    src.append("line2");
    src.append("line3");
    assert_eq!(src.sources.len(), 3);
    assert_eq!(src.combined_source(), "line1\nline2\nline3");
}

// ─── ShaderBuilder ─────────────────────────────────────────────────────────────

#[test]
fn shader_builder_add_uniform() {
    let mut builder = ShaderBuilder::new();
    builder.add_uniform("u_color", "vec4");
    builder.add_uniform("u_opacity", "float");

    assert_eq!(builder.uniforms.len(), 2);
    let vs = builder.build_vertex_source();
    assert!(vs.contains("uniform vec4 u_color;"));
    assert!(vs.contains("uniform float u_opacity;"));
}

#[test]
fn shader_builder_add_uniform_array() {
    let mut builder = ShaderBuilder::new();
    builder.add_uniform_array("u_lights", "vec3", 8);

    let vs = builder.build_vertex_source();
    assert!(vs.contains("uniform vec3 u_lights[8];"));
}

#[test]
fn shader_builder_add_struct() {
    let mut builder = ShaderBuilder::new();
    builder.add_struct(
        "Material",
        vec![
            ShaderUniform {
                name: "diffuse".to_string(),
                glsl_type: "vec3".to_string(),
                count: 1,
            },
            ShaderUniform {
                name: "alpha".to_string(),
                glsl_type: "float".to_string(),
                count: 1,
            },
        ],
    );

    let vs = builder.build_vertex_source();
    assert!(vs.contains("struct Material {"));
    assert!(vs.contains("    vec3 diffuse;"));
    assert!(vs.contains("    float alpha;"));
    assert!(vs.contains("};"));
}

#[test]
fn shader_builder_add_function() {
    let mut builder = ShaderBuilder::new();
    builder.add_function(ShaderFunction {
        name: "getAlpha".to_string(),
        return_type: "float".to_string(),
        parameters: vec![ShaderUniform {
            name: "x".to_string(),
            glsl_type: "float".to_string(),
            count: 1,
        }],
        body: "    return x * 0.5;".to_string(),
    });

    let vs = builder.build_vertex_source();
    assert!(vs.contains("float getAlpha(float x) {"));
    assert!(vs.contains("return x * 0.5;"));
}

#[test]
fn shader_builder_defines() {
    let mut builder = ShaderBuilder::new();
    builder.add_define("HAS_TEXTURE", "1");
    builder.add_define("MAX_LIGHTS", "4");

    let vs = builder.build_vertex_source();
    assert!(vs.contains("#define HAS_TEXTURE 1"));
    assert!(vs.contains("#define MAX_LIGHTS 4"));

    let fs = builder.build_fragment_source();
    assert!(fs.contains("#define HAS_TEXTURE 1"));
}

#[test]
fn shader_builder_append_vertex_fragment() {
    let mut builder = ShaderBuilder::new();
    builder.append_vertex("gl_Position = vec4(0.0);");
    builder.append_fragment("gl_FragColor = vec4(1.0);");

    let vs = builder.build_vertex_source();
    assert!(vs.contains("gl_Position = vec4(0.0);"));

    let fs = builder.build_fragment_source();
    assert!(fs.contains("gl_FragColor = vec4(1.0);"));
}

#[test]
fn shader_builder_full_pipeline() {
    let mut builder = ShaderBuilder::new();
    builder
        .add_define("USE_LIGHTING", "1")
        .add_uniform("u_modelViewMatrix", "mat4")
        .add_struct(
            "VSInput",
            vec![ShaderUniform {
                name: "position".to_string(),
                glsl_type: "vec3".to_string(),
                count: 1,
            }],
        )
        .add_function(ShaderFunction {
            name: "transformPosition".to_string(),
            return_type: "vec4".to_string(),
            parameters: vec![ShaderUniform {
                name: "pos".to_string(),
                glsl_type: "vec3".to_string(),
                count: 1,
            }],
            body: "    return u_modelViewMatrix * vec4(pos, 1.0);".to_string(),
        })
        .append_vertex("void main() { gl_Position = transformPosition(position); }");

    let vs = builder.build_vertex_source();
    // Order: defines → structs → uniforms → functions → source
    let define_pos = vs.find("#define USE_LIGHTING 1").unwrap();
    let struct_pos = vs.find("struct VSInput {").unwrap();
    let uniform_pos = vs.find("uniform mat4 u_modelViewMatrix;").unwrap();
    let func_pos = vs.find("vec4 transformPosition(vec3 pos)").unwrap();
    let main_pos = vs.find("void main()").unwrap();
    assert!(define_pos < struct_pos);
    assert!(struct_pos < uniform_pos);
    assert!(uniform_pos < func_pos);
    assert!(func_pos < main_pos);
}

// ─── ShaderProgram ─────────────────────────────────────────────────────────────

#[test]
fn shader_program_lifecycle() {
    let mut prog = ShaderProgram::new(
        0,
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );

    assert!(!prog.ready);
    assert_eq!(prog.id, 0);
    assert!(prog.uniforms.is_empty());
    assert!(prog.attributes.is_empty());

    prog.add_uniform("u_color", "vec4");
    prog.add_attribute("a_position", "vec3");
    prog.mark_ready();

    assert!(prog.ready);
    assert_eq!(prog.uniforms.len(), 1);
    assert_eq!(prog.uniforms[0].name, "u_color");
    assert_eq!(prog.attributes.len(), 1);
    assert_eq!(prog.attributes[0].name, "a_position");
}

// ─── ShaderCache ───────────────────────────────────────────────────────────────

#[test]
fn shader_cache_dedup() {
    let mut cache = ShaderCache::new();
    assert!(cache.is_empty());

    let id1 = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );
    let id2 = cache.get_or_create(
        ShaderSource::new("void main() {}", ShaderStage::Vertex),
        ShaderSource::new("void main() {}", ShaderStage::Fragment),
    );

    // Same source → same ID (deduplication)
    assert_eq!(id1, id2);
    assert_eq!(cache.len(), 1);
}

#[test]
fn shader_cache_different_sources() {
    let mut cache = ShaderCache::new();

    let id1 = cache.get_or_create(
        ShaderSource::new("void main() { gl_Position = vec4(0.0); }", ShaderStage::Vertex),
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
fn shader_cache_get_by_id() {
    let mut cache = ShaderCache::new();
    let id = cache.get_or_create(
        ShaderSource::new("vertex", ShaderStage::Vertex),
        ShaderSource::new("fragment", ShaderStage::Fragment),
    );

    let prog = cache.get(id).unwrap();
    assert_eq!(prog.id, id);
    assert!(prog.ready);
    assert!(prog.vertex_shader.combined_source().contains("vertex"));

    assert!(cache.get(999).is_none());
}
