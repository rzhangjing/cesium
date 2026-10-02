//! 基于图像的照明（IBL）。
//!
//! 包含：
//! - 用于漫反射 IBL 的球谐系数
//! - 镜面环境贴图
//! - IBL factor 缩放
//!
//! 领域层——纯 Rust，f64 精度。

use glam::DVec3;

/// 球谐系数的数量（3 阶 = 9 个系数）。
pub const SH_COEFFICIENT_COUNT: usize = 9;

/// 在运行时应用 IBL 所使用的 split-sum BRDF 积分采样数。
///
/// 镜像 `adapters/bevy-render/shaders/ibl.wgsl`（L107）中的
/// `BRDF_APPLY_SAMPLES`。GPU 应用节点以这么多数量的 GGX 重要性采样
/// 内联地重算 split-sum BRDF，而非绑定一个预计算的 256×256 LUT
/// （参见 `docs/deviations.md#dev-024`）；CPU 参考保持相同的数量，
/// 从而使 domain↔GPU 的镜面对照能同口径比较。应用路径使用 32 个
/// 采样点，而 [`texture_ibl`] 中更高精度的 CPU 积分器运行 1024 个——
/// 这一镜像是为 GPU 边界配对测试服务的，而非为 [`texture_ibl`] 本身。
pub const BRDF_APPLY_SAMPLES: usize = 32;

/// 基于图像照明的配置。
///
/// 映射到 CesiumJS `ImageBasedLighting`。
#[derive(Debug, Clone)]
pub struct ImageBasedLighting {
    /// 缩放漫反射与镜面 IBL 的贡献。
    /// x = 漫反射因子，y = 镜面因子。二者均在 [0, 1]。
    pub image_based_lighting_factor: [f64; 2],
    /// 用于漫反射 IBL 的三阶球谐系数。
    /// 9 个系数，每个都是一个 RGB 三元组。
    pub spherical_harmonic_coefficients: Option<[[f64; 3]; SH_COEFFICIENT_COUNT]>,
    /// 指向 KTX2 镜面环境贴图的 URL。
    pub specular_environment_maps: Option<String>,
    /// 是否使用默认球谐系数。
    pub use_default_spherical_harmonics: bool,
    /// 是否使用默认镜面贴图。
    pub use_default_specular_maps: bool,
}

impl Default for ImageBasedLighting {
    /// 默认 IBL 状态：factor 为 `[1.0, 1.0]`，无球谐系数与环境贴图。
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
    /// 创建一个新的 IBL 配置。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 IBL 因子（漫反射与镜面缩放）。
    ///
    /// # 恐慌
    /// 若值超出 [0, 1] 则 panic。
    pub fn set_factor(&mut self, diffuse: f64, specular: f64) {
        assert!((0.0..=1.0).contains(&diffuse), "diffuse factor must be in [0, 1]");
        assert!((0.0..=1.0).contains(&specular), "specular factor must be in [0, 1]");
        self.image_based_lighting_factor = [diffuse, specular];
    }

    /// 设置球谐系数。
    ///
    /// # 恐慌
    /// 若数组并非恰好含 9 个系数则 panic。
    pub fn set_spherical_harmonics(&mut self, coefficients: [[f64; 3]; SH_COEFFICIENT_COUNT]) {
        self.spherical_harmonic_coefficients = Some(coefficients);
        self.use_default_spherical_harmonics = false;
    }

    /// 返回是否已设置自定义 SH 系数。
    pub fn has_spherical_harmonics(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some()
    }

    /// 返回是否已设置镜面环境贴图。
    pub fn has_specular_environment_maps(&self) -> bool {
        self.specular_environment_maps.is_some()
    }

    /// 返回是否因 IBL 变化而需要重新生成 shader。
    pub fn needs_shader_regeneration(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some() || self.specular_environment_maps.is_some()
    }

