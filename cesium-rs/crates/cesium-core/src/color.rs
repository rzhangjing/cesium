//! Ported from `packages/engine/Source/Core/Color.js`.
//!
//! ## Method-level alignment table (Color.js -> color.rs)
//!
//! | Color.js                          | color.rs                                   | status |
//! |-----------------------------------|--------------------------------------------|--------|
//! | `constructor(red, green, blue, a)`| `Color::new` (const) / `Default`           | aligned |
//! | `fromCartesian4`                  | `Color::from_cartesian4`                   | aligned |
//! | `fromBytes`                       | `Color::from_bytes`                        | aligned |
//! | `fromAlpha`                       | `Color::from_alpha`                        | aligned |
//! | `fromRgba`                        | `Color::from_rgba`                         | aligned (see endianness note) |
//! | `fromHsl`                         | `Color::from_hsl`                          | aligned |
//! | `fromRandom(options)`             | `Color::from_random` + `FromRandomOptions` | aligned |
//! | `fromCssColorString`              | `Color::from_css_color_string`             | aligned (regexes mirrored by hand) |
//! | `pack` / `unpack`                 | `Color::pack` / `Color::unpack`            | aligned |
//! | `byteToFloat` / `floatToByte`     | `byte_to_float` / `float_to_byte`          | aligned |
//! | `clone`                           | `Clone` / `Clone::clone_from`              | aligned (trait) |
//! | `equals` / `equalsArray`          | `equals` / `equals_array`                  | aligned |
//! | `equalsEpsilon`                   | `equals_epsilon`                           | aligned |
//! | `toString`                        | `Display`                                  | aligned |
//! | `toCssColorString`                | `to_css_color_string`                      | aligned |
//! | `toCssHexString`                  | `to_css_hex_string`                        | aligned |
//! | `toBytes`                         | `to_bytes`                                 | aligned |
//! | `bytesToRgba`                     | `bytes_to_rgba`                            | aligned |
//! | `toRgba`                          | `to_rgba`                                  | aligned |
//! | `brighten(magnitude, result)`     | `brighten` (out-param) / `brighten_new`    | aligned |
//! | `darken(magnitude, result)`       | `darken` (out-param) / `darken_new`        | aligned |
//! | `withAlpha`                       | `with_alpha`                               | aligned |
//! | `add`/`subtract`/`multiply`/      | `add`/`subtract`/`multiply`/`divide`/      | aligned |
//! | `divide`/`mod`/`lerp`/            | `modulo`/`lerp`/`multiply_by_scalar`/      | (`mod` is a Rust keyword, hence |
//! | `multiplyByScalar`/              | `divide_by_scalar`                         |  `modulo`) |
//! | `divideByScalar`                  |                                            |        |
//! | `Color.packedLength`              | `Color::PACKED_LENGTH`                     | aligned |
//! | `Color.ALICEBLUE..TRANSPARENT`    | `pub const` instances below                | aligned |
//! | (not in CesiumJS)                 | `compute_luminance`                        | DEVIATION: extension requested by the port spec |
//!
//! DEVIATION: `Color.fromRgba` / `toRgba` in CesiumJS depend on the system
//! endianness (they round-trip through a shared ArrayBuffer). All supported
//! targets are little-endian, so the Rust port fixes the little-endian layout:
//! red occupies the least-significant byte, alpha the most-significant one.
//!
//! DEVIATION: `compute_luminance` does not exist in CesiumJS; it implements
//! the WCAG 2.x relative-luminance formula and is provided as a port-side
//! extension (see task spec).

use crate::check;
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

/// Returns `true` if `c` belongs to the character set matched by the `\s`
/// escape of the original JS regular expressions (ECMAScript WhiteSpace +
/// LineTerminator).
fn is_css_whitespace(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\x0B'
            | '\x0C'
            | '\u{A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

/// Emulates ECMAScript `parseFloat`: parses the longest leading numeric
/// prefix and returns NaN if none exists.
fn parse_float_js(s: &str) -> f64 {
    for end in (1..=s.len()).rev() {
        let candidate = &s[..end];
        // JS parseFloat never recognises "inf"/"NaN"; restrict to decimal forms.
        if candidate.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        {
            if let Ok(v) = candidate.parse::<f64>() {
                return v;
            }
        }
    }
    f64::NAN
}

/// A color, specified using red, green, blue, and alpha values,
/// which range from `0.0` (no intensity) to `1.0` (full intensity).
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Options for [`Color::from_random`], mirroring the CesiumJS `options`
/// object of `Color.fromRandom`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FromRandomOptions {
    /// If specified, the red component to use instead of a randomized value.
    pub red: Option<f64>,
    /// The minimum red value to generate if none was specified (default 0.0).
    pub minimum_red: Option<f64>,
    /// The maximum red value to generate if none was specified (default 1.0).
    pub maximum_red: Option<f64>,
    /// If specified, the green component to use instead of a randomized value.
    pub green: Option<f64>,
    /// The minimum green value to generate if none was specified (default 0.0).
    pub minimum_green: Option<f64>,
    /// The maximum green value to generate if none was specified (default 1.0).
    pub maximum_green: Option<f64>,
    /// If specified, the blue component to use instead of a randomized value.
    pub blue: Option<f64>,
    /// The minimum blue value to generate if none was specified (default 0.0).
    pub minimum_blue: Option<f64>,
    /// The maximum blue value to generate if none was specified (default 1.0).
    pub maximum_blue: Option<f64>,
    /// If specified, the alpha component to use instead of a randomized value.
    pub alpha: Option<f64>,
    /// The minimum alpha value to generate if none was specified (default 0.0).
    pub minimum_alpha: Option<f64>,
    /// The maximum alpha value to generate if none was specified (default 1.0).
    pub maximum_alpha: Option<f64>,
}

impl Color {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

