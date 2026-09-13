//! Core/TerrainEncodingSpec.js → Rust integration tests
//!
//! Faithful port of CesiumJS `Specs/Core/TerrainEncodingSpec.js` (19 `it()` cases).
//!
//! ## Platform adaptations
//! - The JS `clones with result` variant (writing into a caller-supplied object) is
//!   merged into the `clones` test: Rust `Clone` always returns a fresh owned value.
//! - JS passes a `Cartesian3` where a packed `Cartesian2` normal is expected (dynamic
//!   typing); `octPackFloat` only reads `.x`/`.y`, so the Rust port passes an explicit
//!   `DVec2::new(normal.x, normal.y)`.
//! - JS `undefined` optional arguments map to Rust `Option::None`.

use cesium_geospatial::attribute_compression::oct_encode;
use cesium_geospatial::bounding::AxisAlignedBoundingBox;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::transforms::east_north_up_to_fixed_frame;
use cesium_geospatial::vertical_exaggeration;
use cesium_specs::{assert_approx, assert_vec2_epsilon, assert_vec3_epsilon, epsilon};
use cesium_terrain::{TerrainEncoding, TerrainQuantization};
use glam::{DMat4, DVec2, DVec3};

/// Common `beforeEach` state from the JS spec.
struct Setup {
    center: DVec3,
    aabox: AxisAlignedBoundingBox,
    from_enu: DMat4,
    minimum_height: f64,
    maximum_height: f64,
}

fn setup() -> Setup {
    let center = Ellipsoid::WGS84.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let maximum = DVec3::new(6.0e2, 6.0e2, 6.0e2);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, center);
    let from_enu = east_north_up_to_fixed_frame(center, &Ellipsoid::WGS84);
    Setup {
        center,
        aabox,
        from_enu,
        minimum_height: 6.0e2,
        maximum_height: 6.0e2,
    }
}

/// `it("default constructs")`
#[test]
fn test_terrain_encoding_default_constructs() {
    let encoding = TerrainEncoding::default();
    assert_eq!(encoding.quantization, TerrainQuantization::None);
    assert!(encoding.minimum_height.is_none());
    assert!(encoding.maximum_height.is_none());
    assert!(encoding.center.is_none());
    assert!(encoding.to_scaled_enu.is_none());
    assert!(encoding.from_scaled_enu.is_none());
    assert!(encoding.matrix.is_none());
    assert!(!encoding.has_vertex_normals);
    assert!(!encoding.has_web_mercator_t);
    assert!(!encoding.has_geodetic_surface_normals);
    assert_eq!(encoding.exaggeration, 1.0);
    assert_eq!(encoding.exaggeration_relative_height, 0.0);
    assert_eq!(encoding.stride, 6);
}

/// `it("constructs without quantization")`
#[test]
fn test_terrain_encoding_constructs_without_quantization() {
    let s = setup();
    let maximum = DVec3::new(1.0e6, 1.0e6, 1.0e6);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, s.center);
    let maximum_height = 1.0e6;
    let minimum_height = maximum_height;
    let has_vertex_normals = false;
    let encoding = TerrainEncoding::from_aabb(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    assert_eq!(encoding.quantization, TerrainQuantization::None);
    assert_eq!(encoding.minimum_height, Some(minimum_height));
    assert_eq!(encoding.maximum_height, Some(maximum_height));
    assert_eq!(encoding.center, Some(s.center));
    assert!(encoding.to_scaled_enu.is_some());
    assert!(encoding.from_scaled_enu.is_some());
    assert!(encoding.matrix.is_some());
    assert_eq!(encoding.has_vertex_normals, has_vertex_normals);
}

/// `it("constructs with quantization")`
#[test]
fn test_terrain_encoding_constructs_with_quantization() {
    let s = setup();
    let maximum = DVec3::new(100.0, 100.0, 100.0);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, s.center);
    let minimum_height = -100.0;
    let maximum_height = 100.0;
    let has_vertex_normals = false;
    let encoding = TerrainEncoding::from_aabb(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    assert_eq!(encoding.quantization, TerrainQuantization::Bits12);
    assert_eq!(encoding.minimum_height, Some(minimum_height));
    assert_eq!(encoding.maximum_height, Some(maximum_height));
    assert_eq!(encoding.center, Some(s.center));
    assert!(encoding.to_scaled_enu.is_some());
    assert!(encoding.from_scaled_enu.is_some());
    assert!(encoding.matrix.is_some());
    assert_eq!(encoding.has_vertex_normals, has_vertex_normals);
}

