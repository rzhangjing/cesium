// M6.2: cesiumrust ClippingPlanes — WGSL clip / discard.
// ============================================================================
// Faithful port of the upstream CesiumJS clipping-plane fragment stage:
//   • packages/engine/Source/Shaders/Model/ModelClippingPlanesStageFS.glsl L39-88
//       — clip(): reconstruct eye position from the fragment, per-plane signed
//         distance, union (min-distance, any ≤ 0 ⇒ discard) vs intersection
//         (all ≤ 0 ⇒ discard), then the edge-highlight band.
//   • packages/engine/Source/Scene/ClippingPlaneCollection.js L146-152
//       — unionIntersectFunction (v === OUTSIDE) / defaultIntersectFunction
//         (v === INSIDE): the authoritative union/intersection semantics.
//   • packages/engine/Source/Shaders/Builtin/Functions/unpackClippingExtents.glsl
//       — texture-packed plane lookup (upstream stores planes in a texture; this
//         port uses a fixed-size uniform array — see DEVIATION below).
//   Blueprint (踩坑参考): cesium-rs/crates/cesium-shaders/shaders/Model/…
//
// CPU reference pairing: the union/intersection signed-distance accumulation
//   below (`apply_clipping_planes`) mirrors `ClippingPlaneCollection::
//   clip_signed` in `domain/effects/src/clipping.rs` entry-for-entry (f64 CPU vs
//   f32 GPU) so the two can be cross-validated by
//   `clipping_gpu_signed_accumulation_matches_the_cpu_reference`.
//   FIX-CLIP-CPUREF.
//
// Two roles for this module:
//   1. IMPORT MODULE (faithful forward pass): `apply_clipping_planes(world_pos)`
//      is #import-ed into the globe / tileset fragment shaders so clipping
//      discards fragments during geometry rasterisation — exactly where upstream
//      `modelClippingPlanesStage(inout vec4 color)` runs. That wiring is done by
//      integration task #81 (it touches the material shaders, out of M6.2 scope).
//   2. SELF-CONTAINED NODE ENTRY (`fragment`): a screen-space clip node that
//      reconstructs the world position from the depth prepass (the WGSL analogue
//      of upstream `czm_windowToEyeCoordinates(gl_FragCoord)`) and applies the
//      same `apply_clipping_planes`. Used by ClippingPlanesNode (clipping_planes.rs).
//
// # RED LINES (hard constraints)
//   • METERS_PER_RENDER_UNIT = 6378137 (WGS84 semi-major axis). Domain planes
//     are metric f64; the adapter divides every plane *distance* by 6378137 to
//     bring it into render-unit world space before upload, so the `plane.w`
//     values consumed here are ALREADY render units and match `world_pos`
//     (Bevy world = meters / 6378137). Normals are unit directions (scale-free).
//   • NO FMA CONTRACTION: the signed distance is computed as `dot(n, p)` and
//     then a SEPARATE `+ plane.w` add, keeping two IEEE roundings. The result
//     must not depend on the compiler fusing multiply-add into a single fma.
//   • glam fast-math is disabled repo-wide; nothing here relies on non-IEEE
//     float behaviour.
//   • domain f64 → WGSL f32 boundary: all values in this shader are f32.
//
// # DEVIATION (docs/deviations.md#dev-023)
//   Upstream packs an ARBITRARY number of planes into a texture (uint8 oct-encoded
//   normal + packed float distance, or RGBA float) and transforms each plane
//   per-fragment via `czm_transformPlane(plane, matrix)`. This port instead:
//     (a) caps the count at MAX_CLIPPING_PLANES = 8 (a clip box needs 6), and
//     (b) pre-transforms planes by the collection `model_matrix` on the CPU
//         (`ClippingPlaneCollection::world_planes()`), uploading world-space
//         planes so the shader needs no per-fragment matrix multiply.
//   Both are sign-equivalent to upstream (proved by the domain unit test
//   `test_world_planes_transform_equivalence`). Logged as #dev-023 (a) + (b).
//
// Bevy reversed-Z: Bevy 0.15.3 uses an infinite-far reversed-Z depth buffer, so
// SKY / far plane == depth 0.0 and the near plane == 1.0. The node entry
// therefore early-outs on `depth <= 1e-6` (nothing to clip in the sky).

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

