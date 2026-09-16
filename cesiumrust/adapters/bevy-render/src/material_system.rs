//! Bridge domain Material → Bevy FabricMaterial.
//!
//! Provides the [`CesiumMaterialPlugin`] which reads entity components and
//! applies Fabric procedural materials to Bevy meshes, plus per-frame uniform
//! updates (e.g. water animation time).

// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use cesium_material::{MaterialSystem, UniformValue};

use crate::fabric_material::{
    fabric_material_from_domain, fabric_material_from_domain_with_maps, generate_water_normal_map,
    generate_water_specular_map, FabricKind, FabricMaterial, FabricMaterialPlugin,
};
use cesium_shadow::{OceanConfig, OceanSurface};

/// Component that references a CesiumJS Fabric material to apply to an entity.
///
/// Attach this to any entity with a [`MeshMaterial3d<FabricMaterial>`] target
/// (or insert [`FabricMaterial`] directly) to have the material generated from
/// the domain layer.
#[derive(Component, Clone, Debug)]
pub struct MaterialRef {
    /// The CesiumJS material type name (e.g. `"ElevationContour"`,
    /// `"RimLighting"`, `"Color"`).
    pub type_name: String,
    /// Optional uniform overrides (keyed by Fabric uniform name).
    pub uniforms: std::collections::BTreeMap<String, UniformValue>,
}

impl MaterialRef {
    /// Create a material reference from a type name with default uniforms.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            uniforms: std::collections::BTreeMap::new(),
        }
    }

    /// Create with uniform overrides.
    pub fn with_uniforms(
        type_name: impl Into<String>,
        uniforms: std::collections::BTreeMap<String, UniformValue>,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            uniforms,
        }
    }
}

/// Resource holding time for animated materials (e.g. Water).
#[derive(Resource, Default)]
pub struct MaterialAnimationTime {
    /// Accumulated delta-seconds since startup (legacy wall-clock field).
    pub time: f32,
    /// Monotonic frame counter mirroring CesiumJS `czm_frameNumber`
    /// (Water.glsl L18: `time = czm_frameNumber * animationSpeed`).
    /// DEVIATION: Water animation now advances per *frame* (matching CesiumJS)
    /// rather than per accumulated second; see docs/deviations.md#dev-019.
    pub frame_number: u32,
}

/// Plugin bridging domain [`cesium_material::Material`] → Bevy [`FabricMaterial`].
///
/// Adds systems that:
/// - Apply [`MaterialRef`] components to entities as [`FabricMaterial`] instances.
/// - Update animated uniform values per-frame (water time, etc.).
pub struct CesiumMaterialPlugin;

impl Plugin for CesiumMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FabricMaterialPlugin)
            .init_resource::<MaterialAnimationTime>()
            .add_systems(
                Update,
                (
                    apply_fabric_materials,
                    update_material_uniforms,
                ),
            );
    }
}

/// System: for entities with a [`MaterialRef`] component, look up the material
/// type in the domain [`MaterialSystem`], extract uniform values, and create a
/// Bevy [`FabricMaterial`] instance.
///
/// This runs only when a [`MaterialRef`] is added or changed (via `Changed<MaterialRef>`).
fn apply_fabric_materials(
    mut commands: Commands,
    material_system: Option<Res<MaterialSystemResource>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FabricMaterial>>,
    query: Query<(Entity, &MaterialRef, Option<&MeshMaterial3d<FabricMaterial>>), Changed<MaterialRef>>,
) {
    let system = match &material_system {
        Some(res) => &res.0,
        None => {
            warn!("MaterialSystemResource not available; skipping material application");
            return;
        }
    };

    // Nothing added/changed this frame → skip. Avoids allocating a fresh
    // fallback image into `Assets<Image>` every frame (M5-D).
    if query.is_empty() {
        return;
    }

    // Create a 1x1 white fallback image for materials that need a texture.
    // Materials that don't use textures (Color, etc.) ignore this binding.
    // NOTE: Rgba8UnormSrgb is correct for this *colour* fallback (imagery-like),
    // but Water's normalMap/specularMap are LINEAR and use Rgba8Unorm below —
    // never propagate this Srgb format to those (sRGB red-line).
    let fallback_img = Image::new(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        vec![255u8, 255, 255, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    let fallback_handle = images.add(fallback_img);

    for (entity, mat_ref, _existing_material) in &query {
        let domain_material = match system.from_type(&mat_ref.type_name, mat_ref.uniforms.clone()) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    "Failed to build material '{}' for entity {:?}: {}",
                    mat_ref.type_name, entity, e
                );
                continue;
            }
        };

        let fabric_material =
            if FabricKind::from_type_name(&mat_ref.type_name) == FabricKind::Water {
                // M5-D: Water binds real procedurally-generated normal/specular
                // maps (Rgba8Unorm LINEAR, from the domain cesium_shadow
                // OceanSurface); other kinds use the fallback for all bindings.
                let ocean = OceanSurface::new(OceanConfig::default());
                let normal_map = images.add(generate_water_normal_map(128, &ocean, 200.0));
                let specular_map = images.add(generate_water_specular_map(128, &ocean, 200.0));
                fabric_material_from_domain_with_maps(
                    &domain_material,
                    fallback_handle.clone(),
                    normal_map,
                    specular_map,
                )
            } else {
                fabric_material_from_domain(&domain_material, fallback_handle.clone())
            };
        let handle = materials.add(fabric_material);
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
}

