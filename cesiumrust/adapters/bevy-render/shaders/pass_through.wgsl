// Blueprint: CesiumJS Source/Shaders/PostProcess/PassThrough.glsl (8 lines)
// cesiumrust WGSL equivalent — samples input and outputs unchanged (pixel-neutral).
// DEVIATION: see docs/deviations.md#dev-016 (RenderGraph architecture difference)
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screenTexture: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(screenTexture, samp, in.uv);
}
