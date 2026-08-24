//! Mirror of `packages/engine/Specs/Core/CorridorGeometryLibrarySpec.js`.
//!
//! DEVIATION: the JS `options` object omits `granularity`, `cornerType` and
//! `saveAttributes`; the Rust params struct is exhaustive, so the JS
//! defaults used by `CorridorGeometry` (`CesiumMath.RADIANS_PER_DEGREE`,
//! `CornerType.ROUNDED`, `false`) are supplied explicitly.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::corridor_geometry_library::{
    CorridorComputePositionsParams, CorridorGeometryLibrary,
};
use cesium_core::corner_type::CornerType;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;

#[test]
fn compute_positions_should_not_compute_corners_for_collinear_points() {
    let params = CorridorComputePositionsParams {
        granularity: CesiumMath::RADIANS_PER_DEGREE,
        positions: vec![
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(2.0, 2.0, 2.0),
            Cartesian3::new(3.0, 3.0, 3.0),
            Cartesian3::new(4.0, 4.0, 4.0),
        ],
        ellipsoid: Ellipsoid::WGS84,
        width: 0.15,
        corner_type: CornerType::Rounded,
        save_attributes: false,
    };

    // The fact it doesn't panic also verifies the fix #12255
    let result = CorridorGeometryLibrary::compute_positions(&params);
    assert_eq!(result.corners.len(), 0);
}

#[test]
fn compute_positions_should_compute_corners_for_non_collinear_points() {
    let params = CorridorComputePositionsParams {
        granularity: CesiumMath::RADIANS_PER_DEGREE,
        positions: vec![
            Cartesian3::new(0.0, 0.0, 1.0),
            Cartesian3::new(1.0, 0.0, 2.0),
            Cartesian3::new(1.0, 1.0, 3.0),
            Cartesian3::new(0.0, 1.0, 4.0),
        ],
        ellipsoid: Ellipsoid::WGS84,
        width: 0.15,
        corner_type: CornerType::Rounded,
        save_attributes: false,
    };

    let result = CorridorGeometryLibrary::compute_positions(&params);
    assert_eq!(result.corners.len(), 2);
}
