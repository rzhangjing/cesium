//! 与顺序无关透明度（Order-Independent Transparency，OIT）。
//!
//! 涵盖以下能力模型：
//! - 加权混合 OIT（累加 + revealage）
//! - 半透明多 pass 支持
//! - MRT（多渲染目标）支持检测
//!
//! 领域层——纯 Rust，f64 精度。

use glam::DVec4;

/// 用于 OIT 合成的混合等式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendEquation {
    /// 源 + 目标。
    #[default]
    Add,
    /// 源 - 目标。
    Subtract,
    /// 目标 - 源。
    ReverseSubtract,
    /// Min(源, 目标)。
    Min,
    /// Max(源, 目标)。
    Max,
}

/// 用于 OIT 的混合函数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendFunction {
    /// 零。
    Zero,
    /// 一。
    #[default]
    One,
    /// 源颜色。
    SourceColor,
    /// 一减源颜色。
    OneMinusSourceColor,
    /// 目标颜色。
    DestinationColor,
    /// 一减目标颜色。
    OneMinusDestinationColor,
    /// 源 alpha。
    SourceAlpha,
    /// 一减源 alpha。
    OneMinusSourceAlpha,
    /// 目标 alpha。
    DestinationAlpha,
    /// 一减目标 alpha。
    OneMinusDestinationAlpha,
}

/// OIT 支持能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OitCapabilities {
    /// 是否支持 MRT（多渲染目标）。
    pub mrt_supported: bool,
    /// 是否支持浮点混合。
    pub float_blend_supported: bool,
    /// 是否支持深度纹理。
    pub depth_texture_supported: bool,
    /// 是否支持浮点颜色缓冲。
    pub color_buffer_float: bool,
}

impl OitCapabilities {
    /// 返回是否支持通过 MRT 的加权混合 OIT。
    pub fn translucent_mrt_supported(&self) -> bool {
        self.mrt_supported
            && self.color_buffer_float
            && self.depth_texture_supported
            && self.float_blend_supported
    }

    /// 返回是否支持多 pass OIT（MRT 不可用时的回退）。
    pub fn translucent_multipass_supported(&self) -> bool {
        !self.translucent_mrt_supported()
            && self.color_buffer_float
            && self.depth_texture_supported
            && self.float_blend_supported
    }

    /// 返回是否支持任一 OIT 模式。
    pub fn is_supported(&self) -> bool {
        self.translucent_mrt_supported() || self.translucent_multipass_supported()
    }
}

/// OIT 渲染模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OitMode {
    /// 无 OIT（标准 alpha 混合）。
    #[default]
    None,
    /// 使用 MRT（2 个渲染目标）的加权混合 OIT。
    WeightedBlendedMrt,
    /// 使用多 pass（回退）的加权混合 OIT。
    WeightedBlendedMultipass,
}

/// OIT 配置与状态。
#[derive(Debug, Clone)]
pub struct OitConfig {
    /// 使用的 OIT 模式。
    pub mode: OitMode,
    /// MSAA 采样数。
    pub num_samples: u32,
    /// 是否使用 HDR 渲染。
    pub use_hdr: bool,
    /// 累加缓冲区的混合等式。
    pub blend_equation: BlendEquation,
    /// 源混合函数。
    pub source_blend: BlendFunction,
    /// 目标混合函数。
    pub destination_blend: BlendFunction,
}

impl Default for OitConfig {
    /// 默认无 OIT、单采样、非 HDR、加法混合且源/目标均为 1。
    fn default() -> Self {
        // 退化为标准 alpha 混合的中性配置
        Self {
            mode: OitMode::None,
            num_samples: 1,
            use_hdr: false,
            blend_equation: BlendEquation::Add,
            source_blend: BlendFunction::One,
            destination_blend: BlendFunction::One,
        }
    }
}

impl OitConfig {
    /// 基于设备能力创建 OIT 配置。
    pub fn from_capabilities(caps: &OitCapabilities) -> Self {
        // 优先选 MRT，其次多 pass 回退，否则不启用 OIT
        let mode = if caps.translucent_mrt_supported() {
            OitMode::WeightedBlendedMrt
        } else if caps.translucent_multipass_supported() {
            OitMode::WeightedBlendedMultipass
        } else {
            OitMode::None
        };

        Self {
            mode,
            ..Default::default()
        }
    }

    /// 返回 OIT 是否处于活动状态。
    pub fn is_active(&self) -> bool {
        self.mode != OitMode::None
    }

    /// 基于片元的深度与 alpha 计算其权重。
    ///
    /// 使用 McGuire & Bavoil (2013) 的加权函数：
    /// w = alpha * clamp(0.03 / (1e-5 + pow(depth/200, 4)), 0.01, 3000)
    pub fn compute_weight(&self, alpha: f64, depth: f64) -> f64 {
        let depth_term = (depth / 200.0).powi(4);
        alpha * (0.03 / (1e-5 + depth_term)).clamp(0.01, 3000.0)
    }

