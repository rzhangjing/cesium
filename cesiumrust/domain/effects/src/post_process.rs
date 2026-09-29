//! 后处理效果流水线。
//!
//! 映射到 CesiumJS `Scene/PostProcessStageLibrary.js`：
//! - Bloom
//! - 环境光遮蔽
//! - 雾
//! - 色调映射

use glam::DVec3;

/// 后处理阶段标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostProcessStageType {
    /// Bloom（HDR 光晕）效果。
    Bloom,
    /// 屏幕空间环境光遮蔽。
    AmbientOcclusion,
    /// 距离雾。
    Fog,
    /// 色调映射（HDR → LDR）。
    ToneMapping,
    /// 颜色校正 / 分级。
    ColorCorrection,
}

/// Bloom 效果参数。
/// 映射到 CesiumJS `PostProcessStageLibrary.createBloomStage()`
#[derive(Debug, Clone, PartialEq)]
pub struct BloomConfig {
    /// bloom 是否启用。
    pub enabled: bool,
    /// bloom 强度（0.0 = 无 bloom）。
    pub intensity: f64,
    /// bloom 的亮度阈值（比它更亮的像素发光）。
    pub threshold: f64,
    /// 模糊半径（以像素计）。
    pub blur_radius: f64,
    /// 模糊 pass 数量（越多 = 越平滑但越慢）。
    pub blur_passes: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            threshold: 0.8,
            blur_radius: 4.0,
            blur_passes: 4,
        }
    }
}

impl BloomConfig {
    /// 计算给定像素亮度下的 bloom 贡献。
    ///
    /// 返回 bloom 强度乘子（低于阈值时为 0.0）。
    pub fn compute_bloom(&self, luminance: f64) -> f64 {
        if !self.enabled || luminance <= self.threshold {
            return 0.0;
        }
        let excess = luminance - self.threshold;
        excess * self.intensity
    }
}

/// 环境光遮蔽参数。
/// 映射到 CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage()`
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientOcclusionConfig {
    /// AO 是否启用。
    pub enabled: bool,
    /// AO 强度（0.0 = 无变暗，1.0 = 完全变暗）。
    pub intensity: f64,
    /// 采样半径（世界单位）。
    pub sample_radius: f64,
    /// 每像素采样数。
    pub sample_count: u32,
    /// 用于避免自遮蔽伪影的 bias。
    pub bias: f64,
    /// AO 射线的长度上限。
    pub length_cap: f64,
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 3.0,
            sample_radius: 0.5,
            sample_count: 16,
            bias: 0.001,
            length_cap: 0.26,
        }
    }
}

impl AmbientOcclusionConfig {
    /// 计算给定遮蔽比例（0.0 = 无遮蔽，1.0 = 完全遮蔽）下的 AO 因子。
    ///
    /// 返回一个处于 [0.0, 1.0] 的乘子，用于应用到像素颜色。
    pub fn compute_ao(&self, occlusion_ratio: f64) -> f64 {
        if !self.enabled {
            return 1.0;
        }
        let ao = 1.0 - occlusion_ratio.clamp(0.0, 1.0) * self.intensity;
        ao.clamp(0.0, 1.0)
    }
}

/// 雾效果参数。
/// 映射到 CesiumJS `Scene/Fog.js`
#[derive(Debug, Clone, PartialEq)]
pub struct FogConfig {
    /// 雾是否启用。
    pub enabled: bool,
    /// 表面的雾密度。
    pub density: f64,
    /// 雾颜色（RGB，0-1 范围）。
    pub color: DVec3,
    /// 最小可见距离（米）。
    pub minimum_distance: f64,
    /// 最大可见距离（米，超过此值雾完全不透明）。
    pub maximum_distance: f64,
    /// 是否使用基于屏幕空间误差的雾密度。
    pub use_sse_based_density: bool,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            density: 2.0e-4,
            color: DVec3::new(0.8, 0.85, 0.9),
            minimum_distance: 100.0,
            maximum_distance: 100_000_000.0,
            use_sse_based_density: true,
        }
    }
}

