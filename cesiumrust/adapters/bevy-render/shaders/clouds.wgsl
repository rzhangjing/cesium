// M6.6: cesiumrust Clouds — screen-space raymarch / ellipsoid shading.
// ============================================================================
// Faithful port of the upstream CesiumJS cumulus-cloud fragment shader:
//   • packages/engine/Source/Shaders/CloudCollectionFS.glsl L1-263 (10537 B)
//       — wrap / voxelToUV / sampleNoiseTexture / intersectSphere /
//         intersectEllipsoid / phaseShift{2D,3D} / T (Gardner 1985) / I /
//         drawCloud / main. Every physical constant is preserved verbatim
//         (T0=0.6, k=0.1, C0=0.8, FX0=FY0=0.6, octaves=5, a=0.5, t=0.4, s=0.25,
//         lightDir=normalize(0.2,-1.0,0.7), ellipsoidScale=0.82·maximumSize).
//   • packages/engine/Source/Scene/CloudCollection.js (33 KB) +
//     CumulusCloud.js (12 KB) — the collection / billboard data model.
//   • domain/effects/src/cloud.rs `draw_cloud` / `intersect_ellipsoid` /
//     `gardner_texture` / `henyey_greenstein` / `beer_lambert` / `raymarch_density`
//     — the f64 CPU reference this shader mirrors (cross-validated by tests).
//   Blueprint (踩坑参考): cesium-rs/crates/cesium-shaders/shaders/CloudCollectionFS.glsl
//
// # TWO SHADING PATHS (chosen by `clouds.params.z`)
//   0 = FAITHFUL upstream `drawCloud`: a SINGLE ray/ellipsoid intersection shaded
//       with the Gardner sine texture + three Worley-FBM erosion channels. This is
//       exactly what upstream CesiumJS renders.
//   1 = ADDITIVE physically-based volumetric march: `RAYMARCH_STEPS` (8..16)
//       samples between the ellipsoid entry/exit, accumulating Beer-Lambert
//       extinction + Henyey-Greenstein in-scattering (g = 0.6). NOT upstream —
//       registered as DEVIATION dev-032 (draft). Both share the same 3D noise.
//
// # SPIKE PAYOFF — 3D TEXTURE SAMPLING
//   Upstream packs the 128³ volume into a 2D atlas and hand-rolls trilinear
//   (`voxelToUV` + `lerpSamplesX`, CloudCollectionFS.glsl L25-65). Because the
//   M6.6 SPIKE confirmed wgpu/naga 3D-texture support, this shader samples a real
//   `texture_3d` with a single hardware-trilinear `textureSampleLevel(…, vec3, lod)`
//   — the atlas index gymnastics collapse away. See `sample_noise` below.
//
// # RED LINES (hard constraints)
//   • METERS_PER_RENDER_UNIT = 6378137: `centers` / `scales` / `camera` arrive
//     ALREADY divided into render units by the adapter (CloudsUniform::from_domain)
//     so they match Bevy world space; the domain stays metric f64.
//   • NO `mod(` CALL — `mod` is a WGSL RESERVED WORD; `fract` / `%` are used.
//   • NO FMA CONTRACTION: `dot(n,p) + w`, the Gardner sum, and the march
//     accumulation each keep their multiply and add as two IEEE roundings.
//   • NO SWIZZLE ASSIGNMENT: every vector is rebuilt whole (naga rejects `v.xyz =`).
//   • glam fast-math disabled repo-wide; domain f64 → WGSL f32 at the uniform edge.
//   • GATE DEFAULT OFF ⇒ CloudsNode is never registered ⇒ this shader never runs ⇒
//     dynamic_globe v0 baselines stay pixel-neutral (PSNR = ∞). The `count <= 0`
//     early-out below also returns the source colour untouched.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

const MAX_CLOUDS: i32 = 8;
const CLOUD_PI: f32 = 3.14159265358979;

