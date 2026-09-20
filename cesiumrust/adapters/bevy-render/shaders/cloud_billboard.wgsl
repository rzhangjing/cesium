// M6.6: cesiumrust Clouds — instanced billboard geometry (faithful VS+FS pair).
// ============================================================================
// Faithful port of the upstream CesiumJS cumulus-cloud BILLBOARD render path:
//   • packages/engine/Source/Shaders/CloudCollectionVS.glsl L1-48 (1280 B)
//       — the vertex stage: unpack per-instance attributes, build the eye-facing
//         quad (`offset = dir - 0.5`, `scaledOffset = scale * offset`,
//         `positionEC.xy += scaledOffset`, `positionEC.xyz *= show`), and emit the
//         varyings (v_offset / v_maximumSize / v_color / v_slice / v_brightness).
//   • packages/engine/Source/Shaders/CloudCollectionFS.glsl L161-262
//       — the fragment stage: the billboard-local ray setup in `main`
//         (`coordinate = maximumSize.xy * v_offset`, `ellipsoidScale = 0.82 *
//         maximumSize`, `eye = (0,0,-10-zOffset)`) feeding `drawCloud`.
//   • domain/effects/src/cloud.rs `build_billboard` / `draw_cloud` — the f64 CPU
//     reference for the quad geometry + shading (cross-validated by tests).
//   Blueprint (踩坑参考): cesium-rs/crates/cesium-shaders/shaders/CloudCollection{VS,FS}.glsl
//
// # ROLE vs clouds.wgsl
//   This is the FAITHFUL INSTANCED-GEOMETRY path: one camera-facing quad per cloud,
//   rasterised in the transparent pass, shaded in billboard-local space exactly as
//   upstream does. `clouds.wgsl` is the screen-space NODE path (the one the M6.6
//   adapter scaffold wires, since it needs no instance/mesh buffers). Both sample
//   the SAME 3D noise texture. Wiring this instanced path (instance vertex buffers
//   + transparent draw) is DEFERRED to headless/CI GPU取证 — see the M6.6 report.
//
// # RED LINES (hard constraints)
//   • METERS_PER_RENDER_UNIT = 6378137: instance centres / sizes arrive ALREADY in
//     render units (the adapter divides metric f64 by 6378137 before upload).
//   • NO `mod(` (WGSL RESERVED WORD) — `fract` / `%` only.
//   • NO FMA CONTRACTION and NO SWIZZLE ASSIGNMENT (vectors rebuilt whole).
//   • glam fast-math disabled repo-wide; domain f64 → WGSL f32 at the uniform edge.

#import bevy_render::view::View

const CLOUD_PI: f32 = 3.14159265358979;

// Per-instance cloud data, mirrored from CloudCollection.js attribute packing
// (positionHighAndScaleX / positionLowAndScaleY / packedAttribute0/1 / color).
struct CloudInstance {
    /// xyz = billboard centre (RENDER UNITS), w = show flag (0 hides the quad).
    center_and_show: vec4<f32>,
    /// xy = billboard scale (width, height), zw = pad.
    scale: vec4<f32>,
    /// xyz = maximumSize (the cloud volume extents), w = slice.
    size_and_slice: vec4<f32>,
    /// rgb = colour, a = brightness.
    color_and_brightness: vec4<f32>,
}

struct BillboardVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>,          // v_offset
    @location(1) maximum_size: vec3<f32>,    // v_maximumSize
    @location(2) color: vec4<f32>,           // v_color
    @location(3) slice: f32,                 // v_slice
    @location(4) brightness: f32,            // v_brightness
}

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var noise_texture: texture_3d<f32>;
@group(1) @binding(1) var linear_sampler: sampler;
@group(1) @binding(2) var<uniform> noise_meta: vec4<f32>;   // x = detail, y = dim

// ─── Vertex stage (mirror CloudCollectionVS.glsl) ────────────────────────────

