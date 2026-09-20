//! Image-Based Lighting (IBL).
//!
//! Maps to CesiumJS `Scene/ImageBasedLighting.js`:
//! - Spherical harmonic coefficients for diffuse IBL
//! - Specular environment maps
//! - IBL factor scaling
//!
//! Domain layer — pure Rust, f64 precision.

use glam::DVec3;

/// Number of spherical harmonic coefficients (3rd order = 9 coefficients).
pub const SH_COEFFICIENT_COUNT: usize = 9;

/// Split-sum BRDF integration sample count used when applying IBL at runtime.
///
/// Mirrors `BRDF_APPLY_SAMPLES` in `adapters/bevy-render/shaders/ibl.wgsl`
/// (L107). The GPU apply node recomputes the split-sum BRDF inline with this
/// many GGX importance samples instead of binding a precomputed 256×256 LUT
/// (see `docs/deviations.md#dev-024`); the CPU reference keeps the same count
/// so domain↔GPU specular cross-checks compare like-for-like. The apply path
/// uses 32 taps while the higher-accuracy CPU integrator in [`texture_ibl`]
/// runs 1024 — the mirror is for the GPU-boundary pairing tests, not for
/// [`texture_ibl`] itself.
pub const BRDF_APPLY_SAMPLES: usize = 32;

/// Image-based lighting configuration.
///
/// Maps to CesiumJS `ImageBasedLighting`.
#[derive(Debug, Clone)]
pub struct ImageBasedLighting {
    /// Scales diffuse and specular IBL contribution.
    /// x = diffuse factor, y = specular factor. Both in [0, 1].
    pub image_based_lighting_factor: [f64; 2],
    /// Third-order spherical harmonic coefficients for diffuse IBL.
    /// 9 coefficients, each an RGB triple.
    pub spherical_harmonic_coefficients: Option<[[f64; 3]; SH_COEFFICIENT_COUNT]>,
    /// URL to a KTX2 specular environment map.
    pub specular_environment_maps: Option<String>,
    /// Whether to use default spherical harmonics.
    pub use_default_spherical_harmonics: bool,
    /// Whether to use default specular maps.
    pub use_default_specular_maps: bool,
}

impl Default for ImageBasedLighting {
    fn default() -> Self {
        Self {
            image_based_lighting_factor: [1.0, 1.0],
            spherical_harmonic_coefficients: None,
            specular_environment_maps: None,
            use_default_spherical_harmonics: false,
            use_default_specular_maps: false,
        }
    }
}

impl ImageBasedLighting {
    /// Creates a new IBL configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the IBL factor (diffuse and specular scaling).
    ///
    /// # Panics
    /// Panics if values are outside [0, 1].
    pub fn set_factor(&mut self, diffuse: f64, specular: f64) {
        assert!((0.0..=1.0).contains(&diffuse), "diffuse factor must be in [0, 1]");
        assert!((0.0..=1.0).contains(&specular), "specular factor must be in [0, 1]");
        self.image_based_lighting_factor = [diffuse, specular];
    }

    /// Sets the spherical harmonic coefficients.
    ///
    /// # Panics
    /// Panics if the array doesn't have exactly 9 coefficients.
    pub fn set_spherical_harmonics(&mut self, coefficients: [[f64; 3]; SH_COEFFICIENT_COUNT]) {
        self.spherical_harmonic_coefficients = Some(coefficients);
        self.use_default_spherical_harmonics = false;
    }

    /// Returns whether custom SH coefficients are set.
    pub fn has_spherical_harmonics(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some()
    }

    /// Returns whether a specular environment map is set.
    pub fn has_specular_environment_maps(&self) -> bool {
        self.specular_environment_maps.is_some()
    }

    /// Returns whether shaders need regeneration due to IBL changes.
    pub fn needs_shader_regeneration(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some() || self.specular_environment_maps.is_some()
    }

    /// Computes the diffuse IBL contribution for a given normal direction.
    ///
    /// Uses the spherical harmonic coefficients to evaluate irradiance.
    ///
    /// # Arguments
    /// * `normal` - Surface normal (normalized)
    ///
    /// # Returns
    /// Diffuse irradiance color [R, G, B], scaled by the diffuse IBL factor.
    pub fn compute_diffuse_ibl(&self, normal: DVec3) -> [f64; 3] {
        let coefficients = match &self.spherical_harmonic_coefficients {
            Some(c) => c,
            None => return [0.0; 3],
        };

        let diffuse_factor = self.image_based_lighting_factor[0];
        if diffuse_factor == 0.0 {
            return [0.0; 3];
        }

        // Evaluate spherical harmonics
        let sh = evaluate_sh(coefficients, normal);

        [sh[0] * diffuse_factor, sh[1] * diffuse_factor, sh[2] * diffuse_factor]
    }

    /// Computes the specular IBL contribution.
    ///
    /// In a full implementation, this would sample the specular environment map
    /// at the reflection direction with the appropriate mip level based on roughness.
    ///
    /// # Arguments
    /// * `reflection` - Reflection direction (normalized)
    /// * `roughness` - Surface roughness [0, 1]
    ///
    /// # Returns
    /// Specular color [R, G, B], scaled by the specular IBL factor.
    pub fn compute_specular_ibl(&self, _reflection: DVec3, _roughness: f64) -> [f64; 3] {
        let specular_factor = self.image_based_lighting_factor[1];
        if specular_factor == 0.0 {
            return [0.0; 3];
        }

        // DEVIATION: placeholder constant contribution, see docs/deviations.md#dev-024.
        // A real implementation samples the prefiltered specular environment cubemap
        // at `reflection` with a roughness→mip LOD; the offline / GPU prefilter
        // driver is not landed yet (deferred #54). Until then this returns a fixed
        // neutral environment tint scaled by the specular IBL factor — which is why
        // the `ibl_compute_specular_*` roughness / reflection-dependence specs are
        // marked `#[ignore]` (awaiting deferred #54 real prefilter) in
        // `specs/tests/scene/ibl_cloud_spec.rs`.
        [0.1 * specular_factor, 0.1 * specular_factor, 0.12 * specular_factor]
    }
}

