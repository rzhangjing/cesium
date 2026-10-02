//! TerrainMesh + QuantizedMesh 扩展测试。
//!
//! 参考 CesiumJS：
//! - Core/TerrainMesh
//! - Core/QuantizedMeshTerrainData（网格创建、法线）
//!
//! A 类测试：网格计算、法线、顶点/三角形数量。

use cesium_terrain::TerrainMesh;
use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

fn make_simple_mesh() -> TerrainMesh {
    // XY 平面中的一个简单四边形（2 个三角形）
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

    // 平坦 XY 四边形的所有法线应指向 +Z
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
    // 不应覆盖已有法线
    assert_eq!(mesh.normals.as_ref().unwrap(), &existing_normals);
}

#[test]
fn terrain_mesh_compute_normals_tilted() {
    // 单个倾斜 45 度的三角形
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

    // 所有顶点应有相同法线（单个三角形）
    let n0 = normals[0];
    for n in normals.iter().skip(1) {
        assert!((n[0] - n0[0]).abs() < 1e-6);
        assert!((n[1] - n0[1]).abs() < 1e-6);
        assert!((n[2] - n0[2]).abs() < 1e-6);
    }

    // 法线应被归一化
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
