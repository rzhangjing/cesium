//! JS `Math` helpers with exact ECMAScript semantics.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L1576-1596,
//! the Rust port of the `Math.round` / `Math.min` / `Math.max` behaviour used by
//! upstream `packages/engine/Source/Scene/Expression.js`.
//!
//! These differ from the naive Rust `f64` equivalents in ways the styling
//! language depends on, so they are ported verbatim rather than replaced by
//! `f64::round` / `f64::min` / `f64::max`.

/// Mirrors JS `Math.round`: halves round **towards +infinity**.
///
/// `Math.round(-0.5) == 0` (NOT `-1`). This differs from Rust `f64::round`,
/// which rounds half **away from zero** (`(-0.5f64).round() == -1.0`), so
/// copying `f64::round` here would be wrong. The JS definition is
/// `floor(x + 0.5)`.
pub fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// Mirrors JS `Math.min` NaN propagation: if either operand is `NaN` the result
/// is `NaN` (unlike `f64::min`, which ignores `NaN` and returns the other
/// operand).
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Mirrors JS `Math.max` NaN propagation: if either operand is `NaN` the result
/// is `NaN` (unlike `f64::max`, which ignores `NaN`).
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- js_round: the JS half-towards-+infinity quirk vs Rust round ---

    #[test]
    fn js_round_half_goes_up() {
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(1.5), 2.0);
        assert_eq!(js_round(2.5), 3.0);
        // The critical case: Math.round(-0.5) == 0, not -1.
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(-2.5), -2.0);
    }

    #[test]
    fn js_round_differs_from_rust_round() {
        // Direct evidence that copying f64::round would be wrong.
        assert_eq!((-0.5f64).round(), -1.0); // Rust: half away from zero
        assert_eq!(js_round(-0.5), 0.0); // JS: half towards +infinity
        assert_ne!(js_round(-0.5), (-0.5f64).round());
        // They still agree for non-half values.
        assert_eq!(js_round(1.4), 1.0);
        assert_eq!(js_round(1.4), (1.4f64).round());
        assert_eq!(js_round(-1.4), -1.0);
    }

    #[test]
    fn js_round_integers_and_specials() {
        assert_eq!(js_round(2.0), 2.0);
        assert_eq!(js_round(-3.0), -3.0);
        assert_eq!(js_round(0.0), 0.0);
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
        assert_eq!(js_round(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    // --- js_min / js_max: NaN propagation ---

    #[test]
    fn js_min_nan_propagates() {
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_min(1.0, f64::NAN).is_nan());
        assert!(js_min(f64::NAN, f64::NAN).is_nan());
        assert_eq!(js_min(1.0, 2.0), 1.0);
        assert_eq!(js_min(-1.0, -2.0), -2.0);
        assert_eq!(js_min(3.0, 3.0), 3.0);
    }

    #[test]
    fn js_max_nan_propagates() {
        assert!(js_max(f64::NAN, 1.0).is_nan());
        assert!(js_max(1.0, f64::NAN).is_nan());
        assert_eq!(js_max(1.0, 2.0), 2.0);
        assert_eq!(js_max(-1.0, -2.0), -1.0);
    }

    #[test]
    fn js_min_max_differ_from_rust_on_nan() {
        // f64::min/max ignore NaN; the JS helpers propagate it.
        assert_eq!(f64::NAN.min(1.0), 1.0); // Rust ignores NaN
        assert!(js_min(f64::NAN, 1.0).is_nan()); // JS propagates NaN
        assert_eq!(f64::NAN.max(1.0), 1.0);
        assert!(js_max(f64::NAN, 1.0).is_nan());
    }
}