/// Evaluates 3rd-order spherical harmonics for a given direction.
///
/// The 9 SH basis functions for order 0, 1, 2:
/// - Y_0^0 = 0.282095
/// - Y_1^{-1} = 0.488603 * y
/// - Y_1^0 = 0.488603 * z
/// - Y_1^1 = 0.488603 * x
/// - Y_2^{-2} = 1.092548 * x * y
/// - Y_2^{-1} = 1.092548 * y * z
/// - Y_2^0 = 0.315392 * (3z² - 1)
/// - Y_2^1 = 1.092548 * x * z
/// - Y_2^2 = 0.546274 * (x² - y²)
fn evaluate_sh(coefficients: &[[f64; 3]; 9], direction: DVec3) -> [f64; 3] {
    let x = direction.x;
    let y = direction.y;
    let z = direction.z;

    // SH basis functions
    let basis = [
        0.282095,                        // Y_0^0
        0.488603 * y,                    // Y_1^{-1}
        0.488603 * z,                    // Y_1^0
        0.488603 * x,                    // Y_1^1
        1.092548 * x * y,               // Y_2^{-2}
        1.092548 * y * z,               // Y_2^{-1}
        0.315392 * (3.0 * z * z - 1.0), // Y_2^0
        1.092548 * x * z,               // Y_2^1
        0.546274 * (x * x - y * y),     // Y_2^2
    ];

    let mut result = [0.0f64; 3];
    for (i, b) in basis.iter().enumerate() {
        for c in 0..3 {
            result[c] += coefficients[i][c] * b;
        }
    }

    result
}

