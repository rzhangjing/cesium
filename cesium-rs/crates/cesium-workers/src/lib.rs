//! One-to-one port of `packages/engine/Source/Workers`.

#![forbid(unsafe_code)]
#![allow(dead_code)]

pub mod combine_geometry;
pub mod create_box_geometry;
pub mod create_box_outline_geometry;
pub mod create_circle_geometry;
pub mod create_circle_outline_geometry;
pub mod create_coplanar_polygon_geometry;
pub mod create_coplanar_polygon_outline_geometry;
pub mod create_corridor_geometry;
pub mod create_corridor_outline_geometry;
pub mod create_cylinder_geometry;
pub mod create_cylinder_outline_geometry;
pub mod create_ellipse_geometry;
pub mod create_ellipse_outline_geometry;
pub mod create_ellipsoid_geometry;
pub mod create_ellipsoid_outline_geometry;
pub mod create_frustum_geometry;
pub mod create_frustum_outline_geometry;
pub mod create_geometry;
pub mod create_ground_polyline_geometry;
pub mod create_plane_geometry;
pub mod create_plane_outline_geometry;
pub mod create_polygon_geometry;
pub mod create_polygon_outline_geometry;
pub mod create_polyline_geometry;
pub mod create_polyline_volume_geometry;
pub mod create_polyline_volume_outline_geometry;
pub mod create_rectangle_geometry;
pub mod create_rectangle_outline_geometry;
pub mod create_simple_polyline_geometry;
pub mod create_sphere_geometry;
pub mod create_sphere_outline_geometry;
pub mod create_task_processor_worker;
pub mod create_vector_tile_clamped_polylines;
pub mod create_vector_tile_geometries;
pub mod create_vector_tile_points;
pub mod create_vector_tile_polygons;
pub mod create_vector_tile_polylines;
pub mod create_vertices_from_cesium3_d_tiles_terrain;
pub mod create_vertices_from_google_earth_enterprise_buffer;
pub mod create_vertices_from_heightmap;
pub mod create_vertices_from_quantized_terrain_mesh;
pub mod create_wall_geometry;
pub mod create_wall_outline_geometry;
pub mod decode_draco;
pub mod decode_google_earth_enterprise_packet;
pub mod decode_i3_s;
pub mod gaussian_splat_sorter;
pub mod gaussian_splat_texture_generator;
pub mod incrementally_build_terrain_picker;
pub mod task_processor;
pub mod transcode_ktx2;
pub mod transfer_typed_array_test;
pub mod transferable_objects;
pub mod upsample_quantized_terrain_mesh;
pub mod upsample_vertices_from_cesium3_d_tiles_terrain;
pub mod wasm_worker;

// Re-export the WorkerBackend trait for cross-backend usage.
pub use wasm_worker::WorkerBackend;

/// Builds the standard error returned by worker byte entries whose
/// computation has not been ported yet.
///
/// CesiumJS worker modules always produce real data (geometry / terrain
/// vertices / decoded buffers) or throw; the Rust byte entries must
/// therefore surface an explicit failure instead of a silent empty
/// result, so [`task_processor::process_worker_task`] callers get a
/// proper error signal.
pub fn not_yet_ported_error(worker_name: &str) -> String {
    format!(
        "{worker_name} worker is not yet ported: the packed byte entry has no Rust implementation"
    )
}

