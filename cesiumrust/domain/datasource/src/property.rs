//! Property system for time-dynamic values.
//!
//! Maps to CesiumJS `DataSources/Property.js`, `ConstantProperty.js`,
//! `SampledProperty.js`, `TimeIntervalCollectionProperty.js`

use serde::{Deserialize, Serialize};

/// A color value in RGBA (0.0..1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    /// Creates a new color.
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self { red, green, blue, alpha }
    }

    /// White opaque.
    pub const WHITE: Self = Self { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };

    /// Black opaque.
    pub const BLACK: Self = Self { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 };

    /// Red opaque.
    pub const RED: Self = Self { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };

    /// Green opaque.
    pub const GREEN: Self = Self { red: 0.0, green: 1.0, blue: 0.0, alpha: 1.0 };

    /// Blue opaque.
    pub const BLUE: Self = Self { red: 0.0, green: 0.0, blue: 1.0, alpha: 1.0 };

    /// Yellow opaque.
    pub const YELLOW: Self = Self { red: 1.0, green: 1.0, blue: 0.0, alpha: 1.0 };

    /// Cyan opaque.
    pub const CYAN: Self = Self { red: 0.0, green: 1.0, blue: 1.0, alpha: 1.0 };

    /// Transparent.
    pub const TRANSPARENT: Self = Self { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 };

    /// Creates a color from a CSS hex string (e.g., "#FF0000" or "#FF000080").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::new(
                    r as f64 / 255.0,
                    g as f64 / 255.0,
                    b as f64 / 255.0,
                    a as f64 / 255.0,
                ))
            }
            _ => None,
        }
    }

    /// Converts to [f32; 4] for GPU use.
    pub fn to_f32_array(&self) -> [f32; 4] {
        [self.red as f32, self.green as f32, self.blue as f32, self.alpha as f32]
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

/// A property value that can be constant or time-varying.
///
/// Maps to CesiumJS `DataSources/Property.js`
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Property<T: Clone + PartialEq> {
    /// A constant value.
    Constant(T),
    /// A sampled property with time-value pairs (JulianDate seconds, value).
    Sampled(Vec<(f64, T)>),
    /// Undefined (no value).
    #[default]
    Undefined,
}

impl<T: Clone + PartialEq> Property<T> {
    /// Gets the value at a given time (seconds since epoch).
    ///
    /// For constant properties, always returns the constant.
    /// For sampled properties, returns the nearest value (no interpolation).
    pub fn get_value(&self, time: f64) -> Option<&T> {
        match self {
            Property::Constant(v) => Some(v),
            Property::Sampled(samples) => {
                if samples.is_empty() {
                    return None;
                }
                // Find nearest sample
                let mut nearest = &samples[0];
                let mut min_dist = (time - nearest.0).abs();
                for sample in samples.iter().skip(1) {
                    let dist = (time - sample.0).abs();
                    if dist < min_dist {
                        min_dist = dist;
                        nearest = sample;
                    }
                }
                Some(&nearest.1)
            }
            Property::Undefined => None,
        }
    }

    /// Returns true if this is a constant property.
    pub fn is_constant(&self) -> bool {
        matches!(self, Property::Constant(_))
    }

    /// Returns true if this property has a value.
    pub fn is_defined(&self) -> bool {
        !matches!(self, Property::Undefined)
    }
}



/// A position property (cartographic: lon, lat, height in radians/meters).
pub type PositionProperty = Property<[f64; 3]>;

/// A color property.
pub type ColorProperty = Property<Color>;

/// A numeric property.
pub type NumberProperty = Property<f64>;

/// A boolean property.
pub type BoolProperty = Property<bool>;

/// A string property.
pub type StringProperty = Property<String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF0000").unwrap();
        assert!((color.red - 1.0).abs() < 1e-10);
        assert!((color.green - 0.0).abs() < 1e-10);
        assert!((color.blue - 0.0).abs() < 1e-10);
        assert!((color.alpha - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_color_from_hex_with_alpha() {
        let color = Color::from_hex("#FF000080").unwrap();
        assert!((color.red - 1.0).abs() < 1e-10);
        assert!((color.alpha - 128.0 / 255.0).abs() < 1e-10);
    }

    #[test]
    fn test_constant_property() {
        let prop: Property<f64> = Property::Constant(42.0);
        assert!(prop.is_constant());
        assert!(prop.is_defined());
        assert_eq!(*prop.get_value(0.0).unwrap(), 42.0);
        assert_eq!(*prop.get_value(100.0).unwrap(), 42.0);
    }

    #[test]
    fn test_sampled_property() {
        let prop: Property<f64> = Property::Sampled(vec![
            (0.0, 10.0),
            (10.0, 20.0),
            (20.0, 30.0),
        ]);
        assert!(!prop.is_constant());
        assert!(prop.is_defined());

        // Nearest to time=0
        assert_eq!(*prop.get_value(0.0).unwrap(), 10.0);
        // Nearest to time=9
        assert_eq!(*prop.get_value(9.0).unwrap(), 20.0);
        // Nearest to time=20
        assert_eq!(*prop.get_value(20.0).unwrap(), 30.0);
    }

    #[test]
    fn test_undefined_property() {
        let prop: Property<f64> = Property::Undefined;
        assert!(!prop.is_defined());
        assert!(prop.get_value(0.0).is_none());
    }

    #[test]
    fn test_color_property() {
        let prop: ColorProperty = Property::Constant(Color::RED);
        let color = prop.get_value(0.0).unwrap();
        assert!((color.red - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_color_to_f32() {
        let color = Color::new(1.0, 0.5, 0.25, 1.0);
        let arr = color.to_f32_array();
        assert!((arr[0] - 1.0).abs() < 1e-6);
        assert!((arr[1] - 0.5).abs() < 1e-6);
        assert!((arr[2] - 0.25).abs() < 1e-6);
    }
}
