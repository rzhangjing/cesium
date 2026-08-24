//! Mirror of `packages/engine/Specs/Core/ColorSpec.js` (one-to-one).
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to `#[test] fn` names (snake_case).
//! - `toEqual` -> `assert_eq!`, `toEqualEpsilon` -> `assert_color_epsilon`.
//! - `toThrowDeveloperError` -> `#[should_panic]` (debug builds).
//! - Cases that pass `undefined` / rely on JS result-parameter identity are
//!   statically impossible in Rust's type system; they are folded into the
//!   owning test or kept as commented stubs.

use cesium_core::cartesian4::Cartesian4;
use cesium_core::color::{Color, FromRandomOptions};
use cesium_core::math::CesiumMath;

fn assert_color_epsilon(left: &Color, right: &Color, epsilon: f64) {
    assert!(
        left.equals_epsilon(right, epsilon),
        "expected {:?} to equal {:?} within {}",
        left,
        right,
        epsilon
    );
}

#[test]
fn constructing_without_arguments_produces_expected_defaults() {
    let v = Color::default();
    assert_eq!(v.red, 1.0);
    assert_eq!(v.green, 1.0);
    assert_eq!(v.blue, 1.0);
    assert_eq!(v.alpha, 1.0);
}

#[test]
fn constructing_with_arguments_sets_property_values() {
    let v = Color::new(0.1, 0.2, 0.3, 0.4);
    assert_eq!(v.red, 0.1);
    assert_eq!(v.green, 0.2);
    assert_eq!(v.blue, 0.3);
    assert_eq!(v.alpha, 0.4);
}

#[test]
fn from_bytes_without_arguments_produces_expected_defaults() {
    // Mirrors `new Color()` (the JS case constructs a default Color).
    let v = Color::default();
    assert_eq!(v.red, 1.0);
    assert_eq!(v.green, 1.0);
    assert_eq!(v.blue, 1.0);
    assert_eq!(v.alpha, 1.0);
}

#[test]
fn from_bytes_with_arguments_sets_property_values() {
    let v = Color::from_bytes(0, 255, 51, 102);
    assert_eq!(v.red, 0.0);
    assert_eq!(v.green, 1.0);
    assert_eq!(v.blue, 0.2);
    assert_eq!(v.alpha, 0.4);
}

#[test]
fn from_bytes_works_with_result_parameter() {
    // Rust returns by value; the JS result-parameter identity check
    // (`expect(v).toBe(result)`) is statically impossible and folded here.
    let v = Color::from_bytes(0, 255, 51, 102);
    assert_eq!(v.red, 0.0);
    assert_eq!(v.green, 1.0);
    assert_eq!(v.blue, 0.2);
    assert_eq!(v.alpha, 0.4);
}

#[test]
fn to_bytes_returns_the_same_values_that_from_bytes_took() {
    let r = 5u8;
    let g = 87u8;
    let b = 23u8;
    let a = 88u8;
    let c = Color::from_bytes(r, g, b, a);
    let bytes = c.to_bytes();
    assert_eq!(bytes, [r as i32, g as i32, b as i32, a as i32]);
}

#[test]
fn to_bytes_works_with_a_result_parameter() {
    // Rust returns an owned `[i32; 4]`; the JS `result` array identity check
    // is folded into the value assertion.
    let color = Color::new(0.1, 0.2, 0.3, 0.4);
    let expected_result = [25i32, 51, 76, 102];
    let returned_result = color.to_bytes();
    assert_eq!(returned_result, expected_result);
}

#[test]
fn to_bytes_out_of_range_returns_unclamped_js_values() {
    // CesiumJS `toBytes` returns raw `floatToByte` JS numbers without
    // clamping to [0, 255] (Phase 2 golden: color.toBytes.c6).
    let color = Color::new(1.5, -0.25, 0.3, 2.0);
    assert_eq!(color.to_bytes(), [384i32, -64, 76, 512]);
}

#[test]
fn byte_to_float_works_in_all_cases() {
    assert_eq!(Color::byte_to_float(0), 0.0);
    assert_eq!(Color::byte_to_float(255), 1.0);
    assert_eq!(Color::byte_to_float(127), 127.0 / 255.0);
}

