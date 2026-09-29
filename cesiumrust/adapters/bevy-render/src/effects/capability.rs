//! M11.6 / FIX-CAPPROBE（scoped）：GPU **能力探针** + **质量档** 阶梯。
//!
//! 移植计划（待开展工作.md §阶段五 / FIX-CAPPROBE）要求 "每能力 GPU
//! capability probe + quality tier 降级 + tier=off 回 baseline±3%"。在本
//! 模块之前，唯一类似于探针的东西是 [`graph`] 的
//! [`PassThroughNode`](graph::PassThroughNode) 中那单行
//! `if !pass_through.enabled { return; }` —— 一个组件标志，而非设备
//! 查询，且没有 tier 枚举或降级阶梯。
//!
//! # 哪些是设备无关的（本模块，可无头测试）
//!
//! **决策核心**：给定一个 [`DeviceCapabilitySnapshot`]（对 adapter/device
//! 所提供能力的一份朴素、wgpu 无关的概要），选出一个 [`QualityTier`]，
//! 并把该 tier 翻译成逐能力的降级因子。这是纯
//! 算术 + 分支 —— 它不需要 GPU、不需要 `RenderDevice`、不需要窗口，所以
//! 它的单元测试在无头测试配置下运行，并由强制的
//! `cargo test --workspace` PR 门禁加以验证。
//!
//! # 哪些是设备相关的（暂缓，需要真实硬件）
//!
//! 有两件事没有 GPU 无法验证，因此在此**不**
//! 臆造（参见 `docs/deferred.md#68`，以及项目教训：无头的优雅降级会*掩盖*
//! 真实 GPU 的缺陷 —— 所以 gate-ON 的 GPU
//! 取证是一道强制的、独立的门禁）：
//!
//! 1. 一个 `RenderDevice`→[`DeviceCapabilitySnapshot`] 构造函数（`snapshot_from_
//!    device`），读取活动设备的 `features()` / `limits()`。它在此
//!    被刻意**不**桩化：它在具体消费级 GPU 上返回的 tier
//!    只能在真实硬件上观测，且在没有设备可供校验的情况下命名 wgpu feature
//!    位，恰好会招致 DEV-029 所记录的那种静默、永远绿的失败。它与 (2) 一同落地。
//! 2. 重新接线渲染节点的 `run()` 控制流以*根据 tier 门控渲染*，
//!    并证明计划中的 "tier=off → baseline ±3%" 性能
//!    契约，需要一个真实设备 + 帧时间仪表化
//!    （`xvfb`+llvmpipe / GPU runner，M11.2/M11.3）。在此之前节点保持
//!    其现有的 enabled-check 行为 —— 探针核心作为一个
//!    已测试、可达的构建块提供，供 finish 时的接线消费。
//!
//! 本模块刻意**没有** `bevy` / `wgpu` / `RenderDevice` 导入：
//! 决策核心是纯的，所以它的测试在强制的 `cargo test --workspace` 门禁下
//! 无头地遍历每一个 tier 分支。

/// 一个粗粒度的 GPU 质量档，按从最强→最弱能力排序，外加一个 `Off`
/// 终态，意为“回退到像素中性的 baseline”。
///
/// 序数顺序有意义且刻意固定（`Off < Low < Medium <
/// High` *不是*声明顺序 —— 能力排序请使用 [`QualityTier::rank`]，
/// rank 越高 = 能力越强）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QualityTier {
    /// 回退到 baseline：该能力本帧禁用。
    Off,
    /// 最小特性集；最重的降级（少量采样 / 少量步数）。
    Low,
    /// 典型的独立 / 现代集成 GPU；部分降级。
    Medium,
    /// 完整特性集 + 宽裕的 limits；无降级。
    High,
}

impl QualityTier {
    /// 能力 rank：越高 = 能力越强。`Off` = 0 … `High` = 3。
    pub const fn rank(self) -> u8 {
        match self {
            QualityTier::Off => 0,
            QualityTier::Low => 1,
            QualityTier::Medium => 2,
            QualityTier::High => 3,
        }
    }
}