/// `it("encodes without quantization or normals")`
#[test]
fn test_terrain_encoding_encodes_without_quantization_or_normals() {
    let s = setup();
    let maximum = DVec3::new(6.0e3, 6.0e3, 6.0e3);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, s.center);
    let maximum_height = 6.0e3;
    let minimum_height = maximum_height;
    let has_vertex_normals = false;
    let encoding = TerrainEncoding::from_aabb(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let position = DVec3::new(1.0e3, 1.0e3, 1.0e3);
    let position = s.from_enu.transform_point3(position);

    let mut buffer = Vec::new();
    encoding.encode(&mut buffer, position, DVec2::ZERO, 100.0, None, None, None);

    assert_eq!(encoding.stride, 6);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec3_epsilon!(encoding.decode_position(&buffer, 0), position, epsilon::EPSILON10);
}

/// `it("encodes without quantization and with normals")`
#[test]
fn test_terrain_encoding_encodes_without_quantization_with_normals() {
    let s = setup();
    let maximum = DVec3::new(6.0e3, 6.0e3, 6.0e3);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, s.center);
    let maximum_height = 6.0e3;
    let minimum_height = maximum_height;
    let has_vertex_normals = true;
    let encoding = TerrainEncoding::from_aabb(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let position = DVec3::new(1.0e3, 1.0e3, 1.0e3);
    let position = s.from_enu.transform_point3(position);
    let normal = position.normalize();

    let mut buffer = Vec::new();
    encoding.encode(
        &mut buffer,
        position,
        DVec2::ZERO,
        100.0,
        Some(DVec2::new(normal.x, normal.y)),
        None,
        None,
    );

    assert_eq!(encoding.stride, 7);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec3_epsilon!(encoding.decode_position(&buffer, 0), position, epsilon::EPSILON10);
}

/// `it("encodes position with quantization and without normals")`
#[test]
fn test_terrain_encoding_encodes_position_quantization_no_normals() {
    let s = setup();
    let has_vertex_normals = false;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let position = DVec3::new(1.0e2, 1.0e2, 1.0e2);
    let position = s.from_enu.transform_point3(position);

    let mut buffer = Vec::new();
    encoding.encode(&mut buffer, position, DVec2::ZERO, 100.0, None, None, None);

    assert_eq!(encoding.stride, 3);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec3_epsilon!(encoding.decode_position(&buffer, 0), position, 1.0);
}

/// `it("encodes position with quantization and normals")`
#[test]
fn test_terrain_encoding_encodes_position_quantization_normals() {
    let s = setup();
    let has_vertex_normals = true;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let position = DVec3::new(1.0e2, 1.0e2, 1.0e2);
    let position = s.from_enu.transform_point3(position);
    let normal = position.normalize();

    let mut buffer = Vec::new();
    encoding.encode(
        &mut buffer,
        position,
        DVec2::ZERO,
        100.0,
        Some(DVec2::new(normal.x, normal.y)),
        None,
        None,
    );

    assert_eq!(encoding.stride, 4);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec3_epsilon!(encoding.decode_position(&buffer, 0), position, 1.0);
}

/// `it("encodes position without quantization and with exaggeration")`
#[test]
fn test_terrain_encoding_encodes_with_exaggeration() {
    let has_vertex_normals = false;
    let has_web_mercator_t = false;
    let has_geodetic_surface_normals = true;

    let height = 1_000_000.0f64;
    let position = DVec3::new(height, 0.0, 0.0);
    let geodetic_surface_normal = DVec3::new(1.0, 0.0, 0.0);

    let exaggeration = 2.0;
    let exaggeration_relative_height = 10.0;
    let exaggerated_height = vertical_exaggeration::get_height(
        height,
        exaggeration,
        exaggeration_relative_height,
    );
    let exaggerated_position = DVec3::new(exaggerated_height, 0.0, 0.0);

    let maximum_height = height;
    let minimum_height = -height;
    let maximum = DVec3::new(height, height, height);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, DVec3::ZERO);

    let encoding = TerrainEncoding::new(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        DMat4::IDENTITY,
        has_vertex_normals,
        has_web_mercator_t,
        has_geodetic_surface_normals,
        exaggeration,
        exaggeration_relative_height,
    );

    let mut buffer = Vec::new();
    encoding.encode(
        &mut buffer,
        position,
        DVec2::ZERO,
        height,
        None,
        None,
        Some(geodetic_surface_normal),
    );

    assert_eq!(encoding.stride, 9);
    assert_eq!(buffer.len(), encoding.stride);
    assert_vec3_epsilon!(
        encoding.get_exaggerated_position(&buffer, 0),
        exaggerated_position,
        epsilon::EPSILON5
    );
    assert_vec3_epsilon!(
        encoding.decode_geodetic_surface_normal(&buffer, 0),
        geodetic_surface_normal,
        epsilon::EPSILON5
    );
}