impl FogConfig {
    /// 计算给定距相机距离下的雾因子。
    ///
    /// 返回一个处于 [0.0, 1.0] 的值，0.0 = 无雾，1.0 = 完全被雾遮蔽。
    pub fn compute_fog_factor(&self, distance: f64) -> f64 {
        if !self.enabled {
            return 0.0;
        }

        if distance <= self.minimum_distance {
            return 0.0;
        }

        // 指数雾：factor = 1 - exp(-density * distance)
        let fog = 1.0 - (-self.density * distance).exp();
        fog.clamp(0.0, 1.0)
    }

    /// 根据距离将像素颜色与雾颜色混合。
    ///
    /// # 参数
    /// * `pixel_color` - 原始像素颜色（RGB）
    /// * `distance` - 相机到像素的距离
    ///
    /// # 返回
    /// 施加雾后的像素颜色
    pub fn apply_fog(&self, pixel_color: DVec3, distance: f64) -> DVec3 {
        let factor = self.compute_fog_factor(distance);
        pixel_color.lerp(self.color, factor)
    }
}

/// 色调映射算子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMappingOperator {
    /// 无色调映射（线性）。
    None,
    /// Reinhard 色调映射。
    Reinhard,
    /// ACES Filmic 色调映射。
    AcesFilmic,
    /// Uncharted 2 色调映射。
    Uncharted2,
}

/// 色调映射配置。
#[derive(Debug, Clone, PartialEq)]
pub struct ToneMappingConfig {
    /// 使用的色调映射算子。
    pub operator: ToneMappingOperator,
    /// 曝光值。
    pub exposure: f64,
    /// 白点（用于 Reinhard）。
    pub white_point: f64,
}

impl Default for ToneMappingConfig {
    fn default() -> Self {
        Self {
            operator: ToneMappingOperator::AcesFilmic,
            exposure: 1.0,
            white_point: 1.0,
        }
    }
}

impl ToneMappingConfig {
    /// 将色调映射应用到一个 HDR 颜色值。
    ///
    /// # 参数
    /// * `hdr_color` - HDR 颜色（可超过 1.0）
    ///
    /// # 返回
    /// 色调映射后的 LDR 颜色（0.0 到 1.0）
    pub fn apply(&self, hdr_color: DVec3) -> DVec3 {
        let exposed = hdr_color * self.exposure;

        match self.operator {
            ToneMappingOperator::None => exposed,
            ToneMappingOperator::Reinhard => self.reinhard(exposed),
            ToneMappingOperator::AcesFilmic => self.aces_filmic(exposed),
            ToneMappingOperator::Uncharted2 => self.uncharted2(exposed),
        }
    }

    fn reinhard(&self, color: DVec3) -> DVec3 {
        let white_sq = self.white_point * self.white_point;
        DVec3::new(
            color.x * (1.0 + color.x / white_sq) / (1.0 + color.x),
            color.y * (1.0 + color.y / white_sq) / (1.0 + color.y),
            color.z * (1.0 + color.z / white_sq) / (1.0 + color.z),
        )
    }

    fn aces_filmic(&self, color: DVec3) -> DVec3 {
        // 由 Krzysztof Narkowicz 提出的 ACES 近似
        const A: f64 = 2.51;
        const B: f64 = 0.03;
        const C: f64 = 2.43;
        const D: f64 = 0.59;
        const E: f64 = 0.14;

        DVec3::new(
            aces_curve(color.x, A, B, C, D, E),
            aces_curve(color.y, A, B, C, D, E),
            aces_curve(color.z, A, B, C, D, E),
        )
    }

    fn uncharted2(&self, color: DVec3) -> DVec3 {
        DVec3::new(
            uncharted2_curve(color.x),
            uncharted2_curve(color.y),
            uncharted2_curve(color.z),
        )
    }
}