    /// Creates a new Color.
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Const helper building an opaque color from a 24-bit hex triple,
    /// mirroring `Color.fromCssColorString("#RRGGBB")` used to initialize the
    /// named-color constants in Color.js.
    const fn from_hex_triple(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha: 1.0,
        }
    }

    /// Creates a Color instance from a [`Cartesian4`]. `x`, `y`, `z`, and `w`
    /// map to `red`, `green`, `blue`, and `alpha`, respectively.
    pub fn from_cartesian4(cartesian: &Cartesian4) -> Self {
        Self {
            red: cartesian.x,
            green: cartesian.y,
            blue: cartesian.z,
            alpha: cartesian.w,
        }
    }

    /// Creates a new Color specified using red, green, blue, and alpha values
    /// that are in the range of 0 to 255, converting them internally to a
    /// range of 0.0 to 1.0.
    pub fn from_bytes(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red: Self::byte_to_float(red),
            green: Self::byte_to_float(green),
            blue: Self::byte_to_float(blue),
            alpha: Self::byte_to_float(alpha),
        }
    }

    /// Creates a new Color that has the same red, green, and blue components
    /// of the specified color, but with the specified alpha value.
    ///
    /// Port of `Color.fromAlpha`.
    pub fn from_alpha(color: &Self, alpha: f64) -> Self {
        Self {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha,
        }
    }

    /// Creates a new Color from a single numeric unsigned 32-bit RGBA value,
    /// using the little-endian layout (see module-level DEVIATION note).
    pub fn from_rgba(rgba: u32) -> Self {
        Self::from_bytes(
            (rgba & 0xFF) as u8,
            ((rgba >> 8) & 0xFF) as u8,
            ((rgba >> 16) & 0xFF) as u8,
            ((rgba >> 24) & 0xFF) as u8,
        )
    }

    /// Creates a Color instance from hue, saturation, and lightness.
    ///
    /// `hue`, `saturation`, and `lightness` are all in the range 0...1.
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

        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a random color using the provided options. For reproducible
    /// random colors, call [`CesiumMath::set_random_number_seed`] once at the
    /// beginning of your application.
    ///
    /// Port of `Color.fromRandom`.
    ///
    /// # Panics
    ///
    /// Debug builds panic with a `DeveloperError` when a minimum component
    /// value is greater than its maximum counterpart.
    pub fn from_random(options: Option<&FromRandomOptions>) -> Self {
        let default_options = FromRandomOptions::default();
        let options = options.unwrap_or(&default_options);

        let red = match options.red {
            Some(red) => red,
            None => {
                let minimum_red = options.minimum_red.unwrap_or(0.0);
                let maximum_red = options.maximum_red.unwrap_or(1.0);
                if cfg!(debug_assertions) {
                    check::type_of::number_less_than_or_equals(
                        "minimumRed",
                        minimum_red,
                        maximum_red,
                    );
                }
                minimum_red + CesiumMath::next_random_number() * (maximum_red - minimum_red)
            }
        };

        let green = match options.green {
            Some(green) => green,
            None => {
                let minimum_green = options.minimum_green.unwrap_or(0.0);
                let maximum_green = options.maximum_green.unwrap_or(1.0);
                if cfg!(debug_assertions) {
                    check::type_of::number_less_than_or_equals(
                        "minimumGreen",
                        minimum_green,
                        maximum_green,
                    );
                }
                minimum_green
                    + CesiumMath::next_random_number() * (maximum_green - minimum_green)
            }
        };

        let blue = match options.blue {
            Some(blue) => blue,
            None => {
                let minimum_blue = options.minimum_blue.unwrap_or(0.0);
                let maximum_blue = options.maximum_blue.unwrap_or(1.0);
                if cfg!(debug_assertions) {
                    check::type_of::number_less_than_or_equals(
                        "minimumBlue",
                        minimum_blue,
                        maximum_blue,
                    );
                }
                minimum_blue + CesiumMath::next_random_number() * (maximum_blue - minimum_blue)
            }
        };

        let alpha = match options.alpha {
            Some(alpha) => alpha,
            None => {
                let minimum_alpha = options.minimum_alpha.unwrap_or(0.0);
                let maximum_alpha = options.maximum_alpha.unwrap_or(1.0);
                if cfg!(debug_assertions) {
                    check::type_of::number_less_than_or_equals(
                        "minimumAlpha",
                        minimum_alpha,
                        maximum_alpha,
                    );
                }
                minimum_alpha
                    + CesiumMath::next_random_number() * (maximum_alpha - minimum_alpha)
            }
        };

        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a Color instance from a CSS color value.
    ///
    /// Supported formats: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`,
    /// `rgba()`, `hsl()`, `hsla()`, and all CSS named colors (case
    /// insensitive). Returns `None` (JS `undefined`) when the string is not a
    /// valid CSS color.
    ///
    /// Port of `Color.fromCssColorString`; the four JS regular expressions
    /// (`rgbaMatcher`, `rrggbbaaMatcher`, `rgbParenthesesMatcher`,
    /// `hslParenthesesMatcher`) are mirrored by hand below.
    pub fn from_css_color_string(color: &str) -> Option<Self> {
        // Remove all surrounding whitespaces from the color string
        let color = color.trim();

        let named_color = Self::named_color(&color.to_ascii_uppercase());
        if let Some(named_color) = named_color {
            return Some(named_color);
        }

        // #rgba / #rgb (rgbaMatcher: /^#([0-9a-f])([0-9a-f])([0-9a-f])([0-9a-f])?$/i)
        if let Some(stripped) = color.strip_prefix('#') {
            let hex: Vec<char> = stripped.chars().collect();
            if (hex.len() == 3 || hex.len() == 4)
                && hex.iter().all(|c| c.is_ascii_hexdigit())
            {
                let r = hex[0].to_digit(16).unwrap() as f64 / 15.0;
                let g = hex[1].to_digit(16).unwrap() as f64 / 15.0;
                let b = hex[2].to_digit(16).unwrap() as f64 / 15.0;
                let a = hex.get(3).map(|c| c.to_digit(16).unwrap() as f64).unwrap_or(15.0)
                    / 15.0;
                return Some(Self::new(r, g, b, a));
            }

            // #rrggbbaa / #rrggbb
            // (rrggbbaaMatcher: /^#([0-9a-f]{2}){3}([0-9a-f]{2})?$/i)
            if (hex.len() == 6 || hex.len() == 8)
                && hex.iter().all(|c| c.is_ascii_hexdigit())
            {
                let pair = |i: usize| {
                    u32::from_str_radix(&stripped[i * 2..i * 2 + 2], 16).unwrap() as f64 / 255.0
                };
                let a = if hex.len() == 8 {
                    pair(3)
                } else {
                    1.0
                };
                return Some(Self::new(pair(0), pair(1), pair(2), a));
            }
        }

        // rgb() / rgba() / rgb%() (rgbParenthesesMatcher)
        if let Some(components) = parse_rgb_functional(color) {
            return Some(Self::new(components.0, components.1, components.2, components.3));
        }

        // hsl() / hsla() (hslParenthesesMatcher)
        if let Some(components) = parse_hsl_functional(color) {
            return Some(Self::from_hsl(
                components.0 / 360.0,
                components.1 / 100.0,
                components.2 / 100.0,
                components.3,
            ));
        }

        None
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.red;
        array[starting_index + 1] = value.green;
        array[starting_index + 2] = value.blue;
        array[starting_index + 3] = value.alpha;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            red: array[starting_index],
            green: array[starting_index + 1],
            blue: array[starting_index + 2],
            alpha: array[starting_index + 3],
        }
    }

    /// Converts a 'byte' color component in the range of 0 to 255 into
    /// a 'float' color component in the range of 0 to 1.0.
    pub fn byte_to_float(number: u8) -> f64 {
        number as f64 / 255.0
    }

    /// Converts a 'float' color component in the range of 0 to 1.0 into
    /// a 'byte' color component in the range of 0 to 255.
    ///
    /// Faithful to `Color.floatToByte`, which returns the JS number
    /// `number === 1.0 ? 255 : (number * 256) | 0` WITHOUT clamping to
    /// `[0, 255]`: out-of-range components keep their truncated (possibly
    /// negative or > 255) value; byte truncation happens later, at the
    /// typed-array store sites (`toRgba`), mirroring CesiumJS.
    pub fn float_to_byte(number: f64) -> i32 {
        if number == 1.0 {
            255
        } else {
            // JS `(number * 256.0) | 0`: ToInt32 truncation.
            to_int32(number * 256.0)
        }
    }

    /// Returns true if the first Color equals the second color.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.red == right.red
            && left.green == right.green
            && left.blue == right.blue
            && left.alpha == right.alpha
    }

    /// Returns true if the color equals an array at the given offset.
    ///
    /// Port of `Color.equalsArray` (`@private` in Color.js).
    pub fn equals_array(&self, array: &[f64], offset: usize) -> bool {
        self.red == array[offset]
            && self.green == array[offset + 1]
            && self.blue == array[offset + 2]
            && self.alpha == array[offset + 3]
    }

    /// Returns `true` if this Color equals other componentwise within the
    /// specified epsilon.
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.red - other.red).abs() <= epsilon
            && (self.green - other.green).abs() <= epsilon
            && (self.blue - other.blue).abs() <= epsilon
            && (self.alpha - other.alpha).abs() <= epsilon
    }

    /// Creates a string containing the CSS color value for this color.
    pub fn to_css_color_string(&self) -> String {
        let red = Self::float_to_byte(self.red);
        let green = Self::float_to_byte(self.green);
        let blue = Self::float_to_byte(self.blue);
        if self.alpha == 1.0 {
            format!("rgb({red},{green},{blue})")
        } else {
            format!("rgba({red},{green},{blue},{})", self.alpha)
        }
    }

    /// Creates a string containing CSS hex string color value for this color.
    pub fn to_css_hex_string(&self) -> String {
        let r = js_to_hex16(Self::float_to_byte(self.red));
        let g = js_to_hex16(Self::float_to_byte(self.green));
        let b = js_to_hex16(Self::float_to_byte(self.blue));
        if self.alpha < 1.0 {
            let hex_alpha = js_to_hex16(Self::float_to_byte(self.alpha));
            format!("#{r}{g}{b}{hex_alpha}")
        } else {
            format!("#{r}{g}{b}")
        }
    }

    /// Converts this color to an array of red, green, blue, and alpha values
    /// that are in the range of 0 to 255.
    ///
    /// Faithful to `Color#toBytes`: the returned components are the raw
    /// `float_to_byte` results (JS numbers), NOT clamped to `[0, 255]` for
    /// out-of-range colors.
    pub fn to_bytes(&self) -> [i32; 4] {
        [
            Self::float_to_byte(self.red),
            Self::float_to_byte(self.green),
            Self::float_to_byte(self.blue),
            Self::float_to_byte(self.alpha),
        ]
    }

    /// Converts RGBA bytes to a single numeric unsigned 32-bit RGBA value,
    /// using the little-endian layout (see module-level DEVIATION note).
    pub fn bytes_to_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
        (red as u32) | ((green as u32) << 8) | ((blue as u32) << 16) | ((alpha as u32) << 24)
    }

    /// Converts this color to a single numeric unsigned 32-bit RGBA value,
    /// using the little-endian layout (see module-level DEVIATION note).
    ///
    /// Faithful to `Color#toRgba`/`bytesToRgba`: the `float_to_byte` results
    /// are stored into a `Uint8Array` in JS, which wraps out-of-range values
    /// modulo 256 before they are read back as a `Uint32`.
    pub fn to_rgba(&self) -> u32 {
        Self::bytes_to_rgba(
            wrap_uint8(Self::float_to_byte(self.red)),
            wrap_uint8(Self::float_to_byte(self.green)),
            wrap_uint8(Self::float_to_byte(self.blue)),
            wrap_uint8(Self::float_to_byte(self.alpha)),
        )
    }

    /// Brightens this color by the provided magnitude.
    ///
    /// Port of `Color#brighten(magnitude, result)` (out-parameter form).
    ///
    /// # Panics
    ///
    /// Debug builds panic with a `DeveloperError` when `magnitude` is
    /// negative.
    pub fn brighten(&self, magnitude: f64, result: &mut Color) {
        if cfg!(debug_assertions) {
            check::type_of::number_greater_than_or_equals("magnitude", magnitude, 0.0);
        }

        let magnitude = 1.0 - magnitude;
        result.red = 1.0 - (1.0 - self.red) * magnitude;
        result.green = 1.0 - (1.0 - self.green) * magnitude;
        result.blue = 1.0 - (1.0 - self.blue) * magnitude;
        result.alpha = self.alpha;
    }

    /// Allocating variant of [`Color::brighten`].
    pub fn brighten_new(&self, magnitude: f64) -> Color {
        let mut result = Color::default();
        self.brighten(magnitude, &mut result);
        result
    }

    /// Darkens this color by the provided magnitude.
    ///
    /// Port of `Color#darken(magnitude, result)` (out-parameter form).
    ///
    /// # Panics
    ///
    /// Debug builds panic with a `DeveloperError` when `magnitude` is
    /// negative.
    pub fn darken(&self, magnitude: f64, result: &mut Color) {
        if cfg!(debug_assertions) {
            check::type_of::number_greater_than_or_equals("magnitude", magnitude, 0.0);
        }

        let magnitude = 1.0 - magnitude;
        result.red = self.red * magnitude;
        result.green = self.green * magnitude;
        result.blue = self.blue * magnitude;
        result.alpha = self.alpha;
    }

    /// Allocating variant of [`Color::darken`].
    pub fn darken_new(&self, magnitude: f64) -> Color {
        let mut result = Color::default();
        self.darken(magnitude, &mut result);
        result
    }

    /// Creates a new Color that has the same red, green, and blue components
    /// as this Color, but with the specified alpha value.
    ///
    /// Port of `Color#withAlpha`.
    pub fn with_alpha(&self, alpha: f64) -> Self {
        Self::from_alpha(self, alpha)
    }

    // DEVIATION: `compute_luminance` does not exist in CesiumJS. It computes
    // the WCAG 2.x relative luminance of the color (alpha is ignored) and is
    // provided as a port-side extension required by the port specification.
    /// Computes the WCAG 2.x relative luminance of this color.
    ///
    /// See <https://www.w3.org/TR/WCAG21/#dfn-relative-luminance>.
    pub fn compute_luminance(&self) -> f64 {
        fn linearize(component: f64) -> f64 {
            if component <= 0.03928 {
                component / 12.92
            } else {
                ((component + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linearize(self.red)
            + 0.7152 * linearize(self.green)
            + 0.0722 * linearize(self.blue)
    }

    /// Computes the componentwise sum of two Colors.
    pub fn add(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red + right.red,
            green: left.green + right.green,
            blue: left.blue + right.blue,
            alpha: left.alpha + right.alpha,
        }
    }

    /// Computes the componentwise difference of two Colors.
    pub fn subtract(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red - right.red,
            green: left.green - right.green,
            blue: left.blue - right.blue,
            alpha: left.alpha - right.alpha,
        }
    }

    /// Computes the componentwise product of two Colors.
    pub fn multiply(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red * right.red,
            green: left.green * right.green,
            blue: left.blue * right.blue,
            alpha: left.alpha * right.alpha,
        }
    }

    /// Computes the componentwise quotient of two Colors.
    pub fn divide(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red / right.red,
            green: left.green / right.green,
            blue: left.blue / right.blue,
            alpha: left.alpha / right.alpha,
        }
    }

    /// Computes the componentwise modulus of two Colors.
    ///
    /// Port of `Color.mod` (`mod` is a Rust keyword, hence `modulo`).
    pub fn modulo(left: &Self, right: &Self) -> Self {
        Self {
            red: left.red % right.red,
            green: left.green % right.green,
            blue: left.blue % right.blue,
            alpha: left.alpha % right.alpha,
        }
    }

    /// Computes the linear interpolation or extrapolation at t between the
    /// provided colors.
    pub fn lerp(start: &Self, end: &Self, t: f64) -> Self {
        Self {
            red: CesiumMath::lerp(start.red, end.red, t),
            green: CesiumMath::lerp(start.green, end.green, t),
            blue: CesiumMath::lerp(start.blue, end.blue, t),
            alpha: CesiumMath::lerp(start.alpha, end.alpha, t),
        }
    }

    /// Multiplies the provided Color componentwise by the provided scalar.
    pub fn multiply_by_scalar(color: &Self, scalar: f64) -> Self {
        Self {
            red: color.red * scalar,
            green: color.green * scalar,
            blue: color.blue * scalar,
            alpha: color.alpha * scalar,
        }
    }

    /// Divides the provided Color componentwise by the provided scalar.
    pub fn divide_by_scalar(color: &Self, scalar: f64) -> Self {
        Self {
            red: color.red / scalar,
            green: color.green / scalar,
            blue: color.blue / scalar,
            alpha: color.alpha / scalar,
        }
    }

    // ---- Named color constants (mirroring Color.ALICEBLUE..TRANSPARENT) ----

    /// Returns the named color matching `name` (already upper-cased), or
    /// `None`. Mirrors the `Color[color.toUpperCase()]` lookup performed by
    /// `fromCssColorString`.
    fn named_color(name: &str) -> Option<Self> {
        Some(match name {
            "ALICEBLUE" => Self::ALICEBLUE,
            "ANTIQUEWHITE" => Self::ANTIQUEWHITE,
            "AQUA" => Self::AQUA,
            "AQUAMARINE" => Self::AQUAMARINE,
            "AZURE" => Self::AZURE,
            "BEIGE" => Self::BEIGE,
            "BISQUE" => Self::BISQUE,
            "BLACK" => Self::BLACK,
            "BLANCHEDALMOND" => Self::BLANCHEDALMOND,
            "BLUE" => Self::BLUE,
            "BLUEVIOLET" => Self::BLUEVIOLET,
            "BROWN" => Self::BROWN,
            "BURLYWOOD" => Self::BURLYWOOD,
            "CADETBLUE" => Self::CADETBLUE,
            "CHARTREUSE" => Self::CHARTREUSE,
            "CHOCOLATE" => Self::CHOCOLATE,
            "CORAL" => Self::CORAL,
            "CORNFLOWERBLUE" => Self::CORNFLOWERBLUE,
            "CORNSILK" => Self::CORNSILK,
            "CRIMSON" => Self::CRIMSON,
            "CYAN" => Self::CYAN,
            "DARKBLUE" => Self::DARKBLUE,
            "DARKCYAN" => Self::DARKCYAN,
            "DARKGOLDENROD" => Self::DARKGOLDENROD,
            "DARKGRAY" | "DARKGREY" => Self::DARKGRAY,
            "DARKGREEN" => Self::DARKGREEN,
            "DARKKHAKI" => Self::DARKKHAKI,
            "DARKMAGENTA" => Self::DARKMAGENTA,
            "DARKOLIVEGREEN" => Self::DARKOLIVEGREEN,
            "DARKORANGE" => Self::DARKORANGE,
            "DARKORCHID" => Self::DARKORCHID,
            "DARKRED" => Self::DARKRED,
            "DARKSALMON" => Self::DARKSALMON,
            "DARKSEAGREEN" => Self::DARKSEAGREEN,
            "DARKSLATEBLUE" => Self::DARKSLATEBLUE,
            "DARKSLATEGRAY" | "DARKSLATEGREY" => Self::DARKSLATEGRAY,
            "DARKTURQUOISE" => Self::DARKTURQUOISE,
            "DARKVIOLET" => Self::DARKVIOLET,
            "DEEPPINK" => Self::DEEPPINK,
            "DEEPSKYBLUE" => Self::DEEPSKYBLUE,
            "DIMGRAY" | "DIMGREY" => Self::DIMGRAY,
            "DODGERBLUE" => Self::DODGERBLUE,
            "FIREBRICK" => Self::FIREBRICK,
            "FLORALWHITE" => Self::FLORALWHITE,
            "FORESTGREEN" => Self::FORESTGREEN,
            "FUCHSIA" => Self::FUCHSIA,
            "GAINSBORO" => Self::GAINSBORO,
            "GHOSTWHITE" => Self::GHOSTWHITE,
            "GOLD" => Self::GOLD,
            "GOLDENROD" => Self::GOLDENROD,
            "GRAY" | "GREY" => Self::GRAY,
            "GREEN" => Self::GREEN,
            "GREENYELLOW" => Self::GREENYELLOW,
            "HONEYDEW" => Self::HONEYDEW,
            "HOTPINK" => Self::HOTPINK,
            "INDIANRED" => Self::INDIANRED,
            "INDIGO" => Self::INDIGO,
            "IVORY" => Self::IVORY,
            "KHAKI" => Self::KHAKI,
            "LAVENDER" => Self::LAVENDER,
            "LAWNGREEN" => Self::LAWNGREEN,
            "LEMONCHIFFON" => Self::LEMONCHIFFON,
            "LIGHTBLUE" => Self::LIGHTBLUE,
            "LIGHTCORAL" => Self::LIGHTCORAL,
            "LIGHTCYAN" => Self::LIGHTCYAN,
            "LIGHTGOLDENRODYELLOW" => Self::LIGHTGOLDENRODYELLOW,
            "LIGHTGRAY" | "LIGHTGREY" => Self::LIGHTGRAY,
            "LIGHTGREEN" => Self::LIGHTGREEN,
            "LIGHTPINK" => Self::LIGHTPINK,
            "LIGHTSEAGREEN" => Self::LIGHTSEAGREEN,
            "LIGHTSKYBLUE" => Self::LIGHTSKYBLUE,
            "LIGHTSLATEGRAY" | "LIGHTSLATEGREY" => Self::LIGHTSLATEGRAY,
            "LIGHTSTEELBLUE" => Self::LIGHTSTEELBLUE,
            "LIGHTYELLOW" => Self::LIGHTYELLOW,
            "LIME" => Self::LIME,
            "LIMEGREEN" => Self::LIMEGREEN,
            "LINEN" => Self::LINEN,
            "MAGENTA" => Self::MAGENTA,
            "MAROON" => Self::MAROON,
            "MEDIUMAQUAMARINE" => Self::MEDIUMAQUAMARINE,
            "MEDIUMBLUE" => Self::MEDIUMBLUE,
            "MEDIUMORCHID" => Self::MEDIUMORCHID,
            "MEDIUMPURPLE" => Self::MEDIUMPURPLE,
            "MEDIUMSEAGREEN" => Self::MEDIUMSEAGREEN,
            "MEDIUMSLATEBLUE" => Self::MEDIUMSLATEBLUE,
            "MEDIUMSPRINGGREEN" => Self::MEDIUMSPRINGGREEN,
            "MEDIUMTURQUOISE" => Self::MEDIUMTURQUOISE,
            "MEDIUMVIOLETRED" => Self::MEDIUMVIOLETRED,
            "MIDNIGHTBLUE" => Self::MIDNIGHTBLUE,
            "MINTCREAM" => Self::MINTCREAM,
            "MISTYROSE" => Self::MISTYROSE,
            "MOCCASIN" => Self::MOCCASIN,
            "NAVAJOWHITE" => Self::NAVAJOWHITE,
            "NAVY" => Self::NAVY,
            "OLDLACE" => Self::OLDLACE,
            "OLIVE" => Self::OLIVE,
            "OLIVEDRAB" => Self::OLIVEDRAB,
            "ORANGE" => Self::ORANGE,
            "ORANGERED" => Self::ORANGERED,
            "ORCHID" => Self::ORCHID,
            "PALEGOLDENROD" => Self::PALEGOLDENROD,
            "PALEGREEN" => Self::PALEGREEN,
            "PALETURQUOISE" => Self::PALETURQUOISE,
            "PALEVIOLETRED" => Self::PALEVIOLETRED,
            "PAPAYAWHIP" => Self::PAPAYAWHIP,
            "PEACHPUFF" => Self::PEACHPUFF,
            "PERU" => Self::PERU,
            "PINK" => Self::PINK,
            "PLUM" => Self::PLUM,
            "POWDERBLUE" => Self::POWDERBLUE,
            "PURPLE" => Self::PURPLE,
            "RED" => Self::RED,
            "ROSYBROWN" => Self::ROSYBROWN,
            "ROYALBLUE" => Self::ROYALBLUE,
            "SADDLEBROWN" => Self::SADDLEBROWN,
            "SALMON" => Self::SALMON,
            "SANDYBROWN" => Self::SANDYBROWN,
            "SEAGREEN" => Self::SEAGREEN,
            "SEASHELL" => Self::SEASHELL,
            "SIENNA" => Self::SIENNA,
            "SILVER" => Self::SILVER,
            "SKYBLUE" => Self::SKYBLUE,
            "SLATEBLUE" => Self::SLATEBLUE,
            "SLATEGRAY" | "SLATEGREY" => Self::SLATEGRAY,
            "SNOW" => Self::SNOW,
            "SPRINGGREEN" => Self::SPRINGGREEN,
            "STEELBLUE" => Self::STEELBLUE,
            "TAN" => Self::TAN,
            "TEAL" => Self::TEAL,
            "THISTLE" => Self::THISTLE,
            "TOMATO" => Self::TOMATO,
            "TURQUOISE" => Self::TURQUOISE,
            "VIOLET" => Self::VIOLET,
            "WHEAT" => Self::WHEAT,
            "WHITE" => Self::WHITE,
            "WHITESMOKE" => Self::WHITESMOKE,
            "YELLOW" => Self::YELLOW,
            "YELLOWGREEN" => Self::YELLOWGREEN,
            "TRANSPARENT" => Self::TRANSPARENT,
            _ => return None,
        })
    }
}

