//! TerrainMesh + QuantizedMesh extended tests.
//!
//! Maps to CesiumJS:
//! - Core/TerrainMesh.js
//! - Core/QuantizedMeshTerrainData.js (mesh creation, normals)
//!
//! A-class tests: mesh computation, normals, vertex/triangle counts.

use cesium_terrain::TerrainMesh;
use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

fn make_simple_mesh() -> TerrainMesh {
    // A simple quad (2 triangles) in the XY plane
    TerrainMesh {
        positions: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        normals: None,
        tex_coords: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        indices: vec![0, 1, 2, 0, 2, 3],
        minimum_height: 0.0,
        maximum_height: 0.0,
        bounding_sphere: BoundingSphere::new(DVec3::new(0.5, 0.5, 0.0), 1.0),
    }
}

#[test]
fn terrain_mesh_vertex_count() {
    let mesh = make_simple_mesh();
    assert_eq!(mesh.vertex_count(), 4);
}

#[test]
fn terrain_mesh_triangle_count() {
    let mesh = make_simple_mesh();
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn terrain_mesh_compute_normals_flat() {
    let mut mesh = make_simple_mesh();
    assert!(mesh.normals.is_none());

    mesh.compute_normals();
    let normals = mesh.normals.as_ref().unwrap();
    assert_eq!(normals.len(), 4);

    // All normals should point in +Z for a flat XY quad
    for n in normals {
        assert!((n[0]).abs() < 1e-6, "nx should be 0, got {}", n[0]);
        assert!((n[1]).abs() < 1e-6, "ny should be 0, got {}", n[1]);
        assert!((n[2] - 1.0).abs() < 1e-6, "nz should be 1, got {}", n[2]);
    }
}

#[test]
fn terrain_mesh_compute_normals_preserves_existing() {
    let mut mesh = make_simple_mesh();
    let existing_normals = vec![[0.0, 0.0, -1.0]; 4];
    mesh.normals = Some(existing_normals.clone());

    mesh.compute_normals();
    // Should not overwrite existing normals
    assert_eq!(mesh.normals.as_ref().unwrap(), &existing_normals);
}

#[test]
fn terrain_mesh_compute_normals_tilted() {
    // A single triangle tilted 45 degrees
    let mut mesh = TerrainMesh {
        positions: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 1.0],
        ],
        normals: None,
        tex_coords: None,
        indices: vec![0, 1, 2],
        minimum_height: 0.0,
        maximum_height: 1.0,
        bounding_sphere: BoundingSphere::new(DVec3::new(0.33, 0.33, 0.33), 1.0),
    };

    mesh.compute_normals();
    let normals = mesh.normals.as_ref().unwrap();
    assert_eq!(normals.len(), 3);

    // All vertices should have the same normal (single triangle)
    let n0 = normals[0];
    for n in normals.iter().skip(1) {
        assert!((n[0] - n0[0]).abs() < 1e-6);
        assert!((n[1] - n0[1]).abs() < 1e-6);
        assert!((n[2] - n0[2]).abs() < 1e-6);
    }

    // Normal should be normalized
    let len = (n0[0] * n0[0] + n0[1] * n0[1] + n0[2] * n0[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-6);
}

#[test]
fn terrain_mesh_heights() {
    let mesh = TerrainMesh {
        positions: vec![
            [0.0, 0.0, 100.0],
            [1.0, 0.0, 200.0],
            [0.0, 1.0, 50.0],
        ],
        normals: None,
        tex_coords: None,
        indices: vec![0, 1, 2],
        minimum_height: 50.0,
        maximum_height: 200.0,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, 1.0),
    };
    assert!((mesh.minimum_height - 50.0).abs() < 1e-10);
    assert!((mesh.maximum_height - 200.0).abs() < 1e-10);
}

#[test]
fn terrain_mesh_empty() {
    let mesh = TerrainMesh {
        positions: vec![],
        normals: None,
        tex_coords: None,
        indices: vec![],
        minimum_height: 0.0,
        maximum_height: 0.0,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, 0.0),
    };
    assert_eq!(mesh.vertex_count(), 0);
    assert_eq!(mesh.triangle_count(), 0);
}

#[test]
fn terrain_mesh_bounding_sphere() {
    let mesh = make_simple_mesh();
    assert!((mesh.bounding_sphere.center - DVec3::new(0.5, 0.5, 0.0)).length() < 1e-10);
    assert!((mesh.bounding_sphere.radius - 1.0).abs() < 1e-10);
}
