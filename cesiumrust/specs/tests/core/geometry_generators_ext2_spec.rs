//! Ported from CesiumJS geometry specs (Polyline/Circle/CoplanarPolygon/PolylineVolume).
//!
//! Expanded A-class tests for geometry generators with existing Rust implementations.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{
    coplanar_polygon_geometry, ellipse_geometry, polyline_geometry, polyline_volume_geometry,
    CoplanarPolygonOptions, EllipseOptions, PolylineOptions, PolylineVolumeOptions, VertexFormat,
};
use glam::DVec3;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

fn from_degrees(lon: f64, lat: f64, h: f64) -> DVec3 {
    let e = wgs84();
    let carto = Cartographic::from_degrees(lon, lat, h);
    e.cartographic_to_cartesian(&carto)
}

// ===========================================================================
// PolylineGeometry
// ===========================================================================

#[test]
fn polyline_computes_positions_ribbon() {
    let opts = PolylineOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
            from_degrees(2.0, 0.0, 0.0),
        ],
        width: 10.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);

    // Ribbon: 2 vertices per arc point (left + right)
    assert!(
        geo.positions.len() >= 4,
        "polyline should produce >= 4 positions, got {}",
        geo.positions.len()
    );
    assert_eq!(geo.positions.len() % 2, 0, "positions should be in pairs");
    assert_eq!(geo.indices.len() % 3, 0, "indices must form triangles");
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn polyline_all_vertex_attributes() {
    let opts = PolylineOptions {
        positions: vec![
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        ],
        width: 10.0,
        granularity: std::f64::consts::PI,
        ellipsoid: Ellipsoid::UNIT_SPHERE,
    };
    let geo = polyline_geometry(&opts, VertexFormat::ALL);

    let num_verts = geo.positions.len();
    assert!(num_verts >= 4);
    assert!(geo.normals.is_some(), "normals should be present");
    assert!(geo.tex_coords.is_some(), "tex_coords should be present");

    let normals = geo.normals.as_ref().unwrap();
    let st = geo.tex_coords.as_ref().unwrap();
    assert_eq!(normals.len(), num_verts);
    assert_eq!(st.len(), num_verts);
}

#[test]
fn polyline_returns_empty_for_invalid_input() {
    // Less than 2 positions
    let opts1 = PolylineOptions {
        positions: vec![DVec3::new(1.0, 0.0, 0.0)],
        width: 10.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo1 = polyline_geometry(&opts1, VertexFormat::POSITION_ONLY);
    assert!(geo1.positions.is_empty(), "<2 positions should return empty");

    // Width <= 0
    let opts2 = PolylineOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        width: -1.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo2 = polyline_geometry(&opts2, VertexFormat::POSITION_ONLY);
    assert!(geo2.positions.is_empty(), "width<0 should return empty");

    // Width = 0
    let opts3 = PolylineOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        width: 0.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo3 = polyline_geometry(&opts3, VertexFormat::POSITION_ONLY);
    assert!(geo3.positions.is_empty(), "width=0 should return empty");
}

#[test]
fn polyline_texture_coordinates_pattern() {
    let opts = PolylineOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        width: 100.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo = polyline_geometry(&opts, VertexFormat::POSITION_AND_ST);

    let st = geo.tex_coords.as_ref().expect("tex_coords should be present");
    let n = geo.positions.len();
    assert_eq!(st.len(), n);

    // ST u should go from 0 to 1 along the polyline
    // First pair should have u=0, last pair should have u=1
    assert!((st[0][0] - 0.0).abs() < 1e-6, "first u should be 0");
    assert!(
        (st[n - 2][0] - 1.0).abs() < 1e-6,
        "last u should be 1, got {}",
        st[n - 2][0]
    );
}

#[test]
fn polyline_normals_point_outward() {
    let opts = PolylineOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        width: 100.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo = polyline_geometry(&opts, VertexFormat::ALL);

    let normals = geo.normals.as_ref().expect("normals should be present");
    for (i, (p, n)) in geo.positions.iter().zip(normals.iter()).enumerate() {
        let dot = p[0] * n[0] + p[1] * n[1] + p[2] * n[2];
        assert!(dot > 0.0, "normal[{}] should point outward (dot={})", i, dot);
    }
}

// ===========================================================================
// CircleGeometry (Ellipse with equal axes)
// ===========================================================================

#[test]
fn circle_computes_positions_exact() {
    // CesiumJS CircleGeometry with radius=1, granularity=0.1:
    // 16 vertices (rows 1+4+6+4+1), 22 triangles, boundingSphere.radius=1
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 16, "circle should have 16 positions");
    assert_eq!(geo.indices.len(), 66, "circle should have 66 indices (22 triangles)");
    assert!(
        (geo.bounding_sphere.radius - 1.0).abs() < 1e-10,
        "bounding sphere radius should be 1"
    );
}

#[test]
fn circle_all_vertex_attributes() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::ALL);

    let num_verts = 16;
    assert_eq!(geo.positions.len(), num_verts);
    assert!(geo.normals.is_some());
    assert!(geo.tex_coords.is_some());
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_verts);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_verts);
}

