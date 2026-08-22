//! One-to-one port of `packages/engine/Source/Workers`.
//!
//! Web Worker entry points of CesiumJS (geometry creation,
//! transferable object processing). In Rust these map to off-main-thread
//! tasks (rayon / tokio / wasm web workers); the pure computation kernels
//! live in `cesium-core` and are invoked from here.

#![forbid(unsafe_code)]