fn aces_curve(x: f64, a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

fn uncharted2_curve(x: f64) -> f64 {
    const A: f64 = 0.15;
    const B: f64 = 0.50;
    const C: f64 = 0.10;
    const D: f64 = 0.20;
    const E: f64 = 0.02;
    const F: f64 = 0.30;

    ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F) - E / F).clamp(0.0, 1.0)
}

/// 颜色校正 / 分级参数。
#[derive(Debug, Clone, PartialEq)]
pub struct ColorCorrectionConfig {
    /// 颜色校正是否启用。
    pub enabled: bool,
    /// 亮度调整（-1 到 1）。
    pub brightness: f64,
    /// 对比度调整（0 = 平淡，1 = 正常，2 = 高对比）。
    pub contrast: f64,
    /// 饱和度调整（0 = 灰度，1 = 正常，2 = 过饱和）。
    pub saturation: f64,
    /// 色相旋转（弧度）。
    pub hue: f64,
}

impl Default for ColorCorrectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
        }
    }
}

impl ColorCorrectionConfig {
    /// 将颜色校正应用到一个像素颜色。
    pub fn apply(&self, color: DVec3) -> DVec3 {
        if !self.enabled {
            return color;
        }

        let mut result = color;

        // 亮度
        result += DVec3::splat(self.brightness);

        // 对比度（围绕 0.5 中点）
        result = (result - DVec3::splat(0.5)) * self.contrast + DVec3::splat(0.5);

        // 饱和度
        let luminance = 0.2126 * result.x + 0.7152 * result.y + 0.0722 * result.z;
        result = DVec3::splat(luminance).lerp(result, self.saturation);

        // 限制到有效范围
        result.clamp(DVec3::ZERO, DVec3::ONE)
    }
}

/// 完整的后处理流水线配置。
#[derive(Debug, Clone, Default)]
pub struct PostProcessPipeline {
    /// bloom 阶段。
    pub bloom: BloomConfig,
    /// 环境光遮蔽阶段。
    pub ambient_occlusion: AmbientOcclusionConfig,
    /// 雾阶段。
    pub fog: FogConfig,
    /// 色调映射阶段。
    pub tone_mapping: ToneMappingConfig,
    /// 颜色校正阶段。
    pub color_correction: ColorCorrectionConfig,
}

impl PostProcessPipeline {
    /// 创建一个使用默认设置的新流水线。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回按执行顺序排列的已启用阶段类型列表。
    pub fn enabled_stages(&self) -> Vec<PostProcessStageType> {
        let mut stages = Vec::new();

        if self.ambient_occlusion.enabled {
            stages.push(PostProcessStageType::AmbientOcclusion);
        }
        if self.bloom.enabled {
            stages.push(PostProcessStageType::Bloom);
        }
        if self.fog.enabled {
            stages.push(PostProcessStageType::Fog);
        }
        if self.tone_mapping.operator != ToneMappingOperator::None {
            stages.push(PostProcessStageType::ToneMapping);
        }
        if self.color_correction.enabled {
            stages.push(PostProcessStageType::ColorCorrection);
        }

        stages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_below_threshold() {
        let bloom = BloomConfig {
            enabled: true,
            threshold: 0.8,
            intensity: 1.0,
            ..Default::default()
        };

        assert_eq!(bloom.compute_bloom(0.5), 0.0);
        assert_eq!(bloom.compute_bloom(0.8), 0.0);
    }

    #[test]
    fn test_bloom_above_threshold() {
        let bloom = BloomConfig {
            enabled: true,
            threshold: 0.8,
            intensity: 2.0,
            ..Default::default()
        };

        let result = bloom.compute_bloom(1.0);
        assert!((result - 0.4).abs() < 1e-10); // (1.0 - 0.8) * 2.0
    }

    #[test]
    fn test_bloom_disabled() {
        let bloom = BloomConfig::default(); // 默认禁用
        assert_eq!(bloom.compute_bloom(10.0), 0.0);
    }

    #[test]
    fn test_ao_no_occlusion() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 3.0,
            ..Default::default()
        };

