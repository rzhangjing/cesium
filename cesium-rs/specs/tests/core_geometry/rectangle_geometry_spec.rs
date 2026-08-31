//! Mirror of `packages/engine/Specs/Core/RectangleGeometrySpec.js`.
//!
//! DEVIATION: JS uses an options object and `createPackableSpecs`; the Rust
//! port uses `RectangleGeometry::from_options` / `new` and inlined
//! pack/unpack round-trip assertions.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::geometry_offset_attribute::GeometryOffsetAttribute;
use cesium_core::math::CesiumMath;
use cesium_core::matrix2::Matrix2;
use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_geometry::{RectangleGeometry, RectangleGeometryOptions};
use cesium_core::vertex_format::VertexFormat;

#[test]
fn computes_positions() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle.clone()),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let length = positions.len();

    assert_eq!(positions.len(), 9 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 3);

    let mut expected_nw_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::northwest(&rectangle),
        &mut expected_nw_corner,
    );
    let mut expected_se_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::southeast(&rectangle),
        &mut expected_se_corner,
    );
    let actual_nw = Cartesian3::new(positions[0], positions[1], positions[2]);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual_nw),
        Some(&expected_nw_corner),
        Some(CesiumMath::EPSILON9),
        None,
    ));
    let actual_se = Cartesian3::new(
        positions[length - 3],
        positions[length - 2],
        positions[length - 1],
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&actual_se),
        Some(&expected_se_corner),
        Some(CesiumMath::EPSILON9),
        None,
    ));
}

#[test]
fn computes_positions_across_idl() {
    let rectangle = Rectangle::from_degrees(179.0, -1.0, -179.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle.clone()),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let length = positions.len();

    assert_eq!(positions.len(), 9 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 3);

    let mut expected_nw_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::northwest(&rectangle),
        &mut expected_nw_corner,
    );
    let mut expected_se_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::southeast(&rectangle),
        &mut expected_se_corner,
    );
    let actual_nw = Cartesian3::new(positions[0], positions[1], positions[2]);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual_nw),
        Some(&expected_nw_corner),
        Some(CesiumMath::EPSILON8),
        None,
    ));
    let actual_se = Cartesian3::new(
        positions[length - 3],
        positions[length - 2],
        positions[length - 1],
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&actual_se),
        Some(&expected_se_corner),
        Some(CesiumMath::EPSILON8),
        None,
    ));
}

#[test]
fn computes_positions_at_north_pole() {
    let rectangle = Rectangle::from_degrees(-180.0, 89.0, -179.0, 90.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    assert_eq!(positions.len(), 5 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 3 * 3);
}

#[test]
fn computes_positions_at_south_pole() {
    let rectangle = Rectangle::from_degrees(-180.0, -90.0, -179.0, -89.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    assert_eq!(positions.len(), 5 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 3 * 3);
}

#[test]
fn computes_all_attributes() {
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::all()),
            rectangle: Some(Rectangle::new(-2.0, -1.0, 0.0, 1.0)),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let num_vertices = 9; // 8 around edge + 1 in middle
    let num_triangles = 8; // 4 squares * 2 triangles per square
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("st").unwrap().values.len(),
        num_vertices * 2
    );
    assert_eq!(
        m.attributes.get("normal").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("tangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("bitangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn compute_positions_with_rotation() {
    let rectangle = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let angle = CesiumMath::PI_OVER_TWO;
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle.clone()),
            rotation: Some(angle),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let length = positions.len();

    assert_eq!(length, 9 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 3);

    let unrotated_se_corner = Rectangle::southeast(&rectangle);
    let projection = GeographicProjection::new(None);
    let projected_se_corner = projection.project(&unrotated_se_corner);
    let rotation = Matrix2::from_rotation_new(angle);
    let rotated_2d =
        Matrix2::multiply_by_vector_new(&rotation, &Cartesian2::new(projected_se_corner.x, projected_se_corner.y));
    let rotated_se_corner_cartographic = projection.unproject(&Cartesian3::new(
        rotated_2d.x,
        rotated_2d.y,
        0.0,
    ));
    let mut rotated_se_corner = Cartesian3::default();
    Ellipsoid::WGS84
        .cartographic_to_cartesian(&rotated_se_corner_cartographic, &mut rotated_se_corner);
    let actual = Cartesian3::new(
        positions[length - 3],
        positions[length - 2],
        positions[length - 1],
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&rotated_se_corner),
        Some(CesiumMath::EPSILON6),
        None,
    ));
}

