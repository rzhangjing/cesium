// M5-E2: cesiumrust self-implemented Screen-Space Ambient Occlusion (SSAO).
// ============================================================================
// Two fragment entry points share this module (two render pipelines, one file):
//   • fragment_generate      — hemisphere 16-sample SSAO kernel → AO factor
//   • fragment_blur_modulate — 4×4 box blur of the AO factor, multiplied into
//                              the (tonemapped) scene colour → final output
//
// # Blueprints (paths + line numbers)
//   CesiumJS / cesium-rs (semantic reference, HBAO ray-march variant):
//     • packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionGenerate.glsl L1-144
//         (≡ cesium-rs/crates/cesium-shaders/shaders/PostProcessStages/AmbientOcclusionGenerate.glsl)
//         — L11-23  pixelToEye()      : reconstruct eye-space position from depth
//         — L26-46  getNormalXEdge()  : edge-aware normal reconstruction
//         — L48-53  gaussian()        : distance weight
//         — L55-144 main()            : direction × step march, ao = pow(1-occ, intensity)
//     • packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionModulate.glsl L1-11
//         — multiply AO into the colour buffer (this module's fragment_blur_modulate)
//     • packages/engine/Source/Scene/PostProcessStageLibrary.js L496 createAmbientOcclusionStage
//         / L599 isAmbientOcclusionSupported
//   Bevy (structural / API reference — WGSL + prepass texture access):
//     • bevy_pbr-0.15.3/src/ssao/mod.rs L1-133 (Plugin) / L224-323 (SsaoNode::run)
//         / L682-772 (ViewPrepassTextures::depth_view()/normal_view() binding)
//     • bevy_pbr-0.15.3/src/ssao/ssao.wgsl L72-93
//         — load_normal_view_space() / reconstruct_view_space_position()
//           (view.view_from_clip inverse-projection + view_from_world normal transform)
//
// # DEVIATION
//   CesiumJS AO is an HBAO *ray-march* (directionCount × stepCount, GLSL, eye-space
//   gaussian weighting). Per the M5-E2 plan (hemisphere 16-sample kernel + 4×4 blur),
//   cesiumrust implements the classic *hemisphere-kernel* SSAO family instead
//   (LearnOpenGL / John-Chapman lineage) rewritten in WGSL, because Bevy/wgpu only
//   accept WGSL and the hemisphere kernel maps cleanly onto Bevy's DepthPrepass +
//   NormalPrepass inputs. Domain parameters (intensity=3.0, sample_radius=0.5,
//   sample_count=16, bias=0.001, length_cap=0.26; directionCount=8, stepCount=32)
//   live f64 in domain/effects and are surfaced here as f32 WGSL consts at the GPU
//   boundary. See docs/deviations.md#dev-018.
//
// domain f64 → WGSL f32 boundary; glam fast-math is disabled repo-wide (no reliance
// on non-IEEE float behaviour here).
//
// Bevy reversed-Z: Bevy 0.15.3 uses an infinite-far reversed-Z depth buffer
// (projection.rs `perspective_infinite_reverse_rh`; `Camera3dDepthLoadOp::default()`
// = `Clear(0.0)`), so SKY / far plane == depth 0.0 and the near plane == 1.0. The
// generate pass therefore early-outs on `depth <= 1e-6` (sky), NOT `depth >= 1.0`.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

// ─── Generate-pass bindings (group 0, entry `fragment_generate`) ─────────────
// ViewPrepassTextures::depth_view() / normal_view() (bevy_pbr ssao/mod.rs L730/L744).
@group(0) @binding(0) var depth_prepass: texture_depth_2d;
@group(0) @binding(1) var normal_prepass: texture_2d<f32>;
@group(0) @binding(2) var point_sampler: sampler;      // NonFiltering (depth + normal)
@group(0) @binding(3) var<uniform> view: View;          // dynamic offset per view

