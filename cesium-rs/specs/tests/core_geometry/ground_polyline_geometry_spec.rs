//! Mirror of `packages/engine/Specs/Core/GroundPolylineGeometrySpec.js`.
//!
//! DEVIATION: JS sets the private `_scene3DOnly` field directly; the Rust
//! port exposes `set_scene3d_only` instead.

use cesium_core::approximate_terrain_heights;
use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::geometry_attribute::GeometryAttribute;
use cesium_core::ground_polyline_geometry::{
    project_normal, GroundPolylineGeometry, GroundPolylineProjection,
};
use cesium_core::math::CesiumMath;
use cesium_core::web_mercator_projection::WebMercatorProjection;

fn init_terrain_heights() {
    approximate_terrain_heights::initialize()
        .expect("ApproximateTerrainHeights.initialize should succeed");
}

fn verify_attribute_values_identical(attribute: &GeometryAttribute) {
    let values = &attribute.values;
    let components_per_attribute = attribute.components_per_attribute as usize;
    let vertex_count = values.len() / components_per_attribute;
    let first_vertex = &values[..components_per_attribute];
    let mut identical = true;
    for i in 1..vertex_count {
        let index = i * components_per_attribute;
        let vertex = &values[index..index + components_per_attribute];
        for j in 0..components_per_attribute {
            if vertex[j] != first_vertex[j] {
                identical = false;
                break;
            }
        }
    }
    assert!(identical, "attribute values should be identical across vertices");
}