/// Mirrors `rgbParenthesesMatcher`:
/// `/^rgba?\s*\(\s*([0-9.]+%?)\s*[,\s]+\s*([0-9.]+%?)\s*[,\s]+\s*([0-9.]+%?)(?:\s*[,\s/]+\s*([0-9.]+))?\s*\)$/i`
///
/// Returns `(red, green, blue, alpha)` already divided by the appropriate
/// scale (255.0 or 100.0 for percentages), matching the JS computation
/// `parseFloat(m) / ("%" === m.substr(-1) ? 100.0 : 255.0)`.
fn parse_rgb_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i = 0;
    // "rgb" prefix, then optional "a"
    if n < 3 || lower[0] != 'r' || lower[1] != 'g' || lower[2] != 'b' {
        return None;
    }
    i = 3;
    if i < n && lower[i] == 'a' {
        i += 1;
    }
    // \s*\(
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != '(' {
        return None;
    }
    i += 1;

    let mut components = [0.0f64; 3];
    let mut percentages = [false; 3];
    for k in 0..3usize {
        // \s*([0-9.]+%?)
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        let start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i == start {
            return None;
        }
        let token: String = chars[start..i].iter().collect();
        if i < n && chars[i] == '%' {
            percentages[k] = true;
            i += 1;
        }
        components[k] = parse_float_js(&token);

        if k < 2 {
            // \s*[,\s]+\s* between components ([,\s]+ subsumes the \s* parts)
            let separator_start = i;
            while i < n && (chars[i] == ',' || is_css_whitespace(chars[i])) {
                i += 1;
            }
            if i == separator_start {
                return None;
            }
        }
    }

    // Optional alpha group: (?:\s*[,\s/]+\s*([0-9.]+))?
    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
    if i > separator_start {
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        let number_start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i > number_start {
            let token: String = chars[number_start..i].iter().collect();
            alpha = parse_float_js(&token);
        } else {
            // Regex backtracking: the optional group fails to match entirely.
            i = before_group;
        }
    } else {
        i = before_group;
    }

    // \s*\)$
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    let scale = |k: usize| if percentages[k] { 100.0 } else { 255.0 };
    Some((
        components[0] / scale(0),
        components[1] / scale(1),
        components[2] / scale(2),
        alpha,
    ))
}

