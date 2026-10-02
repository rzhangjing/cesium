//! TopoJSON 解码器。
//!
//! 实现基于拓扑的几何编码的 TopoJSON 规范。
//! TopoJSON 以共享弧段与量化增量编码压缩几何，本模块负责解码重建绝对坐标。

use glam::DVec2;

/// 一个 TopoJSON 拓扑对象。
///
/// 拓扑由若干命名几何对象、共享弧段列表与可选的量化变换组成。
#[derive(Debug, Clone, PartialEq)]
pub struct Topology {
    /// 命名的几何对象。
    pub objects: Vec<TopoObject>,
    /// 弧定义（共享边界）。
    pub arcs: Vec<Vec<DVec2>>,
    /// 变换（可选的量化）。
    pub transform: Option<Transform>,
    /// 包围盒 [min_x, min_y, max_x, max_y]。
    pub bbox: Option<[f64; 4]>,
}

/// 量化变换。
///
/// 将量化后的整数网格坐标经仿射变换还原为真实经纬度：p = q * scale + translate。
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// 缩放因子 [sx, sy]。
    pub scale: [f64; 2],
    /// 平移偏移 [tx, ty]。
    pub translate: [f64; 2],
}

impl Transform {
    /// 将变换应用于一个量化坐标。
    pub fn apply(&self, x: f64, y: f64) -> DVec2 {
        // 先按对应轴缩放再叠加平移偏移，得到解量化后的绝对坐标
        DVec2::new(
            x * self.scale[0] + self.translate[0],
            y * self.scale[1] + self.translate[1],
        )
    }
}

/// 一个命名的 TopoJSON 对象。
///
/// 对象将可读名称绑定到一个几何，对应 TopoJSON objects 表中的一条记录。
#[derive(Debug, Clone, PartialEq)]
pub struct TopoObject {
    /// 对象名称。
    pub name: String,
    /// 几何类型。
    pub geometry: TopoGeometry,
}

/// TopoJSON 几何类型。
///
/// 线与面类几何以弧索引（而非显式坐标）引用拓扑中共享的弧段，反向弧以取反索引表示。
#[derive(Debug, Clone, PartialEq)]
pub enum TopoGeometry {
    /// 一个点。
    Point(DVec2),
    /// 多个点。
    MultiPoint(Vec<DVec2>),
    /// 一条线串（弧索引）。
    LineString(Vec<usize>),
    /// 多条线串。
    MultiLineString(Vec<Vec<usize>>),
    /// 一个多边形（弧索引的环）。
    Polygon(Vec<Vec<usize>>),
    /// 多个多边形。
    MultiPolygon(Vec<Vec<Vec<usize>>>),
    /// 一个几何集合。
    GeometryCollection(Vec<TopoGeometry>),
}

/// 将拓扑中的弧解码为绝对坐标。
pub fn decode_arc(topology: &Topology, arc_index: usize) -> Vec<DVec2> {
    // 越界保护：索引超出弧段数量时返回空
    if arc_index >= topology.arcs.len() {
        return Vec::new();
    }

    let arc = &topology.arcs[arc_index];
    let mut result = Vec::with_capacity(arc.len());

    if let Some(transform) = &topology.transform {
        // 带变换的增量编码：逐点累加前一点的偏移得到绝对量化值，再解量化
        let mut x = 0.0f64;
        let mut y = 0.0f64;
        for point in arc {
            // 弧内存储的是相对上一拐点的增量
            x += point.x;
            y += point.y;
            result.push(transform.apply(x, y));
        }
    } else {
        // 无变换时弧段已为绝对坐标，直接拷贝
        result.clone_from(arc);
    }

    result
}

/// 解码一条反向的弧。
pub fn decode_arc_reversed(topology: &Topology, arc_index: usize) -> Vec<DVec2> {
    // 先正向解码再将顶点顺序反转（共享边界时避免重复解码变换）
    let mut arc = decode_arc(topology, arc_index);
    arc.reverse();
    arc
}

/// 将线串从弧索引解析为坐标。
pub fn resolve_linestring(topology: &Topology, arc_indices: &[usize]) -> Vec<DVec2> {
    // 依次解码每段弧并拼接；相邻弧首尾共享一个顶点，拼接时需去重
    let mut coords = Vec::new();
    for (i, &arc_idx) in arc_indices.iter().enumerate() {
        let arc = if arc_idx & (1 << 31) != 0 {
            // 高位为 1 表示反向弧：按位取反得到真实索引后反向解码
            decode_arc_reversed(topology, !arc_idx)
        } else {
            decode_arc(topology, arc_idx)
        };

        // 跳过后续弧的首个点（与上一条共享）
        let start = if i > 0 && !arc.is_empty() { 1 } else { 0 };
        coords.extend_from_slice(&arc[start..]);
    }
    coords
}

/// 将多边形从环弧索引解析为坐标。
pub fn resolve_polygon(topology: &Topology, rings: &[Vec<usize>]) -> Vec<Vec<DVec2>> {
    // 每个环本质是一条闭合线串，复用 resolve_linestring 逐环解码
    rings
        .iter()
        .map(|ring| resolve_linestring(topology, ring))
        .collect()
}

/// 计算一个环的面积（用于确定绕序方向）。
pub fn ring_area(ring: &[DVec2]) -> f64 {
    // 少于三个点无法围成面，面积为 0
    if ring.len() < 3 {
        return 0.0;
    }

    // 鞋带公式（shoelace）：逐边累加叉积，符号反映绕序方向
    let mut area = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        area += ring[i].x * ring[j].y;
        area -= ring[j].x * ring[i].y;
    }
    // 除以 2 得到有符号面积：正为逆时针，负为顺时针
    area / 2.0
}

/// 若环为顺时针（TopoJSON 中的外环）则返回 true。
pub fn is_clockwise(ring: &[DVec2]) -> bool {
    // TopoJSON 约定外环为顺时针，即有符号面积为负
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
        // 弧 0：(0,0), (1,0), (1,1)
        // 弧 1（跳过首点）：(0,1), (0,0)
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
        // 逆时针正方形
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let area = ring_area(&ring);
        assert!((area - 1.0).abs() < 1e-10); // 正值 = 逆时针
    }

    #[test]
    fn test_is_clockwise() {
        // 顺时针正方形
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