#[test]
fn computes_positions_and_additional_attributes_for_polylines() {
    init_terrain_heights();
    let start_cartographic = Cartographic::from_degrees_new(0.01, 0.0, None);
    let end_cartographic = Cartographic::from_degrees_new(0.02, 0.0, None);
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_radians_array(&[
            start_cartographic.longitude,
            start_cartographic.latitude,
            end_cartographic.longitude,
            end_cartographic.latitude,
        ], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 36);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 24);

    let start_hi_and_forward_offset_x = geometry
        .attributes
        .get("startHiAndForwardOffsetX")
        .unwrap();
    let start_lo_and_forward_offset_y = geometry
        .attributes
        .get("startLoAndForwardOffsetY")
        .unwrap();
    let start_normal_and_forward_offset_z = geometry
        .attributes
        .get("startNormalAndForwardOffsetZ")
        .unwrap();
    let end_normal_and_texcoord_normalization_x = geometry
        .attributes
        .get("endNormalAndTextureCoordinateNormalizationX")
        .unwrap();
    let right_normal_and_texcoord_normalization_y = geometry
        .attributes
        .get("rightNormalAndTextureCoordinateNormalizationY")
        .unwrap();
    let start_hi_lo_2d = geometry.attributes.get("startHiLo2D").unwrap();
    let offset_and_right_2d = geometry.attributes.get("offsetAndRight2D").unwrap();
    let start_end_normals_2d = geometry.attributes.get("startEndNormals2D").unwrap();
    let texcoord_normalization_2d = geometry
        .attributes
        .get("texcoordNormalization2D")
        .unwrap();

    // Expect each entry in the additional attributes to be identical across
    // all vertices since this is a single segment, except
    // endNormalAndTextureCoordinateNormalizationX and texcoordNormalization2D,
    // which should be "sided".
    verify_attribute_values_identical(start_hi_and_forward_offset_x);
    verify_attribute_values_identical(start_lo_and_forward_offset_y);
    verify_attribute_values_identical(start_normal_and_forward_offset_z);
    verify_attribute_values_identical(start_hi_lo_2d);
    verify_attribute_values_identical(offset_and_right_2d);
    verify_attribute_values_identical(start_end_normals_2d);

    // endNormalAndTextureCoordinateNormalizationX and
    // texcoordNormalization2D.x encode the "side" of the geometry.
    let values = &end_normal_and_texcoord_normalization_x.values;
    for i in 0..4usize {
        let index = i * 4 + 3;
        assert_eq!(CesiumMath::sign(values[index]), 1.0);
    }
    for i in 4..8usize {
        let index = i * 4 + 3;
        assert_eq!(CesiumMath::sign(values[index]), -1.0);
    }

    let values = &texcoord_normalization_2d.values;
    for i in 0..4usize {
        let index = i * 2;
        assert_eq!(CesiumMath::sign(values[index]), 1.0);
    }
    for i in 4..8usize {
        let index = i * 2;
        assert_eq!(CesiumMath::sign(values[index]), -1.0);
    }

    // rightNormalAndTextureCoordinateNormalizationY and
    // texcoordNormalization2D.y encode if the vertex is on the bottom.
    let values = &right_normal_and_texcoord_normalization_y.values;
    assert!(values[3] > 1.0);
    assert!(values[1 * 4 + 3] > 1.0);
    assert!(values[4 * 4 + 3] > 1.0);
    assert!(values[5 * 4 + 3] > 1.0);

    let values = &texcoord_normalization_2d.values;
    assert!(values[1] > 1.0);
    assert!(values[1 * 2 + 1] > 1.0);
    assert!(values[4 * 2 + 1] > 1.0);
    assert!(values[5 * 2 + 1] > 1.0);

    // Line segment geometry is encoded as:
    // - start position
    // - offset to the end position
    // - normal for a mitered plane at each end
    // - a right-facing normal
    // - parameters for localizing the position along the line to texture
    //   coordinates
    let start_position_3d = Cartesian3::new(
        start_hi_and_forward_offset_x.values[0] + start_lo_and_forward_offset_y.values[0],
        start_hi_and_forward_offset_x.values[1] + start_lo_and_forward_offset_y.values[1],
        start_hi_and_forward_offset_x.values[2] + start_lo_and_forward_offset_y.values[2],
    );
    let mut reconstructed_carto =
        Cartographic::from_cartesian_new(&start_position_3d, None).unwrap();
    reconstructed_carto.height = 0.0;
    assert!(Cartographic::equals_epsilon(
        Some(&reconstructed_carto),
        Some(&start_cartographic),
        Some(CesiumMath::EPSILON7),
    ));

    let end_position_3d = Cartesian3::new(
        start_position_3d.x + start_hi_and_forward_offset_x.values[3],
        start_position_3d.y + start_lo_and_forward_offset_y.values[3],
        start_position_3d.z + start_normal_and_forward_offset_z.values[3],
    );
    reconstructed_carto = Cartographic::from_cartesian_new(&end_position_3d, None).unwrap();
    reconstructed_carto.height = 0.0;
    assert!(Cartographic::equals_epsilon(
        Some(&reconstructed_carto),
        Some(&end_cartographic),
        Some(CesiumMath::EPSILON7),
    ));

    let start_normal_3d =
        Cartesian3::unpack_new(&start_normal_and_forward_offset_z.values, None);
    assert!(Cartesian3::equals_epsilon(
        Some(&start_normal_3d),
        Some(&Cartesian3::new(0.0, 1.0, 0.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let end_normal_3d =
        Cartesian3::unpack_new(&end_normal_and_texcoord_normalization_x.values, None);
    assert!(Cartesian3::equals_epsilon(
        Some(&end_normal_3d),
        Some(&Cartesian3::new(0.0, -1.0, 0.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let right_normal_3d =
        Cartesian3::unpack_new(&right_normal_and_texcoord_normalization_y.values, None);
    assert!(Cartesian3::equals_epsilon(
        Some(&right_normal_3d),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let texcoord_normalization_x = end_normal_and_texcoord_normalization_x.values[3];
    assert!((texcoord_normalization_x - 1.0).abs() <= CesiumMath::EPSILON3);

    // 2D
    let projection = GeographicProjection::new(None);

    let start_position_2d = Cartesian3::new(
        start_hi_lo_2d.values[0] + start_hi_lo_2d.values[2],
        start_hi_lo_2d.values[1] + start_hi_lo_2d.values[3],
        0.0,
    );
    reconstructed_carto = projection.unproject(&start_position_2d);
    reconstructed_carto.height = 0.0;
    assert!(Cartographic::equals_epsilon(
        Some(&reconstructed_carto),
        Some(&start_cartographic),
        Some(CesiumMath::EPSILON7),
    ));

    let end_position_2d = Cartesian3::new(
        start_position_2d.x + offset_and_right_2d.values[0],
        start_position_2d.y + offset_and_right_2d.values[1],
        0.0,
    );
    reconstructed_carto = projection.unproject(&end_position_2d);
    reconstructed_carto.height = 0.0;
    assert!(Cartographic::equals_epsilon(
        Some(&reconstructed_carto),
        Some(&end_cartographic),
        Some(CesiumMath::EPSILON7),
    ));

    let start_normal_2d = Cartesian3::new(
        start_end_normals_2d.values[0],
        start_end_normals_2d.values[1],
        0.0,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&start_normal_2d),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let end_normal_2d = Cartesian3::new(
        start_end_normals_2d.values[2],
        start_end_normals_2d.values[3],
        0.0,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&end_normal_2d),
        Some(&Cartesian3::new(-1.0, 0.0, 0.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let right_normal_2d = Cartesian3::new(
        offset_and_right_2d.values[2],
        offset_and_right_2d.values[3],
        0.0,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&right_normal_2d),
        Some(&Cartesian3::new(0.0, -1.0, 0.0)),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    let texcoord_normalization_x = texcoord_normalization_2d.values[0];
    assert!((texcoord_normalization_x - 1.0).abs() <= CesiumMath::EPSILON3);
}

#[test]
fn does_not_generate_2d_attributes_when_scene3d_only_is_true() {
    init_terrain_heights();
    let start_cartographic = Cartographic::from_degrees_new(0.01, 0.0, None);
    let end_cartographic = Cartographic::from_degrees_new(0.02, 0.0, None);
    let mut ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_radians_array(&[
            start_cartographic.longitude,
            start_cartographic.latitude,
            end_cartographic.longitude,
            end_cartographic.latitude,
        ], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    ground_polyline_geometry.set_scene3d_only(true);

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert!(geometry.attributes.contains_key("startHiAndForwardOffsetX"));
    assert!(geometry.attributes.contains_key("startLoAndForwardOffsetY"));
    assert!(geometry
        .attributes
        .contains_key("startNormalAndForwardOffsetZ"));
    assert!(geometry
        .attributes
        .contains_key("endNormalAndTextureCoordinateNormalizationX"));
    assert!(geometry
        .attributes
        .contains_key("rightNormalAndTextureCoordinateNormalizationY"));

    assert!(!geometry.attributes.contains_key("startHiLo2D"));
    assert!(!geometry.attributes.contains_key("offsetAndRight2D"));
    assert!(!geometry.attributes.contains_key("startEndNormals2D"));
    assert!(!geometry.attributes.contains_key("texcoordNormalization2D"));
}

#[test]
fn removes_adjacent_positions_with_the_same_latitude_longitude() {
    init_terrain_heights();
    let start_cartographic = Cartographic::from_degrees_new(0.01, 0.0, None);
    let end_cartographic = Cartographic::from_degrees_new(0.02, 0.0, None);
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_radians_array_heights(&[
            start_cartographic.longitude,
            start_cartographic.latitude,
            0.0,
            end_cartographic.longitude,
            end_cartographic.latitude,
            0.0,
            end_cartographic.longitude,
            end_cartographic.latitude,
            0.0,
            end_cartographic.longitude,
            end_cartographic.latitude,
            10.0,
        ], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 36);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 24);
}

#[test]
fn returns_none_if_filtered_points_are_not_a_valid_geometry() {
    init_terrain_heights();
    let start_cartographic = Cartographic::from_degrees_new(0.01, 0.0, None);
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_radians_array_heights(&[
            start_cartographic.longitude,
            start_cartographic.latitude,
            0.0,
            start_cartographic.longitude,
            start_cartographic.latitude,
            0.0,
        ], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry);
    assert!(geometry.is_none());
}

#[test]
fn miters_turns() {
    init_terrain_heights();
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0, 0.02, 0.01], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");
    assert_eq!(geometry.indices.as_ref().unwrap().len(), 72);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 48);

    let start_normal_values = &geometry
        .attributes
        .get("startNormalAndForwardOffsetZ")
        .unwrap()
        .values;
    let end_normal_values = &geometry
        .attributes
        .get("endNormalAndTextureCoordinateNormalizationX")
        .unwrap()
        .values;

    let mitered_start_normal = Cartesian3::unpack_new(start_normal_values, Some(32));
    let mitered_end_normal = Cartesian3::unpack_new(end_normal_values, Some(0));
    let reverse_mitered_end_normal =
        Cartesian3::multiply_by_scalar_new(&mitered_end_normal, -1.0);

    assert!(Cartesian3::equals_epsilon(
        Some(&mitered_start_normal),
        Some(&reverse_mitered_end_normal),
        Some(CesiumMath::EPSILON7),
        None,
    ));

    let approximate_expected_miter_normal =
        Cartesian3::normalize_new(&Cartesian3::new(0.0, 1.0, 1.0));
    assert!(Cartesian3::equals_epsilon(
        Some(&approximate_expected_miter_normal),
        Some(&mitered_start_normal),
        Some(CesiumMath::EPSILON2),
        None,
    ));
}

#[test]
fn breaks_miters_for_tight_turns() {
    init_terrain_heights();
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0, 0.01, CesiumMath::EPSILON7], None, None),
        None,
        Some(0.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    let start_normal_values = geometry
        .attributes
        .get("startNormalAndForwardOffsetZ")
        .unwrap()
        .values
        .clone();
    let end_normal_values = geometry
        .attributes
        .get("endNormalAndTextureCoordinateNormalizationX")
        .unwrap()
        .values
        .clone();

    let mitered_start_normal = Cartesian3::unpack_new(&start_normal_values, Some(32));
    let mitered_end_normal = Cartesian3::unpack_new(&end_normal_values, Some(0));

    assert!(Cartesian3::equals_epsilon(
        Some(&mitered_start_normal),
        Some(&mitered_end_normal),
        Some(CesiumMath::EPSILON7),
        None,
    ));

    let approximate_expected_miter_normal =
        Cartesian3::normalize_new(&Cartesian3::new(0.0, -1.0, 0.0));
    assert!(Cartesian3::equals_epsilon(
        Some(&approximate_expected_miter_normal),
        Some(&mitered_start_normal),
        Some(CesiumMath::EPSILON2),
        None,
    ));

    // Break miter on loop end
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[
            0.01,
            0.0,
            0.02,
            0.0,
            0.015,
            CesiumMath::EPSILON7,
        ], None, None),
        None,
        Some(0.0),
        Some(true),
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    let start_normal_values = geometry
        .attributes
        .get("startNormalAndForwardOffsetZ")
        .unwrap()
        .values
        .clone();
    let end_normal_values = geometry
        .attributes
        .get("endNormalAndTextureCoordinateNormalizationX")
        .unwrap()
        .values
        .clone();

    // Check normals at loop end
    let mitered_start_normal = Cartesian3::unpack_new(&start_normal_values, Some(0));
    let mitered_end_normal = Cartesian3::unpack_new(&end_normal_values, Some(32 * 2));

    assert!(Cartesian3::equals_epsilon(
        Some(&mitered_start_normal),
        Some(&mitered_end_normal),
        Some(CesiumMath::EPSILON7),
        None,
    ));

    let approximate_expected_miter_normal =
        Cartesian3::normalize_new(&Cartesian3::new(0.0, 1.0, 0.0));
    assert!(Cartesian3::equals_epsilon(
        Some(&approximate_expected_miter_normal),
        Some(&mitered_start_normal),
        Some(CesiumMath::EPSILON2),
        None,
    ));
}

#[test]
fn interpolates_long_polyline_segments() {
    init_terrain_heights();
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0], None, None),
        None,
        Some(600.0), // 0.01 to 0.02 is about 1113 meters, expect two segments
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 72);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 48);

    // Interpolate one segment but not the other
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0, 0.0201, 0.0], None, None),
        None,
        Some(600.0),
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 36 * 3);
    assert_eq!(
        geometry.attributes.get("position").unwrap().values.len(),
        24 * 3
    );
}

