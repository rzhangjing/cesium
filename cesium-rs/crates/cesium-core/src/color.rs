//! Ported from `packages/engine/Source/Core/Color.js`.

use crate::cartesian4::Cartesian4;
use crate::math::CesiumMath;

/// Helper: convert hue to rgb component.
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

/// A color, specified using red, green, blue, and alpha values,
/// which range from `0.0` (no intensity) to `1.0` (full intensity).
#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    /// The red component (0.0 to 1.0).
    pub red: f64,
    /// The green component (0.0 to 1.0).
    pub green: f64,
    /// The blue component (0.0 to 1.0).
    pub blue: f64,
    /// The alpha component (0.0 to 1.0).
    pub alpha: f64,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
        }
    }
}

impl Color {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

    /// Creates a new Color.
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a Color from a Cartesian4 (x→red, y→green, z→blue, w→alpha).
    pub fn from_cartesian4(cartesian: &Cartesian4) -> Self {
        Self {
            red: cartesian.x,
            green: cartesian.y,
            blue: cartesian.z,
            alpha: cartesian.w,
        }
    }

    /// Creates a Color from byte values (0-255).
    pub fn from_bytes(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red: Self::byte_to_float(red),
            green: Self::byte_to_float(green),
            blue: Self::byte_to_float(blue),
            alpha: Self::byte_to_float(alpha),
        }
    }

    /// Creates a new Color with the same RGB but a different alpha.
    pub fn from_alpha(color: &Self, alpha: f64) -> Self {
        Self {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha,
        }
    }

    /// Creates a Color from a single 32-bit RGBA value.
    pub fn from_rgba(rgba: u32) -> Self {
        let red = ((rgba >> 24) & 0xFF) as u8;
        let green = ((rgba >> 16) & 0xFF) as u8;
        let blue = ((rgba >> 8) & 0xFF) as u8;
        let alpha = (rgba & 0xFF) as u8;
        Self::from_bytes(red, green, blue, alpha)
    }

    /// Creates a Color from hue, saturation, lightness (all 0..1).
    pub fn from_hsl(
        hue: f64,
        saturation: f64,
        lightness: f64,
        alpha: f64,
    ) -> Self {
        let mut red = lightness;
        let mut green = lightness;
        let mut blue = lightness;

        let hue = hue % 1.0;

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

        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a random color.
    pub fn from_random(
        red: Option<f64>,
        green: Option<f64>,
        blue: Option<f64>,
        alpha: Option<f64>,
        minimum_red: f64,
        maximum_red: f64,
        minimum_green: f64,
        maximum_green: f64,
        minimum_blue: f64,
        maximum_blue: f64,
        minimum_alpha: f64,
        maximum_alpha: f64,
    ) -> Self {
        let r = red.unwrap_or(minimum_red + CesiumMath::next_random_number() * (maximum_red - minimum_red));
        let g = green.unwrap_or(minimum_green + CesiumMath::next_random_number() * (maximum_green - minimum_green));
        let b = blue.unwrap_or(minimum_blue + CesiumMath::next_random_number() * (maximum_blue - minimum_blue));
        let a = alpha.unwrap_or(minimum_alpha + CesiumMath::next_random_number() * (maximum_alpha - minimum_alpha));
        Self { red: r, green: g, blue: b, alpha: a }
    }

    /// Creates a Color from a CSS color string (#rgb, #rrggbb, #rrggbbaa, rgb(), rgba(), hsl(), hsla()).
    pub fn from_css_color_string(color: &str) -> Option<Self> {
        let color = color.trim();
        if color.is_empty() {
            return None;
        }

        // Try named colors (case-insensitive)
        let upper = color.to_uppercase();
        if let Some(named) = Self::named_color(&upper) {
            return Some(named);
        }

        // #rgba or #rgb
        if color.starts_with('#') {
            let hex = &color[1..];
            if hex.len() == 3 || hex.len() == 4 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                ) {
                    let a = if hex.len() == 4 {
                        u8::from_str_radix(&hex[3..4], 16).unwrap_or(0xF)
                    } else {
                        0xF
                    };
                    return Some(Self::new(
                        r as f64 / 15.0,
                        g as f64 / 15.0,
                        b as f64 / 15.0,
                        a as f64 / 15.0,
                    ));
                }
            }
            // #rrggbbaa or #rrggbb
            if hex.len() == 6 || hex.len() == 8 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    let a = if hex.len() == 8 {
                        u8::from_str_radix(&hex[6..8], 16).unwrap_or(0xFF)
                    } else {
                        0xFF
                    };
                    return Some(Self::from_bytes(r, g, b, a));
                }
            }
            return None;
        }

        // rgb()/rgba()
        if let Some(inner) = color.strip_prefix("rgb").and_then(|s| s.strip_prefix('(')).or_else(|| color.strip_prefix("rgba").and_then(|s| s.strip_prefix('('))) {
            let inner = inner.trim_end_matches(')').trim();
            let parts: Vec<&str> = inner.split([',', '/']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 {
                let parse_component = |s: &str| -> f64 {
                    if s.ends_with('%') {
                        s.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0
                    } else {
                        s.parse::<f64>().unwrap_or(0.0) / 255.0
                    }
                };
                let r = parse_component(parts[0]);
                let g = parse_component(parts[1]);
                let b = parse_component(parts[2]);
                let a = if parts.len() >= 4 {
                    parts[3].parse::<f64>().unwrap_or(1.0)
                } else {
                    1.0
                };
                return Some(Self::new(r, g, b, a));
            }
        }

        // hsl()/hsla()
        if let Some(inner) = color.strip_prefix("hsl").and_then(|s| s.strip_prefix('(')).or_else(|| color.strip_prefix("hsla").and_then(|s| s.strip_prefix('('))) {
            let inner = inner.trim_end_matches(')').trim();
            let parts: Vec<&str> = inner.split([',', '/']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 {
                let h = parts[0].trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').parse::<f64>().unwrap_or(0.0) / 360.0;
                let s = parts[1].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
                let l = parts[2].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
                let a = if parts.len() >= 4 {
                    parts[3].parse::<f64>().unwrap_or(1.0)
                } else {
                    1.0
                };
                return Some(Self::from_hsl(h, s, l, a));
            }
        }

        None
    }

    /// Packs the color into an array.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.red;
        array[starting_index + 1] = value.green;
        array[starting_index + 2] = value.blue;
        array[starting_index + 3] = value.alpha;
    }

    /// Unpacks a color from an array.
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            red: array[starting_index],
            green: array[starting_index + 1],
            blue: array[starting_index + 2],
            alpha: array[starting_index + 3],
        }
    }

    /// Converts a byte (0-255) to a float (0.0-1.0).
    pub fn byte_to_float(number: u8) -> f64 {
        number as f64 / 255.0
    }

    /// Converts a float (0.0-1.0) to a byte (0-255).
    pub fn float_to_byte(number: f64) -> u8 {
        if number >= 1.0 {
            255
        } else {
            (number * 256.0) as u8
        }
    }

    /// Returns true if the colors are equal.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.red == right.red
            && left.green == right.green
            && left.blue == right.blue
            && left.alpha == right.alpha
    }

    /// Returns true if the color equals an array at the given offset.
    pub fn equals_array(&self, array: &[f64], offset: usize) -> bool {
        self.red == array[offset]
            && self.green == array[offset + 1]
            && self.blue == array[offset + 2]
            && self.alpha == array[offset + 3]
    }

    /// Returns true if colors are equal within epsilon.
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.red - other.red).abs() <= epsilon
            && (self.green - other.green).abs() <= epsilon
            && (self.blue - other.blue).abs() <= epsilon
            && (self.alpha - other.alpha).abs() <= epsilon
    }

    /// Creates a CSS color string.
    pub fn to_css_color_string(&self) -> String {
        let red = Self::float_to_byte(self.red);
        let green = Self::float_to_byte(self.green);
        let blue = Self::float_to_byte(self.blue);
        if self.alpha == 1.0 {
            format!("rgb({},{},{})", red, green, blue)
        } else {
            format!("rgba({},{},{},{})", red, green, blue, self.alpha)
        }
    }

    /// Creates a CSS hex color string.
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

    /// Converts to byte array [r, g, b, a].
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            Self::float_to_byte(self.red),
            Self::float_to_byte(self.green),
            Self::float_to_byte(self.blue),
            Self::float_to_byte(self.alpha),
        ]
    }

    /// Converts to a single 32-bit RGBA value.
    pub fn to_rgba(&self) -> u32 {
        let bytes = self.to_bytes();
        ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32)
    }

    /// Converts RGBA bytes to a single 32-bit value.
    pub fn bytes_to_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
        ((red as u32) << 24) | ((green as u32) << 16) | ((blue as u32) << 8) | (alpha as u32)
    }

    /// Brightens this color by the provided magnitude (0..1).
    pub fn brighten(&self, magnitude: f64) -> Self {
        let m = 1.0 - magnitude;
        Self {
            red: 1.0 - (1.0 - self.red) * m,
            green: 1.0 - (1.0 - self.green) * m,
            blue: 1.0 - (1.0 - self.blue) * m,
            alpha: self.alpha,
        }
    }

    /// Darkens this color by the provided magnitude (0..1).
    pub fn darken(&self, magnitude: f64) -> Self {
        let m = 1.0 - magnitude;
        Self {
            red: self.red * m,
            green: self.green * m,
            blue: self.blue * m,
            alpha: self.alpha,
        }
    }

    /// Returns a new color with the specified alpha.
    pub fn with_alpha(&self, alpha: f64) -> Self {
        Self::from_alpha(self, alpha)
    }

    /// Componentwise addition.
    pub fn add(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red + right.red,
            green: left.green + right.green,
            blue: left.blue + right.blue,
            alpha: left.alpha + right.alpha,
        }
    }

    /// Componentwise subtraction.
    pub fn subtract(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red - right.red,
            green: left.green - right.green,
            blue: left.blue - right.blue,
            alpha: left.alpha - right.alpha,
        }
    }

    /// Componentwise multiplication.
    pub fn multiply(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red * right.red,
            green: left.green * right.green,
            blue: left.blue * right.blue,
            alpha: left.alpha * right.alpha,
        }
    }

    /// Componentwise division.
    pub fn divide(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red / right.red,
            green: left.green / right.green,
            blue: left.blue / right.blue,
            alpha: left.alpha / right.alpha,
        }
    }

    /// Componentwise modulus.
    pub fn modulo(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red % right.red,
            green: left.green % right.green,
            blue: left.blue % right.blue,
            alpha: left.alpha % right.alpha,
        }
    }

    /// Linear interpolation between two colors.
    pub fn lerp(start: &Self, end: &Self, t: f64) -> Self {
        Self {
            red: CesiumMath::lerp(start.red, end.red, t),
            green: CesiumMath::lerp(start.green, end.green, t),
            blue: CesiumMath::lerp(start.blue, end.blue, t),
            alpha: CesiumMath::lerp(start.alpha, end.alpha, t),
        }
    }

    /// Multiplies by a scalar.
    pub fn multiply_by_scalar(color: &Self, scalar: f64) -> Self {
        Self {
            red: color.red * scalar,
            green: color.green * scalar,
            blue: color.blue * scalar,
            alpha: color.alpha * scalar,
        }
    }

    /// Divides by a scalar.
    pub fn divide_by_scalar(color: &Self, scalar: f64) -> Self {
        Self {
            red: color.red / scalar,
            green: color.green / scalar,
            blue: color.blue / scalar,
            alpha: color.alpha / scalar,
        }
    }

    // ---- Named color constants ----

    fn from_hex(hex: &str) -> Self {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        Self::from_bytes(r, g, b, 255)
    }

    fn named_color(name: &str) -> Option<Self> {
        match name {
            "ALICEBLUE" => Some(Self::from_hex("F0F8FF")),
            "ANTIQUEWHITE" => Some(Self::from_hex("FAEBD7")),
            "AQUA" => Some(Self::from_hex("00FFFF")),
            "AQUAMARINE" => Some(Self::from_hex("7FFFD4")),
            "AZURE" => Some(Self::from_hex("F0FFFF")),
            "BEIGE" => Some(Self::from_hex("F5F5DC")),
            "BISQUE" => Some(Self::from_hex("FFE4C4")),
            "BLACK" => Some(Self::from_hex("000000")),
            "BLANCHEDALMOND" => Some(Self::from_hex("FFEBCD")),
            "BLUE" => Some(Self::from_hex("0000FF")),
            "BLUEVIOLET" => Some(Self::from_hex("8A2BE2")),
            "BROWN" => Some(Self::from_hex("A52A2A")),
            "BURLYWOOD" => Some(Self::from_hex("DEB887")),
            "CADETBLUE" => Some(Self::from_hex("5F9EA0")),
            "CHARTREUSE" => Some(Self::from_hex("7FFF00")),
            "CHOCOLATE" => Some(Self::from_hex("D2691E")),
            "CORAL" => Some(Self::from_hex("FF7F50")),
            "CORNFLOWERBLUE" => Some(Self::from_hex("6495ED")),
            "CORNSILK" => Some(Self::from_hex("FFF8DC")),
            "CRIMSON" => Some(Self::from_hex("DC143C")),
            "CYAN" => Some(Self::from_hex("00FFFF")),
            "DARKBLUE" => Some(Self::from_hex("00008B")),
            "DARKCYAN" => Some(Self::from_hex("008B8B")),
            "DARKGOLDENROD" => Some(Self::from_hex("B8860B")),
            "DARKGRAY" | "DARKGREY" => Some(Self::from_hex("A9A9A9")),
            "DARKGREEN" => Some(Self::from_hex("006400")),
            "DARKKHAKI" => Some(Self::from_hex("BDB76B")),
            "DARKMAGENTA" => Some(Self::from_hex("8B008B")),
            "DARKOLIVEGREEN" => Some(Self::from_hex("556B2F")),
            "DARKORANGE" => Some(Self::from_hex("FF8C00")),
            "DARKORCHID" => Some(Self::from_hex("9932CC")),
            "DARKRED" => Some(Self::from_hex("8B0000")),
            "DARKSALMON" => Some(Self::from_hex("E9967A")),
            "DARKSEAGREEN" => Some(Self::from_hex("8FBC8F")),
            "DARKSLATEBLUE" => Some(Self::from_hex("483D8B")),
            "DARKSLATEGRAY" | "DARKSLATEGREY" => Some(Self::from_hex("2F4F4F")),
            "DARKTURQUOISE" => Some(Self::from_hex("00CED1")),
            "DARKVIOLET" => Some(Self::from_hex("9400D3")),
            "DEEPPINK" => Some(Self::from_hex("FF1493")),
            "DEEPSKYBLUE" => Some(Self::from_hex("00BFFF")),
            "DIMGRAY" | "DIMGREY" => Some(Self::from_hex("696969")),
            "DODGERBLUE" => Some(Self::from_hex("1E90FF")),
            "FIREBRICK" => Some(Self::from_hex("B22222")),
            "FLORALWHITE" => Some(Self::from_hex("FFFAF0")),
            "FORESTGREEN" => Some(Self::from_hex("228B22")),
            "FUCHSIA" => Some(Self::from_hex("FF00FF")),
            "GAINSBORO" => Some(Self::from_hex("DCDCDC")),
            "GHOSTWHITE" => Some(Self::from_hex("F8F8FF")),
            "GOLD" => Some(Self::from_hex("FFD700")),
            "GOLDENROD" => Some(Self::from_hex("DAA520")),
            "GRAY" | "GREY" => Some(Self::from_hex("808080")),
            "GREEN" => Some(Self::from_hex("008000")),
            "GREENYELLOW" => Some(Self::from_hex("ADFF2F")),
            "HONEYDEW" => Some(Self::from_hex("F0FFF0")),
            "HOTPINK" => Some(Self::from_hex("FF69B4")),
            "INDIANRED" => Some(Self::from_hex("CD5C5C")),
            "INDIGO" => Some(Self::from_hex("4B0082")),
            "IVORY" => Some(Self::from_hex("FFFFF0")),
            "KHAKI" => Some(Self::from_hex("F0E68C")),
            "LAVENDER" => Some(Self::from_hex("E6E6FA")),
            "LAVENDAR_BLUSH" => Some(Self::from_hex("FFF0F5")),
            "LAWNGREEN" => Some(Self::from_hex("7CFC00")),
            "LEMONCHIFFON" => Some(Self::from_hex("FFFACD")),
            "LIGHTBLUE" => Some(Self::from_hex("ADD8E6")),
            "LIGHTCORAL" => Some(Self::from_hex("F08080")),
            "LIGHTCYAN" => Some(Self::from_hex("E0FFFF")),
            "LIGHTGOLDENRODYELLOW" => Some(Self::from_hex("FAFAD2")),
            "LIGHTGRAY" | "LIGHTGREY" => Some(Self::from_hex("D3D3D3")),
            "LIGHTGREEN" => Some(Self::from_hex("90EE90")),
            "LIGHTPINK" => Some(Self::from_hex("FFB6C1")),
            "LIGHTSEAGREEN" => Some(Self::from_hex("20B2AA")),
            "LIGHTSKYBLUE" => Some(Self::from_hex("87CEFA")),
            "LIGHTSLATEGRAY" | "LIGHTSLATEGREY" => Some(Self::from_hex("778899")),
            "LIGHTSTEELBLUE" => Some(Self::from_hex("B0C4DE")),
            "LIGHTYELLOW" => Some(Self::from_hex("FFFFE0")),
            "LIME" => Some(Self::from_hex("00FF00")),
            "LIMEGREEN" => Some(Self::from_hex("32CD32")),
            "LINEN" => Some(Self::from_hex("FAF0E6")),
            "MAGENTA" => Some(Self::from_hex("FF00FF")),
            "MAROON" => Some(Self::from_hex("800000")),
            "MEDIUMAQUAMARINE" => Some(Self::from_hex("66CDAA")),
            "MEDIUMBLUE" => Some(Self::from_hex("0000CD")),
            "MEDIUMORCHID" => Some(Self::from_hex("BA55D3")),
            "MEDIUMPURPLE" => Some(Self::from_hex("9370DB")),
            "MEDIUMSEAGREEN" => Some(Self::from_hex("3CB371")),
            "MEDIUMSLATEBLUE" => Some(Self::from_hex("7B68EE")),
            "MEDIUMSPRINGGREEN" => Some(Self::from_hex("00FA9A")),
            "MEDIUMTURQUOISE" => Some(Self::from_hex("48D1CC")),
            "MEDIUMVIOLETRED" => Some(Self::from_hex("C71585")),
            "MIDNIGHTBLUE" => Some(Self::from_hex("191970")),
            "MINTCREAM" => Some(Self::from_hex("F5FFFA")),
            "MISTYROSE" => Some(Self::from_hex("FFE4E1")),
            "MOCCASIN" => Some(Self::from_hex("FFE4B5")),
            "NAVAJOWHITE" => Some(Self::from_hex("FFDEAD")),
            "NAVY" => Some(Self::from_hex("000080")),
            "OLDLACE" => Some(Self::from_hex("FDF5E6")),
            "OLIVE" => Some(Self::from_hex("808000")),
            "OLIVEDRAB" => Some(Self::from_hex("6B8E23")),
            "ORANGE" => Some(Self::from_hex("FFA500")),
            "ORANGERED" => Some(Self::from_hex("FF4500")),
            "ORCHID" => Some(Self::from_hex("DA70D6")),
            "PALEGOLDENROD" => Some(Self::from_hex("EEE8AA")),
            "PALEGREEN" => Some(Self::from_hex("98FB98")),
            "PALETURQUOISE" => Some(Self::from_hex("AFEEEE")),
            "PALEVIOLETRED" => Some(Self::from_hex("DB7093")),
            "PAPAYAWHIP" => Some(Self::from_hex("FFEFD5")),
            "PEACHPUFF" => Some(Self::from_hex("FFDAB9")),
            "PERU" => Some(Self::from_hex("CD853F")),
            "PINK" => Some(Self::from_hex("FFC0CB")),
            "PLUM" => Some(Self::from_hex("DDA0DD")),
            "POWDERBLUE" => Some(Self::from_hex("B0E0E6")),
            "PURPLE" => Some(Self::from_hex("800080")),
            "RED" => Some(Self::from_hex("FF0000")),
            "ROSYBROWN" => Some(Self::from_hex("BC8F8F")),
            "ROYALBLUE" => Some(Self::from_hex("4169E1")),
            "SADDLEBROWN" => Some(Self::from_hex("8B4513")),
            "SALMON" => Some(Self::from_hex("FA8072")),
            "SANDYBROWN" => Some(Self::from_hex("F4A460")),
            "SEAGREEN" => Some(Self::from_hex("2E8B57")),
            "SEASHELL" => Some(Self::from_hex("FFF5EE")),
            "SIENNA" => Some(Self::from_hex("A0522D")),
            "SILVER" => Some(Self::from_hex("C0C0C0")),
            "SKYBLUE" => Some(Self::from_hex("87CEEB")),
            "SLATEBLUE" => Some(Self::from_hex("6A5ACD")),
            "SLATEGRAY" | "SLATEGREY" => Some(Self::from_hex("708090")),
            "SNOW" => Some(Self::from_hex("FFFAFA")),
            "SPRINGGREEN" => Some(Self::from_hex("00FF7F")),
            "STEELBLUE" => Some(Self::from_hex("4682B4")),
            "TAN" => Some(Self::from_hex("D2B48C")),
            "TEAL" => Some(Self::from_hex("008080")),
            "THISTLE" => Some(Self::from_hex("D8BFD8")),
            "TOMATO" => Some(Self::from_hex("FF6347")),
            "TURQUOISE" => Some(Self::from_hex("40E0D0")),
            "VIOLET" => Some(Self::from_hex("EE82EE")),
            "WHEAT" => Some(Self::from_hex("F5DEB3")),
            "WHITE" => Some(Self::from_hex("FFFFFF")),
            "WHITESMOKE" => Some(Self::from_hex("F5F5F5")),
            "YELLOW" => Some(Self::from_hex("FFFF00")),
            "YELLOWGREEN" => Some(Self::from_hex("9ACD32")),
            "TRANSPARENT" => Some(Self::new(0.0, 0.0, 0.0, 0.0)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.red, self.green, self.blue, self.alpha)
    }
}
