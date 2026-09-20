// cesiumrust M6.3 — Panorama / SkyBox (cubemap + equirectangular).
//
// # Blueprint (upstream truth source, `packages/engine/Source/`)
// - `Shaders/SkyBoxVS.glsl`            L1-9   (box scaled to the far plane, view-rotation only)
// - `Shaders/SkyBoxFS.glsl`            L1-9   (`czm_textureCube(normalize(v_texCoord))`,
//                                              `out_FragColor = vec4(czm_gammaCorrect(color).rgb, czm_morphTime)`)
// - `Shaders/CubeMapPanoramaVS.glsl`   L1-11  (`czm_viewRotation * (u_cubeMapPanoramaTransform * (czm_entireFrustum.y * position))`)
// - `Scene/CubeMapPanorama.js`         L100-230
//     * `pass: Pass.ENVIRONMENT` — comment L105: "render before everything else"
//     * `renderState` L207-213: `depthTest: { enabled: false }, depthMask: false,
//       blending: BlendingState.ALPHA_BLEND` — comment L206: "no depth test/write"
//     * geometry L189-192: `BoxGeometry.fromDimensions({ dimensions: (2,2,2),
//       vertexFormat: VertexFormat.POSITION_ONLY })`
//     * L232: `if (!defined(this._cubeMap)) { return undefined; }` — never draw
//       before the cube map finished loading (mirrored on the Rust side:
//       `prepare_panorama_bind_groups` skips views whose `GpuImage` is absent)
// - `Scene/EquirectangularPanorama.js` L102-138
//     * L102-105: `new SphereGeometry({ radius: this._radius, vertexFormat: VertexFormat.ALL })`
//     * L117: `repeat: new Cartesian2(-this._repeatHorizontal, this._repeatVertical)`
//       — comment: "flip horizontally by default to match expected orientation of
//       images inside a sphere, but allow user to override"
//     * L131-135: `renderState: { cull: { enabled: false } }` — comment: "show inside of sphere"
// - `Scene/SkyBox.js`                  L39-43, L100 — `SkyBox` delegates **completely**
//   to `CubeMapPanorama`, so the cubemap panorama *is* the skybox truth source.
//
// # Structure — two independent axes, four combinations
// Upstream ships exactly two pairings:
//
// | upstream primitive            | placement | texture layout |
// |-------------------------------|-----------|----------------|
// | `CubeMapPanorama` / `SkyBox`  | SKYBOX    | CUBEMAP        |
// | `EquirectangularPanorama`     | BUBBLE    | EQUIRECTANGULAR|
//
// `uniforms.mode` (placement) and `uniforms.source` (texture layout) are separate
// fields so the other two pairings — a 360° photograph used as an infinite sky, and
// a cube map anchored to a finite sphere — work without a second shader.
//
// * `MODE_SKYBOX` — infinite, camera-centred. No depth is written; the fragment
//   emits `frag_depth = 0.0`, which under Bevy's **reversed-Z** convention is the
//   far plane. Combined with `depth_compare = GreaterEqual` and
//   `depth_write_enabled = false` this fills exactly the pixels the opaque pass
//   left untouched.
// * `MODE_BUBBLE` — a finite sphere of `uniforms.radius` render units centred on
//   `uniforms.center` (the equirectangular "local bubble", upstream
//   `DEFAULT_RADIUS = 100000.0` m = 0.015678 render units). The ray/sphere entry
//   distance drives a real `frag_depth`, so the bubble correctly occludes the
//   transparent draws that follow it (starfield, sky dome) — the upstream
//   equivalent of `translucent: false` + depth test on.
//
// # DEVIATIONS from the blueprint (all logged in `docs/deviations.md#dev-025`)
// 1. **Fullscreen triangle instead of box geometry.** Upstream scales a 2×2×2 box
//    to `czm_entireFrustum.y` and rasterises it with depth test off. Bevy's own
//    `SkyboxPlugin` (`bevy_core_pipeline-0.15.3/src/skybox/skybox.wgsl` L63-72)
//    draws a fullscreen triangle and reconstructs the ray from `view_from_clip`
//    instead, because an infinite-reverse projection puts the far plane at
//    infinity. We follow Bevy: identical pixels, no 12-triangle vertex buffer.
// 2. **Depth-test substitution for `Pass.ENVIRONMENT`.** Upstream draws the skybox
//    *first* with `depthTest: {enabled: false}`; everything drawn after overwrites
//    it. Bevy's `MainOpaquePass3dNode`
//    (`bevy_core_pipeline-0.15.3/src/core_3d/main_opaque_pass_3d_node.rs` L66) takes
//    the colour attachment through `ViewTarget::get_color_attachment()`, whose
//    `ColorAttachment::get_attachment` (`bevy_render-0.15.3/src/texture/texture_attachment.rs`
//    L62-74) issues `LoadOp::Clear` on its **first** call and `LoadOp::Load`
//    thereafter. A node placed *before* `Node3d::MainOpaquePass` would therefore be
//    erased by the clear. Drawing *after* the opaque pass with
//    `depth_compare = GreaterEqual` + `frag_depth = 0.0` selects precisely the sky
//    pixels, so the final framebuffer is bit-identical to upstream's ordering. Bevy
//    itself reaches the same conclusion — it draws its skybox **inside**
//    `MainOpaquePass3dNode` after the opaque/alpha-mask phases
//    (`main_opaque_pass_3d_node.rs` L113-127) with exactly this depth state
//    (`skybox/mod.rs` L201-216).
// 3. **No `czm_gammaCorrect`.** Upstream applies an explicit `pow(color, 1/2.2)`.
//    Here the panorama colour texture is created as `Rgba8UnormSrgb` (project red
//    line) so the hardware performs sRGB→linear on sample, and the swapchain /
//    `Tonemapping` node performs linear→sRGB on output. Applying the manual gamma
//    pass on top would double-correct.
// 4. **No `czm_morphTime` alpha.** Upstream fades the skybox out while morphing to
//    2D / Columbus View via `out_FragColor.a = czm_morphTime`. cesiumrust has no
//    morph (M6 scope), so the alpha is pinned to 1.0, `BlendingState.ALPHA_BLEND`
//    degenerates to `REPLACE`, and the pipeline uses `blend: None`.
// 5. **Left-handed cube correction.** `direction * vec3(1.0, 1.0, -1.0)` before the
//    cube sample, copied from Bevy's `skybox.wgsl` L78-79 ("Cube maps are
//    left-handed so we negate the z coordinate"). CesiumJS/WebGL cube addressing is
//    right-handed, so this negation is what keeps the domain
//    `CubeMapPanorama::direction_to_face_uv_spec` reference and the GPU result
//    pointing at the same face.
//
// # Numerics
// * Every constant is written once, in decimal, and mirrored by a Rust `const`
//   whose `to_bits()` is asserted equal in `effects::panorama::tests` — the same
//   discipline as `sky_atmosphere.wgsl`.
// * **No FMA reliance.** No expression in this file is written so that its result
//   depends on whether the compiler contracts `a * b + c` into a fused
//   multiply-add: the ray/sphere discriminant is compared against `0.0` with a
//   strict `<`, the degenerate-normalise guard against `1.0e-24`, and every other
//   use feeds a `textureSample` coordinate where a 1-ULP difference is far below
//   the bilinear filter's resolution. Contraction is therefore free to happen.
// * `uniforms.transform` is the **inverse** panorama transform (world→local), the
//   same convention as Bevy's `SkyboxUniforms.transform`
//   (`skybox/mod.rs` L127-129: `Transform::from_rotation(rotation).compute_matrix().inverse()`).
//   Directions are multiplied with `w = 0.0` so translation is ignored; the bubble
//   centre is carried separately in `uniforms.center`.
//
// # Uniform layout (112 bytes; mirrored by `PanoramaUniforms` in `effects/panorama.rs`)
// ```text
//   0: mode      u32      MODE_SKYBOX | MODE_BUBBLE
//   4: source    u32      SOURCE_CUBEMAP | SOURCE_EQUIRECTANGULAR
//   8: brightness f32     radiance multiplier (upstream has none; 1.0 = neutral)
//  12: radius    f32      bubble radius in render units (unused in MODE_SKYBOX)
//  16: repeat    vec2<f32> (-repeat_horizontal, repeat_vertical) — upstream L117
//  24: <pad>     vec2<f32>
//  32: center    vec3<f32> bubble centre in world render units (unused in MODE_SKYBOX)
//  44: <pad>     u32
//  48: transform mat4x4<f32> world→local panorama transform
// ```

