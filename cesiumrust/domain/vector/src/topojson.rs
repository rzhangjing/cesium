//! TopoJSON decoder.
//!
//! Implements TopoJSON specification for topology-based geometry encoding.
//! Maps to CesiumJS `ThirdParty/topojson.js`

use glam::DVec2;

/// A TopoJSON topology object.
#[derive(Debug, Clone, PartialEq)]
pub struct Topology {
    /// Named geometry objects.
    pub objects: Vec<TopoObject>,
    /// Arc definitions (shared boundaries).
    pub arcs: Vec<Vec<DVec2>>,
    /// Transform (optional quantization).
    pub transform: Option<Transform>,
    /// Bounding box [min_x, min_y, max_x, max_y].
    pub bbox: Option<[f64; 4]>,
}

/// Quantization transform.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// Scale factors [sx, sy].
    pub scale: [f64; 2],
    /// Translation offsets [tx, ty].
    pub translate: [f64; 2],
}

impl Transform {
    /// Applies the transform to a quantized coordinate.
    pub fn apply(&self, x: f64, y: f64) -> DVec2 {
        DVec2::new(
            x * self.scale[0] + self.translate[0],
            y * self.scale[1] + self.translate[1],
        )
    }
}

/// A named TopoJSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct TopoObject {
    /// Object name.
    pub name: String,
    /// Geometry type.
    pub geometry: TopoGeometry,
}

/// TopoJSON geometry types.
#[derive(Debug, Clone, PartialEq)]
pub enum TopoGeometry {
    /// A point.
    Point(DVec2),
    /// Multiple points.
    MultiPoint(Vec<DVec2>),
    /// A line string (arc indices).
    LineString(Vec<usize>),
    /// Multiple line strings.
    MultiLineString(Vec<Vec<usize>>),
    /// A polygon (rings of arc indices).
    Polygon(Vec<Vec<usize>>),
    /// Multiple polygons.
    MultiPolygon(Vec<Vec<Vec<usize>>>),
    /// A geometry collection.
    GeometryCollection(Vec<TopoGeometry>),
}

/// Decodes arcs from a topology into absolute coordinates.
pub fn decode_arc(topology: &Topology, arc_index: usize) -> Vec<DVec2> {
    if arc_index >= topology.arcs.len() {
        return Vec::new();
    }

    let arc = &topology.arcs[arc_index];
    let mut result = Vec::with_capacity(arc.len());

    if let Some(transform) = &topology.transform {
        // Delta-encoded with transform
        let mut x = 0.0f64;
        let mut y = 0.0f64;
        for point in arc {
            x += point.x;
            y += point.y;
            result.push(transform.apply(x, y));
        }
    } else {
        result.clone_from(arc);
    }

    result
}

/// Decodes a reversed arc.
pub fn decode_arc_reversed(topology: &Topology, arc_index: usize) -> Vec<DVec2> {
    let mut arc = decode_arc(topology, arc_index);
    arc.reverse();
    arc
}

/// Resolves a line string from arc indices to coordinates.
pub fn resolve_linestring(topology: &Topology, arc_indices: &[usize]) -> Vec<DVec2> {
    let mut coords = Vec::new();
    for (i, &arc_idx) in arc_indices.iter().enumerate() {
        let arc = if arc_idx & (1 << 31) != 0 {
            // Reversed arc (bitwise complement)
            decode_arc_reversed(topology, !arc_idx)
        } else {
            decode_arc(topology, arc_idx)
        };

        // Skip first point of subsequent arcs (shared with previous)
        let start = if i > 0 && !arc.is_empty() { 1 } else { 0 };
        coords.extend_from_slice(&arc[start..]);
    }
    coords
}

/// Resolves a polygon from ring arc indices to coordinates.
pub fn resolve_polygon(topology: &Topology, rings: &[Vec<usize>]) -> Vec<Vec<DVec2>> {
    rings
        .iter()
        .map(|ring| resolve_linestring(topology, ring))
        .collect()
}

/// Computes the area of a ring (for determining winding order).
pub fn ring_area(ring: &[DVec2]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        area += ring[i].x * ring[j].y;
        area -= ring[j].x * ring[i].y;
    }
    area / 2.0
}

