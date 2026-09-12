//! Terrain-picking fidelity batch: one-to-one Rust mirrors of the CesiumJS
//! Jasmine specs for the Track B3-1 terrain-pick substantiation.
//!
//! Mirrors:
//! - `packages/engine/Specs/Core/TerrainEncodingSpec.js`  (decodePosition /
//!   getExaggeratedPosition / decodeHeight / decodeTextureCoordinates)
//! - `packages/engine/Specs/Core/TerrainPickerSpec.js`    (node AABBs,
//!   rayIntersect, back-face culling)
//! - `packages/engine/Specs/Core/TerrainMeshSpec.js`      (getTransform
//!   caching, degenerate z-scale, 2D swizzle, pick)
//! - `packages/engine/Specs/Core/BoundingSphereSpec.js`   (fromOrientedBoundingBox)

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::oriented_bounding_box::OrientedBoundingBox;
use cesium_core::ray::Ray;
use cesium_core::rectangle::Rectangle;
use cesium_core::scene_mode::SceneMode;
use cesium_core::terrain_encoding::TerrainEncoding;
use cesium_core::terrain_mesh::TerrainMesh;
use cesium_core::terrain_picker::{TerrainPicker, TerrainPickerNode};
use cesium_core::transforms;
use cesium_core::vertical_exaggeration::VerticalExaggeration;
use cesium_test_utils::{assert_approx_eq_f64, expect_to_throw_dev_error};

/// Absolute tolerance for anything that round-tripped through the `f32` vertex
/// buffer: an RTC offset of a few hundred metres carries ~1e-4 m of quantisation.
const F32_ABS_EPS: f64 = 1e-2;
/// Relative tolerance paired with [`F32_ABS_EPS`] (the values themselves are
/// ECEF metres, i.e. ~1e7, so the absolute term is what actually binds).
const F32_REL_EPS: f64 = 1e-9;

// ────────────────────────── fixtures ──────────────────────────

/// Packs one `[X, Y, Z, H, U, V]` vertex (stride 6, no normals/water).
fn vertex(x: f64, y: f64, z: f64, height: f64, u: f64, v: f64) -> [f32; 6] {
    [x as f32, y as f32, z as f32, height as f32, u as f32, v as f32]
}

/// A flat two-triangle patch on the `z = 0` plane inside the terrain picker's
/// unit-cube root node, wound so that the triangles face `+z`.
///
/// ```text
/// v3 ────── v2          triangle 0: v0 v1 v2   (x >= y half)
/// │ ╲      │            triangle 1: v0 v2 v3   (x <= y half)
/// │   ╲    │
/// v0 ────── v1
/// ```
fn flat_patch() -> (Vec<f32>, Vec<u32>, TerrainEncoding) {
    let mut vertices = Vec::new();
    for v in [
        vertex(-0.4, -0.4, 0.0, 0.0, 0.0, 0.0),
        vertex(0.4, -0.4, 0.0, 10.0, 1.0, 0.0),
        vertex(0.4, 0.4, 0.0, 20.0, 1.0, 1.0),
        vertex(-0.4, 0.4, 0.0, 30.0, 0.0, 1.0),
    ] {
        vertices.extend_from_slice(&v);
    }
    (vertices, vec![0, 1, 2, 0, 2, 3], TerrainEncoding::new(false, false, 1.0, 0.0))
}

/// A `TerrainMesh` over `rectangle` whose geometry is a flat patch at
/// `local z = -0.5` — the OBB's `minimum_height` face — inside the
/// `[-0.1, 0.1]²` neighbourhood of the tile's local origin.
///
/// The vertex buffer holds RTC offsets from `oriented_bounding_box.center`,
/// exactly as `HeightmapTerrainData.createMesh` produces them, so
/// `TerrainEncoding::decode_position` reconstructs world coordinates.
fn obb_patch_mesh(
    rectangle: Rectangle,
    minimum_height: f64,
    maximum_height: f64,
) -> (TerrainMesh, OrientedBoundingBox, Matrix4) {
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(minimum_height),
        Some(maximum_height),
        Some(Ellipsoid::WGS84),
        None,
    );
    let mut transform = Matrix4::IDENTITY;
    OrientedBoundingBox::compute_transformation(&obb, &mut transform);

    // Local corners of the patch, at the OBB's `minimum_height` face.
    const LOCAL: [[f64; 2]; 4] = [[-0.1, -0.1], [0.1, -0.1], [0.1, 0.1], [-0.1, 0.1]];

    let mut vertices = Vec::new();
    for (i, [lx, ly]) in LOCAL.iter().enumerate() {
        // `computeTransformation` scales the half-axes by 2, so local `l` maps
        // to `center + halfAxes * (2l)`; the stored buffer is that offset.
        let local = Cartesian3::new(*lx, *ly, -0.5);
        let world = Matrix4::multiply_by_point_new(&transform, &local);
        let offset = Cartesian3::subtract_new(&world, &obb.center);
        vertices.extend_from_slice(&vertex(offset.x, offset.y, offset.z, i as f64, 0.0, 0.0));
    }

    let mesh = TerrainMesh {
        center: obb.center,
        vertices,
        stride: 6,
        indices: vec![0, 1, 2, 0, 2, 3],
        index_count_without_skirts: 6,
        vertex_count_without_skirts: 4,
        minimum_height,
        maximum_height,
        rectangle,
        bounding_sphere_3d: BoundingSphere::default(),
        occludee_point_in_scaled_space: Cartesian3::default(),
        encoding: TerrainEncoding::new_with_center(&obb.center, false, false, 1.0, 0.0),
        oriented_bounding_box: Some(obb.clone()),
        west_indices_south_to_north: Vec::new(),
        south_indices_east_to_west: Vec::new(),
        east_indices_north_to_south: Vec::new(),
        north_indices_west_to_east: Vec::new(),
        transform: Matrix4::IDENTITY,
        last_pick_scene_mode: None,
        terrain_picker: TerrainPicker::new(),
    };
    (mesh, obb, transform)
}