#import bevy_render::view::View

// ─── Mode / source selectors ─────────────────────────────────────────────────

/// Infinite, camera-centred placement (`CubeMapPanorama` / `SkyBox`).
const MODE_SKYBOX: u32 = 0u;

/// Finite sphere placement (`EquirectangularPanorama`).
const MODE_BUBBLE: u32 = 1u;

/// Six-face cube texture.
const SOURCE_CUBEMAP: u32 = 0u;

/// 2:1 360° equirectangular texture.
const SOURCE_EQUIRECTANGULAR: u32 = 1u;

// ─── Constants ───────────────────────────────────────────────────────────────

/// π, rounded to the nearest `f32`.
const PANORAMA_PI: f32 = 3.14159265358979323846;

/// 2π, rounded to the nearest `f32`.
const PANORAMA_TAU: f32 = 6.28318530717958647692;

/// π/2, rounded to the nearest `f32`.
const PANORAMA_HALF_PI: f32 = 1.57079632679489661923;

/// Below this squared length a direction is treated as degenerate and replaced by
/// the fallback instead of being `normalize`d into NaN. Mirrors
/// `PANORAMA_DEGENERATE_DIRECTION_SQUARED_EPSILON` in `effects/panorama.rs`.
const DEGENERATE_DIRECTION_SQUARED_EPSILON: f32 = 1.0e-24;