// ─── Blur/modulate-pass bindings (group 0, entry `fragment_blur_modulate`) ───
@group(0) @binding(4) var ao_texture: texture_2d<f32>;  // R8/Rgba8 AO factor from generate
@group(0) @binding(5) var color_texture: texture_2d<f32>; // tonemapped scene colour (source)
@group(0) @binding(6) var linear_sampler: sampler;      // Filtering (AO box blur + colour)

// ─── Domain parameters (f32 projection of domain/effects f64 config) ─────────
// post_process.rs L85-95 AmbientOcclusionConfig::default():
//   intensity=3.0, sample_radius=0.5, sample_count=16, bias=0.001, length_cap=0.26
// post_process_stage.rs L245-263 create_ambient_occlusion_composite():
//   directionCount=8, stepCount=32 (HBAO march counts — informational here; the
//   hemisphere kernel uses SAMPLE_COUNT=16 taps, see DEVIATION above).
const SAMPLE_COUNT: i32 = 16;
const AO_INTENSITY: f32 = 3.0;      // pow exponent on the occlusion term
const AO_RADIUS: f32 = 0.5;         // view-space hemisphere radius (world units)
const AO_BIAS: f32 = 0.001;         // self-occlusion depth bias
const AO_LENGTH_CAP: f32 = 0.26;    // max ray length before discarding a sample

// ─── Hemisphere kernel: 16 raw directions (z > 0) ───────────────────────────
// Raw (unnormalised) hemisphere taps; each is `normalize`d + scale-clustered in
// `hemisphere_kernel()` below, so exact magnitudes are irrelevant — only the
// directions matter. Clustered toward the origin (scale = mix(0.1,1.0,(i/16)^2))
// to reduce self-occlusion noise near the surface, per LearnOpenGL SSAO.
const KERNEL: array<vec3<f32>, 16> = array<vec3<f32>, 16>(
    vec3<f32>( 0.5381,  0.1856, 0.4319),
    vec3<f32>( 0.1379,  0.2486, 0.4430),
    vec3<f32>( 0.3371,  0.5679, 0.0057),
    vec3<f32>(-0.6999, -0.0451, 0.0190),
    vec3<f32>( 0.0689, -0.1598, 0.8547),
    vec3<f32>( 0.0560,  0.0069, 0.1843),
    vec3<f32>(-0.0146,  0.1402, 0.0762),
    vec3<f32>( 0.0100, -0.1924, 0.0344),
    vec3<f32>(-0.3577, -0.5301, 0.4358),
    vec3<f32>(-0.3169,  0.1063, 0.0158),
    vec3<f32>( 0.0103, -0.5869, 0.0046),
    vec3<f32>(-0.0897, -0.4940, 0.3287),
    vec3<f32>( 0.0744, -0.4571, 0.6151),
    vec3<f32>(-0.0618,  0.6419, 0.0513),
    vec3<f32>( 0.0839,  0.4151, 0.1640),
    vec3<f32>(-0.4120,  0.1036, 0.5290),
);

fn hemisphere_kernel(i: i32) -> vec3<f32> {
    // Ensure a valid upper-hemisphere direction, then cluster scale toward origin.
    var v = KERNEL[i];
    v.z = abs(v.z) + 0.05;              // bias into the +z hemisphere
    v = normalize(v);
    let scale = mix(0.1, 1.0, pow(f32(i) / f32(SAMPLE_COUNT), 2.0));
    return v * scale;
}

// Integer bit-mix (Wang hash) — avalanche so adjacent pixels decorrelate.
fn wang_hash(seed: u32) -> u32 {
    var h = seed;
    h = (h ^ 61u) ^ (h >> 16u);
    h = h + (h << 3u);
    h = h ^ (h >> 4u);
    h = h * 0x27d4eb2du;
    h = h ^ (h >> 15u);
    return h;
}