@vertex
fn billboard_vertex(
    // The quad corner in [0,1]² (per-vertex; the four corners of a unit billboard).
    @location(0) corner: vec2<f32>,
    // Per-instance cloud attributes (instance step mode).
    @location(1) center_and_show: vec4<f32>,
    @location(2) scale: vec4<f32>,
    @location(3) size_and_slice: vec4<f32>,
    @location(4) color_and_brightness: vec4<f32>,
) -> BillboardVertexOutput {
    // offset = dir - vec2(0.5, 0.5) ; scaledOffset = scale * offset (VS L34-35).
    let offset = corner - vec2<f32>(0.5, 0.5);
    let scaled_offset = scale.xy * offset;

    // Transform the cloud centre into view space, expand the quad in view space
    // (screen-aligned, exactly like upstream positionEC.xy += scaledOffset).
    let world_center = vec4<f32>(center_and_show.xyz, 1.0);
    let view_center = view.view_from_world * world_center;
    let expanded_xy = view_center.xy + scaled_offset;
    // positionEC.xyz *= show (VS L40): collapse the quad when hidden.
    let show = center_and_show.w;
    var view_pos = vec4<f32>(expanded_xy, view_center.z, view_center.w) * show;
    // Keep w intact (the *show above must not scale w); rebuild explicitly.
    view_pos = vec4<f32>(view_pos.xyz, view_center.w);

    var out: BillboardVertexOutput;
    out.clip_position = view.clip_from_view * view_pos;
    out.offset = offset;
    out.maximum_size = size_and_slice.xyz;
    out.color = vec4<f32>(color_and_brightness.rgb, 1.0);
    out.slice = size_and_slice.w;
    out.brightness = color_and_brightness.a;
    return out;
}

// ─── Shading helpers (mirror CloudCollectionFS.glsl; shared with clouds.wgsl) ──

fn sample_noise(pos: vec3<f32>) -> vec4<f32> {
    let dim = noise_meta.y;
    let uvw = fract((pos + vec3<f32>(dim * 0.5)) / dim);
    return textureSampleLevel(noise_texture, linear_sampler, uvw, 0.0);
}

fn phase_shift_2d(p: vec2<f32>, freq: vec2<f32>) -> vec2<f32> {
    let half_pi = 1.5707963267948966;
    return vec2<f32>(half_pi * sin(freq.y * p.y), half_pi * sin(freq.x * p.x));
}

fn phase_shift_3d(p: vec3<f32>, freq: vec2<f32>) -> vec2<f32> {
    let s = sin(freq.x * p.z);
    return phase_shift_2d(p.xy, freq) + vec2<f32>(CLOUD_PI * s, CLOUD_PI * s);
}

fn gardner_texture(point: vec3<f32>) -> f32 {
    var sum = vec2<f32>(0.0, 0.0);
    var ci: f32 = 0.8;
    var fxy = vec2<f32>(0.6, 0.6);
    for (var i: i32 = 0; i < 5; i = i + 1) {
        let pxy = phase_shift_3d(point, fxy);
        ci = ci * 0.707;
        fxy = fxy * 2.0;
        let sin_term = vec2<f32>(
            sin(fxy.x * point.x + pxy.x),
            sin(fxy.y * point.y + pxy.y),
        );
        sum = sum + ci * sin_term + vec2<f32>(0.6, 0.6);
    }
    return 0.1 * sum.x * sum.y;
}

fn cloud_intensity(id: f32, is: f32, it: f32) -> f32 {
    let a: f32 = 0.5;
    let t: f32 = 0.4;
    let s: f32 = 0.25;
    return (1.0 - a) * ((1.0 - t) * ((1.0 - s) * id + s * is) + t * it) + a;
}

struct EllipsoidHit {
    point: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    hit: f32,
}