/// Compares two `Matrix4`s column by column (the port has no `Matrix4::equals`
/// assertion helper, and `Cartesian4` is what `get_column_new` hands back).
fn assert_matrix4_eq(left: &Matrix4, right: &Matrix4, context: &str) {
    for column in 0..4 {
        let l = Matrix4::get_column_new(left, column);
        let r = Matrix4::get_column_new(right, column);
        assert!(
            (l.x - r.x).abs() <= F32_ABS_EPS
                && (l.y - r.y).abs() <= F32_ABS_EPS
                && (l.z - r.z).abs() <= F32_ABS_EPS
                && (l.w - r.w).abs() <= F32_ABS_EPS,
            "{context}: column {column} differs\n  left:  {l:?}\n  right: {r:?}"
        );
    }
}

// ────────────────────────── TerrainEncoding ──────────────────────────

/// `TerrainEncodingSpec`: the stride charges 6 for XYZHUV, 2 more for the
/// oct-encoded normal pair, and 1 more for the water mask.
#[test]
fn encoding_stride_charges_two_components_for_the_oct_normal_pair() {
    let plain = TerrainEncoding::new(false, false, 1.0, 0.0);
    assert_eq!(plain.stride, 6);
    assert_eq!(plain.offset_vertex_normal, 6);
    assert_eq!(plain.offset_geodetic_surface_normal, 6);

    let normals = TerrainEncoding::new(true, false, 1.0, 0.0);
    assert_eq!(normals.stride, 8, "the oct pair costs two components");
    assert_eq!(normals.offset_vertex_normal, 6);
    assert_eq!(normals.offset_geodetic_surface_normal, 8);

    let both = TerrainEncoding::new(true, true, 1.0, 0.0);
    assert_eq!(both.stride, 9);
    assert_eq!(both.offset_vertex_normal, 6);
    // The geodetic-surface-normal slot follows the water mask, so it lands at
    // 9 rather than 8 when both optional attributes are present.
    assert_eq!(both.offset_geodetic_surface_normal, 9);
}

/// `TerrainEncoding#decodePosition` adds the RTC centre back, which is what
/// turns the packed buffer into world coordinates.
#[test]
fn decode_position_adds_the_rtc_center_back() {
    let center = Cartesian3::new(1.0e6, 2.0e6, 3.0e6);
    let encoding = TerrainEncoding::new_with_center(&center, false, false, 1.0, 0.0);
    assert_eq!(encoding.center, center);

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&vertex(1.0, 2.0, 3.0, 4.0, 0.25, 0.75));
    buffer.extend_from_slice(&vertex(5.0, 6.0, 7.0, 8.0, 0.5, 0.5));

    let mut decoded = Cartesian3::default();
    encoding.decode_position(&buffer, 0, &mut decoded);
    assert_approx_eq_f64!(decoded.x, 1.0e6 + 1.0);
    assert_approx_eq_f64!(decoded.y, 2.0e6 + 2.0);
    assert_approx_eq_f64!(decoded.z, 3.0e6 + 3.0);

    // The `index` is in *vertices*, not components.
    encoding.decode_position(&buffer, 1, &mut decoded);
    assert_approx_eq_f64!(decoded.x, 1.0e6 + 5.0);
    assert_approx_eq_f64!(decoded.y, 2.0e6 + 6.0);
    assert_approx_eq_f64!(decoded.z, 3.0e6 + 7.0);
}

