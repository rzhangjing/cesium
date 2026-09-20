// M6.4: cesiumrust OIT composite pass — fullscreen weighted-blended compositing.
// ============================================================================
// Reads the OIT accumulation buffer (Rgba16Float), the revealage buffer
// (R8Unorm), and the opaque scene colour, then applies the McGuire-Bavoil 2013
// composite formula to produce the final pixel colour.
//
// # Blueprints (paths + line numbers)
//   CesiumJS upstream (authoritative reference — 逐字重写):
//     • packages/engine/Source/Shaders/CompositeOITFS.glsl L1-32 (901 bytes):
//         uniform sampler2D u_opaque;
//         uniform sampler2D u_accumulation;
//         uniform sampler2D u_revealage;
//         in vec2 v_textureCoordinates;
//         void main() {
//             vec4 opaque = texture(u_opaque, v_textureCoordinates);
//             vec4 accum = texture(u_accumulation, v_textureCoordinates);
//             float r = texture(u_revealage, v_textureCoordinates).r;
//         #ifdef MRT
//             vec4 transparent = vec4(accum.rgb / clamp(r, 1e-4, 5e4), accum.a);
//         #else
//             vec4 transparent = vec4(accum.rgb / clamp(accum.a, 1e-4, 5e4), r);
//         #endif
//             out_FragColor = (1.0 - transparent.a) * transparent + transparent.a * opaque;
//             if (opaque != czm_backgroundColor) { out_FragColor.a = 1.0; }
//         }
//
//     • packages/engine/Source/Scene/OIT.js L283-306 (compositeCommand setup):
//         Defines "MRT" when translucentMRTSupport is true.
//         uniformMap: u_opaque, u_accumulation, u_revealage.
//
//   Domain CPU reference:
//     • domain/effects/src/oit.rs L203-234 (OitConfig::composite):
//         avg_color = accumulation.xyz / accumulation.w (when w > 1e-5)
//         translucent_alpha = (1 - revealage) * avg_color.w
//         result = opaque * revealage + avg_color * translucent_alpha
//
// # DEVIATIONS from upstream
//   Our MRT storage layout differs from upstream:
//     Upstream MRT: accumulation.a = Π(1−ai) [revealage], revealage.r = Σ(ai·wzi) [weight sum]
//     Ours:         accumulation.a = Σ(ai·wzi) [weight sum], revealage.r = Π(1−ai) [revealage]
//   Therefore we use the upstream **non-MRT** (multipass) branch formula:
//     transparent = vec4(accum.rgb / clamp(accum.a, 1e-4, 5e4), revealage.r)
//   This is mathematically equivalent to upstream MRT with swapped storage roles.
//
// # RED LINES
//   • NO FMA contraction — each arithmetic step separate.
//   • domain f64 → f32 at WGSL boundary only.
//   • WGSL reserved word avoidance: no `mod` identifier.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

// ─── Bindings ───────────────────────────────────────────────────────────────
@group(0) @binding(0) var opaque_texture: texture_2d<f32>;
@group(0) @binding(1) var accumulation_texture: texture_2d<f32>;
@group(0) @binding(2) var revealage_texture: texture_2d<f32>;
@group(0) @binding(3) var linear_sampler: sampler;

// ─── Background colour constant (CesiumJS czm_backgroundColor = black) ──────
// Upstream: if (opaque != czm_backgroundColor) { out_FragColor.a = 1.0; }
// czm_backgroundColor defaults to Color(0,0,0,0) in CesiumJS.
const BG_COLOR: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

// ─── Fragment entry: composite OIT buffers with opaque scene ─────────────────
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Sample the three inputs (CompositeOITFS.glsl L15-17)
    let opaque = textureSampleLevel(opaque_texture, linear_sampler, uv, 0.0);
    let accum = textureSampleLevel(accumulation_texture, linear_sampler, uv, 0.0);
    let revealage_r = textureSampleLevel(revealage_texture, linear_sampler, uv, 0.0).r;

    // Early out: if revealage ≈ 1, no translucent contribution (domain L209-211).
    if revealage_r >= 1.0 {
        return opaque;
    }

    // CompositeOITFS.glsl L22 (non-MRT branch, adapted for our storage layout):
    //   transparent = vec4(accum.rgb / clamp(accum.a, 1e-4, 5e4), revealage.r)
    let weight_sum = clamp(accum.a, 1e-4, 5e4);
    let avg_rgb = accum.rgb / weight_sum;
    let transparent = vec4<f32>(avg_rgb, revealage_r);

    // CompositeOITFS.glsl L25:
    //   out_FragColor = (1.0 - transparent.a) * transparent + transparent.a * opaque
    let one_minus_ta = 1.0 - transparent.a;
    var out_color = one_minus_ta * transparent + transparent.a * opaque;

    // CompositeOITFS.glsl L27-30: force alpha=1 unless pixel is background.
    // "opaque != czm_backgroundColor" — any non-zero opaque pixel gets alpha 1.
    // WGSL `!=` on vec4 is component-wise → vec4<bool>; `if` needs a scalar bool,
    // so wrap in `any(...)` (true when any channel differs from the background).
    if any(opaque != BG_COLOR) {
        out_color.a = 1.0;
    }

    return out_color;
}