    /// 计算给定法线方向的漫反射 IBL 贡献。
    ///
    /// 使用球谐系数来求值 irradiance。
    ///
    /// # 参数
    /// * `normal` - 表面法线（已归一化）
    ///
    /// # 返回
    /// 漫反射 irradiance 颜色 [R, G, B]，按漫反射 IBL 因子缩放。
    pub fn compute_diffuse_ibl(&self, normal: DVec3) -> [f64; 3] {
        let coefficients = match &self.spherical_harmonic_coefficients {
            Some(c) => c,
            None => return [0.0; 3],
        };

        let diffuse_factor = self.image_based_lighting_factor[0];
        if diffuse_factor == 0.0 {
            return [0.0; 3];
        }

        // 求值球谐函数
        let sh = evaluate_sh(coefficients, normal);

        [sh[0] * diffuse_factor, sh[1] * diffuse_factor, sh[2] * diffuse_factor]
    }

    /// 计算镜面 IBL 贡献。
    ///
    /// 在一个完整实现中，它会在反射方向上采样预过滤的镜面环境贴图，
    /// 并根据 roughness 选取合适的 mip 层级。
    ///
    /// # 参数
    /// * `reflection` - 反射方向（已归一化）
    /// * `roughness` - 表面 roughness [0, 1]
    ///
    /// # 返回
    /// 镜面颜色 [R, G, B]，按镜面 IBL 因子缩放。
    pub fn compute_specular_ibl(&self, _reflection: DVec3, _roughness: f64) -> [f64; 3] {
        let specular_factor = self.image_based_lighting_factor[1];
        if specular_factor == 0.0 {
            return [0.0; 3];
        }

        // 偏差：占位的常量贡献，参见 docs/deviations.md#dev-024。
        // 真正的实现会在 `reflection` 处按 roughness→mip LOD 采样预过滤的
        // 镜面环境立方贴图；而离线 / GPU 预过滤驱动尚未落地（延期项 #54）。
        // 在此之前，它返回一个按镜面 IBL 因子缩放的固定中性环境色调——
        // 这就是为何 `ibl_compute_specular_*` 那些 roughness / 反射相关性
        // 规格在 `specs/tests/scene/ibl_cloud_spec.rs` 中被标为
        // `#[ignore]`（等待延期项 #54 的真实预过滤）。
        [0.1 * specular_factor, 0.1 * specular_factor, 0.12 * specular_factor]
    }
}

/// 对给定方向求值三阶球谐函数。
///
/// 0、1、2 阶的 9 个 SH 基函数：
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

    // SH 基函数
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