/// Returns true if the ring is clockwise (exterior ring in TopoJSON).
pub fn is_clockwise(ring: &[DVec2]) -> bool {
    ring_area(ring) < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_topology() -> Topology {
        Topology {
            objects: vec![],
            arcs: vec![
                vec![DVec2::new(0.0, 0.0), DVec2::new(1.0, 0.0), DVec2::new(1.0, 1.0)],
                vec![DVec2::new(1.0, 1.0), DVec2::new(0.0, 1.0), DVec2::new(0.0, 0.0)],
            ],
            transform: None,
            bbox: Some([0.0, 0.0, 1.0, 1.0]),
        }
    }

    #[test]
    fn test_decode_arc() {
        let topo = create_test_topology();
        let arc = decode_arc(&topo, 0);
        assert_eq!(arc.len(), 3);
        assert_eq!(arc[0], DVec2::new(0.0, 0.0));
        assert_eq!(arc[2], DVec2::new(1.0, 1.0));
    }

    #[test]
    fn test_decode_arc_reversed() {
        let topo = create_test_topology();
        let arc = decode_arc_reversed(&topo, 0);
        assert_eq!(arc.len(), 3);
        assert_eq!(arc[0], DVec2::new(1.0, 1.0));
        assert_eq!(arc[2], DVec2::new(0.0, 0.0));
    }

    #[test]
    fn test_decode_arc_out_of_bounds() {
        let topo = create_test_topology();
        let arc = decode_arc(&topo, 99);
        assert!(arc.is_empty());
    }

    #[test]
    fn test_resolve_linestring() {
        let topo = create_test_topology();
        let coords = resolve_linestring(&topo, &[0, 1]);
        // Arc 0: (0,0), (1,0), (1,1)
        // Arc 1 (skip first): (0,1), (0,0)
        assert_eq!(coords.len(), 5);
        assert_eq!(coords[0], DVec2::new(0.0, 0.0));
        assert_eq!(coords[4], DVec2::new(0.0, 0.0));
    }

    #[test]
    fn test_resolve_polygon() {
        let topo = create_test_topology();
        let rings = resolve_polygon(&topo, &[vec![0, 1]]);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 5);
    }

    #[test]
    fn test_transform() {
        let transform = Transform {
            scale: [0.001, 0.001],
            translate: [100.0, 50.0],
        };
        let result = transform.apply(1000.0, 2000.0);
        assert!((result.x - 101.0).abs() < 1e-10);
        assert!((result.y - 52.0).abs() < 1e-10);
    }

    #[test]
    fn test_decode_arc_with_transform() {
        let topo = Topology {
            objects: vec![],
            arcs: vec![
                vec![DVec2::new(0.0, 0.0), DVec2::new(1000.0, 0.0), DVec2::new(0.0, 1000.0)],
            ],
            transform: Some(Transform {
                scale: [0.001, 0.001],
                translate: [100.0, 50.0],
            }),
            bbox: None,
        };

        let arc = decode_arc(&topo, 0);
        assert_eq!(arc.len(), 3);
        assert!((arc[0].x - 100.0).abs() < 1e-10);
        assert!((arc[0].y - 50.0).abs() < 1e-10);
        assert!((arc[1].x - 101.0).abs() < 1e-10);
        assert!((arc[2].y - 51.0).abs() < 1e-10);
    }

    #[test]
    fn test_ring_area() {
        // Counter-clockwise square
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let area = ring_area(&ring);
        assert!((area - 1.0).abs() < 1e-10); // Positive = CCW
    }

    #[test]
    fn test_is_clockwise() {
        // Clockwise square
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(0.0, 1.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(1.0, 0.0),
        ];
        assert!(is_clockwise(&ring));
    }

    #[test]
    fn test_topo_geometry_types() {
        let point = TopoGeometry::Point(DVec2::new(1.0, 2.0));
        assert!(matches!(point, TopoGeometry::Point(_)));

        let collection = TopoGeometry::GeometryCollection(vec![
            TopoGeometry::Point(DVec2::ZERO),
            TopoGeometry::LineString(vec![0, 1]),
        ]);
        if let TopoGeometry::GeometryCollection(geoms) = collection {
            assert_eq!(geoms.len(), 2);
        }
    }
}