#[test]
fn interpolates_long_polyline_segments_for_rhumb_lines() {
    init_terrain_heights();
    // rhumb distance = 289020, geodesic distance = 288677
    let positions = Cartesian3::from_degrees_array(&[10.0, 75.0, 20.0, 75.0], None, None);

    let rhumb = GroundPolylineGeometry::new(
        positions.clone(),
        None,
        Some(2890.0),
        None,
        Some(ArcType::Rhumb),
    );
    let geodesic = GroundPolylineGeometry::new(
        positions,
        None,
        Some(2890.0),
        None,
        Some(ArcType::Geodesic),
    );

    let rhumb_geometry = GroundPolylineGeometry::create_geometry(&rhumb)
        .expect("geometry should not be None");
    let geodesic_geometry = GroundPolylineGeometry::create_geometry(&geodesic)
        .expect("geometry should not be None");

    assert_eq!(rhumb_geometry.indices.as_ref().unwrap().len(), 3636);
    assert_eq!(geodesic_geometry.indices.as_ref().unwrap().len(), 3600);
    assert_eq!(
        geodesic_geometry
            .attributes
            .get("position")
            .unwrap()
            .values
            .len(),
        2400
    );
    assert_eq!(
        rhumb_geometry
            .attributes
            .get("position")
            .unwrap()
            .values
            .len(),
        2424
    );

    // Interpolate one segment but not the other
    let positions = Cartesian3::from_degrees_array(&[10.0, 75.0, 20.0, 75.0, 20.01, 75.0], None, None);
    let rhumb = GroundPolylineGeometry::new(
        positions.clone(),
        None,
        Some(2890.0),
        None,
        Some(ArcType::Rhumb),
    );
    let geodesic = GroundPolylineGeometry::new(
        positions,
        None,
        Some(2890.0),
        None,
        Some(ArcType::Geodesic),
    );

    let rhumb_geometry = GroundPolylineGeometry::create_geometry(&rhumb)
        .expect("geometry should not be None");
    let geodesic_geometry = GroundPolylineGeometry::create_geometry(&geodesic)
        .expect("geometry should not be None");

    assert_eq!(rhumb_geometry.indices.as_ref().unwrap().len(), 3636 + 36);
    assert_eq!(geodesic_geometry.indices.as_ref().unwrap().len(), 3600 + 36);
    assert_eq!(
        geodesic_geometry
            .attributes
            .get("position")
            .unwrap()
            .values
            .len(),
        2400 + 24
    );
    assert_eq!(
        rhumb_geometry
            .attributes
            .get("position")
            .unwrap()
            .values
            .len(),
        2424 + 24
    );
}