#[test]
fn compute_vertices_with_pi_rotation() {
    let rectangle = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            rectangle: Some(rectangle.clone()),
            rotation: Some(CesiumMath::PI),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let length = positions.len();

    assert_eq!(length, 9 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 3);

    let mut unrotated_nw_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::northwest(&rectangle),
        &mut unrotated_nw_corner,
    );
    let mut unrotated_se_corner = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Rectangle::southeast(&rectangle),
        &mut unrotated_se_corner,
    );

    let actual = Cartesian3::new(positions[0], positions[1], positions[2]);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&unrotated_se_corner),
        Some(CesiumMath::EPSILON8),
        None,
    ));

    let actual = Cartesian3::new(
        positions[length - 3],
        positions[length - 2],
        positions[length - 1],
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&unrotated_nw_corner),
        Some(CesiumMath::EPSILON8),
        None,
    ));
}

#[test]
fn compute_texture_coordinates_with_rotation() {
    let rectangle = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let angle = CesiumMath::PI_OVER_TWO;
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_and_st()),
            rectangle: Some(rectangle),
            st_rotation: Some(angle),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let st = &m.attributes.get("st").unwrap().values;
    let length = st.len();

    assert_eq!(positions.len(), 9 * 3);
    assert_eq!(length, 9 * 2);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 3);

    assert!(st[length - 2].abs() <= CesiumMath::EPSILON14);
    assert!(st[length - 1].abs() <= CesiumMath::EPSILON14);
}

#[test]
fn compute_texture_coordinate_rotation_with_rectangle_rotation() {
    let rectangle = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let angle = CesiumMath::to_radians(30.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_and_st()),
            rectangle: Some(rectangle),
            rotation: Some(angle),
            st_rotation: Some(angle),
            granularity: Some(1.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let st = &m.attributes.get("st").unwrap().values;

    assert_eq!(st[0], 0.0); // top left corner
    assert_eq!(st[1], 1.0);
    assert_eq!(st[4], 1.0); // top right corner
    assert_eq!(st[5], 1.0);
    assert_eq!(st[12], 0.0); // bottom left corner
    assert_eq!(st[13], 0.0);
    assert_eq!(st[16], 1.0); // bottom right corner
    assert_eq!(st[17], 0.0);
}

#[test]
#[should_panic]
fn throws_without_rectangle() {
    let _ = RectangleGeometry::from_options(RectangleGeometryOptions::default());
}

#[test]
#[cfg(debug_assertions)]
#[should_panic]
fn throws_if_rotated_rectangle_is_invalid() {
    let _ = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            rectangle: Some(Rectangle::new(
                -CesiumMath::PI_OVER_TWO,
                1.0,
                CesiumMath::PI_OVER_TWO,
                CesiumMath::PI_OVER_TWO,
            )),
            rotation: Some(CesiumMath::PI_OVER_TWO),
            ..Default::default()
        },
    ));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "north must be greater than or equal to")]
fn throws_if_north_is_less_than_south() {
    let _ = RectangleGeometry::from_options(RectangleGeometryOptions {
        rectangle: Some(Rectangle::new(
            -CesiumMath::PI_OVER_TWO,
            CesiumMath::PI_OVER_TWO,
            CesiumMath::PI_OVER_TWO,
            -CesiumMath::PI_OVER_TWO,
        )),
        ..Default::default()
    });
}