#[test]
fn circle_larger_radius_larger_bounding_sphere() {
    let opts_small = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 100000.0,
        semi_minor_axis: 100000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_small = ellipse_geometry(&opts_small, VertexFormat::POSITION_ONLY);

    let opts_large = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 500000.0,
        semi_minor_axis: 500000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_large = ellipse_geometry(&opts_large, VertexFormat::POSITION_ONLY);

    assert!(
        geo_large.bounding_sphere.radius > geo_small.bounding_sphere.radius,
        "larger circle should have larger bounding sphere"
    );
}

// ===========================================================================
// CoplanarPolygonGeometry
// ===========================================================================

#[test]
fn coplanar_polygon_computes_positions() {
    // A simple quad on the ellipsoid surface
    let opts = CoplanarPolygonOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
            from_degrees(1.0, 1.0, 0.0),
            from_degrees(0.0, 1.0, 0.0),
        ],
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(
        geo.positions.len() >= 4,
        "coplanar polygon should have >= 4 positions, got {}",
        geo.positions.len()
    );
    assert_eq!(geo.indices.len() % 3, 0, "indices must form triangles");
    // A quad triangulates to 2 triangles = 6 indices
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn coplanar_polygon_all_attributes() {
    let opts = CoplanarPolygonOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
            from_degrees(0.5, 1.0, 0.0),
        ],
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::ALL);

    let num_verts = geo.positions.len();
    assert!(num_verts >= 3);
    assert!(geo.normals.is_some());
    assert!(geo.tex_coords.is_some());
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_verts);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_verts);
}

#[test]
fn coplanar_polygon_returns_empty_for_less_than_3_positions() {
    let opts = CoplanarPolygonOptions {
        positions: vec![from_degrees(0.0, 0.0, 0.0), from_degrees(1.0, 0.0, 0.0)],
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.is_empty(), "<3 positions should return empty");
}

#[test]
fn coplanar_polygon_triangle_single_face() {
    // A triangle should produce exactly 1 triangle (3 indices)
    let opts = CoplanarPolygonOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
            from_degrees(0.5, 1.0, 0.0),
        ],
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 3, "triangle should have 3 positions");
    assert_eq!(geo.indices.len(), 3, "triangle should have 3 indices (1 triangle)");
}

// ===========================================================================
// PolylineVolumeGeometry
// ===========================================================================

#[test]
fn polyline_volume_computes_positions() {
    // Square cross-section
    let shape = vec![
        [-50.0, -50.0],
        [50.0, -50.0],
        [50.0, 50.0],
        [-50.0, 50.0],
    ];
    let opts = PolylineVolumeOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        shape,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_ONLY);

    // n arc points × 4 shape vertices
    assert!(
        geo.positions.len() >= 8,
        "polyline volume should have >= 8 positions, got {}",
        geo.positions.len()
    );
    assert_eq!(geo.indices.len() % 3, 0);
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn polyline_volume_all_attributes() {
    let shape = vec![
        [-100.0, 0.0],
        [100.0, 0.0],
        [100.0, 200.0],
        [-100.0, 200.0],
    ];
    let opts = PolylineVolumeOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        shape,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo = polyline_volume_geometry(&opts, VertexFormat::ALL);

    let num_verts = geo.positions.len();
    assert!(num_verts >= 8);
    assert!(geo.normals.is_some());
    assert!(geo.tex_coords.is_some());
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_verts);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_verts);
}

#[test]
fn polyline_volume_returns_empty_for_invalid_input() {
    let shape = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

    // Less than 2 positions
    let opts1 = PolylineVolumeOptions {
        positions: vec![from_degrees(0.0, 0.0, 0.0)],
        shape: shape.clone(),
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo1 = polyline_volume_geometry(&opts1, VertexFormat::POSITION_ONLY);
    assert!(geo1.positions.is_empty(), "<2 positions should return empty");

    // Less than 3 shape points
    let opts2 = PolylineVolumeOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        shape: vec![[0.0, 0.0], [1.0, 0.0]],
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo2 = polyline_volume_geometry(&opts2, VertexFormat::POSITION_ONLY);
    assert!(geo2.positions.is_empty(), "<3 shape points should return empty");
}

#[test]
fn polyline_volume_vertex_count_scales_with_arc() {
    let shape = vec![
        [-50.0, -50.0],
        [50.0, -50.0],
        [50.0, 50.0],
        [-50.0, 50.0],
    ];

    // Short arc (1 degree)
    let opts_short = PolylineVolumeOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(1.0, 0.0, 0.0),
        ],
        shape: shape.clone(),
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo_short = polyline_volume_geometry(&opts_short, VertexFormat::POSITION_ONLY);

    // Longer arc (5 degrees)
    let opts_long = PolylineVolumeOptions {
        positions: vec![
            from_degrees(0.0, 0.0, 0.0),
            from_degrees(5.0, 0.0, 0.0),
        ],
        shape,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
    };
    let geo_long = polyline_volume_geometry(&opts_long, VertexFormat::POSITION_ONLY);

    assert!(
        geo_long.positions.len() > geo_short.positions.len(),
        "longer arc ({}) should produce more vertices than shorter arc ({})",
        geo_long.positions.len(),
        geo_short.positions.len()
    );
}