/// 对探针所关心的能力的一份 wgpu 无关的概要。字段朴素，以便
/// 决策核心及其测试没有设备 / 框架依赖。
///
/// 由 [`snapshot_from_device`] 从活动设备构造，或在
/// 测试中手工构造以表示假设的 adapter。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceCapabilitySnapshot {
    /// 当根本没有 `RenderDevice` 时为 `false`（无头）：探针无法
    /// 查询任何东西，所以它必须保守地降级到
    /// [`QualityTier::Off`]，而不是猜测。
    pub device_present: bool,
    /// `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` / 可浮点滤波的
    /// 颜色 —— HDR 线性域 pass（AO / IBL 回读）所必需。
    pub float32_filterable: bool,
    /// `Features::STORAGE_RESOURCE` —— 较重的 AO / IBL 路径使用的
    /// compute 风格累加。
    pub storage_textures: bool,
    /// `Features::ANISOTROPIC_FILTERING` —— 一个“锦上添花”，只会抬升到
    /// [`QualityTier::High`]，从不门控较低的 tier。
    pub anisotropic_filtering: bool,
    /// `Limits::max_texture_dimension_2d`。
    pub max_texture_dimension_2d: u32,
}

impl DeviceCapabilitySnapshot {
    /// 当**无** `RenderDevice` 存在时（无头 / `MinimalPlugins`）采取的
    /// 保守快照：一切缺失 → [`QualityTier::Off`]。
    pub const ABSENT_DEVICE: DeviceCapabilitySnapshot = DeviceCapabilitySnapshot {
        device_present: false,
        float32_filterable: false,
        storage_textures: false,
        anisotropic_filtering: false,
        max_texture_dimension_2d: 0,
    };
}

/// Medium tier 仍能容忍的一个保守单纹理尺寸；High
/// tier 至少需要四倍于此（8192），Low 降到 2048。
const HIGH_MIN_TEX: u32 = 8192;
const MEDIUM_MIN_TEX: u32 = 4096;
const LOW_MIN_TEX: u32 = 2048;

/// 纯决策核心：将 [`DeviceCapabilitySnapshot`] 映射为一个 [`QualityTier`]。
///
/// 规则（有文档、确定性、可无头测试）：
/// - 无设备 → [`QualityTier::Off`]（从不猜测一个缺失的 adapter）。
/// - `High`：可浮点滤波 **且** storage 纹理 **且** ≥ [`HIGH_MIN_TEX`]
///   纹理尺寸（各向异性只增添质感，不做门控）。
/// - `Medium`：可浮点滤波 **且** ≥ [`MEDIUM_MIN_TEX`]（storage 可选）。
/// - `Low`：任何能采样 ≥ [`LOW_MIN_TEX`] 纹理的设备。
/// - 更弱的任何东西 → `Off`（宁可回退 baseline，也不渲染出错）。
pub fn probe_quality_tier(snapshot: &DeviceCapabilitySnapshot) -> QualityTier {
    if !snapshot.device_present {
        return QualityTier::Off;
    }
    if snapshot.float32_filterable
        && snapshot.storage_textures
        && snapshot.max_texture_dimension_2d >= HIGH_MIN_TEX
    {
        return QualityTier::High;
    }
    if snapshot.float32_filterable && snapshot.max_texture_dimension_2d >= MEDIUM_MIN_TEX {
        return QualityTier::Medium;
    }
    if snapshot.max_texture_dimension_2d >= LOW_MIN_TEX {
        return QualityTier::Low;
    }
    QualityTier::Off
}

/// 某 tier 的 AO 半球采样数，从 `base`（领域默认 16）缩放而来。
/// `Off` 会完全禁用该 pass（返回 0 → 调用方提前 return）。
pub fn ao_sample_count(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => (base / 4).max(1),
        QualityTier::Off => 0,
    }
}

/// 某 tier 的 FXAA 边缘搜索步数，从 preset-12 的 `base`（5）缩放而来。
/// `Off` 返回 0（跳过 FXAA —— 它是最后一个 LDR 节点，跳过是安全的）。
pub fn fxaa_steps(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => 1,
        QualityTier::Off => 0,
    }
}

/// 某 tier 的 IBL 预滤波 mip 数，从 `base`（例如 5）缩放而来。
/// `Off` 返回 0（镜面 IBL 禁用；漫反射 SH 仍是可接受的 baseline）。
pub fn ibl_mip_levels(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => 1,
        QualityTier::Off => 0,
    }
}