/// Mirrors `hslParenthesesMatcher`:
/// `/^hsla?\s*\(\s*([0-9.]+)\s*[,\s]+\s*([0-9.]+%)\s*[,\s]+\s*([0-9.]+%)(?:\s*[,\s/]+\s*([0-9.]+))?\s*\)$/i`
///
/// Returns the raw captured values `(hue, saturation, lightness, alpha)`;
/// saturation/lightness keep their trailing `%` semantics of JS `parseFloat`,
/// i.e. the percent sign is ignored while parsing.
fn parse_hsl_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i = 0;
    if n < 3 || lower[0] != 'h' || lower[1] != 's' || lower[2] != 'l' {
        return None;
    }
    i = 3;
    if i < n && lower[i] == 'a' {
        i += 1;
    }
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != '(' {
        return None;
    }
    i += 1;

    // Group 1: ([0-9.]+) — hue, no percent sign.
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let start = i;
    while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    if i == start {
        return None;
    }
    let hue: String = chars[start..i].iter().collect();
    let hue = parse_float_js(&hue);

    // Groups 2 and 3: ([0-9.]+%) — the percent sign is mandatory.
    let mut captured = [0.0f64; 2];
    for k in 0..2usize {
        let separator_start = i;
        while i < n && (chars[i] == ',' || is_css_whitespace(chars[i])) {
            i += 1;
        }
        if i == separator_start {
            return None;
        }
        let start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i == start || i >= n || chars[i] != '%' {
            return None;
        }
        let token: String = chars[start..i].iter().collect();
        i += 1; // consume '%'
        captured[k] = parse_float_js(&token);
    }

    // Optional alpha group (same shape as the rgb() matcher).
    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
    if i > separator_start {
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        let number_start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i > number_start {
            let token: String = chars[number_start..i].iter().collect();
            alpha = parse_float_js(&token);
        } else {
            i = before_group;
        }
    } else {
        i = before_group;
    }

    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    Some((hue, captured[0], captured[1], alpha))
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.red, self.green, self.blue, self.alpha)
    }
}

