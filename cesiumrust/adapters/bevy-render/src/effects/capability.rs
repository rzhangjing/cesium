//! M11.6 / FIX-CAPPROBE (scoped): GPU **capability probe** + **quality-tier** ladder.
//!
//! The porting plan (待开展工作.md §阶段五 / FIX-CAPPROBE) requires "每能力 GPU
//! capability probe + quality tier 降级 + tier=off 回 baseline±3%". Before this
//! module the only thing resembling a probe was the single
//! `if !pass_through.enabled { return; }` line in [`graph`]'s
//! [`PassThroughNode`](graph::PassThroughNode) — a component flag, not a device
//! query, and with no tier enum or fallback ladder.
//!
//! # What is device-independent (this module, headless-testable)
//!
//! The **decision core**: given a [`DeviceCapabilitySnapshot`] (a plain,
//! wgpu-free summary of what the adapter/device offers) pick a [`QualityTier`],
//! and translate that tier into per-capability degrade factors. This is pure
//! arithmetic + branching — it needs no GPU, no `RenderDevice`, no window, so
//! its unit tests run under the headless test profile and are exercised by the
//! mandatory `cargo test --workspace` PR gate.
//!
//! # What is device-dependent (deferred, needs real hardware)
//!
//! Two things cannot be validated without a GPU and are therefore **not**
//! fabricated here (see `docs/deferred.md#68`, and the project lesson that
//! headless graceful-degradation *masks* real-GPU defects — so gate-ON GPU
//! 取证 is a mandatory, separate gate):
//!
//! 1. A `RenderDevice`→[`DeviceCapabilitySnapshot`] constructor (`snapshot_from_
//!    device`) that reads the live device's `features()` / `limits()`. It is
//!    deliberately **not** stubbed here: its returned tier on a concrete
//!    consumer GPU is only observable on real hardware, and naming wgpu feature
//!    bits without a device to validate against invites exactly the silent
//!    always-green failure DEV-029 documents. It lands with (2).
//! 2. Rewiring the render nodes' `run()` control flow to *gate rendering on the
//!    tier*, and proving the plan's "tier=off → baseline ±3%" performance
//!    contract, require an actual device + frame-time instrumentation
//!    (`xvfb`+llvmpipe / GPU runner, M11.2/M11.3). Until then the nodes keep
//!    their existing enabled-check behaviour — the probe core is provided as a
//!    tested, reachable building block the finish-time wiring will consume.
//!
//! This module intentionally has **no** `bevy` / `wgpu` / `RenderDevice` imports:
//! the decision core is pure, so its tests exercise every tier branch headlessly
//! under the mandatory `cargo test --workspace` gate.

/// A coarse GPU quality tier, ordered most → least capable, plus an `Off`
/// terminal that means "fall back to the pixel-neutral baseline".
///
/// Ordinal order is meaningful and deliberately fixed (`Off < Low < Medium <
/// High` is *not* the declaration order — use [`QualityTier::rank`] for the
/// capability ordering, higher rank = more capable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QualityTier {
    /// Fall back to baseline: the capability is disabled for this frame.
    Off,
    /// Minimal feature set; heaviest degrade (few samples / few steps).
    Low,
    /// Typical discrete / modern integrated GPU; partial degrade.
    Medium,
    /// Full feature set + generous limits; no degrade.
    High,
}

impl QualityTier {
    /// Capability rank: higher = more capable. `Off` = 0 … `High` = 3.
    pub const fn rank(self) -> u8 {
        match self {
            QualityTier::Off => 0,
            QualityTier::Low => 1,
            QualityTier::Medium => 2,
            QualityTier::High => 3,
        }
    }
}

/// A wgpu-free summary of the capabilities a probe cares about. Plain fields so
/// the decision core and its tests have no device / framework dependency.
///
/// Constructed from a live device by [`snapshot_from_device`], or hand-built in
/// tests to represent hypothetical adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceCapabilitySnapshot {
    /// `false` when there is no `RenderDevice` at all (headless): the probe can
    /// interrogate nothing, so it must degrade conservatively to
    /// [`QualityTier::Off`] rather than guess.
    pub device_present: bool,
    /// `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` / float-filterable
    /// colour — required for HDR linear-domain passes (AO / IBL readback).
    pub float32_filterable: bool,
    /// `Features::STORAGE_RESOURCE` — compute-style accumulation used by the
    /// heavier AO / IBL paths.
    pub storage_textures: bool,
    /// `Features::ANISOTROPIC_FILTERING` — a "nice to have" that only bumps to
    /// [`QualityTier::High`], never gates the lower tiers.
    pub anisotropic_filtering: bool,
    /// `Limits::max_texture_dimension_2d`.
    pub max_texture_dimension_2d: u32,
}

