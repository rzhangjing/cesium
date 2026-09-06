//! Ported from `packages/engine/Source/Scene/BlendingState.js`.
//!
//! The blending state combines [`BlendEquation`] and [`BlendFunction`] and the
//! `enabled` flag to define the full blending state for combining source and
//! destination fragments when rendering.

use crate::blend_equation::BlendEquation;
use crate::blend_function::BlendFunction;

/// A complete blending configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendingStateConfig {
    /// Whether blending is enabled.
    pub enabled: bool,
    /// The RGB blend equation.
    pub equation_rgb: BlendEquation,
    /// The alpha blend equation.
    pub equation_alpha: BlendEquation,
    /// The source RGB blend function.
    pub function_source_rgb: BlendFunction,
    /// The source alpha blend function.
    pub function_source_alpha: BlendFunction,
    /// The destination RGB blend function.
    pub function_destination_rgb: BlendFunction,
    /// The destination alpha blend function.
    pub function_destination_alpha: BlendFunction,
}

/// Predefined blending state constants.
pub struct BlendingState;

impl BlendingState {
    /// Blending is disabled.
    pub const DISABLED: BlendingStateConfig = BlendingStateConfig {
        enabled: false,
        equation_rgb: BlendEquation::Add,
        equation_alpha: BlendEquation::Add,
        function_source_rgb: BlendFunction::Zero,
        function_source_alpha: BlendFunction::Zero,
        function_destination_rgb: BlendFunction::Zero,
        function_destination_alpha: BlendFunction::Zero,
    };

    /// Alpha blending: `source(source.alpha) + destination(1 - source.alpha)`.
    pub const ALPHA_BLEND: BlendingStateConfig = BlendingStateConfig {
        enabled: true,
        equation_rgb: BlendEquation::Add,
        equation_alpha: BlendEquation::Add,
        function_source_rgb: BlendFunction::SourceAlpha,
        function_source_alpha: BlendFunction::One,
        function_destination_rgb: BlendFunction::OneMinusSourceAlpha,
        function_destination_alpha: BlendFunction::OneMinusSourceAlpha,
    };

    /// Premultiplied alpha blending: `source + destination(1 - source.alpha)`.
    pub const PRE_MULTIPLIED_ALPHA_BLEND: BlendingStateConfig = BlendingStateConfig {
        enabled: true,
        equation_rgb: BlendEquation::Add,
        equation_alpha: BlendEquation::Add,
        function_source_rgb: BlendFunction::One,
        function_source_alpha: BlendFunction::One,
        function_destination_rgb: BlendFunction::OneMinusSourceAlpha,
        function_destination_alpha: BlendFunction::OneMinusSourceAlpha,
    };

    /// Additive blending: `source(source.alpha) + destination`.
    pub const ADDITIVE_BLEND: BlendingStateConfig = BlendingStateConfig {
        enabled: true,
        equation_rgb: BlendEquation::Add,
        equation_alpha: BlendEquation::Add,
        function_source_rgb: BlendFunction::SourceAlpha,
        function_source_alpha: BlendFunction::One,
        function_destination_rgb: BlendFunction::One,
        function_destination_alpha: BlendFunction::One,
    };
}
