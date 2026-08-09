pub mod blend_system;
pub mod layer_manager;
pub mod tile_loader;

use bevy::prelude::*;

pub use blend_system::{
    imagery_apply_system, imagery_blend_compute_system, ImageryBlendCache,
};
pub use layer_manager::{ImageryLayerManager, ImageryLayerDescriptor};
pub use tile_loader::{
    imagery_tile_load_system, imagery_tile_request_system, ImageryCache, ImageryPendingLoads,
};

pub struct CesiumImageryPlugin;

impl Plugin for CesiumImageryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ImageryLayerManager>()
            .init_resource::<ImageryCache>()
            .init_resource::<ImageryPendingLoads>()
            .init_resource::<ImageryBlendCache>()
            .add_systems(PreUpdate, imagery_tile_request_system)
            .add_systems(
                Update,
                (imagery_tile_load_system, imagery_apply_system, imagery_blend_compute_system),
            );
    }
}
