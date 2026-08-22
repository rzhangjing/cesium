//! Ported from `packages/engine/Source/Core/EasingFunction.js`.
//!
//! Easing functions for use with animation. These are from Tween.js and Robert Penner.

/// Linear easing.
pub fn linear_none(t: f64) -> f64 {
    t
}

/// Quadratic in.
pub fn quadratic_in(t: f64) -> f64 {
    t * t
}

/// Quadratic out.
pub fn quadratic_out(t: f64) -> f64 {
    t * (2.0 - t)
}

/// Quadratic in then out.
pub fn quadratic_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// Cubic in.
pub fn cubic_in(t: f64) -> f64 {
    t * t * t
}

/// Cubic out.
pub fn cubic_out(t: f64) -> f64 {
    let t1 = t - 1.0;
    t1 * t1 * t1 + 1.0
}

/// Cubic in then out.
pub fn cubic_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * t1 * t1 * t1 + 1.0
    }
}

/// Sinusoidal in.
pub fn sinusoidal_in(t: f64) -> f64 {
    1.0 - (t * std::f64::consts::FRAC_PI_2).cos()
}

/// Sinusoidal out.
pub fn sinusoidal_out(t: f64) -> f64 {
    (t * std::f64::consts::FRAC_PI_2).sin()
}

/// Sinusoidal in then out.
pub fn sinusoidal_in_out(t: f64) -> f64 {
    0.5 * (1.0 - (std::f64::consts::PI * t).cos())
}

/// Exponential in.
pub fn exponential_in(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else {
        2.0_f64.powf(10.0 * (t - 1.0))
    }
}

/// Exponential out.
pub fn exponential_out(t: f64) -> f64 {
    if (t - 1.0).abs() < f64::EPSILON {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}

/// Exponential in then out.
pub fn exponential_in_out(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else if (t - 1.0).abs() < f64::EPSILON {
        1.0
    } else if t < 0.5 {
        0.5 * 2.0_f64.powf(20.0 * t - 10.0)
    } else {
        1.0 - 0.5 * 2.0_f64.powf(-20.0 * t + 10.0)
    }
}

/// Circular in.
pub fn circular_in(t: f64) -> f64 {
    1.0 - (1.0 - t * t).sqrt()
}

/// Circular out.
pub fn circular_out(t: f64) -> f64 {
    let t1 = t - 1.0;
    (1.0 - t1 * t1).sqrt()
}

/// Circular in then out.
pub fn circular_in_out(t: f64) -> f64 {
    if t < 0.5 {
        0.5 * (1.0 - (1.0 - 4.0 * t * t).sqrt())
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * ((1.0 - t1 * t1).sqrt() + 1.0)
    }
}

/// Bounce out.
pub fn bounce_out(t: f64) -> f64 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t1 = t - 1.5 / 2.75;
        7.5625 * t1 * t1 + 0.75
    } else if t < 2.5 / 2.75 {
        let t1 = t - 2.25 / 2.75;
        7.5625 * t1 * t1 + 0.9375
    } else {
        let t1 = t - 2.625 / 2.75;
        7.5625 * t1 * t1 + 0.984375
    }
}

/// Bounce in.
pub fn bounce_in(t: f64) -> f64 {
    1.0 - bounce_out(1.0 - t)
}

/// Bounce in then out.
pub fn bounce_in_out(t: f64) -> f64 {
    if t < 0.5 {
        0.5 * bounce_in(t * 2.0)
    } else {
        0.5 * bounce_out(t * 2.0 - 1.0) + 0.5
    }
}