/// `it("encodes texture coordinates with quantization and without normals")`
#[test]
fn test_terrain_encoding_encodes_texcoords_quantization_no_normals() {
    let s = setup();
    let has_vertex_normals = false;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let tex_coords = DVec2::new(0.25, 0.75);

    let mut buffer = Vec::new();
    encoding.encode(&mut buffer, DVec3::ZERO, tex_coords, 100.0, None, None, None);

    assert_eq!(encoding.stride, 3);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec2_epsilon!(
        encoding.decode_texture_coordinates(&buffer, 0),
        tex_coords,
        1.0 / 4095.0
    );
}

/// `it("encodes textureCoordinates with quantization and normals")`
#[test]
fn test_terrain_encoding_encodes_texcoords_quantization_normals() {
    let s = setup();
    let has_vertex_normals = true;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let tex_coords = DVec2::new(0.75, 0.25);

    let mut buffer = Vec::new();
    encoding.encode(
        &mut buffer,
        DVec3::ZERO,
        tex_coords,
        100.0,
        Some(DVec2::new(1.0, 0.0)), // Cartesian3.UNIT_X → octPackFloat reads .x/.y
        None,
        None,
    );

    assert_eq!(encoding.stride, 4);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec2_epsilon!(
        encoding.decode_texture_coordinates(&buffer, 0),
        tex_coords,
        1.0 / 4095.0
    );
}

/// `it("encodes height with quantization and without normals")`
#[test]
fn test_terrain_encoding_encodes_height_quantization_no_normals() {
    let s = setup();
    let has_vertex_normals = false;
    let minimum_height = 0.0;
    let maximum_height = 200.0;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let mut buffer = Vec::new();
    let height = (maximum_height + minimum_height) * 0.5;
    encoding.encode(&mut buffer, s.center, DVec2::ZERO, height, None, None, None);

    assert_eq!(encoding.stride, 3);
    assert_eq!(buffer.len(), encoding.stride);

    assert_approx!(encoding.decode_height(&buffer, 0), height, 200.0 / 4095.0);
}

/// `it("encodes height with quantization and normals")`
#[test]
fn test_terrain_encoding_encodes_height_quantization_normals() {
    let s = setup();
    let has_vertex_normals = true;
    let minimum_height = 0.0;
    let maximum_height = 200.0;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        minimum_height,
        maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let mut buffer = Vec::new();
    let height = (maximum_height + minimum_height) * 0.5;
    encoding.encode(
        &mut buffer,
        s.center,
        DVec2::ZERO,
        height,
        Some(DVec2::new(1.0, 0.0)), // Cartesian3.UNIT_X
        None,
        None,
    );

    assert_eq!(encoding.stride, 4);
    assert_eq!(buffer.len(), encoding.stride);

    assert_approx!(encoding.decode_height(&buffer, 0), height, 200.0 / 4095.0);
}

/// `it("gets oct-encoded normal")`
#[test]
fn test_terrain_encoding_gets_oct_encoded_normal() {
    let s = setup();
    let has_vertex_normals = true;
    let encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let normal = DVec3::new(1.0, 1.0, 1.0).normalize();
    let oct_normal = oct_encode(normal);

    let mut buffer = Vec::new();
    encoding.encode(
        &mut buffer,
        s.center,
        DVec2::ZERO,
        s.minimum_height,
        Some(oct_normal),
        None,
        None,
    );

    assert_eq!(encoding.stride, 4);
    assert_eq!(buffer.len(), encoding.stride);

    assert_vec2_epsilon!(encoding.get_oct_encoded_normal(&buffer, 0), oct_normal, epsilon::EPSILON15);
}

