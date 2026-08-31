//! WGSL ports of `Shaders/Builtin/{Constants,Structs,Functions}/*.glsl`
//! (SH-01 task: the 143 czm_* builtins previously un-ported).
//!
//! DEVIATION: WGSL has no preprocessor or `#include`; each `.wgsl` module is
//! self-contained and inlines the constants/structs/uniforms it needs.
//! GLSL function overloads are mirrored as distinctly suffixed WGSL functions
//! (WGSL has no overloading); `out` parameters become return values/structs;
//! GLSL implicit automatic uniforms become explicit `var<uniform>` bindings
//! under group(2) (group(0)/(1) stay reserved for the smoke-path contract).
//! All such decisions are documented per-file with `DEVIATION` comments.
//!
//! Acceptance criterion: every module parses and validates with naga
//! (`naga::front::wgsl::parse_str` + `Validator::validate`).

/// All 41 `Shaders/Builtin/Constants/*.glsl` constants (+ `czm_eyeHeight`).
pub const CZM_CONSTANTS: &str = include_str!("../wgsl/builtin/czm_constants.wgsl");

/// All 8 `Shaders/Builtin/Structs/*.glsl` structs (+ ray segment constants).
pub const CZM_STRUCTS: &str = include_str!("../wgsl/builtin/czm_structs.wgsl");

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse + full validation; mirrors the check in `wgsl.rs`.
    pub(super) fn parse(source: &str, label: &str) {
        match naga::front::wgsl::parse_str(source) {
            Ok(module) => {
                let result = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module);
                assert!(result.is_ok(), "{label}: validation failed: {:?}", result.err());
            }
            Err(e) => panic!("{label}: WGSL parse failed: {e}"),
        }
    }

    /// Each ported GLSL function must be present as a WGSL `fn` in its
    /// module (overloads carry type suffixes, asserted individually).
    pub(super) fn assert_fn(source: &str, name: &str) {
        assert!(
            source.contains(&format!("fn {name}(")),
            "missing WGSL mirror of GLSL function: {name}"
        );
    }

    #[test]
    fn czm_constants_parses() {
        parse(CZM_CONSTANTS, "czm_constants");
    }

    #[test]
    fn czm_structs_parses() {
        parse(CZM_STRUCTS, "czm_structs");
    }
}