/// System: per-frame uniform updates for animated materials.
///
/// Currently updates:
/// - Water time animation
fn update_material_uniforms(
    time: Res<Time>,
    mut animation_time: ResMut<MaterialAnimationTime>,
    mut materials: ResMut<Assets<FabricMaterial>>,
) {
    animation_time.time += time.delta_secs();
    // M5-D: advance a frame counter mirroring CesiumJS `czm_frameNumber`, which
    // Water.glsl L18 multiplies by animationSpeed. DEVIATION: per-frame (not
    // per-second) semantics — see docs/deviations.md#dev-019.
    animation_time.frame_number = animation_time.frame_number.wrapping_add(1);
    let frame_number = animation_time.frame_number as f32;

    // Update Water material frame-number uniform (extra_c.z).
    for (_, material) in materials.iter_mut() {
        if material.params.kind == FabricKind::Water as u32 {
            material.params.extra_c.z = frame_number;
        }
    }
}

/// Resource wrapping a [`MaterialSystem`] so it can be used as a Bevy resource.
///
/// The [`MaterialSystem`] holds the cached built-in material type definitions
/// (GLSL source + default uniforms) and is required by [`apply_fabric_materials`].
#[derive(Resource)]
pub struct MaterialSystemResource(pub MaterialSystem);

impl MaterialSystemResource {
    /// Create with all built-in CesiumJS material types pre-registered.
    pub fn with_builtin_materials() -> Self {
        Self(MaterialSystem::with_builtin_materials())
    }
}

#[cfg(test)]
mod tests {
    use super::{CesiumMaterialPlugin, MaterialAnimationTime, MaterialRef};
    use crate::CesiumCorePlugin;
    use bevy::prelude::*;

    /// Headless regression guard for `docs/deviations.md#dev-005` /
    /// `docs/deferred.md#6` (resolved at M5.1).
    ///
    /// Replicates the exact scenario of the four
    /// `specs/tests/integration/material_integration_test.rs` cases: a
    /// `create_test_app()`-style app (`MinimalPlugins` + [`CesiumCorePlugin`], with
    /// no `AssetPlugin` / `RenderApp` / wgpu device) plus [`CesiumMaterialPlugin`].
    /// Before M5.1 this panicked inside `FabricMaterialPlugin::build`
    /// (`load_internal_asset!` → missing `Assets<Shader>`, then `MaterialPlugin`
    /// → missing `AssetServer`). The headless-safe guards in
    /// [`crate::shader_registry`] make registration a no-op instead, so the plugin
    /// installs and the CPU-side material API stays usable.
    #[test]
    fn cesium_material_plugin_headless_minimal_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(CesiumCorePlugin)
            .add_plugins(CesiumMaterialPlugin);

        // test_material_animation_time_resource_initialized
        assert!(app.world().get_resource::<MaterialAnimationTime>().is_some());

        // test_material_ref_component
        let entity = app.world_mut().spawn(MaterialRef::new("Color")).id();
        let got = app.world().get::<MaterialRef>(entity);
        assert!(got.is_some());
        assert_eq!(got.unwrap().type_name, "Color");

        // test_material_ref_with_uniforms
        let mut uniforms = std::collections::BTreeMap::new();
        uniforms.insert(
            "color".to_string(),
            cesium_material::UniformValue::Vec4([1.0, 0.0, 0.0, 1.0]),
        );
        let entity2 = app
            .world_mut()
            .spawn(MaterialRef::with_uniforms("Checkerboard", uniforms))
            .id();
        let got2 = app.world().get::<MaterialRef>(entity2).unwrap();
        assert_eq!(got2.type_name, "Checkerboard");
        assert!(got2.uniforms.contains_key("color"));
    }
}