/// `it("adds geodetic surface normals")`
#[test]
fn test_terrain_encoding_adds_geodetic_surface_normals() {
    let s = setup();
    let has_vertex_normals = false;
    let mut encoding = TerrainEncoding::from_aabb(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
    );

    let mut old_buffer = Vec::new();
    encoding.encode(
        &mut old_buffer,
        s.center,
        DVec2::ZERO,
        s.minimum_height,
        None,
        None,
        None,
    );
    let old_stride = encoding.stride;

    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let new_buffer = encoding.add_geodetic_surface_normals(&old_buffer, &ellipsoid);
    let new_stride = encoding.stride;
    let stride_difference = new_stride as i64 - old_stride as i64;

    assert_eq!(stride_difference, 3);
    assert_eq!(old_buffer.len(), old_stride);
    assert_eq!(new_buffer.len(), new_stride);
}

/// `it("removes geodetic surface normals")`
#[test]
fn test_terrain_encoding_removes_geodetic_surface_normals() {
    let s = setup();
    let has_vertex_normals = false;
    let has_web_mercator_t = false;
    let has_geodetic_surface_normals = true;
    let mut encoding = TerrainEncoding::new(
        s.aabox.center,
        &s.aabox,
        s.minimum_height,
        s.maximum_height,
        s.from_enu,
        has_vertex_normals,
        has_web_mercator_t,
        has_geodetic_surface_normals,
        1.0,
        0.0,
    );

    let geodetic_surface_normal = DVec3::new(1.0, 0.0, 0.0);
    let mut old_buffer = Vec::new();
    encoding.encode(
        &mut old_buffer,
        s.center,
        DVec2::ZERO,
        s.minimum_height,
        None,
        None,
        Some(geodetic_surface_normal),
    );
    let old_stride = encoding.stride;

    let new_buffer = encoding.remove_geodetic_surface_normals(&old_buffer);
    let new_stride = encoding.stride;
    let stride_difference = new_stride as i64 - old_stride as i64;

    assert_eq!(stride_difference, -3);
    assert_eq!(old_buffer.len(), old_stride);
    assert_eq!(new_buffer.len(), new_stride);
}

/// Helper for the attribute / clone tests (1e6 bounds → NONE quantization).
fn setup_none_quantization() -> TerrainEncoding {
    let center = Ellipsoid::WGS84.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let maximum = DVec3::new(1.0e6, 1.0e6, 1.0e6);
    let minimum = -maximum;
    let aabox = AxisAlignedBoundingBox::with_center(minimum, maximum, center);
    let maximum_height = 1.0e6;
    let minimum_height = maximum_height;
    let from_enu = east_north_up_to_fixed_frame(center, &Ellipsoid::WGS84);
    let has_vertex_normals = false;
    TerrainEncoding::from_aabb(
        aabox.center,
        &aabox,
        minimum_height,
        maximum_height,
        from_enu,
        has_vertex_normals,
    )
}

/// `it("gets attributes")`
#[test]
fn test_terrain_encoding_gets_attributes() {
    let encoding = setup_none_quantization();
    let attributes = encoding.get_attributes();
    assert_eq!(attributes.len(), 2);
}

/// `it("gets attribute locations")`
#[test]
fn test_terrain_encoding_gets_attribute_locations() {
    let encoding = setup_none_quantization();
    let attribute_locations = encoding.get_attribute_locations();
    // NONE quantization → position3DAndHeight / textureCoordAndEncodedNormals locations.
    assert_eq!(attribute_locations.position_3d_and_height, 0);
    assert_eq!(attribute_locations.texture_coord_and_encoded_normals, 1);
}

/// `it("clones")` and `it("clones with result")` (merged: Rust `Clone` returns a fresh value).
#[test]
fn test_terrain_encoding_clones() {
    let encoding = setup_none_quantization();
    let cloned = encoding.clone();

    assert_eq!(cloned.quantization, encoding.quantization);
    assert_eq!(cloned.minimum_height, encoding.minimum_height);
    assert_eq!(cloned.maximum_height, encoding.maximum_height);
    assert_eq!(cloned.center, encoding.center);
    assert_eq!(cloned.to_scaled_enu, encoding.to_scaled_enu);
    assert_eq!(cloned.from_scaled_enu, encoding.from_scaled_enu);
    assert_eq!(cloned.matrix, encoding.matrix);
    assert_eq!(cloned.has_vertex_normals, encoding.has_vertex_normals);
    assert_eq!(cloned.stride, encoding.stride);
}
