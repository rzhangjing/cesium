// Blueprint: CesiumJS Source/Scene/SplitDirection.js + Splitter.js (split-screen).
// cesiumrust WGSL equivalent — a screen-space overlay that draws the draggable
// vertical split divider at `split_position_px` over the resolved scene colour.
//
// The FAITHFUL upstream split semantics (per-primitive `discard` so Left-assigned
// geometry shows only on the left half and Right-assigned only on the right) live
// in the material shader via `SplitterConfig::wgsl_shader_modification()` and are
// deferred to the real-GPU task — this node only renders the divider handle, the
// one *screen-space* artefact the splitter owns. See docs/deviations.md#dev-034.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

// group(0): the resolved scene colour + a linear sampler (pass-through base).
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// group(1): the split configuration uniform (see `SplitUniform` in split.rs).
struct SplitData {
    // Horizontal centre of the divider, in viewport pixels.
    split_position_px: f32,
    // Divider thickness, in pixels.
    line_width_px: f32,
    // Divider colour (RGBA, straight alpha).
    color: vec4<f32>,
};
@group(1) @binding(0) var<uniform> split: SplitData;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(screen_texture, samp, in.uv);

    // `position.xy` is the pixel-space fragment coordinate (the WGSL analogue of
    // GLSL `gl_FragCoord`), measured from the top-left. A fragment sits on the
    // divider when its distance to `split_position_px` is within half a line.
    let half = split.line_width_px * 0.5;
    let dist = abs(in.position.x - split.split_position_px);
    let line_mask = select(0.0, 1.0, dist <= half);

    return mix(source, split.color, line_mask);
}