// ─── Uniforms ────────────────────────────────────────────────────────────────

struct PanoramaUniforms {
    mode: u32,
    source: u32,
    brightness: f32,
    radius: f32,
    repeat: vec2<f32>,
    _pad_b: vec2<f32>,
    center: vec3<f32>,
    _pad_c: u32,
    transform: mat4x4<f32>,
}

// Both texture slots are **always** bound, because the choice between them is a
// runtime uniform rather than a shader def: `prepare_panorama_bind_groups` binds a
// 1×1 placeholder into whichever slot the current `source` does not use. This keeps
// a single bind-group layout for all four mode × source combinations.
@group(0) @binding(0) var panorama_cube: texture_cube<f32>;
@group(0) @binding(1) var panorama_equirect: texture_2d<f32>;
@group(0) @binding(2) var panorama_sampler: sampler;
@group(0) @binding(3) var<uniform> view: View;
@group(0) @binding(4) var<uniform> uniforms: PanoramaUniforms;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// `normalize` with a NaN guard: returns `fallback` when `value` is shorter than
/// `sqrt(DEGENERATE_DIRECTION_SQUARED_EPSILON)` (i.e. effectively zero).
///
/// Same class of defect as the atmosphere first-frame `sun_direction == ZERO` bug
/// (M5 Ultra Review H1): `normalize(vec3(0.0))` is `0/0 = NaN`, and NaN propagates
/// into every downstream texture coordinate, silently poisoning the whole frame.
fn safe_normalize(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    if (dot(value, value) <= DEGENERATE_DIRECTION_SQUARED_EPSILON) {
        return fallback;
    }
    return normalize(value);
}

/// Equirectangular UV for a unit sampling direction, in the panorama's **local**
/// frame.
///
/// Op-for-op mirror of `EquirectangularPanorama::sample_uv` in
/// `domain/effects/src/panorama.rs`:
/// ```text
///   lon = atan2(y, x)              -> [-π, π]
///   u   = (lon + π) / 2π           -> [0, 1]
///   lat = asin(clamp(z, -1, 1))    -> [-π/2, π/2]
///   v   = (lat + π/2) / π          -> [0, 1]
///   uv  = (u, v) * repeat          repeat = (-repeat_horizontal, repeat_vertical)
/// ```
/// `clamp` before `asin` is mandatory: `direction` is only unit up to f32 rounding,
/// so `abs(z)` can be `1.0 + 1 ULP`, and `asin` of that is NaN.
///
/// The negative `repeat.x` is upstream `EquirectangularPanorama.js` L117 — the
/// horizontal flip that makes an image authored for "viewed from inside a sphere"
/// read correctly. The sampler must be created with `AddressMode::Repeat` so the
/// resulting out-of-[0,1] coordinate wraps exactly like GL's `GL_REPEAT`.
fn direction_to_equirect_uv(direction: vec3<f32>) -> vec2<f32> {
    let lon = atan2(direction.y, direction.x);
    let u = (lon + PANORAMA_PI) / PANORAMA_TAU;
    let lat = asin(clamp(direction.z, -1.0, 1.0));
    let v = (lat + PANORAMA_HALF_PI) / PANORAMA_PI;
    return vec2(u, v) * uniforms.repeat;
}