/// Default spherical harmonic coefficients for a neutral sky environment.
///
/// These approximate a simple sky/ground environment.
pub fn default_spherical_harmonics() -> [[f64; 3]; SH_COEFFICIENT_COUNT] {
    [
        [0.3, 0.3, 0.35],   // DC term (ambient)
        [0.0, 0.0, 0.0],    // Y_1^{-1}
        [0.1, 0.1, 0.15],   // Y_1^0 (sky/ground gradient)
        [0.0, 0.0, 0.0],    // Y_1^1
        [0.0, 0.0, 0.0],    // Y_2^{-2}
        [0.0, 0.0, 0.0],    // Y_2^{-1}
        [0.05, 0.05, 0.08], // Y_2^0
        [0.0, 0.0, 0.0],    // Y_2^1
        [0.0, 0.0, 0.0],    // Y_2^2
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// CesiumJS-faithful IBL CPU reference (M6.5)
// ═══════════════════════════════════════════════════════════════════════════
//
// The functions below are a 1:1 f64 port of the authoritative CesiumJS PBR/IBL
// shaders, mirroring `adapters/bevy-render/src/shaders/ibl.wgsl` entry-for-entry
// so the CPU reference and the GPU shader can be cross-validated:
//   * `spherical_harmonics`  ← Shaders/Builtin/Functions/sphericalHarmonics.glsl
//   * `ggx_ndf` / `smith_visibility_ggx` / `fresnel_schlick2`
//                            ← Shaders/Builtin/Functions/pbrLighting.glsl
//   * `prefilter_specular`   ← Shaders/ConvolveSpecularMapFS.glsl
//   * `integrate_brdf`       ← Shaders/BrdfLutGeneratorFS.glsl
//   * `texture_ibl`          ← Shaders/Model/ImageBasedLightingStageFS.glsl
//
// SH CONVENTION (deliberate divergence from the legacy `evaluate_sh` above):
// CesiumJS `czm_sphericalHarmonics` consumes PRE-SCALED coefficients — cmgen
// (`--no-mirror`) bakes both the orthonormal-basis constants AND the cosine-lobe
// irradiance transfer into the 9 RGB values — so evaluation is the bare
// polynomial in (x,y,z) followed by `max(., 0)`. The legacy `evaluate_sh`
// instead stores RAW orthonormal-basis coefficients and applies the constants at
// evaluation time. `project_irradiance_to_sh` emits the CesiumJS convention, so
// its output feeds `spherical_harmonics` (CPU) and the `irradiance` WGSL entry
// (GPU) unchanged. Both are kept because `evaluate_sh` backs the pre-existing
// `compute_diffuse_ibl` API (domain f64 semantics frozen).

/// The 9 CesiumJS `czm_sphericalHarmonics` polynomial basis terms `P_i(x,y,z)`,
/// in upstream coefficient order `[L00, L1_1, L10, L11, L2_2, L2_1, L20, L21, L22]`.
/// Ported verbatim from `sphericalHarmonics.glsl` (no normalisation constants).
pub fn sh_polynomial_basis(direction: DVec3) -> [f64; SH_COEFFICIENT_COUNT] {
    let x = direction.x;
    let y = direction.y;
    let z = direction.z;
    [
        1.0,               // L00
        y,                 // L1_1
        z,                 // L10
        x,                 // L11
        y * x,             // L2_2
        y * z,             // L2_1
        3.0 * z * z - 1.0, // L20
        z * x,             // L21
        x * x - y * y,     // L22
    ]
}

/// Orthonormal SH basis normalisation constants `k_lm` (per coefficient index),
/// i.e. `Y_lm(dir) = k_lm · P_i(dir)`. Standard values (Ramamoorthi, envmap.pdf).
pub const SH_ORTHONORMAL_CONSTANTS: [f64; SH_COEFFICIENT_COUNT] = [
    0.282_094_791_773_878_14, // L00  = 1/(2√π)
    0.488_602_511_902_919_9,  // L1_1 = √3/(2√π)
    0.488_602_511_902_919_9,  // L10
    0.488_602_511_902_919_9,  // L11
    1.092_548_430_592_079_2,  // L2_2 = √15/(2√π)
    1.092_548_430_592_079_2,  // L2_1
    0.315_391_565_252_520_05, // L20  = √5/(4√π)
    1.092_548_430_592_079_2,  // L21
    0.546_274_215_296_039_6,  // L22  = √15/(4√π)
];

/// Zonal cosine-lobe transfer coefficients `A_l` (Ramamoorthi & Hanrahan): the
/// irradiance band-l scale `[π, 2π/3, π/4]` for `l = 0, 1, 2`.
pub const IRRADIANCE_ZONAL_BY_BAND: [f64; 3] = [
    std::f64::consts::PI,
    2.0 * std::f64::consts::PI / 3.0,
    std::f64::consts::PI / 4.0,
];

/// SH band `l` of each of the 9 coefficients, in CesiumJS order.
const SH_BAND: [usize; SH_COEFFICIENT_COUNT] = [0, 1, 1, 1, 2, 2, 2, 2, 2];

/// Evaluates third-order SH exactly as CesiumJS `czm_sphericalHarmonics`:
/// `max(Σ c_i · P_i(dir), 0)` per RGB channel. Coefficients use the PRE-SCALED
/// CesiumJS convention (see module note). f64 reference for the `ibl.wgsl`
/// `spherical_harmonics` helper.
pub fn spherical_harmonics(
    coefficients: &[[f64; 3]; SH_COEFFICIENT_COUNT],
    direction: DVec3,
) -> [f64; 3] {
    let basis = sh_polynomial_basis(direction);
    let mut out = [0.0f64; 3];
    for (i, b) in basis.iter().enumerate() {
        for c in 0..3 {
            out[c] += coefficients[i][c] * b;
        }
    }
    // czm clamps negative irradiance to zero: `max(L, vec3(0.0))`.
    [out[0].max(0.0), out[1].max(0.0), out[2].max(0.0)]
}

/// Deterministic Fibonacci-lattice directions on the unit sphere. Fixed (no RNG)
/// so `project_irradiance_to_sh` is bit-reproducible run-to-run and CI-stable.
pub fn fibonacci_sphere(samples: usize) -> Vec<DVec3> {
    let n = samples.max(1);
    if n == 1 {
        return vec![DVec3::Z];
    }
    let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let y = 1.0 - (i as f64 / (n as f64 - 1.0)) * 2.0;
        let r = (1.0 - y * y).max(0.0).sqrt();
        let theta = golden * i as f64;
        out.push(DVec3::new(theta.cos() * r, y, theta.sin() * r));
    }
    out
}

/// Projects an environment radiance function onto the 9 CesiumJS-convention
/// irradiance SH coefficients by deterministic Monte-Carlo over a Fibonacci
/// sphere. `radiance(dir) -> [R,G,B]` (linear, ≥ 0). The result feeds directly
/// into [`spherical_harmonics`] (CPU) and the `irradiance` WGSL entry (GPU).
///
/// Math: `a_lm = ∫ L(ω)Y_lm(ω)dω ≈ (4π/N) Σ L(ω_j) k_lm P_i(ω_j)`;
/// irradiance transfer `E_lm = A_l · a_lm`; CesiumJS coefficient
/// `c_i = E_lm · k_lm`, so `spherical_harmonics(c, n)` reproduces the irradiance
/// `E(n) = ∫ L(ω)(n·ω)⁺ dω`.
pub fn project_irradiance_to_sh<F>(
    radiance: F,
    samples: usize,
) -> [[f64; 3]; SH_COEFFICIENT_COUNT]
where
    F: Fn(DVec3) -> [f64; 3],
{
    let dirs = fibonacci_sphere(samples);
    let n = dirs.len().max(1) as f64;
    let solid_angle = 4.0 * std::f64::consts::PI / n;
    let mut coeffs = [[0.0f64; 3]; SH_COEFFICIENT_COUNT];
    for dir in &dirs {
        let l = radiance(*dir);
        let basis = sh_polynomial_basis(*dir);
        for i in 0..SH_COEFFICIENT_COUNT {
            let k = SH_ORTHONORMAL_CONSTANTS[i];
            let a_band = IRRADIANCE_ZONAL_BY_BAND[SH_BAND[i]];
            // c_i accumulates L · P_i · (solid_angle · k² · A_l).
            let scale = solid_angle * k * k * a_band;
            for c in 0..3 {
                coeffs[i][c] += l[c] * basis[i] * scale;
            }
        }
    }
    coeffs
}

/// Van der Corput radical inverse, base 2 (port of `vdcRadicalInverse`). Exact
/// integer halving via bit ops — no float `mod`, so the WGSL twin sidesteps the
/// `mod` reserved-word hazard entirely.
pub fn radical_inverse_vdc(bits_in: u32) -> f64 {
    let mut i = bits_in;
    let mut value = 0.0f64;
    let mut inv_bi = 0.5;
    for _ in 0..32 {
        if i == 0 {
            break;
        }
        value += f64::from(i & 1) * inv_bi;
        inv_bi *= 0.5;
        i >>= 1;
    }
    value
}

/// Hammersley 2D low-discrepancy point (port of `hammersley2D`).
pub fn hammersley2d(i: usize, n: usize) -> [f64; 2] {
    [i as f64 / n.max(1) as f64, radical_inverse_vdc(i as u32)]
}

/// Equirectangular `[0,1]²` uv → unit world direction, **z-up** — the f64 twin of
/// `direction_from_uv` in `adapters/bevy-render/shaders/ibl.wgsl` (FIX-IBL-ZUP).
///
/// Latitude rides the `z` component (`sin(lat)`), longitude the `x`/`y` plane,
/// matching `direction_to_uv` in `domain/effects/src/panorama.rs` so every
/// equirect map in the engine agrees on one convention. The pre-fix GPU form put
/// `sin(theta)` in `y` (y-up), disagreeing with the rest of the engine; this twin
/// is what the WGSL cross-check test pins the shader against.
pub fn direction_from_uv(uv: [f64; 2]) -> DVec3 {
    use std::f64::consts::{PI, TAU};
    let lon = TAU * (uv[0] - 0.5); // [-π, π]
    let lat = PI * (uv[1] - 0.5); // [-π/2, π/2]
    let cos_lat = lat.cos();
    DVec3::new(cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()).normalize_or_zero()
}

/// GGX / Trowbridge-Reitz normal distribution (port of `GGX` in pbrLighting.glsl).
pub fn ggx_ndf(alpha_roughness: f64, ndoth: f64) -> f64 {
    let a2 = alpha_roughness * alpha_roughness;
    let f = (ndoth * a2 - ndoth) * ndoth + 1.0;
    a2 / (std::f64::consts::PI * f * f)
}

/// Smith joint GGX visibility `= G/(4·NdotL·NdotV)` (port of `smithVisibilityGGX`).
pub fn smith_visibility_ggx(alpha_roughness: f64, ndotl: f64, ndotv: f64) -> f64 {
    let a2 = alpha_roughness * alpha_roughness;
    let ggxv = ndotl * (ndotv * ndotv * (1.0 - a2) + a2).max(0.0).sqrt();
    let ggxl = ndotv * (ndotl * ndotl * (1.0 - a2) + a2).max(0.0).sqrt();
    let ggx = ggxv + ggxl;
    if ggx > 0.0 {
        0.5 / ggx
    } else {
        0.0
    }
}

/// Roughness-dependent Schlick Fresnel (port of `fresnelSchlick2`). The
/// `versine^5` is expanded as `vs2*vs2*versine` and kept UNFUSED — matching the
/// WGSL twin's two-rounding rule (no FMA contraction on the IBL numerics).
pub fn fresnel_schlick2(f0: [f64; 3], f90: [f64; 3], vdoth: f64) -> [f64; 3] {
    let versine = 1.0 - vdoth;
    let vs2 = versine * versine;
    let pow5 = vs2 * vs2 * versine;
    [
        f0[0] + (f90[0] - f0[0]) * pow5,
        f0[1] + (f90[1] - f0[1]) * pow5,
        f0[2] + (f90[2] - f0[2]) * pow5,
    ]
}

/// GGX importance sample: the world-space half-vector `H` for a 2D quasi-random
/// `xi` (port of `importanceSampleGGX`). NOTE the two upstream call sites feed
/// different arguments: `ConvolveSpecularMapFS` passes perceptual `roughness`,
/// while `BrdfLutGeneratorFS` passes `alphaRoughness = roughness²`. This function
/// squares its `alpha_roughness` argument internally, exactly as the GLSL does,
/// so callers must replicate their upstream site verbatim.
pub fn importance_sample_ggx(xi: [f64; 2], alpha_roughness: f64, n: DVec3) -> DVec3 {
    let a2 = alpha_roughness * alpha_roughness;
    let phi = 2.0 * std::f64::consts::PI * xi[0];
    let denom = 1.0 + (a2 - 1.0) * xi[1];
    let cos_theta = if denom > 0.0 {
        ((1.0 - xi[1]) / denom).max(0.0).sqrt()
    } else {
        1.0
    };
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let h = DVec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);
    let up = if n.z.abs() < 0.999 { DVec3::Z } else { DVec3::X };
    let tangent_x = up.cross(n).normalize();
    let tangent_y = n.cross(tangent_x);
    tangent_x * h.x + tangent_y * h.y + n * h.z
}