/// `TerrainEncoding#decodeHeight` / `#decodeTextureCoordinates` read slots 3
/// and 4/5 of the vertex.
#[test]
fn decode_height_and_texture_coordinates_use_the_documented_slots() {
    let encoding = TerrainEncoding::new(false, false, 1.0, 0.0);
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&vertex(1.0, 2.0, 3.0, 4.0, 0.25, 0.75));
    buffer.extend_from_slice(&vertex(5.0, 6.0, 7.0, 8.0, 0.5, 0.125));

    assert_approx_eq_f64!(encoding.decode_height(&buffer, 0), 4.0);
    assert_approx_eq_f64!(encoding.decode_height(&buffer, 1), 8.0);

    let mut uv = Cartesian2::default();
    encoding.decode_texture_coordinates(&buffer, 0, &mut uv);
    assert_approx_eq_f64!(uv.x, 0.25);
    assert_approx_eq_f64!(uv.y, 0.75);
    encoding.decode_texture_coordinates(&buffer, 1, &mut uv);
    assert_approx_eq_f64!(uv.x, 0.5);
    assert_approx_eq_f64!(uv.y, 0.125);
}

/// `TerrainEncoding#getExaggeratedPosition` reduces to `decodePosition` when
/// the mesh carries no geodetic surface normals — even with exaggeration on,
/// which is exactly what the JS does (the branch requires both conditions).
#[test]
fn get_exaggerated_position_reduces_to_decode_position_without_geodetic_normals() {
    let center = Cartesian3::new(1.0e6, 0.0, 0.0);
    let encoding = TerrainEncoding::new_with_center(&center, false, false, 5.0, 100.0);
    assert!(!encoding.has_geodetic_surface_normals);

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&vertex(10.0, 20.0, 30.0, 250.0, 0.0, 0.0));

    let mut plain = Cartesian3::default();
    let mut exaggerated = Cartesian3::default();
    encoding.decode_position(&buffer, 0, &mut plain);
    encoding.get_exaggerated_position(&buffer, 0, &mut exaggerated);

    assert_approx_eq_f64!(exaggerated.x, plain.x);
    assert_approx_eq_f64!(exaggerated.y, plain.y);
    assert_approx_eq_f64!(exaggerated.z, plain.z);

    // The exaggeration *is* live on the encoding, so the reduction is because
    // of the missing normals, not a missing scale.
    assert_approx_eq_f64!(
        VerticalExaggeration::get_height(250.0, encoding.exaggeration, encoding.exaggeration_relative_height),
        (250.0 - 100.0) * 5.0 + 100.0
    );
}

// ────────────────────────── TerrainPickerNode ──────────────────────────

/// `TerrainPicker.js#createAABBForNode(0, 0, 0)`: the root of the tree is a
/// unit cube centred on the origin.
#[test]
fn the_root_node_is_the_unit_cube() {
    let root = TerrainPickerNode::new();
    assert_eq!((root.x, root.y, root.level), (0, 0, 0));
    assert_approx_eq_f64!(root.aabb.minimum.x, -0.5);
    assert_approx_eq_f64!(root.aabb.minimum.y, -0.5);
    assert_approx_eq_f64!(root.aabb.minimum.z, -0.5);
    assert_approx_eq_f64!(root.aabb.maximum.x, 0.5);
    assert_approx_eq_f64!(root.aabb.maximum.y, 0.5);
    assert_approx_eq_f64!(root.aabb.maximum.z, 0.5);
    assert_approx_eq_f64!(root.aabb.center.x, 0.0);
    assert_approx_eq_f64!(root.aabb.center.y, 0.0);
    assert_approx_eq_f64!(root.aabb.center.z, 0.0);

    assert!(root.intersecting_triangles.is_empty());
    assert!(root.children.is_empty());
    assert!(!root.building_children);
}

