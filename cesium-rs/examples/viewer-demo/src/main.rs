//! viewer-demo — placeholder entry point of the cesium-rs viewer.
//!
//! TODO(M5): real window + frame loop
//!   1. Create a `winit` event loop and window.
//!   2. Initialize `wgpu` device/queue from the window surface
//!      (see `cesium-renderer`).
//!   3. Instantiate `cesium_widgets::Viewer` equivalents and run the
//!      render loop (Scene render per frame, analogous to CesiumJS
//!      `CesiumWidget`/`Viewer.render()` driven by requestAnimationFrame).

fn main() {
    // Dependency chain verified at compile time:
    // viewer-demo -> cesium-widgets -> cesium-scene -> cesium-renderer -> wgpu.
    println!("cesium-rs viewer-demo (M0 skeleton)");
    println!("winit + wgpu frame loop is planned for milestone M5.");
}
