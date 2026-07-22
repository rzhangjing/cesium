use gpui::{Rgba, rgb};

/// Application color palette — all colors live here so every crate
/// references one source of truth instead of scattering hex literals.
pub struct AppColors;

impl AppColors {
    // ── Background ──────────────────────────────────────────────
    pub fn base() -> Rgba { rgb(0x1e1e2e) }
    pub fn surface() -> Rgba { rgb(0x313244) }
    pub fn overlay() -> Rgba { rgb(0x45475a) }

    // ── Text ────────────────────────────────────────────────────
    pub fn text() -> Rgba { rgb(0xcdd6f4) }
    pub fn text_muted() -> Rgba { rgb(0xa6adc8) }

    // ── Accent ──────────────────────────────────────────────────
    pub fn accent() -> Rgba { rgb(0x89b4fa) }
    pub fn accent_hover() -> Rgba { rgb(0x74c7ec) }

    // ── Status ──────────────────────────────────────────────────
    pub fn success() -> Rgba { rgb(0xa6e3a1) }
    pub fn warning() -> Rgba { rgb(0xf9e2af) }
    pub fn error() -> Rgba { rgb(0xf38ba8) }

    // ── Border ──────────────────────────────────────────────────
    pub fn border() -> Rgba { rgb(0x585b70) }
    pub fn border_active() -> Rgba { rgb(0x89b4fa) }
}

/// Font size presets — mirrors Zed's theme_settings approach.
pub struct FontSizes;

impl FontSizes {
    pub const SM: f32 = 12.0;
    pub const BASE: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 24.0;
}