/// `TerrainPickerNode#addChild`: `x = 2 * parent.x + (idx & 1)`,
/// `y = 2 * parent.y + ((idx >> 1) & 1)`, and the AABB halves in `x`/`y`
/// while keeping the full `-0.5..0.5` height extent.
#[test]
fn add_child_lays_out_the_quadrants() {
    let mut root = TerrainPickerNode::new();
    for child_idx in 0..4 {
        root.add_child(child_idx);
    }
    assert_eq!(root.children.len(), 4);

    // (idx, x, y, minX, minY, maxX, maxY)
    let expected: [(usize, u32, u32, f64, f64, f64, f64); 4] = [
        (0, 0, 0, -0.5, -0.5, 0.0, 0.0),
        (1, 1, 0, 0.0, -0.5, 0.5, 0.0),
        (2, 0, 1, -0.5, 0.0, 0.0, 0.5),
        (3, 1, 1, 0.0, 0.0, 0.5, 0.5),
    ];
    for (idx, x, y, min_x, min_y, max_x, max_y) in expected {
        let child = &root.children[idx];
        assert_eq!((child.x, child.y, child.level), (x, y, 1));
        assert_approx_eq_f64!(child.aabb.minimum.x, min_x);
        assert_approx_eq_f64!(child.aabb.minimum.y, min_y);
        assert_approx_eq_f64!(child.aabb.minimum.z, -0.5);
        assert_approx_eq_f64!(child.aabb.maximum.x, max_x);
        assert_approx_eq_f64!(child.aabb.maximum.y, max_y);
        assert_approx_eq_f64!(child.aabb.maximum.z, 0.5);
    }

    // Recursion: the level-2 boxes are a quarter of the root on each axis. The
    // children are added in index order, as `getClosestTriangleInNode` — the
    // sole JS caller — does; adding a lone index 3 to an empty node would hit
    // the JS's sparse-array assignment instead.
    let mut southwest = root.children[0].clone();
    for child_idx in 0..4 {
        southwest.add_child(child_idx);
    }
    let grandchild = &southwest.children[3];
    assert_eq!((grandchild.x, grandchild.y, grandchild.level), (1, 1, 2));
    assert_approx_eq_f64!(grandchild.aabb.minimum.x, -0.25);
    assert_approx_eq_f64!(grandchild.aabb.minimum.y, -0.25);
    assert_approx_eq_f64!(grandchild.aabb.maximum.x, 0.0);
    assert_approx_eq_f64!(grandchild.aabb.maximum.y, 0.0);
}

/// `TerrainPickerNode#addChild` guards the child index with a DeveloperError.
#[test]
fn add_child_rejects_an_out_of_range_index() {
    expect_to_throw_dev_error(|| {
        let mut node = TerrainPickerNode::new();
        node.add_child(4);
    });
}

/// The JS assigns `this.children[childIdx]`, so re-adding the same index
/// replaces rather than grows; the port's replace-arm keeps that idempotent.
#[test]
fn re_adding_a_child_index_replaces_it() {
    let mut node = TerrainPickerNode::new();
    node.add_child(0);
    node.add_child(0);
    assert_eq!(node.children.len(), 1);
    assert_eq!((node.children[0].x, node.children[0].y), (0, 0));
}

// ────────────────────────── TerrainPicker ──────────────────────────

/// A fresh picker must rebuild its quadtree on the first intersection.
#[test]
fn a_new_picker_needs_a_rebuild() {
    let picker = TerrainPicker::new();
    assert!(picker.needs_rebuild);
}

/// `TerrainPicker#rayIntersect` returns the closest point on the mesh, and
/// clears `needsRebuild` once the root node has taken every triangle.
#[test]
fn ray_intersect_finds_the_closest_point_on_a_flat_patch() {
    let (vertices, indices, encoding) = flat_patch();
    let mut picker = TerrainPicker::new();

    // Local space *is* world space here: the tile transform is the identity, so
    // the root node's unit cube is the whole tile. Aim inside triangle 0
    // (`x >= y`), away from the shared diagonal.
    let ray = Ray::new(
        Some(&Cartesian3::new(0.1, -0.1, 0.4)),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
    );
    let hit = picker
        .ray_intersect(
            &vertices,
            &indices,
            &encoding,
            &ray,
            &Matrix4::IDENTITY,
            true,
            Some(SceneMode::Scene3D),
            None,
        )
        .expect("the ray drops straight onto the patch");

    assert_approx_eq_f64!(hit.x, 0.1);
    assert_approx_eq_f64!(hit.y, -0.1);
    assert_approx_eq_f64!(hit.z, 0.0);
    assert!(!picker.needs_rebuild, "the lazy rebuild must have run");

    // The second call reuses the refined tree and still answers.
    let again = picker
        .ray_intersect(
            &vertices,
            &indices,
            &encoding,
            &ray,
            &Matrix4::IDENTITY,
            true,
            Some(SceneMode::Scene3D),
            None,
        )
        .expect("a cached tree must keep picking");
    assert_approx_eq_f64!(again.x, hit.x);
    assert_approx_eq_f64!(again.y, hit.y);
    assert_approx_eq_f64!(again.z, hit.z);
}

/// `getNodesIntersectingRay` rejects the whole tile when the ray never enters
/// the root node's unit cube, so no triangle is ever tested.
#[test]
fn ray_intersect_returns_none_when_the_ray_misses_the_root_aabb() {
    let (vertices, indices, encoding) = flat_patch();
    let mut picker = TerrainPicker::new();

    let ray = Ray::new(
        Some(&Cartesian3::new(2.0, 0.0, 0.4)),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
    );
    assert!(picker
        .ray_intersect(
            &vertices,
            &indices,
            &encoding,
            &ray,
            &Matrix4::IDENTITY,
            true,
            Some(SceneMode::Scene3D),
            None,
        )
        .is_none());
}