impl DeviceCapabilitySnapshot {
    /// The conservative snapshot taken when **no** `RenderDevice` exists
    /// (headless / `MinimalPlugins`): everything absent → [`QualityTier::Off`].
    pub const ABSENT_DEVICE: DeviceCapabilitySnapshot = DeviceCapabilitySnapshot {
        device_present: false,
        float32_filterable: false,
        storage_textures: false,
        anisotropic_filtering: false,
        max_texture_dimension_2d: 0,
    };
}

/// A conservative single-texture size that the Medium tier still tolerates; the
/// High tier wants at least four times that (8192), Low drops to 2048.
const HIGH_MIN_TEX: u32 = 8192;
const MEDIUM_MIN_TEX: u32 = 4096;
const LOW_MIN_TEX: u32 = 2048;

/// Pure decision core: map a [`DeviceCapabilitySnapshot`] to a [`QualityTier`].
///
/// Rules (documented, deterministic, headless-testable):
/// - No device → [`QualityTier::Off`] (never guess an absent adapter).
/// - `High`: float-filterable **and** storage textures **and** ≥ [`HIGH_MIN_TEX`]
///   texture dimension (anisotropic only adds polish, does not gate).
/// - `Medium`: float-filterable **and** ≥ [`MEDIUM_MIN_TEX`] (storage optional).
/// - `Low`: any device that can sample ≥ [`LOW_MIN_TEX`] textures at all.
/// - Anything weaker → `Off` (fall back to baseline rather than render wrong).
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

/// AO hemisphere-sample count for a tier, scaled from the `base` (domain default
/// 16). `Off` disables the pass entirely (returns 0 → caller early-returns).
pub fn ao_sample_count(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => (base / 4).max(1),
        QualityTier::Off => 0,
    }
}

/// FXAA edge-search steps for a tier, scaled from the preset-12 `base` (5).
/// `Off` returns 0 (skip FXAA — it is the last LDR node, skipping is safe).
pub fn fxaa_steps(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => 1,
        QualityTier::Off => 0,
    }
}

/// IBL prefiltered-mip count for a tier, scaled from the `base` (e.g. 5).
/// `Off` returns 0 (specular IBL disabled; diffuse SH still acceptable baseline).
pub fn ibl_mip_levels(tier: QualityTier, base: u32) -> u32 {
    match tier {
        QualityTier::High => base,
        QualityTier::Medium => (base / 2).max(1),
        QualityTier::Low => 1,
        QualityTier::Off => 0,
    }
}

// GPU-boundary constructor `snapshot_from_device(&RenderDevice) -> DeviceCapabilitySnapshot`
// is intentionally **not** defined here — it is part of the deferred GPU wiring
// (`docs/deferred.md#68`), see the module doc. The pure `probe_quality_tier`
// decision it would feed is fully headless-tested below.

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
        // device_present=false dominates even if the other fields look capable.
        assert_eq!(probe_quality_tier(&snap(false, true, true, 16384)), QualityTier::Off);
    }

    #[test]
    fn full_device_probes_high() {
        assert_eq!(probe_quality_tier(&snap(true, true, true, 8192)), QualityTier::High);
        assert_eq!(probe_quality_tier(&snap(true, true, true, 16384)), QualityTier::High);
    }

    #[test]
    fn tier_boundaries_are_monotonic_in_capability() {
        // storage missing or tex below HIGH_MIN_TEX → drop out of High.
        assert_eq!(probe_quality_tier(&snap(true, true, false, 8192)), QualityTier::Medium);
        assert_eq!(probe_quality_tier(&snap(true, true, true, 4096)), QualityTier::Medium);
        // float-filterable missing → drop out of Medium into Low.
        assert_eq!(probe_quality_tier(&snap(true, false, true, 4096)), QualityTier::Low);
        // tex below LOW_MIN_TEX → Off.
        assert_eq!(probe_quality_tier(&snap(true, false, false, 1024)), QualityTier::Off);
    }

    #[test]
    fn degrade_scales_are_monotonic_and_off_disables() {
        let tiers = [QualityTier::High, QualityTier::Medium, QualityTier::Low, QualityTier::Off];
        let ao: Vec<u32> = tiers.iter().map(|&t| ao_sample_count(t, 16)).collect();
        let fx: Vec<u32> = tiers.iter().map(|&t| fxaa_steps(t, 5)).collect();
        let ib: Vec<u32> = tiers.iter().map(|&t| ibl_mip_levels(t, 5)).collect();
        // Non-increasing as tiers descend.
        for v in [&ao, &fx, &ib] {
            assert!(v[0] >= v[1] && v[1] >= v[2] && v[2] >= v[3], "degrade must be monotonic: {v:?}");
        }
        // Off disables all three; the top tier is a no-degrade passthrough of base.
        assert_eq!((ao[3], fx[3], ib[3]), (0, 0, 0), "Off must disable the capability");
        assert_eq!((ao[0], fx[0], ib[0]), (16, 5, 5), "High must pass through the base unchanged");
    }

    #[test]
    fn low_medium_never_reach_zero_when_enabled() {
        // A real (even weak) device still renders at ≥1 sample/step/mip; only Off zeroes.
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
