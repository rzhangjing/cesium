//! One-to-one port of `packages/engine/Source/Shaders`.
//!
//! GLSL shader sources of the CesiumJS engine, embedded as Rust string
//! assets. The M2 shader strategy (GLSL passthrough via naga `glsl-in`,
//! translation to WGSL, or rewrite) is documented in
//! `docs/shader-strategy.md`.

#![forbid(unsafe_code)]