/// `IntersectionTests.rayTriangleParametric`'s `cullBackFaces` gate: reversing
/// the winding makes the Möller-Trumbore determinant negative, which the
/// culling path rejects and the non-culling path accepts.
#[test]
fn cull_back_faces_rejects_the_reversed_winding() {
    let (vertices, _, encoding) = flat_patch();
    let reversed = vec![0u32, 2, 1, 0, 3, 2];

    let ray = Ray::new(
        Some(&Cartesian3::new(0.1, -0.1, 0.4)),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
    );

    let mut culling = TerrainPicker::new();
    assert!(
        culling
            .ray_intersect(
                &vertices,
                &reversed,
                &encoding,
                &ray,
                &Matrix4::IDENTITY,
                true,
                Some(SceneMode::Scene3D),
                None,
            )
            .is_none(),
        "a back-facing triangle must be culled"
    );

    let mut permissive = TerrainPicker::new();
    let hit = permissive
        .ray_intersect(
            &vertices,
            &reversed,
            &encoding,
            &ray,
            &Matrix4::IDENTITY,
            false,
            Some(SceneMode::Scene3D),
            None,
        )
        .expect("cullBackFaces = false must still hit the reversed patch");
    assert_approx_eq_f64!(hit.x, 0.1);
    assert_approx_eq_f64!(hit.y, -0.1);
    assert_approx_eq_f64!(hit.z, 0.0);
}

/// A ray that starts *behind* the patch has a negative `t`, which
/// `getClosestTriangleInNode` filters with `tri_t >= 0.0`.
#[test]
fn ray_intersect_ignores_intersections_behind_the_origin() {
    let (vertices, indices, encoding) = flat_patch();
    let mut picker = TerrainPicker::new();

    let ray = Ray::new(
        Some(&Cartesian3::new(0.1, -0.1, -0.4)),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
    );
    assert!(picker
        .ray_intersect(
            &vertices,
            &indices,
            &encoding,
            &ray,
            &Matrix4::IDENTITY,
            true,
            Some(SceneMode::Scene3D),
            None,
        )
        .is_none());
}

// ────────────────────────── TerrainMesh ──────────────────────────

/// `TerrainMesh#getTransform` memoises on `_lastPickSceneMode`, returning the
/// cached `transform` without recomputation.
///
/// Note that only `pick` *writes* `_lastPickSceneMode` (`TerrainMesh.js` L342);
/// `getTransform` merely reads it. A mesh that has never been picked therefore
/// recomputes on every `getTransform` call — that is the JS behaviour, quirk
/// included.
#[test]
fn get_transform_caches_on_the_scene_mode() {
    let rectangle = Rectangle::new(-0.01, -0.01, 0.01, 0.01);
    let (mut mesh, obb, _) = obb_patch_mesh(rectangle, 0.0, 1000.0);
    assert_eq!(mesh.last_pick_scene_mode, None);

    let transform = mesh.get_transform(Some(SceneMode::Scene3D), None, None);
    assert_eq!(
        mesh.last_pick_scene_mode,
        None,
        "getTransform must not stamp the scene mode; only pick does"
    );

    // `OrientedBoundingBox.computeTransformation` scales the half-axes by 2 and
    // translates by the centre, so the unit cube maps onto the whole OBB.
    let mut expected = Matrix4::IDENTITY;
    OrientedBoundingBox::compute_transformation(&obb, &mut expected);
    assert_matrix4_eq(&transform, &expected, "3D transform");

    // Once a pick has stamped the mode, clobbering `transform` and re-asking
    // must hand the clobbered value back: that is the JS's
    // `if (this._lastPickSceneMode === mode) return this.transform;`
    mesh.last_pick_scene_mode = Some(SceneMode::Scene3D);
    mesh.transform = Matrix4::IDENTITY;
    let cached = mesh.get_transform(Some(SceneMode::Scene3D), None, None);
    assert_matrix4_eq(&cached, &Matrix4::IDENTITY, "cached transform");

    // A different mode invalidates the cache and recomputes.
    mesh.last_pick_scene_mode = Some(SceneMode::ColumbusView);
    let refreshed = mesh.get_transform(Some(SceneMode::Scene3D), None, None);
    assert_matrix4_eq(&refreshed, &expected, "mode change recomputes");
}