// ─── Clipping-plane uniform (group 0, binding 5) ─────────────────────────────
// Packed on the CPU by ClippingPlanesUniform::from_domain (clipping_planes.rs).
struct ClippingPlanes {
    // xyz = unit normal (world space), w = signed distance in RENDER UNITS
    // (domain metric distance / METERS_PER_RENDER_UNIT). Unused slots are zero.
    planes: array<vec4<f32>, 8>,
    // rgb = edge highlight colour, a = edge width in PIXELS.
    edge_color: vec4<f32>,
    // Written for clipped fragments on the node path (default: transparent).
    background_color: vec4<f32>,
    // x = plane count, y = union flag (0 = intersection, 1 = union),
    // z = enabled (0 ⇒ pass-through, pixel-neutral), w = pad.
    params: vec4<f32>,
};

const MAX_CLIPPING_PLANES: i32 = 8;

// ─── Node bindings (group 0) ─────────────────────────────────────────────────
@group(0) @binding(0) var depth_prepass: texture_depth_2d;   // world-position reconstruction
@group(0) @binding(1) var color_texture: texture_2d<f32>;    // scene colour (post-process source)
@group(0) @binding(2) var point_sampler: sampler;            // NonFiltering (depth)
@group(0) @binding(3) var linear_sampler: sampler;           // Filtering (colour)
@group(0) @binding(4) var<uniform> view: View;               // dynamic offset per view
@group(0) @binding(5) var<uniform> clipping: ClippingPlanes;

// ─── Depth → view-space position (mirrors ao.wgsl reconstruct_view_position) ──
fn reconstruct_view_position(depth: f32, uv: vec2<f32>) -> vec3<f32> {
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = view.view_from_clip * vec4<f32>(clip_xy, depth, 1.0);
    return t.xyz / t.w;
}

// ─── Core clipping decision (shared by node entry + forward-pass import) ─────
struct ClipResult {
    clipped: bool,          // true ⇒ fragment lies in the clipped region
    edge_factor: f32,       // [0,1] blend weight toward edge_color near a boundary
    nearest_signed: f32,    // signed distance (render units) to the nearest plane
}

// Signed distance from a world-space point (render units) to plane `i`.
// NO FMA CONTRACTION: dot() and the + plane.w add are two distinct roundings.
fn plane_signed_distance(world_pos: vec3<f32>, plane: vec4<f32>) -> f32 {
    return dot(plane.xyz, world_pos) + plane.w;
}

