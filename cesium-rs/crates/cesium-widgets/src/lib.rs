//! One-to-one port of `packages/engine/Source/Widget` and
//! `packages/widgets/Source`.
//!
//! Top-level application shell: `Viewer`, `CesiumWidget`, timeline /
//! animation / base-layer-picker UI composition. The DOM-based widgets of
//! CesiumJS are mapped to a native/wasm UI layer built around the scene;
//! windowing & the frame loop use `winit` + `wgpu` (M5).

#![forbid(unsafe_code)]
