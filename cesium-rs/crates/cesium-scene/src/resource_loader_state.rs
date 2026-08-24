//! Ported from `packages/engine/Source/Scene/ResourceLoaderState.js`.

/// The state of a resource loader, mirroring the JS constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLoaderState {
    /// The resource is unloaded.
    Unloaded,
    /// The resource is loading.
    Loading,
    /// The resource is loaded and waiting for processing.
    Loaded,
    /// The resource is processing.
    Processing,
    /// The resource is ready (fully loaded and processed).
    Ready,
    /// The resource failed to load.
    Failed,
}
