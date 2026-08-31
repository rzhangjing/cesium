//! One-to-one port of `packages/engine/Source/Shaders`.
//!
//! GLSL shader sources embedded as Rust string constants via `include_str!`.
//! Each shader file maps to a `pub const NAME: &str` in the corresponding module.

#![allow(dead_code)]

pub mod shader_top;
pub mod appearances;
pub mod builtin;
pub mod materials;
pub mod model;
pub mod post_process_stages;
pub mod voxels;
pub mod preprocessor;
pub mod wgsl;
pub mod builtin_wgsl;

