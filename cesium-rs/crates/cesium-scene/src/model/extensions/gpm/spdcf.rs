//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/Spdcf.js`.

/// Variables for a Strictly Positive-Definite Correlation Function.
///
/// This reflects the `spdcf` definition of the NGA_gpm_local glTF
/// extension. Instances of this type are stored as the parameters within
/// a `CorrelationGroup`.
///
/// Parameters (A, alpha, beta, T) describe the correlation decrease
/// between points as a function of delta time:
/// `spdcf(delta_t) = A_t * (alpha_t + ((1 - alpha_t)(1 + beta_t)) / (beta_t + e^(delta_t/T_t)))`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spdcf {
    /// The factor A, in (0, 1].
    a: f64,
    /// The alpha value, in [0, 1).
    alpha: f64,
    /// The beta value, in [0, 10].
    beta: f64,
    /// The tau value, in (0, +inf).
    t: f64,
}

impl Spdcf {
    /// Creates a new `Spdcf` from the constructor options.
    ///
    /// Port of the `Spdcf(options)` constructor. The range checks are
    /// debug-only, mirroring `includeStart('debug', pragmas.debug)`.
    ///
    /// # Panics
    /// In debug builds, panics (DeveloperError) when the parameters fall
    /// outside their documented ranges.
    pub fn new(a: f64, alpha: f64, beta: f64, t: f64) -> Self {
        #[cfg(debug_assertions)]
        {
            use cesium_core::check::type_of;
            type_of::number_greater_than("options.A", a, 0.0);
            type_of::number_less_than_or_equals("options.A", a, 1.0);
            type_of::number_greater_than_or_equals("options.alpha", alpha, 0.0);
            type_of::number_less_than("options.alpha", alpha, 1.0);
            type_of::number_greater_than_or_equals("options.beta", beta, 0.0);
            type_of::number_less_than_or_equals("options.beta", beta, 10.0);
            type_of::number_greater_than("options.T", t, 0.0);
        }
        Self { a, alpha, beta, t }
    }

    /// The factor A, in (0, 1] (port of the `A` getter).
    pub fn a(&self) -> f64 {
        self.a
    }

    /// The alpha value, in [0, 1) (port of the `alpha` getter).
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// The beta value, in [0, 10] (port of the `beta` getter).
    pub fn beta(&self) -> f64 {
        self.beta
    }

    /// The tau value, in (0, +inf) (port of the `T` getter).
    pub fn t(&self) -> f64 {
        self.t
    }
}