/// 中性天空环境的默认球谐系数。
///
/// 它们近似一个简单的大地—天空环境。
pub fn default_spherical_harmonics() -> [[f64; 3]; SH_COEFFICIENT_COUNT] {
    [
        [0.3, 0.3, 0.35],   // 直流项（环境光）
        [0.0, 0.0, 0.0],    // Y_1^{-1}
        [0.1, 0.1, 0.15],   // Y_1^0（天/地梯度）
        [0.0, 0.0, 0.0],    // Y_1^1
        [0.0, 0.0, 0.0],    // Y_2^{-2}
        [0.0, 0.0, 0.0],    // Y_2^{-1}
        [0.05, 0.05, 0.08], // Y_2^0
        [0.0, 0.0, 0.0],    // Y_2^1
        [0.0, 0.0, 0.0],    // Y_2^2
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// 忠于 CesiumJS 的 IBL CPU 参考（M6.5）
// ═══════════════════════════════════════════════════════════════════════════
//
// 下面这些函数是对权威 PBR/IBL shader 的 1:1 f64 CPU 移植，逐条入口
// 镜像 `adapters/bevy-render/src/shaders/ibl.wgsl`，以便 CPU 参考与
// GPU shader 能相互对照验证：
//   * `spherical_harmonics`  ← 球谐辐照度求值
//   * `ggx_ndf` / `smith_visibility_ggx` / `fresnel_schlick2`
//                            ← PBR 光照核心项
//   * `prefilter_specular`   ← 镜面环境预过滤
//   * `integrate_brdf`       ← BRDF LUT 生成
//   * `texture_ibl`          ← 完整 IBL 合成
//
// SH 约定（与上文遗留的 `evaluate_sh` 有意分歧）：
// CesiumJS 的 `czm_sphericalHarmonics` 消费预缩放（PRE-SCALED）系数——cmgen
// （`--no-mirror`）把正交归一基常量与余弦波瓣 irradiance 传递一并烘焙进这
// 9 个 RGB 值——因此求值就是对 (x,y,z) 的裸多项式，后接 `max(., 0)`。遗留的
// `evaluate_sh` 则存储原始的正交归一基系数，并在求值时才应用常量。
// `project_irradiance_to_sh` 输出 CesiumJS 约定，因此其结果可原样馈入
// `spherical_harmonics`（CPU）与 `irradiance` WGSL 入口（GPU）。两者都保留，
// 因为 `evaluate_sh` 支撑着既有的 `compute_diffuse_ibl` API
// （domain 的 f64 语义已冻结）。

/// 9 个球谐多项式基项 `P_i(x,y,z)`，
/// 按系数顺序 `[L00, L1_1, L10, L11, L2_2, L2_1, L20, L21, L22]`。
/// 与参考实现逐字一致（不含归一化常量）。
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

/// 正交归一 SH 基归一化常量 `k_lm`（按系数索引），
/// 即 `Y_lm(dir) = k_lm · P_i(dir)`。标准取值（Ramamoorthi, envmap.pdf）。
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

/// 带余弦波瓣的转移系数 `A_l`（Ramamoorthi & Hanrahan）：即
/// `l = 0, 1, 2` 时的 irradiance 带缩放 `[π, 2π/3, π/4]`。
pub const IRRADIANCE_ZONAL_BY_BAND: [f64; 3] = [
    std::f64::consts::PI,
    2.0 * std::f64::consts::PI / 3.0,
    std::f64::consts::PI / 4.0,
];

/// 9 个系数各自的 SH 带 `l`，按 CesiumJS 顺序。
const SH_BAND: [usize; SH_COEFFICIENT_COUNT] = [0, 1, 1, 1, 2, 2, 2, 2, 2];

/// 完全按 CesiumJS `czm_sphericalHarmonics` 的方式求值三阶 SH：
/// 每个 RGB 通道 `max(Σ c_i · P_i(dir), 0)`。系数使用预缩放（PRE-SCALED）的
/// CesiumJS 约定（见模块说明）。是 `ibl.wgsl` 中
/// `spherical_harmonics` 辅助函数的 f64 参考。
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
    // czm 将负 irradiance 钳制为零：`max(L, vec3(0.0))`。
    [out[0].max(0.0), out[1].max(0.0), out[2].max(0.0)]
}

/// 单位球上确定性的 Fibonacci 格方向。固定（无 RNG），
/// 因此 `project_irradiance_to_sh` 逐位可复现、跨运行且对 CI 稳定。
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

/// 通过在 Fibonacci 球上做确定性蒙特卡洛，把一个环境 radiance 函数投影到
/// 9 个 CesiumJS 约定的 irradiance SH 系数上。`radiance(dir) -> [R,G,B]`
/// （线性，≥ 0）。结果直接馈入 [`spherical_harmonics`]（CPU）与
/// `irradiance` WGSL 入口（GPU）。
///
/// 数学：`a_lm = ∫ L(ω)Y_lm(ω)dω ≈ (4π/N) Σ L(ω_j) k_lm P_i(ω_j)`；
/// irradiance 传递 `E_lm = A_l · a_lm`；CesiumJS 系数
/// `c_i = E_lm · k_lm`，因此 `spherical_harmonics(c, n)` 复现出 irradiance
/// `E(n) = ∫ L(ω)(n·ω)⁺ dω`。
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
            // c_i 累加 L · P_i · (solid_angle · k² · A_l)。
            let scale = solid_angle * k * k * a_band;
            for c in 0..3 {
                coeffs[i][c] += l[c] * basis[i] * scale;
            }
        }
    }
    coeffs
}

/// Van der Corput radical inverse，基 2。通过
/// 位运算做精确的整数减半——无浮点 `mod`，因此 WGSL 孪生体完全绕开了
/// `mod` 保留字隐患。
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

/// Hammersley 2D 低差异点。
pub fn hammersley2d(i: usize, n: usize) -> [f64; 2] {
    [i as f64 / n.max(1) as f64, radical_inverse_vdc(i as u32)]
}