struct CloudsData {
    /// xyz = ellipsoid centre (RENDER UNITS), w = active flag (1 = shade, 0 = skip).
    centers: array<vec4<f32>, 8>,
    /// xyz = ellipsoid scale (RENDER UNITS, already 0.82·maximumSize), w = slice.
    scales: array<vec4<f32>, 8>,
    /// rgba cloud colour (v_color).
    colors: array<vec4<f32>, 8>,
    /// x = u_noiseDetail, y = raymarch steps, z = mode (0 faithful / 1 volumetric),
    /// w = Beer-Lambert extinction.
    params: vec4<f32>,
    /// xyz = camera world position (RENDER UNITS), w = brightness.
    camera: vec4<f32>,
    /// xyz = light direction, w = Henyey-Greenstein g.
    light: vec4<f32>,
    /// x = active cloud count, y = noise volume edge (128), zw = pad.
    /// (named `info`, not `meta` — `meta` is a WGSL reserved word.)
    info: vec4<f32>,
}

@group(0) @binding(0) var depth_prepass: texture_depth_2d;
@group(0) @binding(1) var noise_texture: texture_3d<f32>;
@group(0) @binding(2) var color_texture: texture_2d<f32>;
@group(0) @binding(3) var linear_sampler: sampler;
@group(0) @binding(4) var point_sampler: sampler;
@group(0) @binding(5) var<uniform> view: View;
@group(0) @binding(6) var<uniform> clouds: CloudsData;

// ─── Ray reconstruction ──────────────────────────────────────────────────────

fn world_ray_direction(uv: vec2<f32>) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0);
    let clip = vec4<f32>(ndc, 1.0, 1.0);
    let view_pos = view.view_from_clip * clip;
    let view_dir = normalize(view_pos.xyz / view_pos.w);
    return normalize((view.world_from_view * vec4<f32>(view_dir, 0.0)).xyz);
}

// ─── 3D noise sampling (SPIKE payoff: hardware trilinear on texture_3d) ───────

fn sample_noise(pos: vec3<f32>) -> vec4<f32> {
    let dim = clouds.info.y;
    // Recenter by +dim/2 (mirrors NoiseVolume::sample_trilinear) then wrap to the
    // [0,1] UVW cube; the sampler's repeat addressing makes the volume seamless.
    let uvw = fract((pos + vec3<f32>(dim * 0.5)) / dim);
    return textureSampleLevel(noise_texture, linear_sampler, uvw, 0.0);
}

// ─── Gardner (1985) texture + intensity (mirror CloudCollectionFS.glsl) ───────

fn phase_shift_2d(p: vec2<f32>, freq: vec2<f32>) -> vec2<f32> {
    let half_pi = 1.5707963267948966;
    return vec2<f32>(half_pi * sin(freq.y * p.y), half_pi * sin(freq.x * p.x));
}

fn phase_shift_3d(p: vec3<f32>, freq: vec2<f32>) -> vec2<f32> {
    let s = sin(freq.x * p.z);
    return phase_shift_2d(p.xy, freq) + vec2<f32>(CLOUD_PI * s, CLOUD_PI * s);
}

// Mirror of CloudCollectionFS.glsl `T` (L136-149).
fn gardner_texture(point: vec3<f32>) -> f32 {
    var sum = vec2<f32>(0.0, 0.0);
    var ci: f32 = 0.8;                 // C0
    var fxy = vec2<f32>(0.6, 0.6);     // FX0, FY0
    for (var i: i32 = 0; i < 5; i = i + 1) {   // octaves = 5
        let pxy = phase_shift_3d(point, fxy);
        ci = ci * 0.707;
        fxy = fxy * 2.0;
        let sin_term = vec2<f32>(
            sin(fxy.x * point.x + pxy.x),
            sin(fxy.y * point.y + pxy.y),
        );
        // NO FMA: the ci·sin multiply and the + T0 add stay separate roundings.
        sum = sum + ci * sin_term + vec2<f32>(0.6, 0.6);   // T0 = 0.6
    }
    return 0.1 * sum.x * sum.y;        // k = 0.1
}

// Mirror of CloudCollectionFS.glsl `I` (L155-157).
fn cloud_intensity(id: f32, is: f32, it: f32) -> f32 {
    let a: f32 = 0.5;
    let t: f32 = 0.4;
    let s: f32 = 0.25;
    return (1.0 - a) * ((1.0 - t) * ((1.0 - s) * id + s * is) + t * it) + a;
}

