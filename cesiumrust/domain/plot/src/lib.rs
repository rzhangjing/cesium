//! cesium-plot — framework-free core of the 2D/3D situational plotting overlay.
//!
//! This crate owns the *scene document* (layers → groups → elements), the
//! geographic geometry model (single source of truth = longitude/latitude/
//! height), the pure geometry algorithms (sampling / tessellation / hit
//! testing), the multi-dimensional visibility evaluation and GeoJSON IO. None of
//! it depends on a game engine, so every rule is deterministically unit-testable
//! and reusable across front-ends.
//!
//! The Bevy integration (ECS view sync, camera picking, the interaction state
//! machine) lives in the sibling crate `cesium-plot-bevy` and consumes this
//! crate's types.
//!
//! Design doc: plan `cesium-plot_标绘系统总体设计`.

pub mod agent;
pub mod geo;
pub mod geom;
pub mod io;
pub mod model;
pub mod ops;
pub mod visibility;
