//! Headless-safe shader / internal-asset registration helpers.
//!
//! Bevy's [`load_internal_asset!`](bevy::asset::load_internal_asset) macro and
//! [`MaterialPlugin`](bevy::pbr::MaterialPlugin) both dereference asset-backend
//! resources (`Assets<Shader>` / `AssetServer`) that are **absent** under a
//! headless `MinimalPlugins` app (no `AssetPlugin`, no `RenderPlugin`). Calling
//! them there panics — this is the root cause tracked in
//! `docs/deviations.md#dev-005` and `docs/deferred.md#6`.
//!
//! This module wraps registration in availability guards so plugins degrade to a
//! no-op (instead of panicking) in headless/CI contexts, while behaving
//! **identically** to the raw Bevy calls when the asset backend is present (the
//! normal GPU app path — same `include_str!`-embedded source inserted at the same
//! `AssetId`, hence pixel-neutral).
//!
//! Reused by every embedded WGSL in this adapter: `fabric_material.wgsl` (M5.1),
//! and the upcoming `sky_atmosphere.wgsl` (M5-C), `fxaa.wgsl` (M5-E1) and
//! `ao.wgsl` (M5-E2).
//!
//! # Why the asset backend, not the render app?
//! Bevy 0.15 *already* guards the `RenderApp` sub-app portion of both
//! `MaterialPlugin::build` and `RenderAssetPlugin::build`
//! (`if let Some(render_app) = app.get_sub_app_mut(RenderApp) { .. }`), so a
//! missing render app is a silent no-op, not a panic. The **only** unguarded
//! headless hazards are:
//! 1. `load_internal_asset!` → `World::resource_mut::<Assets<Shader>>()`; and
//! 2. `MaterialPlugin::build` → `init_asset::<M>()` → `World::resource::<AssetServer>()`.
//!
//! Both are covered by the predicates below.

use bevy::asset::AssetServer;
use bevy::prelude::*;
use bevy::render::render_resource::Shader;

/// Whether Bevy's asset backend ([`AssetServer`]) is present.
///
/// [`AssetPlugin`](bevy::asset::AssetPlugin) inserts `AssetServer`;
/// `MinimalPlugins` does not. `MaterialPlugin::build` internally calls
/// `init_asset::<M>()`, which dereferences `AssetServer` and panics when it is
/// missing — so callers **must** gate `MaterialPlugin` registration on this
/// predicate to stay headless-safe.
pub fn asset_backend_available(app: &App) -> bool {
    app.world().contains_resource::<AssetServer>()
}

/// Whether the embedded-shader storage ([`Assets<Shader>`]) is present.
///
/// This is the exact resource `load_internal_asset!` dereferences; guarding on
/// it makes shader insertion headless-safe. It is inserted by `init_asset::<Shader>()`
/// (via the render stack), so it may be absent even when an `AssetServer` exists
/// in a render-less-but-asset-enabled app.
pub fn shader_assets_available(app: &App) -> bool {
    app.world().contains_resource::<Assets<Shader>>()
}

/// Headless-safe equivalent of [`load_internal_asset!`](bevy::asset::load_internal_asset)
/// for an embedded WGSL shader.
///
/// Inserts `Shader::from_wgsl(source, path)` into [`Assets<Shader>`] under
/// `handle` **iff** the shader storage exists, returning `Some(handle)`. When it
/// does not (headless `MinimalPlugins`), this is a no-op returning `None` rather
/// than panicking on the missing resource.
///
/// On the GPU path (asset backend present) this is byte-for-byte equivalent to
/// the raw macro: both compile the same `include_str!`-embedded `source` and
/// insert it at the same [`AssetId`], so rendering output is unchanged.
///
/// # Example (reusable across M5 WGSL additions)
/// ```text
/// crate::shader_registry::try_load_internal_shader(
///     app,
///     MY_SHADER_HANDLE,
///     include_str!("../shaders/my_shader.wgsl"),
///     std::path::Path::new(file!())
///         .parent()
///         .unwrap()
///         .join("../shaders/my_shader.wgsl")
///         .to_string_lossy(),
/// );
/// ```
pub fn try_load_internal_shader(
    app: &mut App,
    handle: Handle<Shader>,
    source: impl Into<std::borrow::Cow<'static, str>>,
    path: impl Into<String>,
) -> Option<Handle<Shader>> {
    if !shader_assets_available(app) {
        return None;
    }
    let shader = Shader::from_wgsl(source, path);
    app.world_mut()
        .resource_mut::<Assets<Shader>>()
        .insert(handle.id(), shader);
    Some(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `MinimalPlugins` app has no `AssetPlugin`, so neither the asset backend
    /// nor the shader storage exists — registration must degrade to a no-op.
    #[test]
    fn headless_minimal_app_has_no_asset_backend() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        assert!(!asset_backend_available(&app));
        assert!(!shader_assets_available(&app));

        let handle: Handle<Shader> = Handle::weak_from_u128(0xDEAD_BEEF);
        // Must NOT panic; returns None because Assets<Shader> is absent.
        let out = try_load_internal_shader(&mut app, handle, "// empty wgsl", "test.wgsl");
        assert!(out.is_none());
    }
}