#[test]
fn float_to_byte_works_in_all_cases() {
    assert_eq!(Color::float_to_byte(0.0), 0);
    assert_eq!(Color::float_to_byte(1.0), 255);
    assert_eq!(Color::float_to_byte(127.0 / 255.0), 127);
}

#[test]
fn float_to_byte_matches_js_truncation_semantics_out_of_range() {
    // CesiumJS: `number === 1.0 ? 255 : (number * 256) | 0` — no clamping,
    // ToInt32 truncation, NaN coerces to 0 (Phase 2 finding D3).
    assert_eq!(Color::float_to_byte(1.5), 384);
    assert_eq!(Color::float_to_byte(-0.25), -64);
    assert_eq!(Color::float_to_byte(0.3), 76);
    assert_eq!(Color::float_to_byte(2.0), 512);
    assert_eq!(Color::float_to_byte(f64::NAN), 0);
    assert_eq!(Color::float_to_byte(0.999), 255);
}

#[test]
fn to_css_hex_string_out_of_range_matches_js() {
    // CesiumJS golden (color.toCssHexString.c6): unclamped components use
    // `Number.toString(16)` — negatives keep their sign, no byte clamping.
    let color = Color::new(1.5, -0.25, 0.3, 2.0);
    assert_eq!(color.to_css_hex_string(), "#180-404c");
}

#[test]
fn to_rgba_out_of_range_wraps_like_uint8_array() {
    // CesiumJS golden (color.toRgba.c6): bytesToRgba stores into a
    // Uint8Array, wrapping out-of-range floatToByte values mod 256.
    let color = Color::new(1.5, -0.25, 0.3, 2.0);
    assert_eq!(color.to_rgba(), 5030016);
}

