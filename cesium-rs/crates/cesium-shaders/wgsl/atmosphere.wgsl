// Atmosphere shader: Fresnel-based atmospheric glow on a sphere slightly
// larger than the globe. Rendered into the globe's offscreen framebuffer
// so it composites naturally with the globe tiles.
//
// The Fresnel factor (1 - dot(N, V))^power creates a rim-light effect:
// transparent at the center (looking straight down) and bright at the
// limb (grazing angle), mimicking Rayleigh scattering.
//
// Binding contract:
//   group(0): CesiumAutomaticUniforms buffer
//   group(1): per-draw material resources (none needed; color is hardcoded)

struct CesiumAutomaticUniforms {
    czm_modelViewProjection: mat4x4<f32>,
    czm_modelView: mat4x4<f32>,
    czm_projection: mat4x4<f32>,
    czm_view: mat4x4<f32>,
    czm_model: mat4x4<f32>,
    czm_viewport: vec4<f32>,
    czm_sunDirectionWC: vec4<f32>,
};

@group(0) @binding(0) var<uniform> czm: CesiumAutomaticUniforms;

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_worldPos: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) a_position: vec4<f32>,
) -> VSOutput {
    var out: VSOutput;
    out.position = czm.czm_modelViewProjection * a_position;
    // Pass world-space position to the fragment shader for normal/view calc.
    out.v_worldPos = a_position.xyz;
    return out;
}

@fragment
fn fs_main(@location(0) v_worldPos: vec3<f32>) -> @location(0) vec4<f32> {
    // The camera is INSIDE the atmosphere sphere (radius = 1.02 × globe).
    // The geometric normal points outward, but the view direction points
    // inward (toward the origin). To get a meaningful Fresnel factor we
    // negate the normal so it points inward as well.
    let N = normalize(-v_worldPos);

    // Camera position in world space: extract from inverse view matrix.
    // For a standard view matrix: camera_world = -R^T * t.
    let viewRow0 = vec3<f32>(czm.czm_view[0][0], czm.czm_view[1][0], czm.czm_view[2][0]);
    let viewRow1 = vec3<f32>(czm.czm_view[0][1], czm.czm_view[1][1], czm.czm_view[2][1]);
    let viewRow2 = vec3<f32>(czm.czm_view[0][2], czm.czm_view[1][2], czm.czm_view[2][2]);
    let viewTrans = vec3<f32>(czm.czm_view[3][0], czm.czm_view[3][1], czm.czm_view[3][2]);
    let cameraPos = -(viewRow0 * viewTrans.x + viewRow1 * viewTrans.y + viewRow2 * viewTrans.z);

    // View direction: from fragment toward camera (world space).
    let V = normalize(cameraPos - v_worldPos);

    // Fresnel factor: 0 at center (N·V = 1, looking straight through),
    // 1 at limb (N·V = 0, grazing angle through the atmosphere shell).
    let ndv = clamp(dot(N, V), 0.0, 1.0);
    let intensity = pow(1.0 - ndv, 3.0);

    // Sky-blue atmosphere color.
    let atmosphereColor = vec3<f32>(0.3, 0.6, 1.0);
    let alpha = intensity * 0.6;

    return vec4<f32>(atmosphereColor * intensity, alpha);
}