// Applies the whole clipping collection to a world-space position.
// Faithful to ModelClippingPlanesStageFS.glsl clip() + ClippingPlaneCollection.js
// union/intersection semantics.
fn apply_clipping_planes(world_pos: vec3<f32>) -> ClipResult {
    var result: ClipResult;
    result.clipped = false;
    result.edge_factor = 0.0;
    result.nearest_signed = 0.0;

    let count = i32(clipping.params.x);
    if count <= 0 {
        // No planes ⇒ nothing is ever clipped (also the disabled-collection case).
        return result;
    }

    let union_mode = clipping.params.y > 0.5;

    var all_outside = true;   // intersection: clipped only when OUTSIDE ALL planes
    var any_outside = false;  // union:       clipped when OUTSIDE ANY plane
    // Blueprint ModelClippingPlanesStageFS.glsl keeps a SIGNED clipAmount whose
    // accumulation differs per mode — union folds with a signed `min` (L59),
    // intersection with a signed `max` seeded at 0.0 (L64) — and it is that signed
    // value (not the absolute nearest distance) that feeds the edge-highlight band.
    // FIX-CLIP-ACCUM: reproduce the signed min/max exactly.
    var clip_amount = 0.0;

    let n = min(count, MAX_CLIPPING_PLANES);
    for (var i = 0; i < n; i = i + 1) {
        let d = plane_signed_distance(world_pos, clipping.planes[i]);
        // Outside a plane ⇔ signed distance <= 0. The blueprint compares
        // `amount <= 0.0`, so a point lying exactly ON a plane counts as clipped.
        // FIX-CLIP-LTE: `<` -> `<=`.
        if d <= 0.0 {
            any_outside = true;
        } else {
            all_outside = false;
        }
        if union_mode {
            if i == 0 {
                clip_amount = d;
            } else {
                clip_amount = min(d, clip_amount);
            }
        } else {
            clip_amount = max(d, clip_amount);
        }
    }

    if union_mode {
        // unionIntersectFunction: clipped when OUTSIDE ANY plane.
        result.clipped = any_outside;
    } else {
        // defaultIntersectFunction: clipped only when OUTSIDE ALL planes.
        result.clipped = all_outside;
    }
    result.nearest_signed = clip_amount;

    // Edge-highlight band (ModelClippingPlanesStageFS.glsl L85-87). `fwidth` gives
    // the screen-space footprint of the signed distance — the WGSL analogue of
    // upstream `czm_metersPerPixel(position)` — so edge_color.w stays in PIXELS.
    // DEVIATION: see docs/deviations.md#dev-023 (c).
    let edge_width_px = clipping.edge_color.w;
    if edge_width_px > 0.0 {
        let pixel = fwidth(clip_amount);
        let threshold = edge_width_px * pixel;
        let adist = abs(clip_amount);
        if threshold > 0.0 && adist < threshold {
            result.edge_factor = 1.0 - (adist / threshold);
        }
    }

    return result;
}

// ─── Self-contained screen-space clip node entry ─────────────────────────────
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let source = textureSampleLevel(color_texture, linear_sampler, uv, 0.0);

    // Master gate (params.z). Disabled ⇒ pass the source colour straight through,
    // so the node is pixel-neutral (gate OFF ⇒ dynamic_globe v0 zero diff).
    if clipping.params.z < 0.5 {
        return source;
    }

    let depth = textureSampleLevel(depth_prepass, point_sampler, uv, 0.0);
    // Reversed-Z: sky / infinite far == 0.0 ⇒ no surface to clip. This is a PER-
    // FRAGMENT condition, so it must NOT early-return above the `fwidth` inside
    // apply_clipping_planes: derivatives under non-uniform control flow are
    // undefined (naga's DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE lets it *validate*
    // silently — §5.6). Compute the clip unconditionally so `fwidth` stays uniform,
    // then fold the sky test into a final `select`. FIX-CLIP-FWIDTH.
    let is_sky = depth <= 1.0e-6;

    // Reconstruct the world-space (render-unit) position of the surface hit. Clamp
    // the depth off the sky floor so the (selected-away) reconstruction stays finite.
    let view_pos = reconstruct_view_position(max(depth, 1.0e-6), uv);
    let world_h = view.world_from_view * vec4<f32>(view_pos, 1.0);
    let world_pos = world_h.xyz / world_h.w;

    let clip = apply_clipping_planes(world_pos);

    var lit = source;
    if clip.edge_factor > 0.0 {
        lit = vec4<f32>(
            mix(source.rgb, clipping.edge_color.rgb, clip.edge_factor),
            source.a,
        );
    }
    // Node path: a clipped fragment cannot `discard` already-rasterised geometry,
    // so it is overwritten with the configured background colour.
    // DEVIATION: see docs/deviations.md#dev-023 (d). The faithful per-geometry
    // `discard` happens when `apply_clipping_planes` is #import-ed into the
    // globe/tileset fragment shaders — deferred #51, deliberately NOT done by the
    // task #81 integration (which owns only the shared render graph).
    let clipped = select(lit, clipping.background_color, clip.clipped);
    // Sky pixels pass the source straight through.
    return select(clipped, source, is_sky);
}
