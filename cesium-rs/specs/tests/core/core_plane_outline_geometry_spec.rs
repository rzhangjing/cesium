//! Port of `Core/PlaneOutlineGeometrySpec.js`.

use cesium_core::plane_outline_geometry::PlaneOutlineGeometry;

#[test]
fn create_geometry_produces_positions_and_indices() {
    let geometry = PlaneOutlineGeometry::create_geometry();
    // 4 vertices * 3 components (x, y, z)
    let position = geometry.attributes.get("position").unwrap();
    assert_eq!(position.values.len(), 4 * 3);
    // 4 line segments * 2 indices each
    let indices = geometry.indices.as_ref().unwrap();
    assert_eq!(indices.len(), 4 * 2);
}