// GPU 边界构造函数 `snapshot_from_device(&RenderDevice) -> DeviceCapabilitySnapshot`
// 刻意不在此定义 —— 它是暂缓的 GPU 接线的一部分
//（`docs/deferred.md#68`），参见模块 doc。它本会馈入的纯 `probe_quality_tier`
// 决策在下方已得到完整的无头测试。

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(device_present: bool, f32f: bool, storage: bool, tex: u32) -> DeviceCapabilitySnapshot {
        DeviceCapabilitySnapshot {
            device_present,
            float32_filterable: f32f,
            storage_textures: storage,
            anisotropic_filtering: false,
            max_texture_dimension_2d: tex,
        }
    }

    #[test]
    fn absent_device_probes_off_never_guesses() {
        assert_eq!(probe_quality_tier(&DeviceCapabilitySnapshot::ABSENT_DEVICE), QualityTier::Off);
        // device_present=false 占主导，即使其他字段看起来很能干。
        assert_eq!(probe_quality_tier(&snap(false, true, true, 16384)), QualityTier::Off);
    }

    #[test]
    fn full_device_probes_high() {
        assert_eq!(probe_quality_tier(&snap(true, true, true, 8192)), QualityTier::High);
        assert_eq!(probe_quality_tier(&snap(true, true, true, 16384)), QualityTier::High);
    }

    #[test]
    fn tier_boundaries_are_monotonic_in_capability() {
        // storage 缺失或 tex 低于 HIGH_MIN_TEX → 从 High 跌出。
        assert_eq!(probe_quality_tier(&snap(true, true, false, 8192)), QualityTier::Medium);
        assert_eq!(probe_quality_tier(&snap(true, true, true, 4096)), QualityTier::Medium);
        // 可浮点滤波缺失 → 从 Medium 跌入 Low。
        assert_eq!(probe_quality_tier(&snap(true, false, true, 4096)), QualityTier::Low);
        // tex 低于 LOW_MIN_TEX → Off。
        assert_eq!(probe_quality_tier(&snap(true, false, false, 1024)), QualityTier::Off);
    }

    #[test]
    fn degrade_scales_are_monotonic_and_off_disables() {
        let tiers = [QualityTier::High, QualityTier::Medium, QualityTier::Low, QualityTier::Off];
        let ao: Vec<u32> = tiers.iter().map(|&t| ao_sample_count(t, 16)).collect();
        let fx: Vec<u32> = tiers.iter().map(|&t| fxaa_steps(t, 5)).collect();
        let ib: Vec<u32> = tiers.iter().map(|&t| ibl_mip_levels(t, 5)).collect();
        // 随 tier 下降而非递增。
        for v in [&ao, &fx, &ib] {
            assert!(v[0] >= v[1] && v[1] >= v[2] && v[2] >= v[3], "degrade must be monotonic: {v:?}");
        }
        // Off 禁用全部三项；最高 tier 是对 base 无降级的透传。
        assert_eq!((ao[3], fx[3], ib[3]), (0, 0, 0), "Off must disable the capability");
        assert_eq!((ao[0], fx[0], ib[0]), (16, 5, 5), "High must pass through the base unchanged");
    }

    #[test]
    fn low_medium_never_reach_zero_when_enabled() {
        // 一个真实（哪怕弱）的设备仍至少以 ≥1 采样/步/mip 渲染；只有 Off 归零。
        for &base in &[8u32, 16, 32] {
            for &tier in &[QualityTier::High, QualityTier::Medium, QualityTier::Low] {
                assert!(ao_sample_count(tier, base) >= 1);
            }
        }
        for &tier in &[QualityTier::High, QualityTier::Medium, QualityTier::Low] {
            assert!(fxaa_steps(tier, 5) >= 1);
            assert!(ibl_mip_levels(tier, 5) >= 1);
        }
    }

    #[test]
    fn rank_orders_high_above_off() {
        assert!(QualityTier::High.rank() > QualityTier::Medium.rank());
        assert!(QualityTier::Medium.rank() > QualityTier::Low.rank());
        assert!(QualityTier::Low.rank() > QualityTier::Off.rank());
    }
}