#[test]
fn computes_positions_extruded() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            granularity: Some(1.0),
            extruded_height: Some(2.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    // (9 fill + 8 edge + 4 corners) * 2 to duplicate for bottom
    assert_eq!(positions.len(), 42 * 3);
    // 8 * 2 for fill top and bottom + 4 triangles * 4 walls
    assert_eq!(m.indices.as_ref().unwrap().len(), 32 * 3);
}

#[test]
fn computes_positions_extruded_at_the_north_pole() {
    let rectangle = Rectangle::from_degrees(-180.0, 89.0, -179.0, 90.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            extruded_height: Some(2.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    // (5 fill + 5 edge + 3 corners) * 2 to duplicate for bottom
    assert_eq!(positions.len(), 26 * 3);
    // 3 * 2 for fill top and bottom + 2 triangles * 5 walls
    assert_eq!(m.indices.as_ref().unwrap().len(), 16 * 3);
}

#[test]
fn computes_positions_extruded_at_the_south_pole() {
    let rectangle = Rectangle::from_degrees(-180.0, -90.0, -179.0, -89.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            extruded_height: Some(2.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    // (5 fill + 5 edge + 3 corners) * 2 to duplicate for bottom
    assert_eq!(positions.len(), 26 * 3);
    // 3 * 2 for fill top and bottom + 2 triangles * 5 walls
    assert_eq!(m.indices.as_ref().unwrap().len(), 16 * 3);
}

#[test]
fn computes_all_attributes_extruded() {
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::all()),
            rectangle: Some(Rectangle::new(-2.0, -1.0, 0.0, 1.0)),
            granularity: Some(1.0),
            extruded_height: Some(2.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let num_vertices = 42;
    let num_triangles = 32;
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("st").unwrap().values.len(),
        num_vertices * 2
    );
    assert_eq!(
        m.attributes.get("normal").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("tangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("bitangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn compute_positions_with_rotation_extruded() {
    let rectangle = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let angle = CesiumMath::PI_OVER_TWO;
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle.clone()),
            rotation: Some(angle),
            granularity: Some(1.0),
            extruded_height: Some(2.0),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;
    let length = positions.len();

    assert_eq!(length, 42 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 32 * 3);

    let unrotated_se_corner = Rectangle::southeast(&rectangle);
    let projection = GeographicProjection::new(None);
    let projected_se_corner = projection.project(&unrotated_se_corner);
    let rotation = Matrix2::from_rotation_new(angle);
    let rotated_2d =
        Matrix2::multiply_by_vector_new(&rotation, &Cartesian2::new(projected_se_corner.x, projected_se_corner.y));
    let rotated_se_corner_cartographic = projection.unproject(&Cartesian3::new(
        rotated_2d.x,
        rotated_2d.y,
        0.0,
    ));
    let mut rotated_se_corner = Cartesian3::default();
    Ellipsoid::WGS84
        .cartographic_to_cartesian(&rotated_se_corner_cartographic, &mut rotated_se_corner);
    let actual = Cartesian3::new(positions[51], positions[52], positions[53]);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&rotated_se_corner),
        Some(CesiumMath::EPSILON6),
        None,
    ));
}

#[test]
fn computes_non_extruded_rectangle_if_height_is_small() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            granularity: Some(1.0),
            extruded_height: Some(CesiumMath::EPSILON14),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    let num_vertices = 9;
    let num_triangles = 8;
    assert_eq!(positions.len(), num_vertices * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn computes_offset_attribute() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            granularity: Some(1.0),
            offset_attribute: Some(GeometryOffsetAttribute::Top),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    let num_vertices = 9;
    assert_eq!(positions.len(), num_vertices * 3);

    let offset = &m.attributes.get("applyOffset").unwrap().values;
    assert_eq!(offset.len(), num_vertices);
    let expected: Vec<f64> = vec![1.0; offset.len()];
    assert_eq!(offset, &expected);
}

#[test]
fn computes_offset_attribute_extruded_for_top_vertices() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            granularity: Some(1.0),
            extruded_height: Some(2.0),
            offset_attribute: Some(GeometryOffsetAttribute::Top),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    let num_vertices = 42; // (9 fill + 8 edge + 4 corners) * 2 for bottom
    assert_eq!(positions.len(), num_vertices * 3);

    let offset = &m.attributes.get("applyOffset").unwrap().values;
    assert_eq!(offset.len(), num_vertices);
    let mut expected = vec![0.0f64; offset.len()];
    expected[..9].fill(1.0);
    for i in (18..offset.len()).step_by(2) {
        expected[i] = 1.0;
    }
    assert_eq!(offset, &expected);
}

#[test]
fn computes_offset_attribute_extruded_for_all_vertices() {
    let rectangle = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(rectangle),
            granularity: Some(1.0),
            extruded_height: Some(2.0),
            offset_attribute: Some(GeometryOffsetAttribute::All),
            ..Default::default()
        },
    ))
    .expect("geometry should not be None");
    let positions = &m.attributes.get("position").unwrap().values;

    let num_vertices = 42; // (9 fill + 8 edge + 4 corners) * 2 for bottom
    assert_eq!(positions.len(), num_vertices * 3);

    let offset = &m.attributes.get("applyOffset").unwrap().values;
    assert_eq!(offset.len(), num_vertices);
    let expected = vec![1.0; offset.len()];
    assert_eq!(offset, &expected);
}