#[test]
fn loops_when_there_are_enough_positions_and_loop_is_specified() {
    init_terrain_heights();
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0], None, None),
        None,
        Some(0.0),
        Some(true),
        None,
    );

    // Not enough positions to loop, should still be a single segment
    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");
    assert_eq!(geometry.indices.as_ref().unwrap().len(), 36);

    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0, 0.02, 0.02], None, None),
        None,
        Some(0.0),
        Some(true),
        None,
    );

    // Loop should produce 3 segments
    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");
    assert_eq!(geometry.indices.as_ref().unwrap().len(), 108);
}

#[test]
fn subdivides_geometry_across_the_idl_and_prime_meridian() {
    init_terrain_heights();
    // Cross PM
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-1.0, 0.0, 1.0, 0.0], None, None),
        None,
        Some(0.0), // no interpolative subdivision
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 72);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 48);

    // Cross IDL
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-179.0, 0.0, 179.0, 0.0], None, None),
        None,
        Some(0.0), // no interpolative subdivision
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 72);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 48);

    // Cross IDL going opposite direction and loop
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[
            179.0, 0.0, 179.0, 1.0, -179.0, 1.0, -179.0, 0.0,
        ], None, None),
        None,
        Some(0.0), // no interpolative subdivision
        Some(true),
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 6 * 36);
    assert_eq!(
        geometry.attributes.get("position").unwrap().values.len(),
        6 * 24
    );

    // Near-IDL case
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[179.999, 80.0, -179.999, 80.0], None, None),
        None,
        Some(0.0), // no interpolative subdivision
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    assert_eq!(geometry.indices.as_ref().unwrap().len(), 72);
    assert_eq!(geometry.attributes.get("position").unwrap().values.len(), 48);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "At least two positions are required.")]
