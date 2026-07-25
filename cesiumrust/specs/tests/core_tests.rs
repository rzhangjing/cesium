//! Core specs - ported from packages/engine/Specs/Core/
//! Covers: Math, Cartesian, Ellipsoid, Time, Spline, Geometry, Bounding, etc.

mod core {
    pub mod math_spec;
    pub mod cartesian_spec;
    pub mod ellipsoid_spec;
    pub mod time_spec;
    pub mod spline_spec;
    pub mod bounding_spec;
    pub mod intersection_spec;
    pub mod transform_spec;
    pub mod frustum_spec;
    pub mod tiling_spec;
    pub mod terrain_provider_spec;
    pub mod pipeline_spec;
    pub mod misc_spec;
    pub mod geometry_spec;
    pub mod matrix_quaternion_spec;
}
