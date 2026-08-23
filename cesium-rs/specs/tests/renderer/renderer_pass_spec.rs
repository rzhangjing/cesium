//! Port of `Renderer/PassStateSpec.js` (Pass enum portion) + ordering tests.

use cesium_renderer::pass::Pass;

#[test]
fn pass_enum_values_match_cesiumjs() {
    // CesiumJS Pass enum ordering
    assert_eq!(Pass::Environment as u8, 0);
    assert_eq!(Pass::Compute as u8, 1);
    assert_eq!(Pass::Globe as u8, 2);
    assert_eq!(Pass::TerrainClassification as u8, 3);
    assert_eq!(Pass::Cesium3dTileEdges as u8, 4);
    assert_eq!(Pass::Cesium3dTile as u8, 5);
    assert_eq!(Pass::Cesium3dTileClassification as u8, 6);
    assert_eq!(Pass::Cesium3dTileClassificationIgnoreShow as u8, 7);
    assert_eq!(Pass::Opaque as u8, 8);
    assert_eq!(Pass::Translucent as u8, 9);
    assert_eq!(Pass::Voxels as u8, 10);
    assert_eq!(Pass::GaussianSplats as u8, 11);
    assert_eq!(Pass::Cesium3dTileEdgesDirect as u8, 12);
    assert_eq!(Pass::Overlay as u8, 13);
    assert_eq!(Pass::NumberOfPasses as u8, 14);
}

#[test]
fn pass_ordering_is_correct() {
    // Passes should be ordered: Environment < Globe < Opaque < Translucent < Overlay
    assert!(Pass::Environment < Pass::Globe);
    assert!(Pass::Globe < Pass::Opaque);
    assert!(Pass::Opaque < Pass::Translucent);
    assert!(Pass::Translucent < Pass::Overlay);
}
