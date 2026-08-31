//! Ported from `packages/engine/Source/DataSources/StripeMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};
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

/// Port of the CesiumJS `Property` facet of `StripeMaterialProperty`.
impl Property for StripeMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("Stripe".to_string())
    }

    fn is_constant(&self) -> bool {
        true
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<StripeMaterialProperty>())
            .map(|other| {
                self.even_color.red == other.even_color.red
                    && self.even_color.green == other.even_color.green
                    && self.even_color.blue == other.even_color.blue
                    && self.even_color.alpha == other.even_color.alpha
                    && self.odd_color.red == other.odd_color.red
                    && self.odd_color.green == other.odd_color.green
                    && self.odd_color.blue == other.odd_color.blue
                    && self.odd_color.alpha == other.odd_color.alpha
                    && self.orientation == other.orientation
                    && self.repeat == other.repeat
            })
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Stripe")
    }
}
