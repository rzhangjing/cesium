//! Scene specs - ported from packages/engine/Specs/Scene/
//! Covers: Tileset, Imagery, Material, Particles, Camera, Primitives, etc.

mod scene {
    pub mod tileset_spec;
    pub mod imagery_spec;
    pub mod material_spec;
    pub mod particle_spec;
    pub mod camera_spec;
    pub mod primitive_spec;
    pub mod voxel_spec;
    pub mod atmosphere_spec;
    pub mod gltf_spec;
    pub mod vector_spec;
    pub mod implicit_tiling_spec;
    pub mod quadtree_spec;
    pub mod scene_graph_spec;
    pub mod structural_metadata_spec;
    pub mod tile_content_spec;
    pub mod clipping_cloud_spec;
    pub mod post_process_spec;
    pub mod shadow_spec;
    pub mod gltf_animation_spec;
    pub mod imagery_provider_spec;
}