/// `TerrainMesh#pick` stamps `_lastPickSceneMode` after the picker call, which
/// is what makes the next same-mode `getTransform` a cache hit.
#[test]
fn pick_stamps_the_scene_mode_that_get_transform_caches_on() {
    let rectangle = Rectangle::new(-0.01, -0.01, 0.01, 0.01);
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 0.0, 1000.0);

    mesh.get_transform(Some(SceneMode::Scene3D), None, None);
    assert_eq!(mesh.last_pick_scene_mode, None);

    // Any ray will do: the point is the bookkeeping, not the hit.
    let ray = Ray::new(
        Some(&Cartesian3::new(1.0e9, 0.0, 0.0)),
        Some(&Cartesian3::new(-1.0, 0.0, 0.0)),
    );
    mesh.pick(&ray, true, Some(SceneMode::Scene3D), None, None);
    assert_eq!(mesh.last_pick_scene_mode, Some(SceneMode::Scene3D));
}

/// `TerrainMesh#updateSceneMode` / `#updateExaggeration` both drop the cache
/// and flag the picker for a rebuild.
#[test]
fn update_scene_mode_and_exaggeration_reset_the_cache() {
    let rectangle = Rectangle::new(-0.01, -0.01, 0.01, 0.01);
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 0.0, 1000.0);
    mesh.get_transform(Some(SceneMode::Scene3D), None, None);
    mesh.terrain_picker.needs_rebuild = false;

    mesh.update_scene_mode(Some(SceneMode::Scene2D));
    assert_eq!(mesh.last_pick_scene_mode, None);
    assert!(mesh.terrain_picker.needs_rebuild);

    mesh.terrain_picker.needs_rebuild = false;
    mesh.update_exaggeration(3.0, 0.0);
    assert_eq!(mesh.last_pick_scene_mode, None);
    assert!(mesh.terrain_picker.needs_rebuild);
}

/// `TerrainMesh.js#computeTransform` (L239-244) patches a degenerate z-scale
/// *after* building the transform, by calling `Matrix4.setScale` with the scale
/// it just read back out of that same matrix. On a realistic flat tile the
/// patch never fires at all.
///
/// `OrientedBoundingBox.fromRectangle` derives `minZ` from the rectangle's west
/// corners at `minimumHeight`, measured against the tangent plane at the
/// rectangle's *centre*, while `maxZ` is `maximumHeight` verbatim. The
/// ellipsoid sags below that tangent plane, so the two differ by ~1.6 km even
/// when the height range is exactly zero — nowhere near `EPSILON16`.
///
/// Golden values from `golden_terrain_transform.mjs`.
#[test]
fn the_z_scale_fix_up_never_fires_on_a_realistic_flat_tile() {
    let rectangle = Rectangle::new(-0.02, -0.01, 0.02, 0.01);
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 250.0, 250.0);

    let transform = mesh.get_transform(Some(SceneMode::Scene3D), None, None);

    // The z half-axis is the sag, not the (zero) height range.
    let half_axes = mesh.oriented_bounding_box.as_ref().unwrap().half_axes;
    assert_approx_eq_f64!(half_axes.elements[0], 0.0);
    assert_approx_eq_f64!(half_axes.elements[1], 127559.23565408871, 1e-9, 1e-6);
    assert_approx_eq_f64!(half_axes.elements[5], 63355.85853485622, 1e-9, 1e-6);
    assert_approx_eq_f64!(half_axes.elements[6], 796.1767546217889, 1e-9, 1e-6);

    let scale = Matrix4::get_scale_new(&transform);
    assert!(
        scale.z > CesiumMath::EPSILON16,
        "the guard must not trip: zScale = {}",
        scale.z
    );
    assert_approx_eq_f64!(scale.z, 1592.3535092435777, 1e-9, 1e-6);

    let mut expected = Matrix4::IDENTITY;
    expected.elements = [
        0.0,
        255118.47130817742,
        0.0,
        0.0,
        0.0,
        0.0,
        126711.71706971244,
        0.0,
        1592.3535092435777,
        0.0,
        0.0,
        0.0,
        6377590.823245378,
        0.0,
        0.0,
        1.0,
    ];
    assert_matrix4_eq(&transform, &expected, "3D transform of a flat tile");
}