fn throws_errors_if_not_enough_positions_have_been_provided() {
    let _ = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[0.01, 0.0], None, None),
        None,
        Some(0.0),
        Some(true),
        None,
    );
}

#[test]
fn can_unpack_onto_an_existing_instance() {
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-1.0, 0.0, 1.0, 0.0], None, None),
        None,
        None,
        Some(true),
        None,
    );
    let mut ground_polyline_geometry = ground_polyline_geometry;
    ground_polyline_geometry.granularity = 10.0;
    ground_polyline_geometry.set_scene3d_only(true);
    ground_polyline_geometry.set_projection_and_ellipsoid(
        &GroundPolylineProjection::WebMercator(WebMercatorProjection::new(Some(
            Ellipsoid::UNIT_SPHERE,
        ))),
    );

    let mut packed_array = vec![0.0f64; 1 + ground_polyline_geometry.packed_length()];
    ground_polyline_geometry.pack(&mut packed_array, Some(1));
    let mut scratch = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-1.0, 0.0, 1.0, 0.0], None, None),
        None,
        None,
        None,
        None,
    );
    GroundPolylineGeometry::unpack(&packed_array, Some(1), Some(&mut scratch));

    let scratch_positions = scratch.positions();
    assert_eq!(scratch_positions.len(), 2);
    assert!(Cartesian3::equals(
        Some(&scratch_positions[0]),
        Some(&ground_polyline_geometry.positions()[0])
    ));
    assert!(Cartesian3::equals(
        Some(&scratch_positions[1]),
        Some(&ground_polyline_geometry.positions()[1])
    ));
    assert!(scratch.r#loop);
    assert_eq!(scratch.granularity, 10.0);
    assert!(scratch.ellipsoid().equals(&Ellipsoid::UNIT_SPHERE));
    assert!(scratch.scene3d_only());
    assert_eq!(scratch.projection_index(), 1);
}