#[test]
fn from_cartesian4_returns_a_color_with_correct_values() {
    let color = Color::from_cartesian4(&Cartesian4::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(color, Color::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn from_cartesian4_result_param_returns_color_with_correct_values() {
    // Rust returns by value; result-parameter identity folded away.
    let color = Color::from_cartesian4(&Cartesian4::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(color, Color::new(1.0, 2.0, 3.0, 4.0));
}

// Statically impossible in Rust (typed parameter, no `undefined`):
// it("fromCartesian4 throws without a Cartesian4", ...)

#[test]
fn clone_with_no_parameters_returns_a_new_identical_copy() {
    let v = Color::new(0.1, 0.2, 0.3, 0.4);
    let clone = v.clone();
    assert_eq!(clone, v);
    // `expect(clone).not.toBe(v)` — Rust value semantics make this inherent.
}

#[test]
fn clone_with_a_parameter_modifies_the_parameter() {
    // JS `v.clone(v2)` writes into `v2`; in Rust the copy is the value itself.
    let v = Color::new(0.1, 0.2, 0.3, 0.4);
    let v2 = v.clone();
    assert_eq!(v2, v);
}

#[test]
fn equals_works() {
    let v = Color::new(0.1, 0.2, 0.3, 0.4);
    let v2 = Color::new(0.1, 0.2, 0.3, 0.4);
    let v3 = Color::new(0.1, 0.2, 0.3, 0.5);
    let v4 = Color::new(0.1, 0.2, 0.5, 0.4);
    let v5 = Color::new(0.1, 0.5, 0.3, 0.4);
    let v6 = Color::new(0.5, 0.2, 0.3, 0.4);
    assert_eq!(Color::equals(&v, &v2), true);
    assert_eq!(Color::equals(&v, &v3), false);
    assert_eq!(Color::equals(&v, &v4), false);
    assert_eq!(Color::equals(&v, &v5), false);
    assert_eq!(Color::equals(&v, &v6), false);
}

#[test]
fn equals_epsilon_works() {
    let v = Color::new(0.1, 0.2, 0.3, 0.4);
    let v2 = Color::new(0.1, 0.2, 0.3, 0.4);
    let v3 = Color::new(0.1, 0.2, 0.3, 0.5);
    let v4 = Color::new(0.1, 0.2, 0.5, 0.4);
    let v5 = Color::new(0.1, 0.5, 0.3, 0.4);
    let v6 = Color::new(0.5, 0.2, 0.3, 0.4);
    assert_eq!(v.equals_epsilon(&v2, 0.0), true);
    assert_eq!(v.equals_epsilon(&v3, 0.0), false);
    assert_eq!(v.equals_epsilon(&v4, 0.0), false);
    assert_eq!(v.equals_epsilon(&v5, 0.0), false);
    assert_eq!(v.equals_epsilon(&v6, 0.0), false);

    assert_eq!(v.equals_epsilon(&v2, 0.1), true);
    assert_eq!(v.equals_epsilon(&v3, 0.1), true);
    assert_eq!(v.equals_epsilon(&v4, 0.2), true);
    assert_eq!(v.equals_epsilon(&v5, 0.3), true);
    assert_eq!(v.equals_epsilon(&v6, 0.4), true);
}

#[test]
fn to_css_color_string_produces_expected_output() {
    assert_eq!(Color::WHITE.to_css_color_string(), "rgb(255,255,255)");
    assert_eq!(Color::RED.to_css_color_string(), "rgb(255,0,0)");
    assert_eq!(Color::BLUE.to_css_color_string(), "rgb(0,0,255)");
    assert_eq!(Color::LIME.to_css_color_string(), "rgb(0,255,0)");
    assert_eq!(
        Color::new(0.0, 0.0, 0.0, 1.0).to_css_color_string(),
        "rgb(0,0,0)"
    );
    assert_eq!(
        Color::new(0.1, 0.2, 0.3, 0.4).to_css_color_string(),
        "rgba(25,51,76,0.4)"
    );
}

#[test]
fn to_css_hex_string_produces_expected_output() {
    assert_eq!(Color::WHITE.to_css_hex_string(), "#ffffff");
    assert_eq!(Color::RED.to_css_hex_string(), "#ff0000");
    assert_eq!(Color::BLUE.to_css_hex_string(), "#0000ff");
    assert_eq!(Color::LIME.to_css_hex_string(), "#00ff00");
    assert_eq!(Color::new(0.0, 0.0, 0.0, 1.0).to_css_hex_string(), "#000000");
    assert_eq!(
        Color::new(0.1, 0.2, 0.3, 0.4).to_css_hex_string(),
        "#19334c66"
    );
}

#[test]
fn from_css_color_string_supports_transparent() {
    assert_eq!(
        Color::from_css_color_string("transparent").unwrap(),
        Color::new(0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_format() {
    assert_eq!(
        Color::from_css_color_string("#369").unwrap(),
        Color::new(0.2, 0.4, 0.6, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_format_with_lowercase() {
    assert_eq!(Color::from_css_color_string("#f00").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("#0f0").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("#00f").unwrap(), Color::BLUE);
}

#[test]
fn from_css_color_string_supports_the_rgb_format_with_uppercase() {
    assert_eq!(Color::from_css_color_string("#F00").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("#0F0").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("#00F").unwrap(), Color::BLUE);
}

#[test]
fn from_css_color_string_supports_the_rgba_format() {
    assert_eq!(
        Color::from_css_color_string("#369c").unwrap(),
        Color::new(0.2, 0.4, 0.6, 0.8)
    );
}

#[test]
fn from_css_color_string_supports_the_rgba_format_with_uppercase() {
    assert_eq!(
        Color::from_css_color_string("#369C").unwrap(),
        Color::new(0.2, 0.4, 0.6, 0.8)
    );
}

#[test]
fn from_css_color_string_supports_the_rrggbb_format() {
    assert_eq!(
        Color::from_css_color_string("#336699").unwrap(),
        Color::new(0.2, 0.4, 0.6, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rrggbb_format_with_lowercase() {
    assert_eq!(Color::from_css_color_string("#ff0000").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("#00ff00").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("#0000ff").unwrap(), Color::BLUE);
}

#[test]
fn from_css_color_string_supports_the_rrggbb_format_with_uppercase() {
    assert_eq!(Color::from_css_color_string("#FF0000").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("#00FF00").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("#0000FF").unwrap(), Color::BLUE);
}

#[test]
fn from_css_color_string_supports_the_rrggbbaa_format() {
    assert_eq!(
        Color::from_css_color_string("#336699cc").unwrap(),
        Color::new(0.2, 0.4, 0.6, 0.8)
    );
}

#[test]
fn from_css_color_string_supports_the_rrggbbaa_format_with_uppercase() {
    assert_eq!(
        Color::from_css_color_string("#336699CC").unwrap(),
        Color::new(0.2, 0.4, 0.6, 0.8)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_function_format_with_absolute_values() {
    assert_eq!(Color::from_css_color_string("rgb(255, 0, 0)").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("rgb(0, 255, 0)").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("rgb(0, 0, 255)").unwrap(), Color::BLUE);
    assert_eq!(
        Color::from_css_color_string("rgb(51, 102, 204)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_function_format_space_separated() {
    assert_eq!(Color::from_css_color_string("rgb(255 0 0)").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("rgb(0 255 0)").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("rgb(0 0 255)").unwrap(), Color::BLUE);
    assert_eq!(
        Color::from_css_color_string("rgb(51 102 204)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_function_format_with_percentages() {
    assert_eq!(Color::from_css_color_string("rgb(100%, 0, 0)").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("rgb(0, 100%, 0)").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("rgb(0, 0, 100%)").unwrap(), Color::BLUE);
    assert_eq!(
        Color::from_css_color_string("rgb(20%, 40%, 80%)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgb_function_percentages_space_separated() {
    assert_eq!(Color::from_css_color_string("rgb(100% 0 0)").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("rgb(0 100% 0)").unwrap(), Color::LIME);
    assert_eq!(Color::from_css_color_string("rgb(0 0 100%)").unwrap(), Color::BLUE);
    assert_eq!(
        Color::from_css_color_string("rgb(20% 40% 80%)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 1.0)
    );
}

#[test]
fn from_css_color_string_supports_the_rgba_function_format_with_absolute_values() {
    assert_eq!(
        Color::from_css_color_string("rgba(255, 0, 0, 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0, 255, 0, 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0, 0, 255, 1.0)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("rgba(51, 102, 204, 0.6)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 0.6)
    );
}

#[test]
fn from_css_color_string_supports_the_rgba_function_absolute_space_separated() {
    assert_eq!(
        Color::from_css_color_string("rgba(255 0 0 / 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0 255 0 / 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0 0 255 / 1.0)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("rgba(51 102 204 / 0.6)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 0.6)
    );
}

#[test]
fn from_css_color_string_supports_the_rgba_function_format_with_percentages() {
    assert_eq!(
        Color::from_css_color_string("rgba(100%, 0, 0, 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0, 100%, 0, 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0, 0, 100%, 1.0)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("rgba(20%, 40%, 80%, 0.6)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 0.6)
    );
}

#[test]
fn from_css_color_string_supports_the_rgba_function_percentages_space_separated() {
    assert_eq!(
        Color::from_css_color_string("rgba(100% 0 0 / 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0 100% 0 / 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("rgba(0 0 100% / 1.0)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("rgba(20% 40% 80% / 0.6)").unwrap(),
        Color::new(0.2, 0.4, 0.8, 0.6)
    );
}

#[test]
fn from_css_color_string_supports_named_colors_regardless_of_case() {
    assert_eq!(Color::from_css_color_string("red").unwrap(), Color::RED);
    assert_eq!(Color::from_css_color_string("GREEN").unwrap(), Color::GREEN);
    assert_eq!(Color::from_css_color_string("BLue").unwrap(), Color::BLUE);
}

#[test]
fn from_css_color_string_supports_the_hsl_format() {
    assert_eq!(
        Color::from_css_color_string("hsl(0, 100%, 50%)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsl(120, 100%, 50%)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("hsl(240, 100%, 50%)").unwrap(),
        Color::BLUE
    );
    assert_color_epsilon(
        &Color::from_css_color_string("hsl(220, 60%, 50%)").unwrap(),
        &Color::new(0.2, 0.4, 0.8, 1.0),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn from_css_color_string_supports_the_hsl_format_space_separated() {
    assert_eq!(
        Color::from_css_color_string("hsl(0 100% 50%)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsl(120 100% 50%)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("hsl(240 100% 50%)").unwrap(),
        Color::BLUE
    );
    assert_color_epsilon(
        &Color::from_css_color_string("hsl(220 60% 50%)").unwrap(),
        &Color::new(0.2, 0.4, 0.8, 1.0),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn from_css_color_string_supports_the_hsla_format() {
    assert_eq!(
        Color::from_css_color_string("hsla(0, 100%, 50%, 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsla(120, 100%, 50%, 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("hsla(240, 100%, 50%, 1.0)").unwrap(),
        Color::BLUE
    );
    assert_color_epsilon(
        &Color::from_css_color_string("hsla(220, 60%, 50%, 0.6)").unwrap(),
        &Color::new(0.2, 0.4, 0.8, 0.6),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn from_css_color_string_supports_the_hsla_format_space_separated() {
    assert_eq!(
        Color::from_css_color_string("hsla(0 100% 50% / 1.0)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsla(120 100% 50% / 1.0)").unwrap(),
        Color::LIME
    );
    assert_eq!(
        Color::from_css_color_string("hsla(240 100% 50% / 1.0)").unwrap(),
        Color::BLUE
    );
    assert_color_epsilon(
        &Color::from_css_color_string("hsla(220 60% 50% / 0.6)").unwrap(),
        &Color::new(0.2, 0.4, 0.8, 0.6),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn from_css_color_string_wraps_hue_into_valid_range_for_hsl_format() {
    assert_eq!(
        Color::from_css_color_string("hsl(720, 100%, 50%)").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsla(720, 100%, 50%, 1.0)").unwrap(),
        Color::RED
    );
}

#[test]
fn from_css_color_string_returns_none_for_unknown_colors() {
    assert!(Color::from_css_color_string("not a color").is_none());
}

// Statically impossible in Rust (typed `&str` parameter, no `undefined`):
// it("fromCssColorString throws with undefined", ...)

#[test]
fn from_css_color_string_works_with_a_result_parameter() {
    // Rust has no result parameter; the JS case exercises repeated parsing
    // and the alpha reset on re-parse, which is mirrored here.
    let c = Color::from_css_color_string("yellow").unwrap();
    assert_eq!(c, Color::YELLOW);

    let c = Color::from_css_color_string("#f00").unwrap();
    assert_eq!(c, Color::RED);

    // resets alpha to 1.0
    let c = Color::from_css_color_string("#f00").unwrap();
    assert_eq!(c, Color::RED);

    let c = Color::from_css_color_string("#0000ff").unwrap();
    assert_eq!(c, Color::BLUE);

    let c = Color::from_css_color_string("rgb(0, 255, 255)").unwrap();
    assert_eq!(c, Color::CYAN);

    let c = Color::from_css_color_string("hsl(120, 100%, 50%)").unwrap();
    assert_eq!(c, Color::LIME);
}

#[test]
fn from_css_color_string_understands_strings_with_unnecessary_spaces() {
    assert_eq!(
        Color::from_css_color_string(" rgb( 0, 0, 255)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("rgb( 255, 255, 255) ").unwrap(),
        Color::WHITE
    );
    assert_eq!(
        Color::from_css_color_string("rgb (0 0 255) ").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::from_css_color_string("  #FF0000").unwrap(),
        Color::RED
    );
    assert_eq!(Color::from_css_color_string("#FF0  ").unwrap(), Color::YELLOW);
    assert_eq!(
        Color::from_css_color_string(" hsla(720,   100%, 50%, 1.0)  ").unwrap(),
        Color::RED
    );
    assert_eq!(
        Color::from_css_color_string("hsl (720, 100%, 50%)").unwrap(),
        Color::RED
    );
}

#[test]
fn from_hsl_produces_expected_output() {
    assert_eq!(Color::from_hsl(0.0, 1.0, 0.5, 1.0), Color::RED);
    assert_eq!(Color::from_hsl(120.0 / 360.0, 1.0, 0.5, 1.0), Color::LIME);
    assert_eq!(Color::from_hsl(240.0 / 360.0, 1.0, 0.5, 1.0), Color::BLUE);
    assert_color_epsilon(
        &Color::from_hsl(220.0 / 360.0, 0.6, 0.5, 0.7),
        &Color::new(0.2, 0.4, 0.8, 0.7),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn from_hsl_properly_wraps_hue_into_valid_range() {
    assert_eq!(Color::from_hsl(5.0, 1.0, 0.5, 1.0), Color::RED);
}

#[test]
fn from_hsl_works_with_result_parameter() {
    // Rust returns by value; result-parameter identity folded away.
    let c = Color::from_hsl(5.0, 1.0, 0.5, 1.0);
    assert_eq!(c, Color::RED);
}

#[test]
fn from_random_generates_a_random_color_with_no_options() {
    let color = Color::from_random(None);
    assert!((0.0..=1.0).contains(&color.red));
    assert!((0.0..=1.0).contains(&color.green));
    assert!((0.0..=1.0).contains(&color.blue));
    assert!((0.0..=1.0).contains(&color.alpha));
}

#[test]
fn from_random_generates_a_random_color_with_empty_options() {
    // Mirrors the `Color.fromRandom({}, result)` case (result folded away).
    let color = Color::from_random(Some(&FromRandomOptions::default()));
    assert!((0.0..=1.0).contains(&color.red));
    assert!((0.0..=1.0).contains(&color.green));
    assert!((0.0..=1.0).contains(&color.blue));
    assert!((0.0..=1.0).contains(&color.alpha));
}

#[test]
fn from_random_uses_specified_exact_values() {
    let options = FromRandomOptions {
        red: Some(0.1),
        green: Some(0.2),
        blue: Some(0.3),
        alpha: Some(0.4),
        ..Default::default()
    };
    let color = Color::from_random(Some(&options));
    assert_eq!(color.red, 0.1);
    assert_eq!(color.green, 0.2);
    assert_eq!(color.blue, 0.3);
    assert_eq!(color.alpha, 0.4);
}

#[test]
fn from_random_generates_a_random_kind_of_color_within_intervals() {
    let options = FromRandomOptions {
        minimum_red: Some(0.1),
        maximum_red: Some(0.2),
        minimum_green: Some(0.3),
        maximum_green: Some(0.4),
        minimum_blue: Some(0.5),
        maximum_blue: Some(0.6),
        minimum_alpha: Some(0.7),
        maximum_alpha: Some(0.8),
        ..Default::default()
    };

    for _ in 0..100 {
        let color = Color::from_random(Some(&options));
        assert!((0.1..=0.2).contains(&color.red));
        assert!((0.3..=0.4).contains(&color.green));
        assert!((0.5..=0.6).contains(&color.blue));
        assert!((0.7..=0.8).contains(&color.alpha));
    }
}

#[test]
#[should_panic]
fn from_random_throws_with_invalid_minimum_maximum_red_values() {
    let options = FromRandomOptions {
        minimum_red: Some(1.0),
        maximum_red: Some(0.0),
        ..Default::default()
    };
    let _ = Color::from_random(Some(&options));
}

#[test]
#[should_panic]
fn from_random_throws_with_invalid_minimum_maximum_green_values() {
    let options = FromRandomOptions {
        minimum_green: Some(1.0),
        maximum_green: Some(0.0),
        ..Default::default()
    };
    let _ = Color::from_random(Some(&options));
}

#[test]
#[should_panic]
fn from_random_throws_with_invalid_minimum_maximum_blue_values() {
    let options = FromRandomOptions {
        minimum_blue: Some(1.0),
        maximum_blue: Some(0.0),
        ..Default::default()
    };
    let _ = Color::from_random(Some(&options));
}

#[test]
#[should_panic]
fn from_random_throws_with_invalid_minimum_maximum_alpha_values() {
    let options = FromRandomOptions {
        minimum_alpha: Some(1.0),
        maximum_alpha: Some(0.0),
        ..Default::default()
    };
    let _ = Color::from_random(Some(&options));
}

#[test]
fn from_alpha_works() {
    let result = Color::from_alpha(&Color::RED, 0.5);
    assert_eq!(result, Color::new(1.0, 0.0, 0.0, 0.5));
}

#[test]
fn from_alpha_works_with_result_parameter() {
    // Rust returns by value; result-parameter identity folded away.
    let result = Color::from_alpha(&Color::RED, 0.5);
    assert_eq!(result, Color::new(1.0, 0.0, 0.0, 0.5));
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("fromAlpha throws with undefined color", ...) x2
// it("fromAlpha throws with undefined alpha", ...)

#[test]
fn with_alpha_works() {
    // Rust `withAlpha` returns an owned Color; result parameter folded away.
    let result = Color::RED.with_alpha(0.5);
    assert_eq!(result, Color::new(1.0, 0.0, 0.0, 0.5));
}

#[test]
fn to_string_produces_correct_results() {
    assert_eq!(
        format!("{}", Color::new(0.1, 0.2, 0.3, 0.4)),
        "(0.1, 0.2, 0.3, 0.4)"
    );
}

#[test]
fn can_convert_to_and_from_rgba() {
    // exact values will depend on endianness, but it should round-trip.
    let color = Color::from_bytes(0xff, 0xcc, 0x00, 0xee);

    let rgba = color.to_rgba();
    assert!(rgba > 0);

    let new_color = Color::from_rgba(rgba);
    assert_eq!(color, new_color);
}

#[test]
fn from_rgba_works_with_result_parameter() {
    // Rust returns by value; result-parameter identity folded away.
    let color = Color::from_bytes(0xff, 0xcc, 0x00, 0xee);
    let rgba = color.to_rgba();

    let new_color = Color::from_rgba(rgba);
    assert_eq!(color, new_color);
}

#[test]
fn can_brighten() {
    let dark = Color::new(0.2, 0.4, 0.6, 0.8);
    let mut brighter = Color::default();
    dark.brighten(0.5, &mut brighter);
    assert_eq!(brighter.red, 0.6);
    assert_eq!(brighter.green, 0.7);
    assert_eq!(brighter.blue, 0.8);
    assert_eq!(brighter.alpha, 0.8);
}

#[test]
fn can_darken() {
    let dark = Color::new(0.1, 0.6, 0.8, 0.8);
    let mut darker = Color::default();
    dark.darken(0.2, &mut darker);
    assert!((darker.red - 0.08).abs() < CesiumMath::EPSILON15);
    assert!((darker.green - 0.48).abs() < CesiumMath::EPSILON15);
    assert!((darker.blue - 0.64).abs() < CesiumMath::EPSILON15);
    assert!((darker.alpha - 0.8).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (`result` is a required `&mut Color`):
// it("brighten throws without result", ...)
// it("darken throws without result", ...)

#[test]
#[should_panic]
fn brighten_throws_negative_magnitude() {
    let mut result = Color::default();
    Color::RED.brighten(-0.5, &mut result);
}

#[test]
#[should_panic]
fn darken_throws_negative_magnitude() {
    let mut result = Color::default();
    Color::RED.darken(-0.5, &mut result);
}

// Statically impossible in Rust (typed `f64` magnitude, no `undefined`):
// it("brighten throws undefined magnitude", ...)
// it("darken throws undefined magnitude", ...)

#[test]
fn can_add() {
    let left = Color::new(0.1, 0.2, 0.3, 0.4);
    let right = Color::new(0.3, 0.3, 0.3, 0.3);
    let result = Color::add(&left, &right);
    assert_eq!(result.red, 0.4);
    assert_eq!(result.green, 0.5);
    assert_eq!(result.blue, 0.6);
    assert_eq!(result.alpha, 0.7);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("add throws with undefined parameters", ...)
// The "result parameter that is an input parameter" case is inherent to
// Rust's by-value semantics and folded into `can_add`.

#[test]
fn can_subtract() {
    let left = Color::new(1.0, 1.0, 1.0, 1.0);
    let right = Color::new(0.1, 0.2, 0.3, 0.4);
    let result = Color::subtract(&left, &right);
    assert_eq!(result.red, 0.9);
    assert_eq!(result.green, 0.8);
    assert_eq!(result.blue, 0.7);
    assert_eq!(result.alpha, 0.6);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("subtract throws with undefined parameters", ...)

#[test]
fn can_multiply() {
    let left = Color::new(0.1, 0.2, 0.3, 0.4);
    let right = Color::new(0.2, 0.2, 0.2, 0.2);
    let result = Color::multiply(&left, &right);
    assert!((result.red - 0.02).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 0.04).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.06).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.08).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("multiply throws with undefined parameters", ...)

#[test]
fn can_divide() {
    let left = Color::new(0.1, 0.2, 0.1, 0.2);
    let right = Color::new(0.2, 0.2, 0.4, 0.4);
    let result = Color::divide(&left, &right);
    assert!((result.red - 0.5).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 1.0).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.25).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.5).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("divide throws with undefined parameters", ...)

#[test]
fn can_mod() {
    let left = Color::new(0.1, 0.2, 0.3, 0.2);
    let right = Color::new(0.2, 0.2, 0.2, 0.4);
    let result = Color::modulo(&left, &right);
    assert!((result.red - 0.1).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 0.0).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.1).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.2).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("mod throws with undefined parameters", ...)

#[test]
fn can_multiply_by_scalar() {
    let color = Color::new(0.1, 0.2, 0.3, 0.4);
    let result = Color::multiply_by_scalar(&color, 2.0);
    assert!((result.red - 0.2).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 0.4).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.6).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.8).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("multiply by scalar throws with undefined parameters", ...)

#[test]
fn can_divide_by_scalar() {
    let color = Color::new(0.1, 0.2, 0.3, 0.4);
    let result = Color::divide_by_scalar(&color, 2.0);
    assert!((result.red - 0.05).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 0.1).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.15).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.2).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("divide by scalar throws with undefined parameters", ...)
// it("lerp throws with undefined parameters", ...)

#[test]
fn can_lerp_between_two_colors() {
    let color_a = Color::new(0.0, 0.0, 0.0, 0.0);
    let color_b = Color::new(1.0, 1.0, 1.0, 1.0);
    let result = Color::lerp(&color_a, &color_b, 0.5);

    assert!((result.red - 0.5).abs() < CesiumMath::EPSILON15);
    assert!((result.green - 0.5).abs() < CesiumMath::EPSILON15);
    assert!((result.blue - 0.5).abs() < CesiumMath::EPSILON15);
    assert!((result.alpha - 0.5).abs() < CesiumMath::EPSILON15);
}

// --- createPackableSpecs(Color, new Color(0.1, 0.2, 0.3, 0.4), [0.1, 0.2, 0.3, 0.4]) ---

#[test]
fn packable_pack_works() {
    let mut packed = vec![0.0f64; Color::PACKED_LENGTH];
    Color::pack(&Color::new(0.1, 0.2, 0.3, 0.4), &mut packed, 0);
    assert_eq!(packed, [0.1, 0.2, 0.3, 0.4]);
}

#[test]
fn packable_pack_works_with_starting_index() {
    let mut packed = vec![0.0f64; Color::PACKED_LENGTH + 2];
    Color::pack(&Color::new(0.1, 0.2, 0.3, 0.4), &mut packed, 1);
    assert_eq!(packed, [0.0, 0.1, 0.2, 0.3, 0.4, 0.0]);
}

#[test]
fn packable_packed_length_is_correct() {
    assert_eq!(Color::PACKED_LENGTH, 4);
}

#[test]
fn packable_unpack_works() {
    let unpacked = Color::unpack(&[0.1, 0.2, 0.3, 0.4], 0);
    assert_eq!(unpacked, Color::new(0.1, 0.2, 0.3, 0.4));
}

#[test]
fn packable_unpack_works_with_starting_index() {
    let unpacked = Color::unpack(&[0.0, 0.1, 0.2, 0.3, 0.4], 1);
    assert_eq!(unpacked, Color::new(0.1, 0.2, 0.3, 0.4));
}