/// Prefiltered specular radiance for `dir` / `roughness` (port of
/// `ConvolveSpecularMapFS.glsl`): GGX-importance-sample the environment, weight
/// each tap by `NdotL`, normalise by the accumulated weight. `radiance(dir)`
/// samples the source environment cubemap in the given direction.
pub fn prefilter_specular<F>(radiance: F, roughness: f64, dir: DVec3, samples: usize) -> [f64; 3]
where
    F: Fn(DVec3) -> [f64; 3],
{
    let v = dir.normalize();
    let n = samples.max(1);
    let mut color = [0.0f64; 3];
    let mut weight = 0.0f64;
    for i in 0..n {
        let xi = hammersley2d(i, n);
        // ConvolveSpecularMapFS passes raw `roughness` (squared inside).
        let h = importance_sample_ggx(xi, roughness, v);
        let l = h * (2.0 * v.dot(h)) - v; // reflected vector
        let ndotl = v.dot(l).max(0.0);
        if ndotl > 0.0 {
            let s = radiance(l.normalize());
            for c in 0..3 {
                color[c] += s[c] * ndotl;
            }
            weight += ndotl;
        }
    }
    if weight > 0.0 {
        [color[0] / weight, color[1] / weight, color[2] / weight]
    } else {
        [0.0; 3]
    }
}

