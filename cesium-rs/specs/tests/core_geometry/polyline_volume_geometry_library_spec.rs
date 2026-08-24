//! Mirror of `packages/engine/Specs/Core/PolylineVolumeGeometryLibrarySpec.js`.
//!
//! DEVIATION: the JS spec stubs `ellipsoid.scaleToGeodeticSurface` with the
//! identity function so that the near-center positions `(1,1,1)..(4,4,4)`
//! stay collinear after scaling. The Rust `Ellipsoid` has no virtual methods
//! to stub, so the positions are placed along the same radial direction at
//! ellipsoid-scale radii instead: `scale_to_geodetic_surface` then maps them
//! all onto the same surface point, which is equally collinear and exercises
//! the same code path (fix #12255).

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::polyline_volume_geometry_library::{
    ComputePositionsGeometry, PolylineVolumeGeometryLibrary,
};

// Tests the fix #12255
#[test]
fn compute_positions_should_not_throw_error_if_positions_are_collinear_after_scaling_to_geodetic_surface(
) {
    let direction = Cartesian3::new(1.0, 1.0, 1.0);
    let mut positions = vec![
        Cartesian3::multiply_by_scalar_new(&direction, 6378137.0),
        Cartesian3::multiply_by_scalar_new(&direction, 6378237.0),
        Cartesian3::multiply_by_scalar_new(&direction, 6378337.0),
        Cartesian3::multiply_by_scalar_new(&direction, 6378437.0),
    ];

    let shape = [
        Cartesian2::new(-0.15, -0.15),
        Cartesian2::new(0.15, -0.15),
        Cartesian2::new(0.15, 0.15),
        Cartesian2::new(-0.15, 0.15),
    ];

    let ellipsoid = Ellipsoid::new(6378137.0, 6378137.0, 6356752.3142451793);

    let bounding_rectangle = BoundingRectangle {
        x: -0.15,
        y: -0.15,
        width: 0.3,
        height: 0.3,
    };
    let geometry = ComputePositionsGeometry {
        ellipsoid,
        granularity: CesiumMath::RADIANS_PER_DEGREE,
        corner_type: CornerType::Rounded,
    };

    // Expect no developer error (no panic in the Rust port).
    let _ = PolylineVolumeGeometryLibrary::compute_positions(
        &mut positions,
        &shape,
        &bounding_rectangle,
        &geometry,
        true,
    );
}