#[test]
fn none_is_returned_if_any_side_is_of_length_zero() {
    let rectangle0 = RectangleGeometry::from_options(RectangleGeometryOptions {
        rectangle: Some(Rectangle::from_degrees(-80.0, 39.0, -80.0, 42.0)),
        ..Default::default()
    });
    let rectangle1 = RectangleGeometry::from_options(RectangleGeometryOptions {
        rectangle: Some(Rectangle::from_degrees(-81.0, 42.0, -80.0, 42.0)),
        ..Default::default()
    });
    let rectangle2 = RectangleGeometry::from_options(RectangleGeometryOptions {
        rectangle: Some(Rectangle::from_degrees(-80.0, 39.0, -80.0, 39.0)),
        ..Default::default()
    });

    let geometry0 = RectangleGeometry::create_geometry(&rectangle0);
    let geometry1 = RectangleGeometry::create_geometry(&rectangle1);
    let geometry2 = RectangleGeometry::create_geometry(&rectangle2);

    assert!(geometry0.is_none());
    assert!(geometry1.is_none());
    assert!(geometry2.is_none());
}

#[test]
fn computing_rectangle_property() {
    let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    let mut geometry = RectangleGeometry::from_options(RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(rectangle),
        granularity: Some(1.0),
        ..Default::default()
    });

    let r = geometry.rectangle();
    assert_eq!(CesiumMath::to_degrees(r.north), 1.0);
    assert_eq!(CesiumMath::to_degrees(r.south), -1.0);
    assert_eq!(CesiumMath::to_degrees(r.east), 1.0);
    assert_eq!(CesiumMath::to_degrees(r.west), -1.0);
}

#[test]
fn computing_rectangle_property_with_rotation() {
    let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    let mut geometry = RectangleGeometry::from_options(RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(rectangle),
        granularity: Some(1.0),
        rotation: Some(CesiumMath::to_radians(45.0)),
        ..Default::default()
    });

    let r = geometry.rectangle();
    assert!(
        (CesiumMath::to_degrees(r.north) - 1.414213562373095).abs() <= CesiumMath::EPSILON15
    );
    assert!(
        (CesiumMath::to_degrees(r.south) - (-1.414213562373095)).abs() <= CesiumMath::EPSILON15
    );
    assert!(
        (CesiumMath::to_degrees(r.east) - 1.414213562373095).abs() <= CesiumMath::EPSILON15
    );
    assert!(
        (CesiumMath::to_degrees(r.west) - (-1.4142135623730951)).abs()
            <= CesiumMath::EPSILON15
    );
}