/// The one input that *does* trip the guard is a rectangle so small the sag
/// underflows, leaving `halfAxes` column 2 exactly zero. `setScale` then
/// divides the target z-scale by the zero it just read back, and
/// `0.0 * Infinity` poisons the whole third column.
///
/// CesiumJS does exactly the same thing — the "avoid zero scale" patch makes
/// the transform *less* usable, not more — so the port reproduces the NaN bit
/// for bit instead of papering over it. Verified against
/// `golden_terrain_transform.mjs`. (The 2D path has no such trap:
/// `computeTransform2D` substitutes the 1.0 *before* building the matrix, so
/// `setScale` divides by the identity's unit scale — see
/// [`the_2d_transform_avoids_a_zero_z_scale`].)
#[test]
fn the_z_scale_fix_up_propagates_nan_on_a_zero_z_half_axis() {
    let rectangle = Rectangle::new(-1.0e-9, -1.0e-9, 1.0e-9, 1.0e-9);
    // The vertex buffer comes out NaN too (it is built through this same
    // transform); irrelevant here — only the transform is under test.
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 500.0, 500.0);

    let half_axes = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(500.0),
        Some(500.0),
        Some(Ellipsoid::WGS84),
        None,
    )
    .half_axes;
    assert_eq!(
        [half_axes.elements[6], half_axes.elements[7], half_axes.elements[8]],
        [0.0, 0.0, 0.0],
        "the sag must underflow for the guard to trip"
    );

    let transform = mesh.get_transform(Some(SceneMode::Scene3D), None, None);

    // Column 2 is NaN; the rest of the matrix survives untouched.
    assert!(transform.elements[8].is_nan());
    assert!(transform.elements[9].is_nan());
    assert!(transform.elements[10].is_nan());
    assert!(Matrix4::get_scale_new(&transform).z.is_nan());

    let mut expected = Matrix4::IDENTITY;
    expected.elements = [
        0.0,
        0.012757273999999999,
        0.0,
        0.0,
        0.0,
        0.0,
        0.01267187865458564,
        0.0,
        f64::NAN,
        f64::NAN,
        f64::NAN,
        0.0,
        6378637.0,
        0.0,
        0.0,
        1.0,
    ];
    for i in 0..16 {
        if expected.elements[i].is_nan() {
            assert!(transform.elements[i].is_nan(), "elements[{i}] should be NaN");
        } else {
            assert_approx_eq_f64!(transform.elements[i], expected.elements[i], 1e-12, 1e-12);
        }
    }
}

/// `TerrainMesh.js#computeTransform2D` builds the projected box and then
/// left-multiplies `Transforms.SWIZZLE_3D_TO_2D_MATRIX`.
#[test]
fn the_2d_transform_left_multiplies_the_swizzle_matrix() {
    let rectangle = Rectangle::new(-0.02, -0.01, 0.02, 0.01);
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 100.0, 900.0);
    let projection = GeographicProjection::new(None);

    let transform = mesh.get_transform(Some(SceneMode::Scene2D), Some(&projection), None);
    assert_eq!(
        mesh.last_pick_scene_mode,
        None,
        "getTransform must not stamp the scene mode; only pick does"
    );

    let southwest = projection.project(&Cartographic::from_radians_new(
        rectangle.west,
        rectangle.south,
        Some(0.0),
    ));
    let northeast = projection.project(&Cartographic::from_radians_new(
        rectangle.east,
        rectangle.north,
        Some(0.0),
    ));
    let height_range = 900.0 - 100.0;
    let scale = Cartesian3::new(
        northeast.x - southwest.x,
        northeast.y - southwest.y,
        height_range,
    );
    let center = Cartesian3::new(
        southwest.x + scale.x * 0.5,
        southwest.y + scale.y * 0.5,
        100.0 + scale.z * 0.5,
    );

    let translated = Matrix4::from_translation_new(&center);
    let mut scaled = Matrix4::IDENTITY;
    Matrix4::set_scale(&translated, &scale, &mut scaled);
    let expected = Matrix4::multiply_new(&transforms::SWIZZLE_3D_TO_2D_MATRIX, &scaled);

    assert_matrix4_eq(&transform, &expected, "2D transform");
}

/// A flat tile (no height range) must not produce a zero z-scale in 2D either:
/// `computeTransform2D` substitutes 1 for the "avoid zero scale" case.
#[test]
fn the_2d_transform_avoids_a_zero_z_scale() {
    let rectangle = Rectangle::new(-0.02, -0.01, 0.02, 0.01);
    let (mut mesh, _, _) = obb_patch_mesh(rectangle, 250.0, 250.0);
    let projection = GeographicProjection::new(None);

    let transform = mesh.get_transform(Some(SceneMode::Scene2D), Some(&projection), None);
    let scale = Matrix4::get_scale_new(&transform);
    assert_approx_eq_f64!(scale.z, 1.0, 1e-9, 1e-9);
}