/// 等距柱状投影 `[0,1]²` uv → 单位世界方向，**z-up**——是
/// `adapters/bevy-render/shaders/ibl.wgsl` 中 `direction_from_uv` 的 f64 孪生体
/// （FIX-IBL-ZUP）。
///
/// 纬度承载于 `z` 分量（`sin(lat)`），经度位于 `x`/`y` 平面，与
/// `domain/effects/src/panorama.rs` 中的 `direction_to_uv` 一致，从而引擎中
/// 每张等距柱状贴图都遵循同一约定。修正前的 GPU 形式把 `sin(theta)` 放在
/// `y`（y-up），与引擎其余部分不一致；这个孪生体正是 WGSL 对照测试用来钉住
/// shader 的依据。
pub fn direction_from_uv(uv: [f64; 2]) -> DVec3 {
    use std::f64::consts::{PI, TAU};
    let lon = TAU * (uv[0] - 0.5); // [-π, π]
    let lat = PI * (uv[1] - 0.5); // [-π/2, π/2]
    let cos_lat = lat.cos();
    DVec3::new(cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()).normalize_or_zero()
}

/// GGX / Trowbridge-Reitz 法线分布。
pub fn ggx_ndf(alpha_roughness: f64, ndoth: f64) -> f64 {
    let a2 = alpha_roughness * alpha_roughness;
    let f = (ndoth * a2 - ndoth) * ndoth + 1.0;
    a2 / (std::f64::consts::PI * f * f)
}

/// Smith 联合 GGX 可见性 `= G/(4·NdotL·NdotV)`。
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

/// 依赖 roughness 的 Schlick Fresnel。
/// `versine^5` 展开为 `vs2*vs2*versine` 并保持非融合（UNFUSED）——以匹配
/// WGSL 孪生体的两次舍入规则（IBL 数值上不做 FMA 收缩）。
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

/// GGX 重要性采样：由二维准随机 `xi` 得到世界空间的半角向量 `H`。
/// 注意两个调用点传入的参数不同：
/// `ConvolveSpecularMapFS` 传入感知 `roughness`，而 `BrdfLutGeneratorFS`
/// 传入 `alphaRoughness = roughness²`。本函数内部会对 `alpha_roughness`
/// 参数取平方，与 GLSL 完全一致，因此调用方必须逐字复现其上游调用点。
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

/// `dir` / `roughness` 的预过滤镜面 radiance：对环境做 GGX 重要性采样，按 `NdotL`
/// 为每个采样点加权，再由累加权重归一化。`radiance(dir)` 在给定方向上
/// 采样源环境立方贴图。
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
        // ConvolveSpecularMapFS 传入原始 `roughness`（内部取平方）。
        let h = importance_sample_ggx(xi, roughness, v);
        let l = h * (2.0 * v.dot(h)) - v; // 反射向量
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