// ─── Ray / ellipsoid intersection (mirror CloudCollectionFS.glsl) ─────────────

struct EllipsoidHit {
    point: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    hit: f32,   // 1.0 = hit, 0.0 = miss (WGSL has no Option)
}

// Mirror of `intersectSphere` (L68-94): unit sphere radius 0.5 at the origin.
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
        // point.z = slice/2 - 0.5 (GLSL swizzle write → whole-vector rebuild).
        point = vec3<f32>(point.x, point.y, slice / 2.0 - 0.5);
        if (length(point) > 0.5) {
            return miss;
        }
    }
    let normal = normalize(point);
    let eps: f32 = 1e-5;   // czm_epsilon2
    let offset_point = point - eps * normal;
    return EllipsoidHit(offset_point, normal, t, 1.0);
}

// Mirror of `intersectEllipsoid` (L98-113). The normal stays in unit-sphere space.
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

// ─── FAITHFUL path: mirror of `drawCloud` (L161-230) ─────────────────────────

fn draw_cloud_faithful(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    center: vec3<f32>,
    scale: vec3<f32>,
    slice: f32,
    brightness: f32,
    color: vec4<f32>,
) -> vec4<f32> {
    let h = intersect_ellipsoid(ray_origin, ray_dir, center, scale, slice);
    if (h.hit < 0.5) {
        return vec4<f32>(0.0);
    }
    let light_dir = normalize(clouds.light.xyz);
    let id = clamp(dot(h.normal, -light_dir), 0.0, 1.0);       // diffuse
    let is = max(pow(dot(-light_dir, -ray_dir), 2.0), 0.0);    // specular
    let it = gardner_texture(h.point);                         // texture
    let intensity = cloud_intensity(id, is, it);
    let shaded = intensity * clamp(brightness, 0.1, 1.0);

    let n = sample_noise(h.point * clouds.params.x);
    let w = n.x;
    let w2 = n.y;
    let w3 = n.z;

    let nd_dot = clamp(dot(h.normal, -ray_dir), 0.0, 1.0);
    var tr = pow(nd_dot, 3.0) - w;   // translucency
    tr = tr * 1.3;
    let minus_dot = 0.5 - nd_dot;
    tr = tr - min(minus_dot * w2, 0.0);
    tr = tr - 0.8 * (minus_dot + 0.25) * w3;

    var shading = mix(1.0 - 0.8 * w * w, 1.0, id * tr);
    shading = clamp(shading + 0.2, 0.3, 1.0);

    // finalColor = mix(vec3(0.5), shading * color, 1.15); return vec4(finalColor, TR) * v_color
    let sc = shading * shaded;
    let fr = mix(vec3<f32>(0.5), vec3<f32>(sc), 1.15);
    let alpha = clamp(tr, 0.0, 1.0);
    return vec4<f32>(fr, alpha) * color;
}

// ─── ADDITIVE path: HG phase + Beer-Lambert volumetric march (dev-032) ────────

fn ellipsoid_interval(
    origin: vec3<f32>,
    dir: vec3<f32>,
    center: vec3<f32>,
    scale: vec3<f32>,
) -> vec2<f32> {
    if (scale.x <= 0.01 || scale.y < 0.01 || scale.z < 0.01) {
        return vec2<f32>(-1.0, -1.0);
    }
    let o = (origin - center) / scale;
    let d = dir / scale;
    let a = dot(d, d);
    let b = dot(o, d);
    let c = dot(o, o) - 0.25;
    let discriminant = (b * b) - (a * c);
    if (discriminant < 0.0) {
        return vec2<f32>(-1.0, -1.0);
    }
    let root = sqrt(discriminant);
    let t0 = max((-b - root) / a, 0.0);
    let t1 = (-b + root) / a;
    if (t1 <= t0) {
        return vec2<f32>(-1.0, -1.0);
    }
    return vec2<f32>(t0, t1);
}

fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    if (abs(denom) < 1e-12) {
        return 0.0;
    }
    return (1.0 - g2) / (4.0 * CLOUD_PI * pow(denom, 1.5));
}

