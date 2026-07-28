//! Color - RGBA color with CSS parsing, HSL conversion, and arithmetic.
//! Maps to CesiumJS `Core/Color.js`

use crate::math_utils;

/// A color specified using red, green, blue, and alpha values (0.0 to 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Default for Color {
    fn default() -> Self {
        Self { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }
    }
}

impl Color {
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self { red, green, blue, alpha }
    }

    // --- Named color constants (subset used in tests + common ones) ---
    pub const WHITE: Self = Self { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };
    pub const BLACK: Self = Self { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 };
    pub const RED: Self = Self { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
    pub const GREEN: Self = Self { red: 0.0, green: 0.5019607843137255, blue: 0.0, alpha: 1.0 };
    pub const LIME: Self = Self { red: 0.0, green: 1.0, blue: 0.0, alpha: 1.0 };
    pub const BLUE: Self = Self { red: 0.0, green: 0.0, blue: 1.0, alpha: 1.0 };
    pub const YELLOW: Self = Self { red: 1.0, green: 1.0, blue: 0.0, alpha: 1.0 };
    pub const CYAN: Self = Self { red: 0.0, green: 1.0, blue: 1.0, alpha: 1.0 };
    pub const MAGENTA: Self = Self { red: 1.0, green: 0.0, blue: 1.0, alpha: 1.0 };
    pub const TRANSPARENT: Self = Self { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 };
    pub const ORANGE: Self = Self { red: 1.0, green: 0.6470588235294118, blue: 0.0, alpha: 1.0 };
    pub const PURPLE: Self = Self { red: 0.5019607843137255, green: 0.0, blue: 0.5019607843137255, alpha: 1.0 };
    pub const PINK: Self = Self { red: 1.0, green: 0.7529411764705882, blue: 0.796078431372549, alpha: 1.0 };
    pub const GRAY: Self = Self { red: 0.5019607843137255, green: 0.5019607843137255, blue: 0.5019607843137255, alpha: 1.0 };
    pub const GREY: Self = Self { red: 0.5019607843137255, green: 0.5019607843137255, blue: 0.5019607843137255, alpha: 1.0 };
    pub const BROWN: Self = Self { red: 0.6470588235294118, green: 0.16470588235294117, blue: 0.16470588235294117, alpha: 1.0 };
    pub const NAVY: Self = Self { red: 0.0, green: 0.0, blue: 0.5019607843137255, alpha: 1.0 };
    pub const TEAL: Self = Self { red: 0.0, green: 0.5019607843137255, blue: 0.5019607843137255, alpha: 1.0 };
    pub const OLIVE: Self = Self { red: 0.5019607843137255, green: 0.5019607843137255, blue: 0.0, alpha: 1.0 };
    pub const MAROON: Self = Self { red: 0.5019607843137255, green: 0.0, blue: 0.0, alpha: 1.0 };
    pub const SILVER: Self = Self { red: 0.7529411764705882, green: 0.7529411764705882, blue: 0.7529411764705882, alpha: 1.0 };
    pub const AQUA: Self = Self { red: 0.0, green: 1.0, blue: 1.0, alpha: 1.0 };
    pub const FUCHSIA: Self = Self { red: 1.0, green: 0.0, blue: 1.0, alpha: 1.0 };
    pub const CORNFLOWERBLUE: Self = Self { red: 0.39215686274509803, green: 0.5843137254901961, blue: 0.9294117647058824, alpha: 1.0 };
    pub const ROYALBLUE: Self = Self { red: 0.2549019607843137, green: 0.4117647058823529, blue: 0.8823529411764706, alpha: 1.0 };
    pub const SKYBLUE: Self = Self { red: 0.5294117647058824, green: 0.807843137254902, blue: 0.9215686274509803, alpha: 1.0 };
    pub const STEELBLUE: Self = Self { red: 0.27450980392156865, green: 0.5098039215686274, blue: 0.7058823529411765, alpha: 1.0 };
    pub const DARKBLUE: Self = Self { red: 0.0, green: 0.0, blue: 0.5450980392156862, alpha: 1.0 };
    pub const DARKGREEN: Self = Self { red: 0.0, green: 0.39215686274509803, blue: 0.0, alpha: 1.0 };
    pub const DARKRED: Self = Self { red: 0.5450980392156862, green: 0.0, blue: 0.0, alpha: 1.0 };
    pub const GOLD: Self = Self { red: 1.0, green: 0.8431372549019608, blue: 0.0, alpha: 1.0 };
    pub const INDIGO: Self = Self { red: 0.29411764705882354, green: 0.0, blue: 0.5098039215686274, alpha: 1.0 };
    pub const IVORY: Self = Self { red: 1.0, green: 1.0, blue: 0.9411764705882353, alpha: 1.0 };
    pub const KHAKI: Self = Self { red: 0.9411764705882353, green: 0.9019607843137255, blue: 0.5490196078431373, alpha: 1.0 };
    pub const LAVENDER: Self = Self { red: 0.9019607843137255, green: 0.9019607843137255, blue: 0.9803921568627451, alpha: 1.0 };
    pub const SALMON: Self = Self { red: 0.9803921568627451, green: 0.5019607843137255, blue: 0.4470588235294118, alpha: 1.0 };
    pub const TOMATO: Self = Self { red: 1.0, green: 0.38823529411764707, blue: 0.2784313725490196, alpha: 1.0 };
    pub const VIOLET: Self = Self { red: 0.9333333333333333, green: 0.5098039215686274, blue: 0.9333333333333333, alpha: 1.0 };
    pub const WHEAT: Self = Self { red: 0.9607843137254902, green: 0.8705882352941177, blue: 0.7019607843137254, alpha: 1.0 };

    /// Creates a Color from byte values (0-255).
    pub fn from_bytes(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha: alpha as f64 / 255.0,
        }
    }

    /// Converts to byte values [r, g, b, a] (0-255).
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            Self::float_to_byte(self.red),
            Self::float_to_byte(self.green),
            Self::float_to_byte(self.blue),
            Self::float_to_byte(self.alpha),
        ]
    }

    /// Converts a byte (0-255) to float (0-1).
    pub fn byte_to_float(value: u8) -> f64 {
        value as f64 / 255.0
    }

    /// Converts a float (0-1) to byte (0-255).
    pub fn float_to_byte(value: f64) -> u8 {
        if value == 1.0 {
            255
        } else {
            (value * 256.0) as u8
        }
    }

    /// Creates a Color from a Cartesian4 (x=red, y=green, z=blue, w=alpha).
    pub fn from_cartesian4(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { red: x, green: y, blue: z, alpha: w }
    }

    /// Creates a Color from HSL values. Hue is 0..1 (wraps), saturation 0..1, lightness 0..1.
    pub fn from_hsl(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Self {
        let hue = hue % 1.0;
        let mut red = lightness;
        let mut green = lightness;
        let mut blue = lightness;

        if saturation != 0.0 {
            let m2 = if lightness < 0.5 {
                lightness * (1.0 + saturation)
            } else {
                lightness + saturation - lightness * saturation
            };
            let m1 = 2.0 * lightness - m2;
            red = hue2rgb(m1, m2, hue + 1.0 / 3.0);
            green = hue2rgb(m1, m2, hue);
            blue = hue2rgb(m1, m2, hue - 1.0 / 3.0);
        }

        Self { red, green, blue, alpha }
    }

    /// Creates a Color from a CSS color string.
    /// Supports: #rgb, #rgba, #rrggbb, #rrggbbaa, rgb(), rgba(), hsl(), hsla(), named colors.
    /// Returns None if the string is not a valid CSS color.
    pub fn from_css_color_string(color: &str) -> Option<Self> {
        let color = color.trim();

        // Check named colors
        if let Some(named) = Self::named_color(color) {
            return Some(named);
        }

        // #rgba or #rgb
        if let Some(hex) = color.strip_prefix('#') {
            let hex_lower = hex.to_lowercase();
            let chars: Vec<char> = hex_lower.chars().collect();
            match chars.len() {
                3 => {
                    let r = u8::from_str_radix(&hex_lower[0..1], 16).ok()? as f64 / 15.0;
                    let g = u8::from_str_radix(&hex_lower[1..2], 16).ok()? as f64 / 15.0;
                    let b = u8::from_str_radix(&hex_lower[2..3], 16).ok()? as f64 / 15.0;
                    return Some(Self::new(r, g, b, 1.0));
                }
                4 => {
                    let r = u8::from_str_radix(&hex_lower[0..1], 16).ok()? as f64 / 15.0;
                    let g = u8::from_str_radix(&hex_lower[1..2], 16).ok()? as f64 / 15.0;
                    let b = u8::from_str_radix(&hex_lower[2..3], 16).ok()? as f64 / 15.0;
                    let a = u8::from_str_radix(&hex_lower[3..4], 16).ok()? as f64 / 15.0;
                    return Some(Self::new(r, g, b, a));
                }
                6 => {
                    let r = u8::from_str_radix(&hex_lower[0..2], 16).ok()? as f64 / 255.0;
                    let g = u8::from_str_radix(&hex_lower[2..4], 16).ok()? as f64 / 255.0;
                    let b = u8::from_str_radix(&hex_lower[4..6], 16).ok()? as f64 / 255.0;
                    return Some(Self::new(r, g, b, 1.0));
                }
                8 => {
                    let r = u8::from_str_radix(&hex_lower[0..2], 16).ok()? as f64 / 255.0;
                    let g = u8::from_str_radix(&hex_lower[2..4], 16).ok()? as f64 / 255.0;
                    let b = u8::from_str_radix(&hex_lower[4..6], 16).ok()? as f64 / 255.0;
                    let a = u8::from_str_radix(&hex_lower[6..8], 16).ok()? as f64 / 255.0;
                    return Some(Self::new(r, g, b, a));
                }
                _ => return None,
            }
        }

        // rgb() / rgba()
        let lower = color.to_lowercase();
        if lower.starts_with("rgb") {
            return Self::parse_rgb_functional(color);
        }

        // hsl() / hsla()
        if lower.starts_with("hsl") {
            return Self::parse_hsl_functional(color);
        }

        None
    }

    fn parse_rgb_functional(color: &str) -> Option<Self> {
        // Extract content between parentheses
        let open = color.find('(')?;
        let close = color.rfind(')')?;
        let inner = &color[open + 1..close];

        // Split by comma or whitespace, handling '/' for alpha
        let normalized = inner.replace('/', " ");
        let parts: Vec<&str> = normalized
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.len() < 3 {
            return None;
        }

        let parse_component = |s: &str| -> Option<f64> {
            let s = s.trim();
            if s.ends_with('%') {
                s[..s.len() - 1].parse::<f64>().ok().map(|v| v / 100.0)
            } else {
                s.parse::<f64>().ok().map(|v| v / 255.0)
            }
        };

        let red = parse_component(parts[0])?;
        let green = parse_component(parts[1])?;
        let blue = parse_component(parts[2])?;
        let alpha = if parts.len() > 3 {
            parts[3].trim().parse::<f64>().ok()?
        } else {
            1.0
        };

        Some(Self::new(red, green, blue, alpha))
    }

    fn parse_hsl_functional(color: &str) -> Option<Self> {
        let open = color.find('(')?;
        let close = color.rfind(')')?;
        let inner = &color[open + 1..close];

        let normalized = inner.replace('/', " ");
        let parts: Vec<&str> = normalized
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.len() < 3 {
            return None;
        }

        let hue = parts[0].trim().parse::<f64>().ok()? / 360.0;
        let sat_str = parts[1].trim();
        let sat = if sat_str.ends_with('%') {
            sat_str[..sat_str.len() - 1].parse::<f64>().ok()? / 100.0
        } else {
            sat_str.parse::<f64>().ok()?
        };
        let light_str = parts[2].trim();
        let light = if light_str.ends_with('%') {
            light_str[..light_str.len() - 1].parse::<f64>().ok()? / 100.0
        } else {
            light_str.parse::<f64>().ok()?
        };
        let alpha = if parts.len() > 3 {
            parts[3].trim().parse::<f64>().ok()?
        } else {
            1.0
        };

        Some(Self::from_hsl(hue, sat, light, alpha))
    }

    fn named_color(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "WHITE" => Some(Self::WHITE),
            "BLACK" => Some(Self::BLACK),
            "RED" => Some(Self::RED),
            "GREEN" => Some(Self::GREEN),
            "LIME" => Some(Self::LIME),
            "BLUE" => Some(Self::BLUE),
            "YELLOW" => Some(Self::YELLOW),
            "CYAN" => Some(Self::CYAN),
            "MAGENTA" => Some(Self::MAGENTA),
            "TRANSPARENT" => Some(Self::TRANSPARENT),
            "ORANGE" => Some(Self::ORANGE),
            "PURPLE" => Some(Self::PURPLE),
            "PINK" => Some(Self::PINK),
            "GRAY" | "GREY" => Some(Self::GRAY),
            "BROWN" => Some(Self::BROWN),
            "NAVY" => Some(Self::NAVY),
            "TEAL" => Some(Self::TEAL),
            "OLIVE" => Some(Self::OLIVE),
            "MAROON" => Some(Self::MAROON),
            "SILVER" => Some(Self::SILVER),
            "AQUA" => Some(Self::AQUA),
            "FUCHSIA" => Some(Self::FUCHSIA),
            "CORNFLOWERBLUE" => Some(Self::CORNFLOWERBLUE),
            "ROYALBLUE" => Some(Self::ROYALBLUE),
            "SKYBLUE" => Some(Self::SKYBLUE),
            "STEELBLUE" => Some(Self::STEELBLUE),
            "DARKBLUE" => Some(Self::DARKBLUE),
            "DARKGREEN" => Some(Self::DARKGREEN),
            "DARKRED" => Some(Self::DARKRED),
            "GOLD" => Some(Self::GOLD),
            "INDIGO" => Some(Self::INDIGO),
            "IVORY" => Some(Self::IVORY),
            "KHAKI" => Some(Self::KHAKI),
            "LAVENDER" => Some(Self::LAVENDER),
            "SALMON" => Some(Self::SALMON),
            "TOMATO" => Some(Self::TOMATO),
            "VIOLET" => Some(Self::VIOLET),
            "WHEAT" => Some(Self::WHEAT),
            _ => None,
        }
    }

    /// Returns a CSS rgb()/rgba() string.
    pub fn to_css_color_string(&self) -> String {
        let r = Self::float_to_byte(self.red);
        let g = Self::float_to_byte(self.green);
        let b = Self::float_to_byte(self.blue);
        if self.alpha == 1.0 {
            format!("rgb({},{},{})", r, g, b)
        } else {
            format!("rgba({},{},{},{})", r, g, b, self.alpha)
        }
    }

    /// Returns a CSS hex string (#rrggbb or #rrggbbaa).
    pub fn to_css_hex_string(&self) -> String {
        let r = Self::float_to_byte(self.red);
        let g = Self::float_to_byte(self.green);
        let b = Self::float_to_byte(self.blue);
        if self.alpha < 1.0 {
            let a = Self::float_to_byte(self.alpha);
            format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Converts to a u32 RGBA value (little-endian byte order: R in lowest byte).
    pub fn to_rgba(&self) -> u32 {
        let r = Self::float_to_byte(self.red) as u32;
        let g = Self::float_to_byte(self.green) as u32;
        let b = Self::float_to_byte(self.blue) as u32;
        let a = Self::float_to_byte(self.alpha) as u32;
        r | (g << 8) | (b << 16) | (a << 24)
    }

    /// Creates a Color from a u32 RGBA value (little-endian byte order).
    pub fn from_rgba(rgba: u32) -> Self {
        Self::from_bytes(
            (rgba & 0xFF) as u8,
            ((rgba >> 8) & 0xFF) as u8,
            ((rgba >> 16) & 0xFF) as u8,
            ((rgba >> 24) & 0xFF) as u8,
        )
    }

    /// Returns a new Color with the given alpha.
    pub fn with_alpha(&self, alpha: f64) -> Self {
        Self { alpha, ..*self }
    }

    /// Creates a new Color from an existing color with a different alpha.
    pub fn from_alpha(color: &Self, alpha: f64) -> Self {
        Self { alpha, ..*color }
    }

    /// Brightens this color by the given magnitude (0..1).
    pub fn brighten(&self, magnitude: f64) -> Self {
        let magnitude = 1.0 - magnitude;
        Self {
            red: 1.0 - (1.0 - self.red) * magnitude,
            green: 1.0 - (1.0 - self.green) * magnitude,
            blue: 1.0 - (1.0 - self.blue) * magnitude,
            alpha: self.alpha,
        }
    }

    /// Darkens this color by the given magnitude (0..1).
    pub fn darken(&self, magnitude: f64) -> Self {
        let magnitude = 1.0 - magnitude;
        Self {
            red: self.red * magnitude,
            green: self.green * magnitude,
            blue: self.blue * magnitude,
            alpha: self.alpha,
        }
    }

    /// Component-wise addition.
    pub fn add(&self, other: &Self) -> Self {
        Self {
            red: self.red + other.red,
            green: self.green + other.green,
            blue: self.blue + other.blue,
            alpha: self.alpha + other.alpha,
        }
    }

    /// Component-wise subtraction.
    pub fn subtract(&self, other: &Self) -> Self {
        Self {
            red: self.red - other.red,
            green: self.green - other.green,
            blue: self.blue - other.blue,
            alpha: self.alpha - other.alpha,
        }
    }

    /// Component-wise multiplication.
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            red: self.red * other.red,
            green: self.green * other.green,
            blue: self.blue * other.blue,
            alpha: self.alpha * other.alpha,
        }
    }

    /// Component-wise division.
    pub fn divide(&self, other: &Self) -> Self {
        Self {
            red: self.red / other.red,
            green: self.green / other.green,
            blue: self.blue / other.blue,
            alpha: self.alpha / other.alpha,
        }
    }

    /// Component-wise modulo.
    pub fn modulo(&self, other: &Self) -> Self {
        Self {
            red: self.red % other.red,
            green: self.green % other.green,
            blue: self.blue % other.blue,
            alpha: self.alpha % other.alpha,
        }
    }

    /// Multiplies all components by a scalar.
    pub fn multiply_by_scalar(&self, scalar: f64) -> Self {
        Self {
            red: self.red * scalar,
            green: self.green * scalar,
            blue: self.blue * scalar,
            alpha: self.alpha * scalar,
        }
    }

    /// Divides all components by a scalar.
    pub fn divide_by_scalar(&self, scalar: f64) -> Self {
        Self {
            red: self.red / scalar,
            green: self.green / scalar,
            blue: self.blue / scalar,
            alpha: self.alpha / scalar,
        }
    }

    /// Linear interpolation between two colors.
    pub fn lerp(start: &Self, end: &Self, t: f64) -> Self {
        Self {
            red: math_utils::lerp(start.red, end.red, t),
            green: math_utils::lerp(start.green, end.green, t),
            blue: math_utils::lerp(start.blue, end.blue, t),
            alpha: math_utils::lerp(start.alpha, end.alpha, t),
        }
    }

    /// Returns true if this color equals other within the given epsilon.
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.red - other.red).abs() <= epsilon
            && (self.green - other.green).abs() <= epsilon
            && (self.blue - other.blue).abs() <= epsilon
            && (self.alpha - other.alpha).abs() <= epsilon
    }

    /// Packs into an array [red, green, blue, alpha] starting at index.
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.red;
        array[starting_index + 1] = self.green;
        array[starting_index + 2] = self.blue;
        array[starting_index + 3] = self.alpha;
    }

    /// Unpacks from an array starting at index.
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            red: array[starting_index],
            green: array[starting_index + 1],
            blue: array[starting_index + 2],
            alpha: array[starting_index + 3],
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.red, self.green, self.blue, self.alpha)
    }
}

/// HSL to RGB helper (maps to CesiumJS hue2rgb).
fn hue2rgb(m1: f64, m2: f64, mut h: f64) -> f64 {
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }
    if h * 6.0 < 1.0 {
        return m1 + (m2 - m1) * 6.0 * h;
    }
    if h * 2.0 < 1.0 {
        return m2;
    }
    if h * 3.0 < 2.0 {
        return m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0;
    }
    m1
}
