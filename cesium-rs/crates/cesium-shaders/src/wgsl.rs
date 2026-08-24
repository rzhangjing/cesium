//! Hand-written WGSL shaders for the smoke render path.
//!
//! DEVIATION: CesiumJS ships only GLSL (`shaders/*.glsl`, embedded elsewhere
//! in this crate). Per `docs/shader-strategy.md` (hybrid route), the key
//! shaders on the critical render path are hand-translated to WGSL because
//! naga's GLSL frontend cannot parse CesiumJS's sampler/uniform conventions
//! (v2 translation success rate: 0/108). Shader-less passes may still use the
//! naga GLSL→WGSL pipeline.
//!
//! Each `.wgsl` file carries a header comment naming its GLSL source and the
//! exact trimming scope (what was kept / what was dropped).
//!
//! Shared binding contract:
//! - group(0) binding(0): `CesiumAutomaticUniforms` uniform buffer
//!   (mat4x4 f32 column-major: czm_modelViewProjection, czm_modelView,
//!   czm_projection, czm_view, czm_model; vec4 czm_viewport). Total 336 bytes.
//!   Only declared in shaders that actually consume automatic uniforms.
//! - group(1): per-draw material resources (uniform buffers / textures /
//!   samplers), assigned by the renderer from `DrawCommand` uniform overrides.

/// Vertex shader for the full-viewport quad (port of `ViewportQuadVS.glsl`).
pub const VIEWPORT_QUAD_VS: &str = include_str!("../wgsl/viewport_quad_vs.wgsl");

/// Fragment shader for the viewport quad: solid color material
/// (port of `ViewportQuadFS.glsl`, `Material.ColorType` baked in).
pub const VIEWPORT_QUAD_COLOR_FS: &str = include_str!("../wgsl/viewport_quad_color_fs.wgsl");

/// Fragment shader for the viewport quad: texture sampling variant
/// (port of `ViewportQuadFS.glsl` with an image material baked in).
pub const VIEWPORT_QUAD_TEXTURE_FS: &str = include_str!("../wgsl/viewport_quad_texture_fs.wgsl");

/// Vertex shader for globe tiles, TEXONLY trimmed variant
/// (port of `GlobeVS.glsl`; see file header for the trimming scope).
pub const GLOBE_VS: &str = include_str!("../wgsl/globe_vs.wgsl");

/// Fragment shader for globe tiles, TEXONLY trimmed variant
/// (port of `GlobeFS.glsl`; see file header for the trimming scope).
pub const GLOBE_FS: &str = include_str!("../wgsl/globe_fs.wgsl");

/// Vertex shader for geometry-instance primitives (trimmed port of
/// `PerInstanceColorAppearanceVS.glsl`; see file header for the scope).
pub const PRIMITIVE_VS: &str = include_str!("../wgsl/primitive_vs.wgsl");

/// Fragment shader for geometry-instance primitives (trimmed port of
/// `PerInstanceColorAppearanceFS.glsl`; see file header for the scope).
pub const PRIMITIVE_FS: &str = include_str!("../wgsl/primitive_fs.wgsl");

/// Vertex shader for the billboard batch (trimmed port of
/// `BillboardCollectionVS.glsl`; shared by the BillboardCollection /
/// PointPrimitiveCollection / LabelCollection batches; see file header for
/// the scope).
pub const BILLBOARD_VS: &str = include_str!("../wgsl/billboard_vs.wgsl");

/// Fragment shader for the billboard batch (trimmed port of
/// `BillboardCollectionFS.glsl`; see file header for the scope).
pub const BILLBOARD_FS: &str = include_str!("../wgsl/billboard_fs.wgsl");

/// Vertex shader for model runtime primitives without a base color texture
/// (trimmed port of `ModelVS.glsl`; position only; see file header).
pub const MODEL_COLOR_VS: &str = include_str!("../wgsl/model_color_vs.wgsl");

/// Fragment shader for model runtime primitives without a base color
/// texture (trimmed port of `ModelFS.glsl`; flat base color factor).
pub const MODEL_COLOR_FS: &str = include_str!("../wgsl/model_color_fs.wgsl");

/// Vertex shader for textured model runtime primitives (trimmed port of
/// `ModelVS.glsl`; position + TEXCOORD_0; see file header).
pub const MODEL_TEXTURED_VS: &str = include_str!("../wgsl/model_textured_vs.wgsl");

/// Fragment shader for textured model runtime primitives (trimmed port of
/// `ModelFS.glsl`; base color texture × factor; see file header).
pub const MODEL_TEXTURED_FS: &str = include_str!("../wgsl/model_textured_fs.wgsl");

/// Byte size of the `CesiumAutomaticUniforms` buffer declared at group(0)
/// binding(0): 5 × mat4x4&lt;f32&gt; (64 bytes each) + 1 × vec4&lt;f32&gt;.
pub const CESIUM_AUTOMATIC_UNIFORMS_SIZE: usize = 5 * 64 + 16;

#[cfg(test)]
mod tests {
    use super::*;

    /// All hand-written WGSL must parse with naga. This is the compile-time
    /// equivalent of CesiumJS's `log_shader_compilation` sanity check.
    fn parse(source: &str, label: &str) {
        match naga::front::wgsl::parse_str(source) {
            Ok(module) => {
                // Validate as well: catches interface mismatches early.
                let result = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module);
                assert!(result.is_ok(), "{label}: validation failed: {:?}", result.err());
            }
            Err(e) => panic!("{label}: WGSL parse failed: {e}"),
        }
    }

    #[test]
    fn viewport_quad_vs_parses() {
        parse(VIEWPORT_QUAD_VS, "viewport_quad_vs");
    }

    #[test]
    fn viewport_quad_color_fs_parses() {
        parse(VIEWPORT_QUAD_COLOR_FS, "viewport_quad_color_fs");
    }

    #[test]
    fn viewport_quad_texture_fs_parses() {
        parse(VIEWPORT_QUAD_TEXTURE_FS, "viewport_quad_texture_fs");
    }

    #[test]
    fn globe_vs_parses() {
        parse(GLOBE_VS, "globe_vs");
    }

    #[test]
    fn globe_fs_parses() {
        parse(GLOBE_FS, "globe_fs");
    }

    #[test]
    fn primitive_vs_parses() {
        parse(PRIMITIVE_VS, "primitive_vs");
    }

    #[test]
    fn primitive_fs_parses() {
        parse(PRIMITIVE_FS, "primitive_fs");
    }

    #[test]
    fn billboard_vs_parses() {
        parse(BILLBOARD_VS, "billboard_vs");
    }

    #[test]
    fn billboard_fs_parses() {
        parse(BILLBOARD_FS, "billboard_fs");
    }

    #[test]
    fn model_color_vs_parses() {
        parse(MODEL_COLOR_VS, "model_color_vs");
    }

    #[test]
    fn model_color_fs_parses() {
        parse(MODEL_COLOR_FS, "model_color_fs");
    }

    #[test]
    fn model_textured_vs_parses() {
        parse(MODEL_TEXTURED_VS, "model_textured_vs");
    }

    #[test]
    fn model_textured_fs_parses() {
        parse(MODEL_TEXTURED_FS, "model_textured_fs");
    }

    #[test]
    fn automatic_uniforms_size_matches_struct_layout() {
        // 5 mat4x4<f32> + 1 vec4<f32>, mat4 alignment 16 → no padding holes.
        assert_eq!(CESIUM_AUTOMATIC_UNIFORMS_SIZE, 336);
    }
}