#[test]
fn computing_texture_coordinate_rotation_points_property() {
    let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    let mut geometry = RectangleGeometry::from_options(RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(rectangle),
        granularity: Some(1.0),
        rotation: Some(CesiumMath::to_radians(90.0)),
        ..Default::default()
    });

    // 90 degree rotation means (0, 1) should be the new min and
    // (1, 1) (0, 0) are extents
    let points = geometry.texture_coordinate_rotation_points();
    assert_eq!(points.len(), 6);
    let expected = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    for i in 0..6 {
        assert!(
            (points[i] - expected[i]).abs() <= CesiumMath::EPSILON7,
            "points[{i}] = {} != {}",
            points[i],
            expected[i]
        );
    }

    // Second call exercises the cache path.
    let points = geometry.texture_coordinate_rotation_points();
    assert_eq!(points.len(), 6);
    for i in 0..6 {
        assert!((points[i] - expected[i]).abs() <= CesiumMath::EPSILON7);
    }
}

#[test]
fn compute_rectangle_matches_instance_rectangle() {
    let options = RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0)),
        granularity: Some(1.0),
        ellipsoid: Some(Ellipsoid::UNIT_SPHERE),
        rotation: Some(CesiumMath::PI),
        ..Default::default()
    };
    let mut geometry = RectangleGeometry::from_options(options.clone());

    let expected = geometry.rectangle();
    let result = RectangleGeometry::compute_rectangle(&options);

    assert_eq!(result, expected);
}

#[test]
fn compute_rectangle_without_rotation() {
    let options = RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0)),
        ..Default::default()
    };
    let mut geometry = RectangleGeometry::from_options(options.clone());

    let expected = geometry.rectangle();
    let result = RectangleGeometry::compute_rectangle(&options);

    assert_eq!(result, expected);
}

#[test]
fn computing_rectangle_property_with_zero_rotation_does_not_panic() {
    let m = RectangleGeometry::create_geometry(&RectangleGeometry::from_options(
        RectangleGeometryOptions {
            vertex_format: Some(VertexFormat::position_only()),
            rectangle: Some(Rectangle::MAX_VALUE),
            granularity: Some(1.0),
            rotation: Some(0.0),
            ..Default::default()
        },
    ));
    // MAX_VALUE with granularity 1.0 generates geometry (or None), but must
    // not panic (JS `not.toThrowDeveloperError()`).
    let _ = m;
}

#[test]
fn can_create_rectangle_geometry_where_nw_corner_and_center_cross_the_idl() {
    let rectangle = Rectangle::new(
        std::f64::consts::PI - 0.005,
        CesiumMath::PI_OVER_SIX + 0.02,
        0.01 - std::f64::consts::PI,
        CesiumMath::PI_OVER_SIX + 0.04,
    );

    let geometry = RectangleGeometry::from_options(RectangleGeometryOptions {
        rectangle: Some(rectangle),
        rotation: Some(0.5),
        ..Default::default()
    });

    let _ = RectangleGeometry::create_geometry(&geometry);
}

#[test]
fn packs_the_object_into_an_array() {
    // Mirrors `createPackableSpecs` with the JS packedInstance:
    // [-2, -1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, -1]
    let rectangle = RectangleGeometry::from_options(RectangleGeometryOptions {
        vertex_format: Some(VertexFormat::position_only()),
        rectangle: Some(Rectangle::new(-2.0, -1.0, 0.0, 1.0)),
        granularity: Some(1.0),
        ellipsoid: Some(Ellipsoid::UNIT_SPHERE),
        ..Default::default()
    });
    let packed_instance: Vec<f64> = vec![
        -2.0, -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, -1.0,
    ];

    assert_eq!(
        RectangleGeometry::PACKED_LENGTH,
        packed_instance.len()
    );
    let mut packed = vec![0.0f64; RectangleGeometry::PACKED_LENGTH];
    rectangle.pack(&mut packed, None);
    assert_eq!(packed, packed_instance);

    // Round trip through unpack.
    let unpacked = RectangleGeometry::unpack(&packed, None, None);
    let mut unpacked = unpacked;
    assert_eq!(unpacked.rectangle(), rectangle.clone().rectangle());
    assert_eq!(unpacked.granularity(), 1.0);
}
