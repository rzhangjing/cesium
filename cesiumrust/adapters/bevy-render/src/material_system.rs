//! Bridge domain Material → Bevy FabricMaterial.
//!
//! Provides the [`CesiumMaterialPlugin`] which reads entity components and
//! applies Fabric procedural materials to Bevy meshes, plus per-frame uniform
//! updates (e.g. water animation time).

use bevy::prelude::*;
use cesium_material::{MaterialSystem, UniformValue};

use crate::fabric_material::{fabric_material_from_domain, FabricKind, FabricMaterial, FabricMaterialPlugin};

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
    pub time: f32,
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

    // Create a 1x1 white fallback image for materials that need a texture.
    // Materials that don't use textures (Color, etc.) ignore this binding.
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

        let fabric_material = fabric_material_from_domain(&domain_material, fallback_handle.clone());
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

    // Update Water material time uniforms
    for (_, material) in materials.iter_mut() {
        if material.params.kind == FabricKind::Water as u32 {
            material.params.extra_c.z = animation_time.time;
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
