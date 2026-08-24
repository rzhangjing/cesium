// Hand-written WGSL port of the CesiumJS billboard batch vertex shader
// (packages/engine/Source/Shaders/BillboardCollectionVS.glsl trimmed to the
// fixed layout used by the cesium-rs BillboardCollection /
// PointPrimitiveCollection / LabelCollection batches).
//
// DEVIATION: the CesiumJS original expands per-billboard state through a
// vertex texture fetch path (sizeInMeters, eyeOffset, alignedAxis, rotation,
// translucencyByDistance, scaleByDistance, distanceDisplayCondition...). The
// wgpu port uses a CPU-expanded fixed attribute layout:
//   position            – the billboard anchor in world coordinates
//   corner              – the corner offset in pixels (screen space)
//   texture_coordinate  – atlas texture coordinates
//   color               – the billboard color
// The pixel offset is applied in NDC after the perspective divide, which is
// the screen-space contract of the JS `czm_viewport`-based pixel offset.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer
//     binding(0): struct with czm_modelViewProjection / czm_modelView /
//                 czm_projection / czm_view / czm_model / czm_viewport

struct CesiumAutomaticUniforms {
    czm_modelViewProjection: mat4x4<f32>,
    czm_modelView: mat4x4<f32>,
    czm_projection: mat4x4<f32>,
    czm_view: mat4x4<f32>,
    czm_model: mat4x4<f32>,
    czm_viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> czm: CesiumAutomaticUniforms;

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_textureCoordinates: vec2<f32>,
    @location(1) v_color: vec4<f32>,
};

@vertex
fn main(
    @location(0) position: vec3<f32>,
    @location(1) corner: vec2<f32>,
    @location(2) texture_coordinate: vec2<f32>,
    @location(3) color: vec4<f32>,
) -> VSOutput {
    var out: VSOutput;
    let clip = czm.czm_modelViewProjection * vec4<f32>(position, 1.0);
    // Screen-space pixel offset (JS: billboard pixelOffset / point size
    // applied against the drawing buffer size after projection).
    let viewport_size = czm.czm_viewport.zw;
    let ndc_offset = corner / viewport_size * 2.0;
    out.position = vec4<f32>(clip.xy + ndc_offset * clip.w, clip.z, clip.w);
    out.v_textureCoordinates = texture_coordinate;
    out.v_color = color;
    return out;
}