fn beer_lambert(density: f32, extinction: f32, distance: f32) -> f32 {
    // NO FMA: density·extinction·distance is a single product, then negated + exp.
    return exp(-(density * extinction * distance));
}

fn raymarch_volumetric(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    center: vec3<f32>,
    scale: vec3<f32>,
    color: vec4<f32>,
) -> vec4<f32> {
    let interval = ellipsoid_interval(ray_origin, ray_dir, center, scale);
    if (interval.x < 0.0) {
        return vec4<f32>(0.0);
    }
    let steps = i32(clamp(clouds.params.y, 8.0, 16.0));
    let dt = (interval.y - interval.x) / f32(steps);
    let extinction = clouds.params.w;
    let g = clouds.light.w;
    let cos_theta = dot(normalize(ray_dir), normalize(clouds.light.xyz));
    let phase = henyey_greenstein(cos_theta, g);
    var transmittance: f32 = 1.0;
    var scattered: f32 = 0.0;
    for (var i: i32 = 0; i < steps; i = i + 1) {
        let t = interval.x + (f32(i) + 0.5) * dt;
        let world_point = ray_origin + ray_dir * t;
        let density = clamp(sample_noise(world_point * clouds.params.x).x, 0.0, 1.0);
        let step_t = beer_lambert(density, extinction, dt);
        // NO FMA: each term is a separate multiply, then a separate accumulate.
        scattered = scattered + transmittance * (1.0 - step_t) * phase;
        transmittance = transmittance * step_t;
    }
    let alpha = clamp(1.0 - transmittance, 0.0, 1.0);
    let rgb = color.rgb * scattered * clouds.camera.w;
    return vec4<f32>(rgb, alpha * color.a);
}

// ─── Fragment entry (screen-space cloud composite) ───────────────────────────

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(color_texture, linear_sampler, in.uv);
    let count = i32(clouds.info.x);
    // No clouds ⇒ pure pass-through (pixel-neutral; also the gate-OFF guarantee).
    if (count <= 0) {
        return source;
    }

    let cam_pos = view.world_from_view[3].xyz;
    let ray_dir = world_ray_direction(in.uv);

    // Depth occlusion: reconstruct the opaque scene world position so clouds behind
    // terrain are not composited over it (reversed-Z: sky = 0.0).
    let depth = textureSample(depth_prepass, point_sampler, in.uv);
    var scene_t = 1e9;
    if (depth > 1e-6) {
        let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, (1.0 - in.uv.y) * 2.0 - 1.0);
        let clip = vec4<f32>(ndc, depth, 1.0);
        let view_pos = view.view_from_clip * clip;
        let world_pos = (view.world_from_view * vec4<f32>(view_pos.xyz / view_pos.w, 1.0)).xyz;
        scene_t = length(world_pos - cam_pos);
    }

    var accum = vec3<f32>(0.0);
    var transmittance: f32 = 1.0;
    for (var i: i32 = 0; i < count; i = i + MAX_CLOUDS - MAX_CLOUDS + 1) {
        if (clouds.centers[i].w < 0.5) {
            continue;
        }
        let center = clouds.centers[i].xyz;
        // Skip clouds entirely behind the opaque scene along this ray.
        if (length(center - cam_pos) > scene_t) {
            continue;
        }
        let scale = clouds.scales[i].xyz;
        let slice = clouds.scales[i].w;
        let color = clouds.colors[i];
        var cloud_rgba: vec4<f32>;
        if (clouds.params.z < 0.5) {
            cloud_rgba = draw_cloud_faithful(
                cam_pos, ray_dir, center, scale, slice, clouds.camera.w, color,
            );
        } else {
            cloud_rgba = raymarch_volumetric(cam_pos, ray_dir, center, scale, color);
        }
        // Over-composite (front-to-back; #81 sorts the uniform array by depth).
        accum = accum + transmittance * cloud_rgba.a * cloud_rgba.rgb;
        transmittance = transmittance * (1.0 - cloud_rgba.a);
    }

    let final_rgb = source.rgb * transmittance + accum;
    return vec4<f32>(final_rgb, source.a);
}