#[test]
fn can_unpack_onto_a_new_instance() {
    let mut ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-1.0, 0.0, 1.0, 0.0], None, None),
        None,
        None,
        Some(true),
        None,
    );
    ground_polyline_geometry.granularity = 10.0;
    ground_polyline_geometry.set_scene3d_only(true);
    ground_polyline_geometry.set_projection_and_ellipsoid(
        &GroundPolylineProjection::WebMercator(WebMercatorProjection::new(Some(
            Ellipsoid::UNIT_SPHERE,
        ))),
    );

    let mut packed_array = vec![0.0f64; 1 + ground_polyline_geometry.packed_length()];
    ground_polyline_geometry.pack(&mut packed_array, Some(1));
    let result = GroundPolylineGeometry::unpack(&packed_array, Some(1), None);

    let result_positions = result.positions();
    assert_eq!(result_positions.len(), 2);
    assert!(Cartesian3::equals(
        Some(&result_positions[0]),
        Some(&ground_polyline_geometry.positions()[0])
    ));
    assert!(Cartesian3::equals(
        Some(&result_positions[1]),
        Some(&ground_polyline_geometry.positions()[1])
    ));
    assert!(result.r#loop);
    assert_eq!(result.granularity, 10.0);
    assert!(result.ellipsoid().equals(&Ellipsoid::UNIT_SPHERE));
    assert!(result.scene3d_only());
    assert_eq!(result.projection_index(), 1);
}

#[test]
fn provides_a_method_for_setting_projection_and_ellipsoid() {
    let mut ground_polyline_geometry = GroundPolylineGeometry::new(
        Cartesian3::from_degrees_array(&[-1.0, 0.0, 1.0, 0.0], None, None),
        None,
        None,
        Some(true),
        None,
    );
    ground_polyline_geometry.granularity = 10.0;

    ground_polyline_geometry.set_projection_and_ellipsoid(
        &GroundPolylineProjection::WebMercator(WebMercatorProjection::new(Some(
            Ellipsoid::UNIT_SPHERE,
        ))),
    );

    assert_eq!(ground_polyline_geometry.projection_index(), 1);
    assert!(ground_polyline_geometry
        .ellipsoid()
        .equals(&Ellipsoid::UNIT_SPHERE));
}