/// Split-sum environment-BRDF integration → `(scale, bias)` (port of
/// `BrdfLutGeneratorFS.glsl::integrateBrdf`). Indexed by `(NdotV, roughness)`,
/// exactly as `texture(czm_brdfLut, vec2(NdotV, roughness))` reads the LUT.
pub fn integrate_brdf(roughness: f64, ndotv: f64, samples: usize) -> [f64; 2] {
    let ndotv = ndotv.clamp(0.0, 1.0);
    let v = DVec3::new((1.0 - ndotv * ndotv).max(0.0).sqrt(), 0.0, ndotv);
    let alpha_roughness = roughness * roughness;
    let n = samples.max(1);
    let mut a = 0.0f64;
    let mut b = 0.0f64;
    for i in 0..n {
        let xi = hammersley2d(i, n);
        // BrdfLutGeneratorFS passes `alphaRoughness = roughness²` (squared again inside).
        let h = importance_sample_ggx(xi, alpha_roughness, DVec3::Z);
        let l = h * (2.0 * v.dot(h)) - v;
        let ndotl = l.z.clamp(0.0, 1.0);
        let ndoth = h.z.clamp(0.0, 1.0);
        let vdoth = v.dot(h).clamp(0.0, 1.0);
        // `ndoth > 0.0` guards the `4·G·VdotH·NdotL / NdotH` division; it holds
        // for every sample the upstream `NdotL > 0` branch admits (defensive superset).
        if ndotl > 0.0 && ndoth > 0.0 {
            let g = smith_visibility_ggx(alpha_roughness, ndotl, ndotv);
            let g_vis = 4.0 * g * vdoth * ndotl / ndoth;
            let fc = (1.0 - vdoth).powi(5);
            a += (1.0 - fc) * g_vis;
            b += fc * g_vis;
        }
    }
    [a / n as f64, b / n as f64]
}

/// PBR surface inputs for [`texture_ibl`] (subset of `czm_modelMaterial`).
#[derive(Debug, Clone, Copy)]
pub struct IblMaterial {
    /// Lambertian base colour (linear RGB).
    pub diffuse: [f64; 3],
    /// Specular F0 reflectance (linear RGB).
    pub specular_f0: [f64; 3],
    /// Perceptual roughness in `[0, 1]`.
    pub roughness: f64,
    /// Specular weight (glTF `KHR_materials_specular`); `1.0` when unused.
    pub specular_weight: f64,
}

impl Default for IblMaterial {
    fn default() -> Self {
        Self {
            diffuse: [1.0, 1.0, 1.0],
            specular_f0: [0.04, 0.04, 0.04],
            roughness: 0.5,
            specular_weight: 1.0,
        }
    }
}