/// Sample the active panorama texture for a unit local-frame direction.
///
/// The branch is on a uniform-buffer value, so control flow is uniform across the
/// whole invocation group and `textureSample` (which needs implicit derivatives)
/// stays legal — naga's uniformity analysis proves this, and
/// `panorama_wgsl_parses_and_type_checks_under_naga` is the regression gate.
fn sample_panorama(direction: vec3<f32>) -> vec4<f32> {
    if (uniforms.source == SOURCE_CUBEMAP) {
        // Cube maps are left-handed so we negate the z coordinate
        // (bevy_core_pipeline-0.15.3/src/skybox/skybox.wgsl L78-79).
        return textureSample(
            panorama_cube,
            panorama_sampler,
            direction * vec3(1.0, 1.0, -1.0),
        );
    }
    return textureSample(
        panorama_equirect,
        panorama_sampler,
        direction_to_equirect_uv(direction),
    );
}

/// Reconstruct the **world-space** view ray for a fragment.
///
/// Mirrors `coords_to_ray_direction` in Bevy's `skybox.wgsl` L19-46:
/// 1. framebuffer coords → viewport UV (`coords_to_viewport_uv`,
///    `bevy_pbr-0.15.3/src/render/utils.wgsl` L41-43: `(position - viewport.xy) / viewport.zw`)
/// 2. UV → NDC with the y flip (framebuffer y grows down, clip y grows up)
/// 3. unproject at the **near** plane (`z = 1.0, w = 1.0`). The far plane of an
///    infinite reverse projection is at infinity, so `w = 0.0` would produce
///    `±inf` world positions — upstream's `skybox.wgsl` L26-27 says exactly this.
/// 4. rotate into world space with `w = 0.0` so the view translation is ignored
///    (a sky ray has no position, only a direction).
fn fragment_ray_direction_world(position: vec2<f32>) -> vec3<f32> {
    let viewport_uv = (position - view.viewport.xy) / view.viewport.zw;
    let ndc = viewport_uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let view_homogeneous = view.view_from_clip * vec4(ndc, 1.0, 1.0);
    let view_direction = view_homogeneous.xyz / view_homogeneous.w;
    // `ray_sphere_entry`'s geometric form assumes a unit `direction` (its docstring:
    // "`direction` is unit, so `a == 1`"). `world_from_view` can carry scale / non-
    // orthonormal components, so the reconstructed world direction is not
    // guaranteed unit — normalize at the single return point so every caller
    // (including the bubble `ray_sphere_entry` call site) receives a unit ray.
    // Fallback `vec3(0,0,1)` only triggers on a fully degenerate ray.
    return safe_normalize((view.world_from_view * vec4(view_direction, 0.0)).xyz, vec3(0.0, 0.0, 1.0));
}