#[test]
fn projects_normals_that_cross_the_idl() {
    let projection = GroundPolylineProjection::Geographic(GeographicProjection::new(None));
    let cartographic = Cartographic::from_radians_new(
        CesiumMath::PI - CesiumMath::EPSILON11,
        0.0,
        None,
    );
    let normal = Cartesian3::new(0.0, -1.0, 0.0);
    let projected_position = match &projection {
        GroundPolylineProjection::Geographic(p) => p.project(&cartographic),
        GroundPolylineProjection::WebMercator(p) => p.project(&cartographic),
    };
    let mut result = Cartesian3::default();

    project_normal(
        &projection,
        &cartographic,
        &normal,
        &projected_position,
        &mut result,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&result),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
        Some(CesiumMath::EPSILON7),
        None,
    ));
}

#[test]
fn creates_bounding_spheres_that_cover_the_entire_polyline_volume_height() {
    init_terrain_heights();
    let positions = Cartesian3::from_degrees_array(&[
        -122.17580380403314,
        46.19984918190237,
        -122.17581380403314,
        46.19984918190237,
    ], None, None);

    // Mt. St. Helens - provided coordinates are a few meters apart
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        positions.clone(),
        None,
        Some(0.0), // no interpolative subdivision
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    let bounding_sphere = geometry
        .bounding_sphere
        .as_ref()
        .expect("bounding sphere should be present");
    let points_distance = Cartesian3::distance(&positions[0], &positions[1]);

    assert!(bounding_sphere.radius > points_distance);
    assert!(bounding_sphere.radius > 1000.0); // starting top/bottom height
}

#[test]
fn creates_bounding_spheres_that_cover_the_entire_polyline_volume_height_in_negative_elevation_regions(
) {
    init_terrain_heights();
    let positions = Cartesian3::from_degrees_array(&[
        35.549174, 31.377954, 35.549174, 31.377953,
    ], None, None);

    // Dead Sea - provided coordinates from below sea level to above sea
    // level; the min/max approximateTerrainHeight values for this region are
    // -398.55, 2689.12
    let ground_polyline_geometry = GroundPolylineGeometry::new(
        positions.clone(),
        None,
        Some(0.0), // no interpolative subdivision
        None,
        None,
    );

    let geometry = GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
        .expect("geometry should not be None");

    let bounding_sphere = geometry
        .bounding_sphere
        .as_ref()
        .expect("bounding sphere should be present");
    let points_distance = Cartesian3::distance(&positions[0], &positions[1]);

    assert!(bounding_sphere.radius > points_distance);
    // in the GroundPolylineCode radius is sumHeights / 2 -- so we expect
    // radius to be at least double
    assert!(bounding_sphere.radius > 3087.0 / 2.0);
}

#[test]
fn packs_and_unpacks_roundtrip() {
    let positions = Cartesian3::from_degrees_array(&[0.01, 0.0, 0.02, 0.0, 0.02, 0.1], None, None);
    let mut polyline = GroundPolylineGeometry::new(
        positions.clone(),
        None,
        Some(1000.0),
        Some(true),
        None,
    );

    let mut expected = vec![positions.len() as f64];
    expected.resize(1 + positions.len() * 3, 0.0);
    for (i, p) in positions.iter().enumerate() {
        Cartesian3::pack(p, &mut expected, Some(1 + i * 3));
    }
    expected.push(polyline.granularity);
    expected.push(if polyline.r#loop { 1.0 } else { 0.0 });
    expected.push(ArcType::Geodesic as i32 as f64);
    {
        let start = expected.len();
        expected.resize(start + Ellipsoid::PACKED_LENGTH, 0.0);
        Ellipsoid::pack(&Ellipsoid::WGS84, &mut expected, Some(start));
    }
    expected.push(0.0); // projection index for Geographic (default)
    expected.push(0.0); // scene3DModeOnly = false

    let mut packed = vec![0.0f64; polyline.packed_length()];
    polyline.pack(&mut packed, None);
    assert_eq!(packed, expected);

    // Round trip back.
    polyline.granularity = 0.0;
    let unpacked = GroundPolylineGeometry::unpack(&packed, None, None);
    assert_eq!(unpacked.positions().len(), 3);
    assert!(unpacked.r#loop);
    assert_eq!(unpacked.granularity, 1000.0);
    assert_eq!(unpacked.arc_type, ArcType::Geodesic);
    assert!(unpacked.ellipsoid().equals(&Ellipsoid::WGS84));
    assert_eq!(unpacked.projection_index(), 0);
    assert!(!unpacked.scene3d_only());
}