// Deterministic per-pixel rotation noise (replaces CesiumJS `randomTexture`,
// AmbientOcclusionGenerate.glsl L83-86) — avoids a separate noise texture.
//
// Ryan L1: the old hash `x*12 + y*57 == 3(4x + 19y)` produced a ~19px diagonal
// lattice; replaced with coprime primes + a Wang bit-mix.
// Ryan M4: emits a TRUE 3-D rotation vector (the old z was hard-0, which
// degenerated the Gram-Schmidt TBN build when the view-space normal lies near the
// xy-plane — the silhouette / grazing-angle region where AO is most visible).
fn pixel_noise(pixel: vec2<i32>) -> vec3<f32> {
    let base = (u32(pixel.x) * 73856093u) ^ (u32(pixel.y) * 19349663u);
    let r1 = f32(wang_hash(base) & 0xffffu) * (1.0 / 65535.0);
    let r2 = f32(wang_hash(base + 0x9e3779b9u) & 0xffffu) * (1.0 / 65535.0);
    let r3 = f32(wang_hash(base + 0x85ebca6bu) & 0xffffu) * (1.0 / 65535.0);
    return vec3<f32>(r1, r2, r3) * 2.0 - 1.0;
}

// Reconstruct view-space position from the depth prepass.
// Mirrors bevy_pbr ssao.wgsl L83-88 (view.view_from_clip inverse-projection) and
// CesiumJS pixelToEye() (AmbientOcclusionGenerate.glsl L11-23).
fn reconstruct_view_position(depth: f32, uv: vec2<f32>) -> vec3<f32> {
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = view.view_from_clip * vec4<f32>(clip_xy, depth, 1.0);
    return t.xyz / t.w;
}

// Load the view-space surface normal from the normal prepass.
// Bevy stores *world* normals encoded as (n*0.5+0.5); transform by view_from_world.
// Mirrors bevy_pbr ssao.wgsl L72-81.
fn load_view_normal(uv: vec2<f32>) -> vec3<f32> {
    var world_normal = textureSampleLevel(normal_prepass, point_sampler, uv, 0.0).xyz;
    world_normal = (world_normal * 2.0) - 1.0;
    let view_from_world = mat3x3<f32>(
        view.view_from_world[0].xyz,
        view.view_from_world[1].xyz,
        view.view_from_world[2].xyz,
    );
    return normalize(view_from_world * world_normal);
}