/// `TerrainMesh#pick` end-to-end: the ray is expressed in world coordinates,
/// the transform comes from the mesh's own OBB, and the hit is the world point
/// the ray aimed at.
#[test]
fn pick_returns_the_world_intersection_and_records_the_scene_mode() {
    let rectangle = Rectangle::new(-0.01, -0.01, 0.01, 0.01);
    let (mut mesh, _, transform) = obb_patch_mesh(rectangle, 0.0, 1000.0);

    // Aim at the patch's local origin from straight above it.
    let target_local = Cartesian3::new(0.0, 0.0, -0.5);
    let start_local = Cartesian3::new(0.0, 0.0, 0.4);
    let target = Matrix4::multiply_by_point_new(&transform, &target_local);
    let start = Matrix4::multiply_by_point_new(&transform, &start_local);
    let direction = Cartesian3::normalize_new(&Cartesian3::subtract_new(&target, &start));

    let ray = Ray::new(Some(&start), Some(&direction));
    let hit = mesh
        .pick(&ray, true, Some(SceneMode::Scene3D), None, None)
        .expect("the ray drops onto the mesh's minimum-height face");

    assert_approx_eq_f64!(hit.x, target.x, F32_ABS_EPS, F32_REL_EPS);
    assert_approx_eq_f64!(hit.y, target.y, F32_ABS_EPS, F32_REL_EPS);
    assert_approx_eq_f64!(hit.z, target.z, F32_ABS_EPS, F32_REL_EPS);
    assert_eq!(mesh.last_pick_scene_mode, Some(SceneMode::Scene3D));

    // The patch sits at `minimum_height`, so the hit is that far below the OBB
    // centre along the box's z half-axis.
    let z_axis = Matrix3::get_column_new(
        &mesh.oriented_bounding_box.as_ref().unwrap().half_axes,
        2,
    );
    let below = Cartesian3::subtract_new(&mesh.center, &z_axis);
    assert_approx_eq_f64!(hit.x, below.x, F32_ABS_EPS, F32_REL_EPS);
    assert_approx_eq_f64!(hit.y, below.y, F32_ABS_EPS, F32_REL_EPS);
    assert_approx_eq_f64!(hit.z, below.z, F32_ABS_EPS, F32_REL_EPS);
}

/// `TerrainMesh#pick` returns `None` for a ray that misses the tile, and still
/// stamps `_lastPickSceneMode` — the JS assigns it after the picker call.
#[test]
fn pick_returns_none_for_a_missing_ray_but_records_the_scene_mode() {
    let rectangle = Rectangle::new(-0.01, -0.01, 0.01, 0.01);
    let (mut mesh, _, transform) = obb_patch_mesh(rectangle, 0.0, 1000.0);

    let start_local = Cartesian3::new(4.0, 4.0, 0.4);
    let start = Matrix4::multiply_by_point_new(&transform, &start_local);
    let up = Matrix4::multiply_by_point_as_vector_new(&transform, &Cartesian3::new(0.0, 0.0, 1.0));
    let ray = Ray::new(Some(&start), Some(&Cartesian3::normalize_new(&Cartesian3::negate_new(&up))));

    assert!(mesh
        .pick(&ray, true, Some(SceneMode::Scene3D), None, None)
        .is_none());
    assert_eq!(mesh.last_pick_scene_mode, Some(SceneMode::Scene3D));
}

// ────────────────────────── BoundingSphere ──────────────────────────

/// `BoundingSphere.fromOrientedBoundingBox`: centre is the box centre and the
/// radius is the magnitude of the summed half-axes columns.
#[test]
fn from_oriented_bounding_box_circumscribes_the_half_axes() {
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-0.05, -0.02, 0.05, 0.02)),
        Some(0.0),
        Some(1500.0),
        Some(Ellipsoid::WGS84),
        None,
    );

    let u = Matrix3::get_column_new(&obb.half_axes, 0);
    let v = Matrix3::get_column_new(&obb.half_axes, 1);
    let w = Matrix3::get_column_new(&obb.half_axes, 2);
    let summed = Cartesian3::add_new(&Cartesian3::add_new(&u, &v), &w);

    let sphere = BoundingSphere::from_oriented_bounding_box(&obb, None);
    assert_approx_eq_f64!(sphere.center.x, obb.center.x);
    assert_approx_eq_f64!(sphere.center.y, obb.center.y);
    assert_approx_eq_f64!(sphere.center.z, obb.center.z);
    assert_approx_eq_f64!(sphere.radius, Cartesian3::magnitude(&summed));

    // Every box corner must fall inside the sphere.
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                let offset = Cartesian3::add_new(
                    &Cartesian3::multiply_by_scalar_new(&u, sx),
                    &Cartesian3::add_new(
                        &Cartesian3::multiply_by_scalar_new(&v, sy),
                        &Cartesian3::multiply_by_scalar_new(&w, sz),
                    ),
                );
                let corner = Cartesian3::add_new(&obb.center, &offset);
                let distance = Cartesian3::magnitude(&Cartesian3::subtract_new(&corner, &sphere.center));
                assert!(
                    distance <= sphere.radius + 1e-6,
                    "corner {corner:?} escapes the sphere (d = {distance}, r = {})",
                    sphere.radius
                );
            }
        }
    }
}
