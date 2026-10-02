//! 带精确 ECMAScript 语义的 JS `Math` 辅助函数。
//!
//! 这些辅助函数复现 `Math.round` / `Math.min` / `Math.max` 在 ECMAScript 下的
//! 精确行为，与朴素 Rust `f64` 方法在取舍方向上存在差异。
//!
//! 它们与朴素的 Rust `f64` 对应物在 styling 语言所依赖的方式上有所不同，
//! 因此逐字移植而非用 `f64::round` / `f64::min` / `f64::max` 替代。

/// 镜像 JS `Math.round`：半数**朝 +infinity 方向**舍入。
///
/// `Math.round(-0.5) == 0`（不是 `-1`）。这不同于 Rust `f64::round`，
/// 后者半数**远离零**舍入（`(-0.5f64).round() == -1.0`），所以在此照搬
/// `f64::round` 会是错的。JS 的定义是 `floor(x + 0.5)`。
pub fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// 镜像 JS `Math.min` 的 NaN 传播：若任一操作数为 `NaN`，结果
/// 为 `NaN`（不同于 `f64::min`，它忽略 `NaN` 而返回另一个操作数）。
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// 镜像 JS `Math.max` 的 NaN 传播：若任一操作数为 `NaN`，结果
/// 为 `NaN`（不同于 `f64::max`，它忽略 `NaN`）。
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

    // --- js_round：JS 半数朝 +infinity 的怪癖 vs Rust round ---

    #[test]
    fn js_round_half_goes_up() {
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(1.5), 2.0);
        assert_eq!(js_round(2.5), 3.0);
        // 关键情形：Math.round(-0.5) == 0，而非 -1。
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(-2.5), -2.0);
    }

    #[test]
    fn js_round_differs_from_rust_round() {
        // 直接证据：照搬 f64::round 会是错的。
        assert_eq!((-0.5f64).round(), -1.0); // Rust：半数远离零
        assert_eq!(js_round(-0.5), 0.0); // JS：半数朝 +infinity
        assert_ne!(js_round(-0.5), (-0.5f64).round());
        // 对非半数值它们仍然一致。
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

    // --- js_min / js_max：NaN 传播 ---

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
        // f64::min/max 忽略 NaN；这些 JS 辅助函数传播它。
        assert_eq!(f64::NAN.min(1.0), 1.0); // Rust 忽略 NaN
        assert!(js_min(f64::NAN, 1.0).is_nan()); // JS 传播 NaN
        assert_eq!(f64::NAN.max(1.0), 1.0);
        assert!(js_max(f64::NAN, 1.0).is_nan());
    }
}