        assert_eq!(ao.compute_ao(0.0), 1.0);
    }

    #[test]
    fn test_ao_full_occlusion() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 3.0,
            ..Default::default()
        };

        // 完全遮蔽，强度 3.0 → 被限制为 0.0
        assert_eq!(ao.compute_ao(1.0), 0.0);
    }

    #[test]
    fn test_ao_partial() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 1.0,
            ..Default::default()
        };

        let result = ao.compute_ao(0.5);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fog_near() {
        let fog = FogConfig::default();
        assert_eq!(fog.compute_fog_factor(50.0), 0.0); // 低于最小距离
    }

    #[test]
    fn test_fog_far() {
        let fog = FogConfig {
            enabled: true,
            density: 1.0e-3,
            ..Default::default()
        };

        let factor = fog.compute_fog_factor(10000.0);
        assert!(factor > 0.99); // 几乎完全被雾遮蔽
    }

    #[test]
    fn test_fog_apply() {
        let fog = FogConfig {
            enabled: true,
            density: 1.0e-3,
            color: DVec3::new(1.0, 1.0, 1.0),
            ..Default::default()
        };

        let pixel = DVec3::new(0.0, 0.0, 0.0);
        let result = fog.apply_fog(pixel, 10000.0);

        // 应主要为白色（雾颜色）
        assert!(result.x > 0.9);
        assert!(result.y > 0.9);
        assert!(result.z > 0.9);
    }

    #[test]
    fn test_tone_mapping_reinhard() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::Reinhard,
            exposure: 1.0,
            white_point: 100.0, // 很大的白点 ≈ 简单 Reinhard
        };

        let hdr = DVec3::new(2.0, 2.0, 2.0);
        let ldr = config.apply(hdr);

        // 简单 Reinhard：x / (1 + x) = 2 / 3 ≈ 0.667
        assert!((ldr.x - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_tone_mapping_aces() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::AcesFilmic,
            exposure: 1.0,
            white_point: 1.0,
        };

        let hdr = DVec3::new(1.0, 1.0, 1.0);
        let ldr = config.apply(hdr);

        // ACES 应将 1.0 映射为小于 1.0 的值
        assert!(ldr.x < 1.0);
        assert!(ldr.x > 0.0);
    }

    #[test]
    fn test_tone_mapping_none() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::None,
            exposure: 1.0,
            white_point: 1.0,
        };

        let hdr = DVec3::new(0.5, 0.7, 0.9);
        let ldr = config.apply(hdr);

        assert!((ldr - hdr).length() < 1e-10);
    }

    #[test]
    fn test_color_correction_brightness() {
        let cc = ColorCorrectionConfig {
            enabled: true,
            brightness: 0.1,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
        };

        let color = DVec3::new(0.5, 0.5, 0.5);
        let result = cc.apply(color);

        assert!((result.x - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_color_correction_saturation_zero() {
        let cc = ColorCorrectionConfig {
            enabled: true,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 0.0, // 灰度
            hue: 0.0,
        };

        let color = DVec3::new(1.0, 0.0, 0.0);
        let result = cc.apply(color);

        // 应为灰度（各通道相等）
        assert!((result.x - result.y).abs() < 1e-10);
        assert!((result.y - result.z).abs() < 1e-10);
    }

    #[test]
    fn test_pipeline_enabled_stages() {
        let mut pipeline = PostProcessPipeline::new();
        pipeline.bloom.enabled = true;
        pipeline.fog.enabled = true;

        let stages = pipeline.enabled_stages();

        assert!(stages.contains(&PostProcessStageType::Bloom));
        assert!(stages.contains(&PostProcessStageType::Fog));
        assert!(stages.contains(&PostProcessStageType::ToneMapping)); // 默认为 ACES
    }

    #[test]
    fn test_pipeline_default_stages() {
        let pipeline = PostProcessPipeline::new();
        let stages = pipeline.enabled_stages();

        // 默认情况下：仅雾与色调映射启用
        assert_eq!(stages.len(), 2);
        assert!(stages.contains(&PostProcessStageType::Fog));
        assert!(stages.contains(&PostProcessStageType::ToneMapping));
    }
}