// ─── SSAO generate: hemisphere 16-sample kernel ─────────────────────────────
@fragment
fn fragment_generate(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let uv = in.uv;

    let depth = textureSampleLevel(depth_prepass, point_sampler, uv, 0.0);
    // Ryan M1 (reversed-Z): sky / infinite far plane == depth 0.0 in Bevy 0.15.3.
    // Early-out → no occlusion (ao = 1.0), matching CesiumJS
    // (AmbientOcclusionGenerate.glsl L61-65, out_FragColor = vec4(1.0)). The prior
    // `depth >= 1.0` predicate was inverted (it fired only on the unreachable near
    // plane, letting real sky fall through to reconstruct_view_position(0.0)).
    if depth <= 1.0e-6 {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let frag_pos = reconstruct_view_position(depth, uv);
    let normal = load_view_normal(uv);

    // Build a TBN matrix aligning tangent-space +z with the view-space normal,
    // rotated per-pixel by the noise vector to decorrelate the kernel.
    let random_vec = pixel_noise(pixel);
    // Gram-Schmidt: tangent = random_vec projected onto the plane ⊥ normal.
    var tangent = random_vec - normal * dot(random_vec, normal);
    // Ryan M4 degeneracy guard: if random_vec is (near) parallel to normal the
    // residual collapses and normalize(0) = NaN, which the 4×4 box blur then
    // spreads to neighbours. Fall back to the Cartesian axis least aligned with
    // the normal (guarantees a non-degenerate orthogonalisation).
    if dot(tangent, tangent) < 1e-8 {
        let an = abs(normal);
        var helper = vec3<f32>(1.0, 0.0, 0.0);
        if an.x <= an.y && an.x <= an.z {
            helper = vec3<f32>(1.0, 0.0, 0.0);
        } else if an.y <= an.z {
            helper = vec3<f32>(0.0, 1.0, 0.0);
        } else {
            helper = vec3<f32>(0.0, 0.0, 1.0);
        }
        tangent = helper - normal * dot(helper, normal);
    }
    let tangent_n = normalize(tangent);
    let bitangent = cross(normal, tangent_n);
    let tbn = mat3x3<f32>(tangent_n, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < SAMPLE_COUNT; i = i + 1) {
        // Sample point in view space: rotate the hemisphere tap by the TBN, scale
        // by the AO radius, offset from the fragment position.
        let sample_vec = tbn * hemisphere_kernel(i);
        let sample_pos = frag_pos + sample_vec * AO_RADIUS;

        // Project the sample back to screen space.
        let offset = vec4<f32>(sample_pos, 1.0);
        let clip = view.clip_from_view * offset;
        // Ryan L2: reject samples behind the camera (clip.w <= 0). Without this the
        // perspective divide flips the UV instead of culling it, sampling the wrong
        // side of the screen.
        if clip.w <= 1.0e-6 {
            continue;
        }
        var sample_uv = (clip.xy / clip.w) * 0.5 + 0.5;
        sample_uv.y = 1.0 - sample_uv.y;   // flip to texture space

        // Ryan M4/L1 NaN-safe bounds check: a non-finite coordinate fails every
        // `>=`/`<=` comparison, so negating the conjunction discards NaN/inf samples
        // as well as genuine off-screen ones.
        if !(sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
            continue;                       // off-screen / non-finite → no contribution
        }

        let sample_depth = textureSampleLevel(depth_prepass, point_sampler, sample_uv, 0.0);
        let sampled_view_pos = reconstruct_view_position(sample_depth, sample_uv);

        // Range check + bias: only occlude if the sampled surface is in front of
        // (closer than) the kernel sample point, within the length cap.
        // Mirrors CesiumJS dot/bias/lengthCap weighting (AmbientOcclusionGenerate.glsl L127-135).
        let delta = sampled_view_pos.z - sample_pos.z;
        let range_check = smoothstep(0.0, 1.0, AO_RADIUS / max(abs(frag_pos.z - sampled_view_pos.z), 1e-4));
        let occludes = select(0.0, 1.0, delta > AO_BIAS && abs(delta) < AO_LENGTH_CAP);
        occlusion += occludes * range_check;
    }

    occlusion = 1.0 - (occlusion / f32(SAMPLE_COUNT));
    // Apply intensity exponent (CesiumJS ao = pow(ao, intensity), L142).
    let ao = pow(clamp(occlusion, 0.0, 1.0), AO_INTENSITY);
    return vec4<f32>(ao, ao, ao, 1.0);
}

// ─── 4×4 box blur + modulate ────────────────────────────────────────────────
// Blur the (noisy) 16-tap AO factor with a 4×4 box kernel, then multiply into
// the tonemapped scene colour (CesiumJS AmbientOcclusionModulate.glsl L1-11).
@fragment
fn fragment_blur_modulate(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(view.viewport.zw);
    let uv = in.uv;

    // 4×4 box blur centred on the pixel (16 taps, half-texel offsets).
    var ao_sum = 0.0;
    for (var y = 0; y < 4; y = y + 1) {
        for (var x = 0; x < 4; x = x + 1) {
            let offset = (vec2<f32>(f32(x), f32(y)) - 1.5) * texel;
            ao_sum += textureSampleLevel(ao_texture, linear_sampler, uv + offset, 0.0).r;
        }
    }
    let ao = ao_sum / 16.0;

    let color = textureSampleLevel(color_texture, linear_sampler, uv, 0.0);
    // Modulate: darken the colour by the (blurred) ambient-occlusion factor.
    return vec4<f32>(color.rgb * ao, color.a);
}
