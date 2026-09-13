//! Translucency specification for Fabric materials.
//!
//! CesiumJS stores translucency either as a boolean or as a function of the
//! material's current uniform values (e.g. `material.uniforms.color.alpha <
//! 1.0`). This module captures the full set of built-in function shapes with a
//! single data-driven enum so the domain layer stays closure-free and
//! serializable.

use crate::uniform::UniformValue;
use std::collections::BTreeMap;

/// How a material decides whether it is translucent.
///
/// Maps to the `translucent` member of each entry in CesiumJS
/// `Material._materialCache`:
/// - `translucent: true`  -> [`TranslucentSpec::Always`]
/// - `translucent: false` -> [`TranslucentSpec::Never`]
/// - `translucent: function (material) { return <uniform>.alpha < 1.0 || ... }`
///   -> [`TranslucentSpec::AnyAlphaLt1`] listing the inspected uniform names.
///
/// For color (`vec4`) uniforms the function reads the alpha component; for
/// `float` uniforms (e.g. Grid's `cellAlpha`) it reads the scalar itself. Both
/// cases are handled by [`UniformValue::alpha_or_scalar`].
#[derive(Debug, Clone, PartialEq)]
pub enum TranslucentSpec {
    /// Always translucent (`translucent: true`).
    Always,
    /// Never translucent (`translucent: false`).
    Never,
    /// Translucent when any of the named uniforms has an alpha/scalar `< 1.0`.
    AnyAlphaLt1(Vec<&'static str>),
}

impl TranslucentSpec {
    /// Evaluates the spec against a material's current uniform values.
    ///
    /// A uniform that is absent or has no alpha/scalar component contributes
    /// `false` (it cannot make the material translucent), matching the
    /// CesiumJS functions which only ever read uniforms that exist.
    pub fn evaluate(&self, uniforms: &BTreeMap<String, UniformValue>) -> bool {
        match self {
            TranslucentSpec::Always => true,
            TranslucentSpec::Never => false,
            TranslucentSpec::AnyAlphaLt1(names) => names.iter().any(|name| {
                uniforms
                    .get(*name)
                    .and_then(UniformValue::alpha_or_scalar)
                    .map(|v| v < 1.0)
                    .unwrap_or(false)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniforms(pairs: &[(&str, UniformValue)]) -> BTreeMap<String, UniformValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_always_never() {
        let empty = uniforms(&[]);
        assert!(TranslucentSpec::Always.evaluate(&empty));
        assert!(!TranslucentSpec::Never.evaluate(&empty));
    }

    #[test]
    fn test_any_alpha_lt1_color() {
        let spec = TranslucentSpec::AnyAlphaLt1(vec!["color"]);
        assert!(spec.evaluate(&uniforms(&[(
            "color",
            UniformValue::Vec4([1.0, 0.0, 0.0, 0.5])
        )])));
        assert!(!spec.evaluate(&uniforms(&[(
            "color",
            UniformValue::Vec4([1.0, 0.0, 0.0, 1.0])
        )])));
    }

    #[test]
    fn test_any_alpha_lt1_scalar() {
        // Grid: cellAlpha is a float uniform.
        let spec = TranslucentSpec::AnyAlphaLt1(vec!["color", "cellAlpha"]);
        assert!(spec.evaluate(&uniforms(&[
            ("color", UniformValue::Vec4([0.0, 1.0, 0.0, 1.0])),
            ("cellAlpha", UniformValue::Float(0.1)),
        ])));
        assert!(!spec.evaluate(&uniforms(&[
            ("color", UniformValue::Vec4([0.0, 1.0, 0.0, 1.0])),
            ("cellAlpha", UniformValue::Float(1.0)),
        ])));
    }

    #[test]
    fn test_any_alpha_lt1_multiple_names_or_semantics() {
        // Stripe: evenColor.alpha < 1.0 || oddColor.alpha < 1.0
        let spec = TranslucentSpec::AnyAlphaLt1(vec!["evenColor", "oddColor"]);
        assert!(spec.evaluate(&uniforms(&[
            ("evenColor", UniformValue::Vec4([1.0, 1.0, 1.0, 1.0])),
            ("oddColor", UniformValue::Vec4([0.0, 0.0, 1.0, 0.5])),
        ])));
        assert!(!spec.evaluate(&uniforms(&[
            ("evenColor", UniformValue::Vec4([1.0, 1.0, 1.0, 1.0])),
            ("oddColor", UniformValue::Vec4([0.0, 0.0, 1.0, 1.0])),
        ])));
    }

    #[test]
    fn test_missing_uniform_is_not_translucent() {
        let spec = TranslucentSpec::AnyAlphaLt1(vec!["color"]);
        assert!(!spec.evaluate(&uniforms(&[])));
    }
}