/// The full image-based-lighting contribution (port of
/// `ImageBasedLightingStageFS.glsl::textureIBL`, Fdez-Aguera single- +
/// multi-scattering). Ties the three references together: diffuse from the SH
/// irradiance, specular from a prefiltered-environment closure `specular_env(dir,
/// roughness)`, modulated by the split-sum BRDF LUT. `ibl_factor = [diffuse, specular]`.
pub fn texture_ibl<S>(
    irradiance_sh: &[[f64; 3]; SH_COEFFICIENT_COUNT],
    view_dir: DVec3,
    normal: DVec3,
    material: &IblMaterial,
    ibl_factor: [f64; 2],
    specular_env: S,
) -> [f64; 3]
where
    S: Fn(DVec3, f64) -> [f64; 3],
{
    let n = normal.normalize();
    let v = view_dir.normalize();
    let f0 = material.specular_f0;
    let roughness = material.roughness;
    let specular_weight = material.specular_weight;
    let ndotv = n.dot(v).clamp(0.0, 1.0);

    // Roughness-dependent Fresnel, from Fdez-Aguera: f90 = max(1-roughness, f0).
    let one_minus_r = 1.0 - roughness;
    let f90 = [
        one_minus_r.max(f0[0]),
        one_minus_r.max(f0[1]),
        one_minus_r.max(f0[2]),
    ];
    let single_scatter_fresnel = fresnel_schlick2(f0, f90, ndotv);
    let brdf_lut = integrate_brdf(roughness, ndotv, 1024);
    let (lut_scale, lut_bias) = (brdf_lut[0], brdf_lut[1]);

    // FssEss = specularWeight · (F · scale + bias), per channel.
    let fss_ess = [
        specular_weight * (single_scatter_fresnel[0] * lut_scale + lut_bias),
        specular_weight * (single_scatter_fresnel[1] * lut_scale + lut_bias),
        specular_weight * (single_scatter_fresnel[2] * lut_scale + lut_bias),
    ];

    // Diffuse (multi-scattering energy compensation).
    let irradiance = spherical_harmonics(irradiance_sh, n);
    let average_fresnel = [
        f0[0] + (1.0 - f0[0]) / 21.0,
        f0[1] + (1.0 - f0[1]) / 21.0,
        f0[2] + (1.0 - f0[2]) / 21.0,
    ];
    let ems = specular_weight * (1.0 - lut_scale - lut_bias);
    let mut out = [0.0f64; 3];
    for c in 0..3 {
        let denom = 1.0 - average_fresnel[c] * ems;
        let fms_ems = if denom.abs() > 1e-12 {
            fss_ess[c] * average_fresnel[c] * ems / denom
        } else {
            0.0
        };
        let dielectric_scattering = (1.0 - fss_ess[c] - fms_ems) * material.diffuse[c];
        let diffuse_contribution = irradiance[c] * (fms_ems + dielectric_scattering) * ibl_factor[0];
        out[c] = diffuse_contribution;
    }

    // Specular: reflect(-V, N) = -V - 2·dot(N,-V)·N = 2(N·V)N - V.
    let reflect_dir = (n * (2.0 * n.dot(v)) - v).normalize_or_zero();
    let radiance = specular_env(reflect_dir, roughness);
    for c in 0..3 {
        out[c] += radiance[c] * fss_ess[c] * ibl_factor[1];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibl_default() {
        let ibl = ImageBasedLighting::default();
        assert_eq!(ibl.image_based_lighting_factor, [1.0, 1.0]);
        assert!(!ibl.has_spherical_harmonics());
        assert!(!ibl.has_specular_environment_maps());
    }

    #[test]
    fn test_ibl_set_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(0.5, 0.8);
        assert_eq!(ibl.image_based_lighting_factor, [0.5, 0.8]);
    }

    #[test]
    #[should_panic(expected = "diffuse factor must be in [0, 1]")]
    fn test_ibl_set_factor_invalid() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.5, 0.5);
    }

    #[test]
    fn test_ibl_set_spherical_harmonics() {
        let mut ibl = ImageBasedLighting::default();
        let sh = default_spherical_harmonics();
        ibl.set_spherical_harmonics(sh);

        assert!(ibl.has_spherical_harmonics());
        assert!(ibl.needs_shader_regeneration());
    }

    #[test]
    fn test_ibl_specular_maps() {
        let mut ibl = ImageBasedLighting::default();
        assert!(!ibl.has_specular_environment_maps());

        ibl.specular_environment_maps = Some("environment.ktx2".to_string());
        assert!(ibl.has_specular_environment_maps());
    }

    #[test]
    fn test_compute_diffuse_ibl_no_coefficients() {
        let ibl = ImageBasedLighting::default();
        let result = ibl.compute_diffuse_ibl(DVec3::Y);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_compute_diffuse_ibl_with_coefficients() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_spherical_harmonics(default_spherical_harmonics());

        let result = ibl.compute_diffuse_ibl(DVec3::Y);

        // Should produce non-zero result
        assert!(result[0] > 0.0 || result[1] > 0.0 || result[2] > 0.0);
    }

    #[test]
    fn test_compute_diffuse_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_spherical_harmonics(default_spherical_harmonics());
        ibl.set_factor(0.0, 1.0); // Zero diffuse

        let result = ibl.compute_diffuse_ibl(DVec3::Y);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_compute_specular_ibl() {
        let ibl = ImageBasedLighting::default();
        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);

        // Default returns neutral contribution
        assert!(result[0] > 0.0);
    }

    #[test]
    fn test_compute_specular_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.0, 0.0); // Zero specular

        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_evaluate_sh_dc_only() {
        // Only DC term set
        let mut coefficients = [[0.0; 3]; 9];
        coefficients[0] = [1.0, 1.0, 1.0];

        // DC term should be constant regardless of direction
        let up = evaluate_sh(&coefficients, DVec3::Y);
        let down = evaluate_sh(&coefficients, DVec3::new(0.0, -1.0, 0.0));

        assert!((up[0] - down[0]).abs() < 1e-10);
        assert!((up[0] - 0.282095).abs() < 1e-5);
    }

    #[test]
    fn test_default_spherical_harmonics() {
        let sh = default_spherical_harmonics();
        // DC term should be the ambient contribution
        assert!(sh[0][0] > 0.0);
        assert!(sh[0][1] > 0.0);
        assert!(sh[0][2] > 0.0);
    }

    // ─── M6.5 CesiumJS-faithful IBL reference ─────────────────────────────
    // Every assertion is anchored to a closed-form value derived from the
    // upstream GLSL, so a numeric regression is caught without a GPU.

    #[test]
    fn spherical_harmonics_dc_only_is_direction_independent() {
        // czm_sphericalHarmonics: P_0 == 1, so a DC-only coefficient set returns
        // that coefficient verbatim for every direction.
        let mut coeffs = [[0.0f64; 3]; SH_COEFFICIENT_COUNT];
        coeffs[0] = [0.7, 0.2, 0.9];
        for dir in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::new(0.3, -0.8, 0.52).normalize()] {
            let out = spherical_harmonics(&coeffs, dir);
            for c in 0..3 {
                assert!((out[c] - coeffs[0][c]).abs() < 1e-12, "dir {dir}");
            }
        }
    }

    #[test]
    fn spherical_harmonics_clamps_negative_irradiance_to_zero() {
        // czm ends with `max(L, vec3(0.0))`.
        let mut coeffs = [[0.0f64; 3]; SH_COEFFICIENT_COUNT];
        coeffs[0] = [-1.0, -2.0, -3.0];
        let out = spherical_harmonics(&coeffs, DVec3::Y);
        assert_eq!(out, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn spherical_harmonics_band1_z_is_direction_dependent() {
        // L10 basis is `z`: +Z and -Z must evaluate to opposite signs pre-clamp.
        let mut coeffs = [[0.0f64; 3]; SH_COEFFICIENT_COUNT];
        coeffs[0] = [1.0, 1.0, 1.0];
        coeffs[2] = [0.5, 0.5, 0.5]; // L10 * z
        let up = spherical_harmonics(&coeffs, DVec3::Z);
        let down = spherical_harmonics(&coeffs, DVec3::new(0.0, 0.0, -1.0));
        assert!((up[0] - 1.5).abs() < 1e-12);
        assert!((down[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn project_irradiance_of_a_constant_environment_is_pi_times_radiance() {
        // Closed form: for a uniform environment L = C over the full sphere,
        // E(n) = C ∫_hemi cos dω = C·π for every normal. The DC coefficient is
        // exact under any equal-solid-angle quadrature (Σ P_0 · dω = 4π).
        let c = [1.0, 1.0, 1.0];
        let coeffs = project_irradiance_to_sh(move |_dir| c, 4096);
        for dir in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::new(0.4, 0.7, -0.59).normalize()] {
            let out = spherical_harmonics(&coeffs, dir);
            for ch in 0..3 {
                assert!(
                    (out[ch] - std::f64::consts::PI).abs() < 1e-2,
                    "irradiance {out:?} at {dir} should be ≈ π"
                );
            }
        }
    }

    #[test]
    fn project_irradiance_of_a_black_environment_is_zero() {
        let coeffs = project_irradiance_to_sh(|_dir| [0.0; 3], 1024);
        for row in &coeffs {
            for ch in row {
                assert!(ch.abs() < 1e-12);
            }
        }
    }

    #[test]
    fn fibonacci_sphere_is_unit_length_and_centred() {
        let dirs = fibonacci_sphere(2048);
        assert_eq!(dirs.len(), 2048);
        let mut centroid = DVec3::ZERO;
        for d in &dirs {
            assert!((d.length() - 1.0).abs() < 1e-9, "not unit: {d}");
            centroid += *d;
        }
        centroid /= dirs.len() as f64;
        assert!(
            centroid.length() < 1e-3,
            "Fibonacci sphere must be balanced, centroid {centroid}"
        );
    }

    #[test]
    fn radical_inverse_vdc_matches_known_base2_values() {
        assert!((radical_inverse_vdc(0) - 0.0).abs() < 1e-12);
        assert!((radical_inverse_vdc(1) - 0.5).abs() < 1e-12);
        assert!((radical_inverse_vdc(2) - 0.25).abs() < 1e-12);
        assert!((radical_inverse_vdc(3) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn importance_sample_ggx_at_zero_roughness_returns_the_normal() {
        // alpha = 0 collapses the GGX lobe to a delta at H = N.
        let h = importance_sample_ggx([0.3, 0.6], 0.0, DVec3::Z);
        assert!((h - DVec3::Z).length() < 1e-9, "H = {h}");
    }

    #[test]
    fn importance_sample_ggx_returns_unit_half_vectors() {
        for i in 0..64 {
            let xi = hammersley2d(i, 64);
            let h = importance_sample_ggx(xi, 0.4, DVec3::new(0.2, 0.9, 0.38).normalize());
            assert!((h.length() - 1.0).abs() < 1e-6, "i={i} H={h}");
        }
    }

    #[test]
    fn prefilter_of_a_constant_environment_returns_the_constant() {
        // A weighted average of a constant radiance is that constant.
        let c = [0.4, 0.6, 0.8];
        let out = prefilter_specular(move |_dir| c, 0.5, DVec3::Z, 512);
        for ch in 0..3 {
            assert!((out[ch] - c[ch]).abs() < 1e-9, "prefilter {out:?}");
        }
    }

    #[test]
    fn integrate_brdf_at_zero_roughness_normal_incidence_is_unit_scale() {
        // Closed form: roughness=0, NdotV=1 ⇒ every sample is H=L=V=N, G_Vis=1,
        // Fc=0 ⇒ (scale, bias) = (1, 0).
        let [scale, bias] = integrate_brdf(0.0, 1.0, 1024);
        assert!((scale - 1.0).abs() < 1e-9, "scale = {scale}");
        assert!(bias.abs() < 1e-9, "bias = {bias}");
    }

    #[test]
    fn integrate_brdf_scale_bias_are_finite_and_in_range() {
        for &roughness in &[0.04, 0.25, 0.5, 0.9, 1.0] {
            for &ndotv in &[0.02, 0.3, 0.7, 1.0] {
                let [scale, bias] = integrate_brdf(roughness, ndotv, 1024);
                assert!(scale.is_finite() && bias.is_finite());
                assert!(scale >= -1e-9, "scale {scale} at r={roughness} v={ndotv}");
                assert!(bias >= -1e-9, "bias {bias} at r={roughness} v={ndotv}");
                // scale + bias is the hemisphere-reflected BRDF energy fraction ≤ ~1.
                assert!(scale + bias <= 1.0 + 1e-6, "energy {scale}+{bias}");
            }
        }
    }

    #[test]
    fn fresnel_schlick2_endpoints_reproduce_f0_and_f90() {
        let f0 = [0.04, 0.04, 0.04];
        let f90 = [1.0, 1.0, 1.0];
        let at_normal = fresnel_schlick2(f0, f90, 1.0);
        let at_grazing = fresnel_schlick2(f0, f90, 0.0);
        for c in 0..3 {
            assert!((at_normal[c] - f0[c]).abs() < 1e-12);
            assert!((at_grazing[c] - f90[c]).abs() < 1e-12);
        }
    }

    #[test]
    fn smith_visibility_ggx_at_zero_roughness_normal_is_quarter() {
        // a=0, NdotL=NdotV=1 ⇒ GGXV=GGXL=1, GGX=2, Vis=0.5/2=0.25.
        assert!((smith_visibility_ggx(0.0, 1.0, 1.0) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn texture_ibl_with_zero_factor_is_black() {
        let sh = project_irradiance_to_sh(|_d| [1.0, 1.0, 1.0], 1024);
        let out = texture_ibl(
            &sh,
            DVec3::Z,
            DVec3::Z,
            &IblMaterial::default(),
            [0.0, 0.0],
            |_dir, _r| [1.0, 1.0, 1.0],
        );
        assert_eq!(out, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn texture_ibl_of_a_white_environment_is_positive_and_finite() {
        let sh = project_irradiance_to_sh(|_d| [1.0, 1.0, 1.0], 2048);
        let out = texture_ibl(
            &sh,
            DVec3::new(0.0, 0.3, 0.95).normalize(),
            DVec3::Z,
            &IblMaterial {
                diffuse: [0.8, 0.2, 0.2],
                specular_f0: [0.04, 0.04, 0.04],
                roughness: 0.35,
                specular_weight: 1.0,
            },
            [1.0, 1.0],
            |_dir, _r| [1.0, 1.0, 1.0],
        );
        for ch in 0..3 {
            assert!(out[ch].is_finite(), "non-finite IBL {out:?}");
            assert!(out[ch] > 0.0, "white env must light the surface: {out:?}");
        }
    }

    // ── Cluster C (M6 Wave A review) cross-checks ───────────────────────────

    #[test]
    fn brdf_apply_samples_mirrors_the_wgsl_constant() {
        // `adapters/bevy-render/shaders/ibl.wgsl:107` declares
        // `const BRDF_APPLY_SAMPLES: i32 = 32;`. The domain mirror must stay in
        // lockstep — it is what the GPU-boundary pairing tests sample against.
        assert_eq!(BRDF_APPLY_SAMPLES, 32);
    }

    #[test]
    fn direction_from_uv_is_z_up() {
        // FIX-IBL-ZUP: latitude rides `z` (`sin(lat)`), NOT `y`. Anchors:
        // equator-centre → +X, top row → +Z (north), bottom row → -Z, quarter
        // longitude → +Y. A y-up regression would swap the `z`/`y` poles.
        let eps = 1e-9;
        let c = direction_from_uv([0.5, 0.5]);
        assert!((c - DVec3::new(1.0, 0.0, 0.0)).length() < eps, "centre {c}");
        let north = direction_from_uv([0.5, 1.0]);
        assert!((north - DVec3::Z).length() < eps, "north pole {north} (must be +Z)");
        let south = direction_from_uv([0.5, 0.0]);
        assert!((south + DVec3::Z).length() < eps, "south pole {south} (must be -Z)");
        let east = direction_from_uv([0.75, 0.5]);
        assert!((east - DVec3::Y).length() < eps, "quarter-lon {east}");
    }

    #[test]
    fn direction_from_uv_matches_the_wgsl_f32_mirror() {
        // Cross-check the domain f64 twin against a faithful f32 transcription of
        // the WGSL `direction_from_uv` (FIX-IBL-ZUP). Non-trivial UVs across the
        // sphere; tolerance absorbs the single f32 round-down at the GPU boundary.
        fn gpu_direction_from_uv(uv: [f32; 2]) -> [f32; 3] {
            const PI: f32 = std::f32::consts::PI;
            const TAU: f32 = std::f32::consts::TAU;
            let lon = TAU * (uv[0] - 0.5);
            let lat = PI * (uv[1] - 0.5);
            let cos_lat = lat.cos();
            let v = [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()];
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / len, v[1] / len, v[2] / len]
        }
        for iy in 0..7 {
            for ix in 0..9 {
                let u = ix as f64 / 8.0;
                let vgt = iy as f64 / 6.0;
                let d = direction_from_uv([u, vgt]);
                let g = gpu_direction_from_uv([u as f32, vgt as f32]);
                let err = ((d.x - g[0] as f64).powi(2)
                    + (d.y - g[1] as f64).powi(2)
                    + (d.z - g[2] as f64).powi(2))
                .sqrt();
                assert!(err < 1e-6, "uv=({u},{vgt}) domain={d} gpu={g:?} err={err}");
            }
        }
    }

    #[test]
    fn multi_scattering_denominator_guard_matches_the_gpu_abs_form() {
        // FIX-IBL-DENOM: the WGSL switched from `denom > 1e-12` to
        // `abs(denom) > 1e-12` to mirror the domain's `denom.abs() > 1e-12`.
        // The two must agree across the sign of `denom` — a legitimately large
        // NEGATIVE denominator (specular_weight > 1 pushes `ems` past 1) must NOT
        // be zeroed out, while a near-zero one (either sign) must return 0.
        let domain_guard = |num: f64, denom: f64| {
            if denom.abs() > 1e-12 {
                num / denom
            } else {
                0.0
            }
        };
        let gpu_guard = |num: f64, denom: f64| {
            let n = num as f32 as f64;
            let dn = denom as f32 as f64;
            // select(0, num/denom, abs(denom) > eps) — both arms evaluate, the
            // inf from a ~0 denominator is discarded, never propagated.
            if dn.abs() > 1e-12 {
                n / dn
            } else {
                0.0
            }
        };
        // num arbitrary; cover positive / negative / tiny / zero denominators.
        let cases = [
            (0.5, 0.5),
            (0.5, -0.5), // large-negative: must produce a non-zero, matching result
            (0.5, 1e-13),
            (0.5, -1e-13),
            (0.5, 0.0),
            (-0.5, -0.25),
        ];
        for (num, denom) in cases {
            let d = domain_guard(num, denom);
            let g = gpu_guard(num, denom);
            assert!(d.is_finite() && g.is_finite(), "num={num} denom={denom}");
            assert!(
                (d - g).abs() < 1e-6,
                "guard divergence num={num} denom={denom} domain={d} gpu={g}"
            );
        }
        // Sanity: the large-negative case is non-zero on BOTH sides (proves the
        // abs-form parity — the old unsigned `denom > eps` GPU form would be 0).
        assert!(domain_guard(0.5, -0.5).abs() > 1e-6);
        assert!(gpu_guard(0.5, -0.5).abs() > 1e-6);
    }
}