    /// 将一个半透明片元累加到 OIT 缓冲区。
    ///
    /// # 参数
    /// * `color` - 片元颜色（RGBA，预期为预乘 alpha）
    /// * `depth` - 片元深度（视图空间，正值）
    ///
    /// # 返回
    /// (accumulation, revealage)——要分别加到对应缓冲区的值。
    pub fn accumulate_fragment(&self, color: DVec4, depth: f64) -> (DVec4, f64) {
        let weight = self.compute_weight(color.w, depth);

        // 累加：color * weight
        let accumulation = DVec4::new(
            color.x * weight,
            color.y * weight,
            color.z * weight,
            color.w * weight,
        );

        // Revealage：1 - alpha（所有 revealage 的乘积）
        let revealage = 1.0 - color.w;

        (accumulation, revealage)
    }

    /// 将 OIT 缓冲区与不透明场景合成。
    ///
    /// # 参数
    /// * `opaque_color` - 不透明场景颜色（RGB）
    /// * `accumulation` - 已累加的半透明颜色（RGBA）
    /// * `revealage` - revealage 值（0 = 完全透明，1 = 完全不透明）
    ///
    /// # 返回
    /// 最终合成的颜色（RGB）。
    pub fn composite(
        &self,
        opaque_color: DVec4,
        accumulation: DVec4,
        revealage: f64,
    ) -> DVec4 {
        if revealage >= 1.0 {
            // 无半透明贡献
            return opaque_color;
        }

        // 平均半透明颜色
        let avg_color = if accumulation.w > 1e-5 {
            DVec4::new(
                accumulation.x / accumulation.w,
                accumulation.y / accumulation.w,
                accumulation.z / accumulation.w,
                accumulation.w,
            )
        } else {
            DVec4::ZERO
        };

        // 混合：opaque * revealage + translucent * (1 - revealage)
        let translucent_alpha = (1.0 - revealage) * avg_color.w;
        DVec4::new(
            opaque_color.x * revealage + avg_color.x * translucent_alpha,
            opaque_color.y * revealage + avg_color.y * translucent_alpha,
            opaque_color.z * revealage + avg_color.z * translucent_alpha,
            1.0 - revealage * (1.0 - opaque_color.w),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oit_capabilities_mrt() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(caps.translucent_mrt_supported());
        assert!(!caps.translucent_multipass_supported());
        assert!(caps.is_supported());
    }

    #[test]
    fn test_oit_capabilities_multipass() {
        let caps = OitCapabilities {
            mrt_supported: false,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(!caps.translucent_mrt_supported());
        assert!(caps.translucent_multipass_supported());
        assert!(caps.is_supported());
    }

    #[test]
    fn test_oit_capabilities_unsupported() {
        let caps = OitCapabilities {
            mrt_supported: false,
            float_blend_supported: false,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(!caps.is_supported());
    }

    #[test]
    fn test_oit_config_from_capabilities() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        let config = OitConfig::from_capabilities(&caps);
        assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
        assert!(config.is_active());
    }

    #[test]
    fn test_oit_config_unsupported() {
        let caps = OitCapabilities::default();
        let config = OitConfig::from_capabilities(&caps);
        assert_eq!(config.mode, OitMode::None);
        assert!(!config.is_active());
    }

    #[test]
    fn test_compute_weight() {
        let config = OitConfig::default();

        // 近处片元应有更高的权重
        let near_weight = config.compute_weight(1.0, 1.0);
        let far_weight = config.compute_weight(1.0, 100.0);
        assert!(near_weight > far_weight);

        // alpha 为零 → 权重为零
        let zero_alpha = config.compute_weight(0.0, 10.0);
        assert!((zero_alpha).abs() < 1e-10);
    }

    #[test]
    fn test_accumulate_fragment() {
        let config = OitConfig::default();
        let color = DVec4::new(1.0, 0.0, 0.0, 0.5); // 半透明红
        let depth = 10.0;

        let (accumulation, revealage) = config.accumulate_fragment(color, depth);

        // 累加值应非零
        assert!(accumulation.x > 0.0);
        // Revealage 应为 1 - alpha = 0.5
        assert!((revealage - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_composite_no_translucent() {
        let config = OitConfig::default();
        let opaque = DVec4::new(0.5, 0.5, 0.5, 1.0);
        let accumulation = DVec4::ZERO;
        let revealage = 1.0; // 完全不透明（无半透明）

        let result = config.composite(opaque, accumulation, revealage);
        assert!((result - opaque).length() < 1e-10);
    }

    #[test]
    fn test_composite_with_translucent() {
        let config = OitConfig::default();
        let opaque = DVec4::new(0.0, 0.0, 1.0, 1.0); // 蓝色背景
        let accumulation = DVec4::new(0.5, 0.0, 0.0, 0.5); // 红色贡献
        let revealage = 0.5; // 50% 透明

        let result = config.composite(opaque, accumulation, revealage);

        // 结果应为蓝与红的混合
        assert!(result.x > 0.0); // 含红
        assert!(result.z > 0.0); // 含蓝
    }

    #[test]
    fn test_blend_equation_default() {
        assert_eq!(BlendEquation::default(), BlendEquation::Add);
    }

    #[test]
    fn test_blend_function_default() {
        assert_eq!(BlendFunction::default(), BlendFunction::One);
    }
}