fn intersect_sphere(origin: vec3<f32>, dir: vec3<f32>, slice: f32) -> EllipsoidHit {
    let miss = EllipsoidHit(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, 0.0);
    let a = dot(dir, dir);
    let b = dot(origin, dir);
    let c = dot(origin, origin) - 0.25;
    let discriminant = (b * b) - (a * c);
    if (discriminant < 0.0) {
        return miss;
    }
    let root = sqrt(discriminant);
    var t = (-b - root) / a;
    if (t < 0.0) {
        t = (-b + root) / a;
    }
    var point = origin + dir * t;
    if (slice >= 0.0) {
        point = vec3<f32>(point.x, point.y, slice / 2.0 - 0.5);
        if (length(point) > 0.5) {
            return miss;
        }
    }
    let normal = normalize(point);
    let offset_point = point - 1e-5 * normal;
    return EllipsoidHit(offset_point, normal, t, 1.0);
}

fn intersect_ellipsoid(
    origin: vec3<f32>,
    dir: vec3<f32>,
    center: vec3<f32>,
    scale: vec3<f32>,
    slice: f32,
) -> EllipsoidHit {
    let miss = EllipsoidHit(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, 0.0);
    if (scale.x <= 0.01 || scale.y < 0.01 || scale.z < 0.01) {
        return miss;
    }
    let o = (origin - center) / scale;
    let d = dir / scale;
    var h = intersect_sphere(o, d, slice);
    if (h.hit < 0.5) {
        return miss;
    }
    h.point = (h.point * scale) + center;
    return h;
}

// ─── Fragment stage (mirror CloudCollectionFS.glsl main + drawCloud) ─────────

@fragment
fn billboard_fragment(in: BillboardVertexOutput) -> @location(0) vec4<f32> {
    // Billboard-local ray setup — mirror CloudCollectionFS.glsl `main` (L236-256).
    let coordinate = in.maximum_size.xy * in.offset;
    let ellipsoid_scale = vec3<f32>(0.82) * in.maximum_size;
    let ellipsoid_center = vec3<f32>(0.0);
    let z_offset = max(ellipsoid_scale.z - 10.0, 0.0);
    let eye = vec3<f32>(0.0, 0.0, -10.0 - z_offset);
    let ray_dir = normalize(vec3<f32>(coordinate, 1.0) - eye);
    let ray_origin = eye;

    let light_dir = normalize(vec3<f32>(0.2, -1.0, 0.7));
    let h = intersect_ellipsoid(ray_origin, ray_dir, ellipsoid_center, ellipsoid_scale, in.slice);
    if (h.hit < 0.5) {
        // cloud.w < 0.01 ⇒ discard upstream; here output transparent (blended out).
        return vec4<f32>(0.0);
    }

    let id = clamp(dot(h.normal, -light_dir), 0.0, 1.0);
    let is = max(pow(dot(-light_dir, -ray_dir), 2.0), 0.0);
    let it = gardner_texture(h.point);
    let intensity = cloud_intensity(id, is, it);
    let shaded = intensity * clamp(in.brightness, 0.1, 1.0);

    let n = sample_noise(h.point * noise_meta.x);
    let w = n.x;
    let w2 = n.y;
    let w3 = n.z;
    let nd_dot = clamp(dot(h.normal, -ray_dir), 0.0, 1.0);
    var tr = pow(nd_dot, 3.0) - w;
    tr = tr * 1.3;
    let minus_dot = 0.5 - nd_dot;
    tr = tr - min(minus_dot * w2, 0.0);
    tr = tr - 0.8 * (minus_dot + 0.25) * w3;
    var shading = mix(1.0 - 0.8 * w * w, 1.0, id * tr);
    shading = clamp(shading + 0.2, 0.3, 1.0);

    let sc = shading * shaded;
    let fr = mix(vec3<f32>(0.5), vec3<f32>(sc), 1.15);
    let alpha = clamp(tr, 0.0, 1.0);
    // alpha < 0.01 ⇒ upstream discards; emulate with a fully transparent output.
    if (alpha < 0.01) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(fr, alpha) * in.color;
}
