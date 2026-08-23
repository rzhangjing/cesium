//! Ported from `packages/engine/Source/DataSources/StripeMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;
use crate::stripe_orientation::StripeOrientation;

/// A material property that defines a stripe pattern appearance.
pub struct StripeMaterialProperty {
    /// The even color.
    pub even_color: Color,
    /// The odd color.
    pub odd_color: Color,
    /// The orientation of stripes (horizontal or vertical).
    pub orientation: StripeOrientation,
    /// The number of stripe repetitions.
    pub repeat: f64,
}

impl StripeMaterialProperty {
    /// Creates a new stripe material property.
    pub fn new() -> Self {
        Self {
            even_color: Color::new(1.0, 1.0, 1.0, 1.0),
            odd_color: Color::new(0.0, 0.0, 0.0, 1.0),
            orientation: StripeOrientation::Horizontal,
            repeat: 1.0,
        }
    }
}

impl Default for StripeMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for StripeMaterialProperty {
    fn type_name(&self) -> &str { "Stripe" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