/// Nearest **positive** ray/sphere intersection distance, or `-1.0` on a miss.
///
/// CPU reference: `EquirectangularPanorama::ray_sphere_entry` in
/// `domain/effects/src/panorama.rs` (f64). The geometric form is used instead of
/// the quadratic `a t² + b t + c` because `direction` is unit, so `a == 1` and the
/// whole `2a` denominator disappears — that is also what removes the
/// divide-by-zero NaN path that the quadratic form has when `direction` degenerates.
///
/// `origin` inside the sphere (`entry < 0 <= exit_distance`) returns the far hit:
/// that is the street-view case, where the camera sits at the bubble centre and
/// sees the inside of the far wall.
fn ray_sphere_entry(
    origin: vec3<f32>,
    direction: vec3<f32>,
    center: vec3<f32>,
    radius: f32,
) -> f32 {
    let to_center = center - origin;
    let projection = dot(to_center, direction);
    let center_distance_squared = dot(to_center, to_center);
    let half_chord_squared = radius * radius - (center_distance_squared - projection * projection);
    if (half_chord_squared < 0.0) {
        return -1.0;
    }
    let half_chord = sqrt(half_chord_squared);
    let entry = projection - half_chord;
    if (entry > 0.0) {
        return entry;
    }
    let exit_distance = projection + half_chord;
    if (exit_distance > 0.0) {
        return exit_distance;
    }
    return -1.0;
}

// ─── Entry points ────────────────────────────────────────────────────────────

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
}

//  3 |  2.
//  2 |  :  `.
//  1 |  x-----x.
//  0 |  |  s  |  `.
// -1 |  0-----x.....1
//    +---------------
//      -1  0  1  2  3
//
// Fullscreen triangle covering clip space, at `z = 0.0` — the reversed-Z far
// plane, so `depth_compare = GreaterEqual` accepts every pixel the opaque pass
// did not claim. Identical construction to Bevy's `skybox_vertex`
// (skybox.wgsl L52-72).
@vertex
fn panorama_vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let clip_position = vec2(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    ) * 4.0 - vec2(1.0);
    return VertexOutput(vec4(clip_position, 0.0, 1.0));
}

@fragment
fn panorama_fragment(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;
    output.depth = 0.0;
    output.color = vec4<f32>(0.0);

    let world_direction = fragment_ray_direction_world(input.position.xy);
    // `uniforms.transform` is world→local, so this is the ray direction expressed
    // in the panorama's own frame — upstream's `u_cubeMapPanoramaTransform` applied
    // to the (already view-rotated) box position, in reverse.
    let local_ray_direction = safe_normalize(
        (uniforms.transform * vec4(world_direction, 0.0)).xyz,
        world_direction,
    );

    // Bubble placement: intersect the finite panorama sphere. `-1.0` means "miss",
    // which is turned into a `discard` *after* the texture sample below so that no
    // `textureSample` ever sits under non-uniform control flow.
    var hit_distance = -1.0;
    var sample_direction = local_ray_direction;
    if (uniforms.mode == MODE_BUBBLE) {
        let origin = view.world_position;
        hit_distance = ray_sphere_entry(
            origin,
            world_direction,
            uniforms.center,
            uniforms.radius,
        );
        // On a miss `max(..., 0.0)` keeps `hit_point` finite and inside the sphere;
        // the fragment is discarded below, so the sampled colour is never stored.
        let hit_point = origin + world_direction * max(hit_distance, 0.0);
        sample_direction = safe_normalize(
            (uniforms.transform * vec4(hit_point - uniforms.center, 0.0)).xyz,
            local_ray_direction,
        );
    }

    let texel = sample_panorama(sample_direction);

    if (uniforms.mode == MODE_BUBBLE) {
        // A finite bubble writes its real depth so the transparent draws that
        // follow (starfield at r = 50, sky dome at r = 40) are correctly occluded
        // by it. Upstream equivalent: `MaterialAppearance({ translucent: false })`
        // puts `EquirectangularPanorama` in the opaque pass with depth write on.
        let hit_point = view.world_position + world_direction * hit_distance;
        let hit_view = (view.view_from_world * vec4(hit_point, 1.0)).xyz;
        let hit_clip = view.clip_from_view * vec4(hit_view, 1.0);
        output.depth = hit_clip.z / hit_clip.w;
    } else {
        // Reversed-Z far plane: `depth_compare = GreaterEqual` passes only where
        // the depth buffer still holds its cleared `0.0`.
        output.depth = 0.0;
    }

    // No `czm_gammaCorrect`: the sRGB texture format + sRGB framebuffer already
    // perform the encode/decode pair (DEVIATION 3 in the header).
    output.color = vec4(texel.rgb * uniforms.brightness, texel.a);

    if (uniforms.mode == MODE_BUBBLE && hit_distance < 0.0) {
        discard;
    }
    return output;
}
