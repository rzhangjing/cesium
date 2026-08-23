//! Port of `Core/PlaneGeometrySpec.js`.
use cesium_core::cartesian3::Cartesian3;
use cesium_core::plane_geometry::PlaneGeometry;
use cesium_core::vertex_format::VertexFormat;

#[test]
fn create_geometry_position_only() {
    let pg = PlaneGeometry::new(Some(VertexFormat::position_only()));
    let g = pg.create_geometry();

    let position = g.attributes.get("position").unwrap();
    assert_eq!(position.values.len(), 4 * 3); // 4 corners × xyz
    let indices = g.indices.as_ref().unwrap();
    assert_eq!(indices.len(), 2 * 3); // 2 triangles
}

#[test]
fn create_geometry_all_vertex_formats() {
    let pg = PlaneGeometry::new(Some(VertexFormat::all()));
    let g = pg.create_geometry();

    let num_vertices = 4;
    let num_triangles = 2;

    assert_eq!(g.attributes.get("position").unwrap().values.len(), num_vertices * 3);
    assert_eq!(g.attributes.get("normal").unwrap().values.len(), num_vertices * 3);
    assert_eq!(g.attributes.get("tangent").unwrap().values.len(), num_vertices * 3);
    assert_eq!(g.attributes.get("bitangent").unwrap().values.len(), num_vertices * 3);
    assert_eq!(g.attributes.get("st").unwrap().values.len(), num_vertices * 2);
    assert_eq!(g.indices.as_ref().unwrap().len(), num_triangles * 3);

    let bs = g.bounding_sphere.as_ref().unwrap();
    assert_eq!(bs.center, Cartesian3::ZERO);
    assert!((bs.radius - 2.0f64.sqrt()).abs() < 1e-15);
}

#[test]
fn pack_and_unpack() {
    let pg = PlaneGeometry::new(Some(VertexFormat::position_and_normal()));
    let mut array = vec![0.0; VertexFormat::PACKED_LENGTH];
    pg.pack(&mut array, None);

    let unpacked = PlaneGeometry::unpack(&array, None);
    let mut array2 = vec![0.0; VertexFormat::PACKED_LENGTH];
    unpacked.pack(&mut array2, None);
    assert_eq!(array, array2);
}