// ---- Named color constants, mirroring `Color.ALICEBLUE` ... `Color.TRANSPARENT` ----

impl Color {
    /// An Color instance initialized to CSS color #F0F8FF.
    pub const ALICEBLUE: Color = Color::from_hex_triple(0xF0, 0xF8, 0xFF);
    /// A Color instance initialized to CSS color #FAEBD7.
    pub const ANTIQUEWHITE: Color = Color::from_hex_triple(0xFA, 0xEB, 0xD7);
    /// A Color instance initialized to CSS color #00FFFF.
    pub const AQUA: Color = Color::from_hex_triple(0x00, 0xFF, 0xFF);
    /// A Color instance initialized to CSS color #7FFFD4.
    pub const AQUAMARINE: Color = Color::from_hex_triple(0x7F, 0xFF, 0xD4);
    /// A Color instance initialized to CSS color #F0FFFF.
    pub const AZURE: Color = Color::from_hex_triple(0xF0, 0xFF, 0xFF);
    /// A Color instance initialized to CSS color #F5F5DC.
    pub const BEIGE: Color = Color::from_hex_triple(0xF5, 0xF5, 0xDC);
    /// A Color instance initialized to CSS color #FFE4C4.
    pub const BISQUE: Color = Color::from_hex_triple(0xFF, 0xE4, 0xC4);
    /// A Color instance initialized to CSS color #000000.
    pub const BLACK: Color = Color::from_hex_triple(0x00, 0x00, 0x00);
    /// A Color instance initialized to CSS color #FFEBCD.
    pub const BLANCHEDALMOND: Color = Color::from_hex_triple(0xFF, 0xEB, 0xCD);
    /// A Color instance initialized to CSS color #0000FF.
    pub const BLUE: Color = Color::from_hex_triple(0x00, 0x00, 0xFF);
    /// A Color instance initialized to CSS color #8A2BE2.
    pub const BLUEVIOLET: Color = Color::from_hex_triple(0x8A, 0x2B, 0xE2);
    /// A Color instance initialized to CSS color #A52A2A.
    pub const BROWN: Color = Color::from_hex_triple(0xA5, 0x2A, 0x2A);
    /// A Color instance initialized to CSS color #DEB887.
    pub const BURLYWOOD: Color = Color::from_hex_triple(0xDE, 0xB8, 0x87);
    /// A Color instance initialized to CSS color #5F9EA0.
    pub const CADETBLUE: Color = Color::from_hex_triple(0x5F, 0x9E, 0xA0);
    /// A Color instance initialized to CSS color #7FFF00.
    pub const CHARTREUSE: Color = Color::from_hex_triple(0x7F, 0xFF, 0x00);
    /// A Color instance initialized to CSS color #D2691E.
    pub const CHOCOLATE: Color = Color::from_hex_triple(0xD2, 0x69, 0x1E);
    /// A Color instance initialized to CSS color #FF7F50.
    pub const CORAL: Color = Color::from_hex_triple(0xFF, 0x7F, 0x50);
    /// A Color instance initialized to CSS color #6495ED.
    pub const CORNFLOWERBLUE: Color = Color::from_hex_triple(0x64, 0x95, 0xED);
    /// A Color instance initialized to CSS color #FFF8DC.
    pub const CORNSILK: Color = Color::from_hex_triple(0xFF, 0xF8, 0xDC);
    /// A Color instance initialized to CSS color #DC143C.
    pub const CRIMSON: Color = Color::from_hex_triple(0xDC, 0x14, 0x3C);
    /// A Color instance initialized to CSS color #00FFFF.
    pub const CYAN: Color = Color::from_hex_triple(0x00, 0xFF, 0xFF);
    /// A Color instance initialized to CSS color #00008B.
    pub const DARKBLUE: Color = Color::from_hex_triple(0x00, 0x00, 0x8B);
    /// A Color instance initialized to CSS color #008B8B.
    pub const DARKCYAN: Color = Color::from_hex_triple(0x00, 0x8B, 0x8B);
    /// A Color instance initialized to CSS color #B8860B.
    pub const DARKGOLDENROD: Color = Color::from_hex_triple(0xB8, 0x86, 0x0B);
    /// A Color instance initialized to CSS color #A9A9A9.
    pub const DARKGRAY: Color = Color::from_hex_triple(0xA9, 0xA9, 0xA9);
    /// A Color instance initialized to CSS color #006400.
    pub const DARKGREEN: Color = Color::from_hex_triple(0x00, 0x64, 0x00);
    /// Alias of [`Color::DARKGRAY`] (as in Color.js).
    pub const DARKGREY: Color = Color::DARKGRAY;
    /// A Color instance initialized to CSS color #BDB76B.
    pub const DARKKHAKI: Color = Color::from_hex_triple(0xBD, 0xB7, 0x6B);
    /// A Color instance initialized to CSS color #8B008B.
    pub const DARKMAGENTA: Color = Color::from_hex_triple(0x8B, 0x00, 0x8B);
    /// A Color instance initialized to CSS color #556B2F.
    pub const DARKOLIVEGREEN: Color = Color::from_hex_triple(0x55, 0x6B, 0x2F);
    /// A Color instance initialized to CSS color #FF8C00.
    pub const DARKORANGE: Color = Color::from_hex_triple(0xFF, 0x8C, 0x00);
    /// A Color instance initialized to CSS color #9932CC.
    pub const DARKORCHID: Color = Color::from_hex_triple(0x99, 0x32, 0xCC);
    /// A Color instance initialized to CSS color #8B0000.
    pub const DARKRED: Color = Color::from_hex_triple(0x8B, 0x00, 0x00);
    /// A Color instance initialized to CSS color #E9967A.
    pub const DARKSALMON: Color = Color::from_hex_triple(0xE9, 0x96, 0x7A);
    /// A Color instance initialized to CSS color #8FBC8F.
    pub const DARKSEAGREEN: Color = Color::from_hex_triple(0x8F, 0xBC, 0x8F);
    /// A Color instance initialized to CSS color #483D8B.
    pub const DARKSLATEBLUE: Color = Color::from_hex_triple(0x48, 0x3D, 0x8B);
    /// A Color instance initialized to CSS color #2F4F4F.
    pub const DARKSLATEGRAY: Color = Color::from_hex_triple(0x2F, 0x4F, 0x4F);
    /// Alias of [`Color::DARKSLATEGRAY`] (as in Color.js).
    pub const DARKSLATEGREY: Color = Color::DARKSLATEGRAY;
    /// A Color instance initialized to CSS color #00CED1.
    pub const DARKTURQUOISE: Color = Color::from_hex_triple(0x00, 0xCE, 0xD1);
    /// A Color instance initialized to CSS color #9400D3.
    pub const DARKVIOLET: Color = Color::from_hex_triple(0x94, 0x00, 0xD3);
    /// A Color instance initialized to CSS color #FF1493.
    pub const DEEPPINK: Color = Color::from_hex_triple(0xFF, 0x14, 0x93);
    /// A Color instance initialized to CSS color #00BFFF.
    pub const DEEPSKYBLUE: Color = Color::from_hex_triple(0x00, 0xBF, 0xFF);
    /// A Color instance initialized to CSS color #696969.
    pub const DIMGRAY: Color = Color::from_hex_triple(0x69, 0x69, 0x69);
    /// Alias of [`Color::DIMGRAY`] (as in Color.js).
    pub const DIMGREY: Color = Color::DIMGRAY;
    /// A Color instance initialized to CSS color #1E90FF.
    pub const DODGERBLUE: Color = Color::from_hex_triple(0x1E, 0x90, 0xFF);
    /// A Color instance initialized to CSS color #B22222.
    pub const FIREBRICK: Color = Color::from_hex_triple(0xB2, 0x22, 0x22);
    /// A Color instance initialized to CSS color #FFFAF0.
    pub const FLORALWHITE: Color = Color::from_hex_triple(0xFF, 0xFA, 0xF0);
    /// A Color instance initialized to CSS color #228B22.
    pub const FORESTGREEN: Color = Color::from_hex_triple(0x22, 0x8B, 0x22);
    /// A Color instance initialized to CSS color #FF00FF.
    pub const FUCHSIA: Color = Color::from_hex_triple(0xFF, 0x00, 0xFF);
    /// A Color instance initialized to CSS color #DCDCDC.
    pub const GAINSBORO: Color = Color::from_hex_triple(0xDC, 0xDC, 0xDC);
    /// A Color instance initialized to CSS color #F8F8FF.
    pub const GHOSTWHITE: Color = Color::from_hex_triple(0xF8, 0xF8, 0xFF);
    /// A Color instance initialized to CSS color #FFD700.
    pub const GOLD: Color = Color::from_hex_triple(0xFF, 0xD7, 0x00);
    /// A Color instance initialized to CSS color #DAA520.
    pub const GOLDENROD: Color = Color::from_hex_triple(0xDA, 0xA5, 0x20);
    /// A Color instance initialized to CSS color #808080.
    pub const GRAY: Color = Color::from_hex_triple(0x80, 0x80, 0x80);
    /// A Color instance initialized to CSS color #008000.
    pub const GREEN: Color = Color::from_hex_triple(0x00, 0x80, 0x00);
    /// A Color instance initialized to CSS color #ADFF2F.
    pub const GREENYELLOW: Color = Color::from_hex_triple(0xAD, 0xFF, 0x2F);
    /// Alias of [`Color::GRAY`] (as in Color.js).
    pub const GREY: Color = Color::GRAY;
    /// A Color instance initialized to CSS color #F0FFF0.
    pub const HONEYDEW: Color = Color::from_hex_triple(0xF0, 0xFF, 0xF0);
    /// A Color instance initialized to CSS color #FF69B4.
    pub const HOTPINK: Color = Color::from_hex_triple(0xFF, 0x69, 0xB4);
    /// A Color instance initialized to CSS color #CD5C5C.
    pub const INDIANRED: Color = Color::from_hex_triple(0xCD, 0x5C, 0x5C);
    /// A Color instance initialized to CSS color #4B0082.
    pub const INDIGO: Color = Color::from_hex_triple(0x4B, 0x00, 0x82);
    /// A Color instance initialized to CSS color #FFFFF0.
    pub const IVORY: Color = Color::from_hex_triple(0xFF, 0xFF, 0xF0);
    /// A Color instance initialized to CSS color #F0E68C.
    pub const KHAKI: Color = Color::from_hex_triple(0xF0, 0xE6, 0x8C);
    /// A Color instance initialized to CSS color #E6E6FA.
    pub const LAVENDER: Color = Color::from_hex_triple(0xE6, 0xE6, 0xFA);
    /// A Color instance initialized to CSS color #7CFC00.
    pub const LAWNGREEN: Color = Color::from_hex_triple(0x7C, 0xFC, 0x00);
    /// A Color instance initialized to CSS color #FFFACD.
    pub const LEMONCHIFFON: Color = Color::from_hex_triple(0xFF, 0xFA, 0xCD);
    /// A Color instance initialized to CSS color #ADD8E6.
    pub const LIGHTBLUE: Color = Color::from_hex_triple(0xAD, 0xD8, 0xE6);
    /// A Color instance initialized to CSS color #F08080.
    pub const LIGHTCORAL: Color = Color::from_hex_triple(0xF0, 0x80, 0x80);
    /// A Color instance initialized to CSS color #E0FFFF.
    pub const LIGHTCYAN: Color = Color::from_hex_triple(0xE0, 0xFF, 0xFF);
    /// A Color instance initialized to CSS color #FAFAD2.
    pub const LIGHTGOLDENRODYELLOW: Color = Color::from_hex_triple(0xFA, 0xFA, 0xD2);
    /// A Color instance initialized to CSS color #D3D3D3.
    pub const LIGHTGRAY: Color = Color::from_hex_triple(0xD3, 0xD3, 0xD3);
    /// A Color instance initialized to CSS color #90EE90.
    pub const LIGHTGREEN: Color = Color::from_hex_triple(0x90, 0xEE, 0x90);
    /// Alias of [`Color::LIGHTGRAY`] (as in Color.js).
    pub const LIGHTGREY: Color = Color::LIGHTGRAY;
    /// A Color instance initialized to CSS color #FFB6C1.
    pub const LIGHTPINK: Color = Color::from_hex_triple(0xFF, 0xB6, 0xC1);
    /// A Color instance initialized to CSS color #20B2AA.
    pub const LIGHTSEAGREEN: Color = Color::from_hex_triple(0x20, 0xB2, 0xAA);
    /// A Color instance initialized to CSS color #87CEFA.
    pub const LIGHTSKYBLUE: Color = Color::from_hex_triple(0x87, 0xCE, 0xFA);
    /// A Color instance initialized to CSS color #778899.
    pub const LIGHTSLATEGRAY: Color = Color::from_hex_triple(0x77, 0x88, 0x99);
    /// Alias of [`Color::LIGHTSLATEGRAY`] (as in Color.js).
    pub const LIGHTSLATEGREY: Color = Color::LIGHTSLATEGRAY;
    /// A Color instance initialized to CSS color #B0C4DE.
    pub const LIGHTSTEELBLUE: Color = Color::from_hex_triple(0xB0, 0xC4, 0xDE);
    /// A Color instance initialized to CSS color #FFFFE0.
    pub const LIGHTYELLOW: Color = Color::from_hex_triple(0xFF, 0xFF, 0xE0);
    /// A Color instance initialized to CSS color #00FF00.
    pub const LIME: Color = Color::from_hex_triple(0x00, 0xFF, 0x00);
    /// A Color instance initialized to CSS color #32CD32.
    pub const LIMEGREEN: Color = Color::from_hex_triple(0x32, 0xCD, 0x32);
    /// A Color instance initialized to CSS color #FAF0E6.
    pub const LINEN: Color = Color::from_hex_triple(0xFA, 0xF0, 0xE6);
    /// A Color instance initialized to CSS color #FF00FF.
    pub const MAGENTA: Color = Color::from_hex_triple(0xFF, 0x00, 0xFF);
    /// A Color instance initialized to CSS color #800000.
    pub const MAROON: Color = Color::from_hex_triple(0x80, 0x00, 0x00);
    /// A Color instance initialized to CSS color #66CDAA.
    pub const MEDIUMAQUAMARINE: Color = Color::from_hex_triple(0x66, 0xCD, 0xAA);
    /// A Color instance initialized to CSS color #0000CD.
    pub const MEDIUMBLUE: Color = Color::from_hex_triple(0x00, 0x00, 0xCD);
    /// A Color instance initialized to CSS color #BA55D3.
    pub const MEDIUMORCHID: Color = Color::from_hex_triple(0xBA, 0x55, 0xD3);
    /// A Color instance initialized to CSS color #9370DB.
    pub const MEDIUMPURPLE: Color = Color::from_hex_triple(0x93, 0x70, 0xDB);
    /// A Color instance initialized to CSS color #3CB371.
    pub const MEDIUMSEAGREEN: Color = Color::from_hex_triple(0x3C, 0xB3, 0x71);
    /// A Color instance initialized to CSS color #7B68EE.
    pub const MEDIUMSLATEBLUE: Color = Color::from_hex_triple(0x7B, 0x68, 0xEE);
    /// A Color instance initialized to CSS color #00FA9A.
    pub const MEDIUMSPRINGGREEN: Color = Color::from_hex_triple(0x00, 0xFA, 0x9A);
    /// A Color instance initialized to CSS color #48D1CC.
    pub const MEDIUMTURQUOISE: Color = Color::from_hex_triple(0x48, 0xD1, 0xCC);
    /// A Color instance initialized to CSS color #C71585.
    pub const MEDIUMVIOLETRED: Color = Color::from_hex_triple(0xC7, 0x15, 0x85);
    /// A Color instance initialized to CSS color #191970.
    pub const MIDNIGHTBLUE: Color = Color::from_hex_triple(0x19, 0x19, 0x70);
    /// A Color instance initialized to CSS color #F5FFFA.
    pub const MINTCREAM: Color = Color::from_hex_triple(0xF5, 0xFF, 0xFA);
    /// A Color instance initialized to CSS color #FFE4E1.
    pub const MISTYROSE: Color = Color::from_hex_triple(0xFF, 0xE4, 0xE1);
    /// A Color instance initialized to CSS color #FFE4B5.
    pub const MOCCASIN: Color = Color::from_hex_triple(0xFF, 0xE4, 0xB5);
    /// A Color instance initialized to CSS color #FFDEAD.
    pub const NAVAJOWHITE: Color = Color::from_hex_triple(0xFF, 0xDE, 0xAD);
    /// A Color instance initialized to CSS color #000080.
    pub const NAVY: Color = Color::from_hex_triple(0x00, 0x00, 0x80);
    /// A Color instance initialized to CSS color #FDF5E6.
    pub const OLDLACE: Color = Color::from_hex_triple(0xFD, 0xF5, 0xE6);
    /// A Color instance initialized to CSS color #808000.
    pub const OLIVE: Color = Color::from_hex_triple(0x80, 0x80, 0x00);
    /// A Color instance initialized to CSS color #6B8E23.
    pub const OLIVEDRAB: Color = Color::from_hex_triple(0x6B, 0x8E, 0x23);
    /// A Color instance initialized to CSS color #FFA500.
    pub const ORANGE: Color = Color::from_hex_triple(0xFF, 0xA5, 0x00);
    /// A Color instance initialized to CSS color #FF4500.
    pub const ORANGERED: Color = Color::from_hex_triple(0xFF, 0x45, 0x00);
    /// A Color instance initialized to CSS color #DA70D6.
    pub const ORCHID: Color = Color::from_hex_triple(0xDA, 0x70, 0xD6);
    /// A Color instance initialized to CSS color #EEE8AA.
    pub const PALEGOLDENROD: Color = Color::from_hex_triple(0xEE, 0xE8, 0xAA);
    /// A Color instance initialized to CSS color #98FB98.
    pub const PALEGREEN: Color = Color::from_hex_triple(0x98, 0xFB, 0x98);
    /// A Color instance initialized to CSS color #AFEEEE.
    pub const PALETURQUOISE: Color = Color::from_hex_triple(0xAF, 0xEE, 0xEE);
    /// A Color instance initialized to CSS color #DB7093.
    pub const PALEVIOLETRED: Color = Color::from_hex_triple(0xDB, 0x70, 0x93);
    /// A Color instance initialized to CSS color #FFEFD5.
    pub const PAPAYAWHIP: Color = Color::from_hex_triple(0xFF, 0xEF, 0xD5);
    /// A Color instance initialized to CSS color #FFDAB9.
    pub const PEACHPUFF: Color = Color::from_hex_triple(0xFF, 0xDA, 0xB9);
    /// A Color instance initialized to CSS color #CD853F.
    pub const PERU: Color = Color::from_hex_triple(0xCD, 0x85, 0x3F);
    /// A Color instance initialized to CSS color #FFC0CB.
    pub const PINK: Color = Color::from_hex_triple(0xFF, 0xC0, 0xCB);
    /// A Color instance initialized to CSS color #DDA0DD.
    pub const PLUM: Color = Color::from_hex_triple(0xDD, 0xA0, 0xDD);
    /// A Color instance initialized to CSS color #B0E0E6.
    pub const POWDERBLUE: Color = Color::from_hex_triple(0xB0, 0xE0, 0xE6);
    /// A Color instance initialized to CSS color #800080.
    pub const PURPLE: Color = Color::from_hex_triple(0x80, 0x00, 0x80);
    /// A Color instance initialized to CSS color #FF0000.
    pub const RED: Color = Color::from_hex_triple(0xFF, 0x00, 0x00);
    /// A Color instance initialized to CSS color #BC8F8F.
    pub const ROSYBROWN: Color = Color::from_hex_triple(0xBC, 0x8F, 0x8F);
    /// A Color instance initialized to CSS color #4169E1.
    pub const ROYALBLUE: Color = Color::from_hex_triple(0x41, 0x69, 0xE1);
    /// A Color instance initialized to CSS color #8B4513.
    pub const SADDLEBROWN: Color = Color::from_hex_triple(0x8B, 0x45, 0x13);
    /// A Color instance initialized to CSS color #FA8072.
    pub const SALMON: Color = Color::from_hex_triple(0xFA, 0x80, 0x72);
    /// A Color instance initialized to CSS color #F4A460.
    pub const SANDYBROWN: Color = Color::from_hex_triple(0xF4, 0xA4, 0x60);
    /// A Color instance initialized to CSS color #2E8B57.
    pub const SEAGREEN: Color = Color::from_hex_triple(0x2E, 0x8B, 0x57);
    /// A Color instance initialized to CSS color #FFF5EE.
    pub const SEASHELL: Color = Color::from_hex_triple(0xFF, 0xF5, 0xEE);
    /// A Color instance initialized to CSS color #A0522D.
    pub const SIENNA: Color = Color::from_hex_triple(0xA0, 0x52, 0x2D);
    /// A Color instance initialized to CSS color #C0C0C0.
    pub const SILVER: Color = Color::from_hex_triple(0xC0, 0xC0, 0xC0);
    /// A Color instance initialized to CSS color #87CEEB.
    pub const SKYBLUE: Color = Color::from_hex_triple(0x87, 0xCE, 0xEB);
    /// A Color instance initialized to CSS color #6A5ACD.
    pub const SLATEBLUE: Color = Color::from_hex_triple(0x6A, 0x5A, 0xCD);
    /// A Color instance initialized to CSS color #708090.
    pub const SLATEGRAY: Color = Color::from_hex_triple(0x70, 0x80, 0x90);
    /// Alias of [`Color::SLATEGRAY`] (as in Color.js).
    pub const SLATEGREY: Color = Color::SLATEGRAY;
    /// A Color instance initialized to CSS color #FFFAFA.
    pub const SNOW: Color = Color::from_hex_triple(0xFF, 0xFA, 0xFA);
    /// A Color instance initialized to CSS color #00FF7F.
    pub const SPRINGGREEN: Color = Color::from_hex_triple(0x00, 0xFF, 0x7F);
    /// A Color instance initialized to CSS color #4682B4.
    pub const STEELBLUE: Color = Color::from_hex_triple(0x46, 0x82, 0xB4);
    /// A Color instance initialized to CSS color #D2B48C.
    pub const TAN: Color = Color::from_hex_triple(0xD2, 0xB4, 0x8C);
    /// A Color instance initialized to CSS color #008080.
    pub const TEAL: Color = Color::from_hex_triple(0x00, 0x80, 0x80);
    /// A Color instance initialized to CSS color #D8BFD8.
    pub const THISTLE: Color = Color::from_hex_triple(0xD8, 0xBF, 0xD8);
    /// A Color instance initialized to CSS color #FF6347.
    pub const TOMATO: Color = Color::from_hex_triple(0xFF, 0x63, 0x47);
    /// A Color instance initialized to CSS color #40E0D0.
    pub const TURQUOISE: Color = Color::from_hex_triple(0x40, 0xE0, 0xD0);
    /// A Color instance initialized to CSS color #EE82EE.
    pub const VIOLET: Color = Color::from_hex_triple(0xEE, 0x82, 0xEE);
    /// A Color instance initialized to CSS color #F5DEB3.
    pub const WHEAT: Color = Color::from_hex_triple(0xF5, 0xDE, 0xB3);
    /// A Color instance initialized to CSS color #FFFFFF.
    pub const WHITE: Color = Color::from_hex_triple(0xFF, 0xFF, 0xFF);
    /// A Color instance initialized to CSS color #F5F5F5.
    pub const WHITESMOKE: Color = Color::from_hex_triple(0xF5, 0xF5, 0xF5);
    /// A Color instance initialized to CSS color #FFFF00.
    pub const YELLOW: Color = Color::from_hex_triple(0xFF, 0xFF, 0x00);
    /// A Color instance initialized to CSS color #9ACD32.
    pub const YELLOWGREEN: Color = Color::from_hex_triple(0x9A, 0xCD, 0x32);
    /// A completely transparent color (`new Color(0, 0, 0, 0)` in Color.js).
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
}

// ---------------------------------------------------------------------------
// Private helpers mirroring JS coercion semantics
// ---------------------------------------------------------------------------

/// Mirrors the JS `ToInt32` coercion used by `(x) | 0`: non-finite values
/// map to `0`; finite values are truncated toward zero and wrapped modulo
/// 2^32 into the signed 32-bit range.
fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let m = value.trunc().rem_euclid(4294967296.0);
    if m >= 2147483648.0 {
        (m - 4294967296.0) as i32
    } else {
        m as i32
    }
}

/// Mirrors `Number.prototype.toString(16)` for the integer results of
/// `float_to_byte`: negative values keep their `-` sign (e.g. `-40`), and
/// results shorter than two characters are zero-prefixed (`"f" -> "0f"`).
fn js_to_hex16(value: i32) -> String {
    let s = if value < 0 {
        format!("-{:x}", -(value as i64))
    } else {
        format!("{:x}", value)
    };
    if s.len() < 2 {
        format!("0{s}")
    } else {
        s
    }
}

/// Mirrors storing a JS number into a `Uint8Array` element: truncation
/// followed by wrapping modulo 256.
fn wrap_uint8(value: i32) -> u8 {
    value.rem_euclid(256) as u8
}