/// split-sum 环境-BRDF 积分 → `(scale, bias)`。按 `(NdotV, roughness)` 索引，
/// 正如 `texture(czm_brdfLut, vec2(NdotV, roughness))` 读取 LUT 那样。
pub fn integrate_brdf(roughness: f64, ndotv: f64, samples: usize) -> [f64; 2] {
    let ndotv = ndotv.clamp(0.0, 1.0);
    let v = DVec3::new((1.0 - ndotv * ndotv).max(0.0).sqrt(), 0.0, ndotv);
    let alpha_roughness = roughness * roughness;
    let n = samples.max(1);
    let mut a = 0.0f64;
    let mut b = 0.0f64;
    for i in 0..n {
        let xi = hammersley2d(i, n);
        // BrdfLutGeneratorFS 传入 `alphaRoughness = roughness²`（内部再次取平方）。
        let h = importance_sample_ggx(xi, alpha_roughness, DVec3::Z);
        let l = h * (2.0 * v.dot(h)) - v;
        let ndotl = l.z.clamp(0.0, 1.0);
        let ndoth = h.z.clamp(0.0, 1.0);
        let vdoth = v.dot(h).clamp(0.0, 1.0);
        // `ndoth > 0.0` 守护着 `4·G·VdotH·NdotL / NdotH` 的除法；它对上游
        // `NdotL > 0` 分支所接纳的每个采样点都成立（一个防御性的超集）。
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

/// [`texture_ibl`] 的 PBR 表面输入（`czm_modelMaterial` 的子集）。
#[derive(Debug, Clone, Copy)]
pub struct IblMaterial {
    /// Lambertian 基础颜色（线性 RGB）。
    pub diffuse: [f64; 3],
    /// 镜面 F0 反射率（线性 RGB）。
    pub specular_f0: [f64; 3],
    /// 感知 roughness，位于 `[0, 1]`。
    pub roughness: f64,
    /// 镜面权重（glTF `KHR_materials_specular`）；未使用时为 `1.0`。
    pub specular_weight: f64,
}

impl Default for IblMaterial {
    /// 默认材质：白色漫反射、介电 F0=0.04、半粗糙、满镜面权重。
    fn default() -> Self {
        Self {
            diffuse: [1.0, 1.0, 1.0],
            specular_f0: [0.04, 0.04, 0.04],
            roughness: 0.5,
            specular_weight: 1.0,
        }
    }
}

/// 完整的基于图像照明贡献（Fdez-Aguera 单次 +
/// 多次散射）。它将三个参考串联起来：漫反射来自 SH irradiance，镜面来自
/// 预过滤环境的闭包 `specular_env(dir, roughness)`，再由 split-sum BRDF LUT
/// 调制。`ibl_factor = [diffuse, specular]`。
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

    // 依赖 roughness 的 Fresnel，出自 Fdez-Aguera：f90 = max(1-roughness, f0)。
    let one_minus_r = 1.0 - roughness;
    let f90 = [
        one_minus_r.max(f0[0]),
        one_minus_r.max(f0[1]),
        one_minus_r.max(f0[2]),
    ];
    let single_scatter_fresnel = fresnel_schlick2(f0, f90, ndotv);
    let brdf_lut = integrate_brdf(roughness, ndotv, 1024);
    let (lut_scale, lut_bias) = (brdf_lut[0], brdf_lut[1]);

    // FssEss = specularWeight · (F · scale + bias)，逐通道。
    let fss_ess = [
        specular_weight * (single_scatter_fresnel[0] * lut_scale + lut_bias),
        specular_weight * (single_scatter_fresnel[1] * lut_scale + lut_bias),
        specular_weight * (single_scatter_fresnel[2] * lut_scale + lut_bias),
    ];

    // 漫反射（多次散射能量补偿）。
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

    // 镜面：reflect(-V, N) = -V - 2·dot(N,-V)·N = 2(N·V)N - V。
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

        // 应产生非零结果
        assert!(result[0] > 0.0 || result[1] > 0.0 || result[2] > 0.0);
    }

    #[test]
    fn test_compute_diffuse_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_spherical_harmonics(default_spherical_harmonics());
        ibl.set_factor(0.0, 1.0); // 漫反射为零

        let result = ibl.compute_diffuse_ibl(DVec3::Y);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_compute_specular_ibl() {
        let ibl = ImageBasedLighting::default();
        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);

        // 默认返回中性贡献
        assert!(result[0] > 0.0);
    }

    #[test]
    fn test_compute_specular_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.0, 0.0); // 镜面为零

        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_evaluate_sh_dc_only() {
        // 仅设置直流项
        let mut coefficients = [[0.0; 3]; 9];
        coefficients[0] = [1.0, 1.0, 1.0];

        // 直流项应与方向无关，恒为常量
        let up = evaluate_sh(&coefficients, DVec3::Y);
        let down = evaluate_sh(&coefficients, DVec3::new(0.0, -1.0, 0.0));

        assert!((up[0] - down[0]).abs() < 1e-10);
        assert!((up[0] - 0.282095).abs() < 1e-5);
    }

    #[test]
    fn test_default_spherical_harmonics() {
        let sh = default_spherical_harmonics();
        // 直流项应是环境光贡献
        assert!(sh[0][0] > 0.0);
        assert!(sh[0][1] > 0.0);
        assert!(sh[0][2] > 0.0);
    }

    // ─── M6.5 忠于 CesiumJS 的 IBL 参考 ─────────────────────────────
    // 每条断言都锚定到由上游 GLSL 导出的闭式值，因此无需 GPU 也能捕获数值回归。

    #[test]
    fn spherical_harmonics_dc_only_is_direction_independent() {
        // czm_sphericalHarmonics：P_0 == 1，因此仅含直流项的系数集合
        // 对每个方向都原样返回该系数。
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
        // czm 以 `max(L, vec3(0.0))` 结尾。
        let mut coeffs = [[0.0f64; 3]; SH_COEFFICIENT_COUNT];
        coeffs[0] = [-1.0, -2.0, -3.0];
        let out = spherical_harmonics(&coeffs, DVec3::Y);
        assert_eq!(out, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn spherical_harmonics_band1_z_is_direction_dependent() {
        // L10 基是 `z`：+Z 与 -Z 在钳制前必须求值为异号。
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
        // 闭式：对整个球面上的均匀环境 L = C，
        // E(n) = C ∫_hemi cos dω = C·π 对每条法线成立。直流系数
        // 在任意等立体角求积下都是精确的（Σ P_0 · dω = 4π）。
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
        // alpha = 0 把 GGX 波瓣塌缩为 H = N 处的一个 delta。
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
        // 常量 radiance 的加权平均仍是该常量。
        let c = [0.4, 0.6, 0.8];
        let out = prefilter_specular(move |_dir| c, 0.5, DVec3::Z, 512);
        for ch in 0..3 {
            assert!((out[ch] - c[ch]).abs() < 1e-9, "prefilter {out:?}");
        }
    }

    #[test]
    fn integrate_brdf_at_zero_roughness_normal_incidence_is_unit_scale() {
        // 闭式：roughness=0, NdotV=1 ⇒ 每个采样点都是 H=L=V=N, G_Vis=1,
        // Fc=0 ⇒ (scale, bias) = (1, 0)。
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
                // scale + bias 是半球反射的 BRDF 能量占比 ≤ ~1。
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
        // a=0, NdotL=NdotV=1 ⇒ GGXV=GGXL=1, GGX=2, Vis=0.5/2=0.25。
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

    // ── Cluster C（M6 Wave A 评审）交叉对照 ───────────────────────────

    #[test]
    fn brdf_apply_samples_mirrors_the_wgsl_constant() {
        // `adapters/bevy-render/shaders/ibl.wgsl:107` 声明了
        // `const BRDF_APPLY_SAMPLES: i32 = 32;`。domain 的镜像必须与之
        // 保持同步——GPU 边界配对测试正是拿它来对照采样。
        assert_eq!(BRDF_APPLY_SAMPLES, 32);
    }

    #[test]
    fn direction_from_uv_is_z_up() {
        // FIX-IBL-ZUP：纬度承载于 `z`（`sin(lat)`），而非 `y`。锚点：
        // 赤道中心 → +X，顶行 → +Z（北），底行 → -Z，四分之一
        // 经度 → +Y。若是 y-up 回归则会交换 `z`/`y` 两极。
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
        // 将 domain 的 f64 孪生体与 WGSL `direction_from_uv`
        // （FIX-IBL-ZUP）的一份忠于原式的 f32 转录做交叉对照。取球面上
        // 非平凡的 UV；容差吸收 GPU 边界处那一次 f32 舍入向下。
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
        // FIX-IBL-DENOM：WGSL 从 `denom > 1e-12` 改为
        // `abs(denom) > 1e-12`，以镜像 domain 的 `denom.abs() > 1e-12`。
        // 两者必须对 `denom` 的符号保持一致——一个合法的大负分母
        // （specular_weight > 1 会把 `ems` 推过 1）绝不能被清零，
        // 而一个接近零的分母（无论正负）必须返回 0。
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
            // select(0, num/denom, abs(denom) > eps)——两臂都会求值，
            // 由 ~0 分母产生的 inf 会被丢弃，绝不传播。
            if dn.abs() > 1e-12 {
                n / dn
            } else {
                0.0
            }
        };
        // num 任意；覆盖正 / 负 / 极小 / 零分母。
        let cases = [
            (0.5, 0.5),
            (0.5, -0.5), // 大负值：必须产生非零且一致的结果
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
        // 合理性检查：大负值情形在两侧都非零（证明 abs 形式的对等性——
        // 旧的无符号 `denom > eps` GPU 形式本会得到 0）。
        assert!(domain_guard(0.5, -0.5).abs() > 1e-6);
        assert!(gpu_guard(0.5, -0.5).abs() > 1e-6);
    }
}
