//! Vector 3D Tile 内容类型。
//!
//! 建模 3D Tiles 中的矢量瓦片内容，覆盖点（Points）、折线（Polylines）
//! 与多边形（Polygons）三类基本要素。每类要素以展平数组存储顶点位置、
//! 三角化索引、批次 ID 与样式属性（颜色、高度、是否贴地），并提供
//! 按要素切片的访问接口、数量查询与几何字节长度估算。

use glam::DVec3;

// ============================================================================
// Vector3DTileType
// ============================================================================

/// 3D Tile 中矢量几何的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vector3DTileType {
    /// 点要素。
    Points,
    /// 折线要素。
    Polylines,
    /// 多边形要素。
    Polygons,
}

// ============================================================================
// Vector3DTilePoints
// ============================================================================

/// 矢量 3D 瓦片中的点要素。
///
/// 位置、批次 ID、颜色与大小按同一索引对齐，构成并行的属性数组。
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePoints {
    /// 点的位置（世界坐标）。
    pub positions: Vec<DVec3>,
    /// 每个点的批次 ID（映射到批次表）。
    pub batch_ids: Vec<u32>,
    /// 点颜色（RGBA，0-1）。
    pub colors: Vec<[f64; 4]>,
    /// 点大小（像素）。
    pub sizes: Vec<f64>,
    /// 点是否贴地。
    pub clamp_to_ground: bool,
}

impl Vector3DTilePoints {
    /// 创建空的点集合。
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            batch_ids: Vec::new(),
            colors: Vec::new(),
            sizes: Vec::new(),
            clamp_to_ground: false,
        }
    }

    /// 获取点的数量。
    pub fn points_length(&self) -> usize {
        self.positions.len()
    }

    /// 获取几何数据的字节长度。
    pub fn geometry_byte_length(&self) -> usize {
        // 每个位置 3 个 f64 + 每个 batch_id 1 个 u32
        self.positions.len() * 24 + self.batch_ids.len() * 4
    }

    /// 添加一个点。
    pub fn add_point(&mut self, position: DVec3, batch_id: u32) {
        // 位置与批次 ID 同步追加，保持并行数组索引对齐
        self.positions.push(position);
        self.batch_ids.push(batch_id);
    }
}

impl Default for Vector3DTilePoints {
    /// 默认点集合为空，等价于 [`Vector3DTilePoints::new`]。
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTilePolylines
// ============================================================================

/// 矢量 3D 瓦片中的折线要素。
///
/// 多条折线共享一个展平的 positions 数组，以 polyline_starts/counts 定位各线。
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePolylines {
    /// 所有折线的位置（展平存储）。
    pub positions: Vec<DVec3>,
    /// 每条折线在 positions 中的起始索引。
    pub polyline_starts: Vec<usize>,
    /// 每条折线的顶点数。
    pub polyline_counts: Vec<usize>,
    /// 每条折线的批次 ID。
    pub batch_ids: Vec<u32>,
    /// 折线宽度（米）。
    pub widths: Vec<f64>,
    /// 折线颜色（RGBA）。
    pub colors: Vec<[f64; 4]>,
    /// 折线是否贴地。
    pub clamp_to_ground: bool,
}

impl Vector3DTilePolylines {
    /// 创建空的折线集合。
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            polyline_starts: Vec::new(),
            polyline_counts: Vec::new(),
            batch_ids: Vec::new(),
            widths: Vec::new(),
            colors: Vec::new(),
            clamp_to_ground: false,
        }
    }

    /// 获取折线的数量。
    pub fn polylines_length(&self) -> usize {
        self.polyline_starts.len()
    }

    /// 获取三角形数量（以四边形渲染）。
    pub fn triangles_length(&self) -> usize {
        // 每段变为 2 个三角形
        let segments: usize = self.polyline_counts.iter().map(|c| c.saturating_sub(1)).sum();
        segments * 2
    }

    /// 获取几何数据的字节长度。
    pub fn geometry_byte_length(&self) -> usize {
        self.positions.len() * 24
    }

    /// 添加一条折线。
    pub fn add_polyline(&mut self, positions: &[DVec3], batch_id: u32, width: f64) {
        // 记录当前末尾作为本线起点，追加顶点后同步 starts/counts/widths
        let start = self.positions.len();
        self.positions.extend_from_slice(positions);
        self.polyline_starts.push(start);
        self.polyline_counts.push(positions.len());
        self.batch_ids.push(batch_id);
        self.widths.push(width);
    }

    /// 获取指定折线的位置。
    pub fn get_polyline(&self, index: usize) -> Option<&[DVec3]> {
        // 越界安全：索引超出线数时返回 None
        if index >= self.polyline_starts.len() {
            return None;
        }
        let start = self.polyline_starts[index];
        let count = self.polyline_counts[index];
        Some(&self.positions[start..start + count])
    }
}

impl Default for Vector3DTilePolylines {
    /// 默认折线集合为空，等价于 [`Vector3DTilePolylines::new`]。
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTilePolygons
// ============================================================================

/// 矢量 3D 瓦片中的多边形要素。
///
/// 多边形已预先三角化，索引按顶点偏移量写入共享的 indices 数组。
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePolygons {
    /// 所有多边形的位置（展平存储）。
    pub positions: Vec<DVec3>,
    /// 多边形索引（已三角化）。
    pub indices: Vec<u32>,
    /// 每个多边形索引的起始索引。
    pub polygon_index_starts: Vec<usize>,
    /// 每个多边形的索引数量。
    pub polygon_index_counts: Vec<usize>,
    /// 每个多边形的批次 ID。
    pub batch_ids: Vec<u32>,
    /// 多边形颜色（RGBA）。
    pub colors: Vec<[f64; 4]>,
    /// 多边形高度（用于拉伸）。
    pub heights: Vec<f64>,
    /// 多边形拉伸高度。
    pub extruded_heights: Vec<f64>,
    /// 多边形是否贴地。
    pub clamp_to_ground: bool,
}

impl Vector3DTilePolygons {
    /// 创建空的多边形集合。
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            indices: Vec::new(),
            polygon_index_starts: Vec::new(),
            polygon_index_counts: Vec::new(),
            batch_ids: Vec::new(),
            colors: Vec::new(),
            heights: Vec::new(),
            extruded_heights: Vec::new(),
            clamp_to_ground: false,
        }
    }

    /// 获取多边形的数量。
    pub fn polygons_length(&self) -> usize {
        self.polygon_index_starts.len()
    }

    /// 获取三角形的数量。
    pub fn triangles_length(&self) -> usize {
        self.indices.len() / 3
    }

    /// 获取几何数据的字节长度。
    pub fn geometry_byte_length(&self) -> usize {
        self.positions.len() * 24 + self.indices.len() * 4
    }

    /// 添加一个带三角化索引的多边形。
    pub fn add_polygon(
        &mut self,
        positions: &[DVec3],
        indices: &[u32],
        batch_id: u32,
        height: f64,
        extruded_height: f64,
    ) {
        let vertex_offset = self.positions.len() as u32;
        self.positions.extend_from_slice(positions);

        let index_start = self.indices.len();
        // 按顶点偏移量调整索引
        self.indices.extend(indices.iter().map(|i| i + vertex_offset));

        self.polygon_index_starts.push(index_start);
        self.polygon_index_counts.push(indices.len());
        self.batch_ids.push(batch_id);
        self.heights.push(height);
        self.extruded_heights.push(extruded_height);
    }
}

impl Default for Vector3DTilePolygons {
    /// 默认多边形集合为空，等价于 [`Vector3DTilePolygons::new`]。
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTileContent
// ============================================================================

/// 完整的矢量 3D 瓦片内容。
///
/// 聚合点/线/面三类要素（均为可选）与 MVT 图层列表，作为瓦片的统一载体。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vector3DTileContent {
    /// 点要素。
    pub points: Option<Vector3DTilePoints>,
    /// 折线要素。
    pub polylines: Option<Vector3DTilePolylines>,
    /// 多边形要素。
    pub polygons: Option<Vector3DTilePolygons>,
    /// 来自批次表的要素数量。
    pub features_length: usize,
}

impl Vector3DTileContent {
    /// 创建空的矢量瓦片内容。
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取点的总数。
    pub fn points_length(&self) -> usize {
        self.points.as_ref().map_or(0, |p| p.points_length())
    }

    /// 获取三角形的总数。
    pub fn triangles_length(&self) -> usize {
        let mut count = 0;
        if let Some(ref polys) = self.polygons {
            count += polys.triangles_length();
        }
        if let Some(ref lines) = self.polylines {
            count += lines.triangles_length();
        }
        count
    }

    /// 获取几何数据的总字节长度。
    pub fn geometry_byte_length(&self) -> usize {
        let mut bytes = 0;
        if let Some(ref pts) = self.points {
            bytes += pts.geometry_byte_length();
        }
        if let Some(ref polys) = self.polygons {
            bytes += polys.geometry_byte_length();
        }
        if let Some(ref lines) = self.polylines {
            bytes += lines.geometry_byte_length();
        }
        bytes
    }

    /// 获取当前存在的内容类型。
    pub fn content_types(&self) -> Vec<Vector3DTileType> {
        let mut types = Vec::new();
        if self.points.is_some() {
            types.push(Vector3DTileType::Points);
        }
        if self.polylines.is_some() {
            types.push(Vector3DTileType::Polylines);
        }
        if self.polygons.is_some() {
            types.push(Vector3DTileType::Polygons);
        }
        types
    }
}

// ============================================================================
// MVT（Mapbox Vector Tile）支持
// ============================================================================

/// MVT 几何类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvtGeometryType {
    /// 未知几何。
    Unknown,
    /// 点几何。
    Point,
    /// 线串（LineString）几何。
    LineString,
    /// 多边形几何。
    Polygon,
}

/// 一个 MVT 图层。
#[derive(Debug, Clone, PartialEq)]
pub struct MvtLayer {
    /// 图层名称。
    pub name: String,
    /// 图层版本。
    pub version: u32,
    /// 瓦片 extent（通常为 4096）。
    pub extent: u32,
    /// 本图层中的要素。
    pub features: Vec<MvtFeature>,
    /// 键（属性名）。
    pub keys: Vec<String>,
    /// 值（属性值）。
    pub values: Vec<MvtValue>,
}

impl MvtLayer {
    /// 创建一个新的 MVT 图层。
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: 2,
            extent: 4096,
            features: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
        }
    }
}

/// 一个 MVT 要素。
#[derive(Debug, Clone, PartialEq)]
pub struct MvtFeature {
    /// 要素 ID。
    pub id: Option<u64>,
    /// 几何类型。
    pub geometry_type: MvtGeometryType,
    /// 几何命令（已编码）。
    pub geometry: Vec<u32>,
    /// 属性标签（key_idx、value_idx 成对）。
    pub tags: Vec<u32>,
}

impl MvtFeature {
    /// 创建一个新的要素。
    pub fn new(geometry_type: MvtGeometryType) -> Self {
        Self {
            id: None,
            geometry_type,
            geometry: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// MVT 属性值。
#[derive(Debug, Clone, PartialEq)]
pub enum MvtValue {
    /// 字符串值。
    String(String),
    /// 单精度浮点值。
    Float(f64),
    /// 双精度浮点值。
    Double(f64),
    /// 整数值。
    Int(i64),
    /// 无符号整数值。
    Uint(u64),
    /// 有符号整数值。
    Sint(i64),
    /// 布尔值。
    Bool(bool),
}

/// 将 MVT 几何命令解码为位置。
///
/// MVT 使用基于命令的编码：
/// - MoveTo（command_id = 1）
/// - LineTo（command_id = 2）
/// - ClosePath（command_id = 7）
pub fn decode_mvt_geometry(commands: &[u32], extent: u32) -> Vec<Vec<DVec3>> {
    let mut rings: Vec<Vec<DVec3>> = Vec::new();
    let mut current_ring: Vec<DVec3> = Vec::new();
    let mut cursor_x: i32 = 0;
    let mut cursor_y: i32 = 0;
    let mut i = 0;

    while i < commands.len() {
        let command = commands[i];
        let command_id = command & 0x7;
        let count = (command >> 3) as usize;
        i += 1;

        match command_id {
            1 => {
                // MoveTo
                for _ in 0..count {
                    if i + 1 >= commands.len() {
                        break;
                    }
                    let dx = zigzag_decode(commands[i]);
                    let dy = zigzag_decode(commands[i + 1]);
                    cursor_x += dx;
                    cursor_y += dy;
                    i += 2;

                    if !current_ring.is_empty() {
                        rings.push(std::mem::take(&mut current_ring));
                    }
                    current_ring.push(DVec3::new(
                        cursor_x as f64 / extent as f64,
                        cursor_y as f64 / extent as f64,
                        0.0,
                    ));
                }
            }
            2 => {
                // LineTo
                for _ in 0..count {
                    if i + 1 >= commands.len() {
                        break;
                    }
                    let dx = zigzag_decode(commands[i]);
                    let dy = zigzag_decode(commands[i + 1]);
                    cursor_x += dx;
                    cursor_y += dy;
                    i += 2;

                    current_ring.push(DVec3::new(
                        cursor_x as f64 / extent as f64,
                        cursor_y as f64 / extent as f64,
                        0.0,
                    ));
                }
            }
            7 if !current_ring.is_empty() => {
                // ClosePath
                let first = current_ring[0];
                current_ring.push(first);
                rings.push(std::mem::take(&mut current_ring));
            }
            7 => {}
            _ => {}
        }
    }

    if !current_ring.is_empty() {
        rings.push(current_ring);
    }

    rings
}

/// 将无符号整数进行 zigzag 解码为有符号。
fn zigzag_decode(n: u32) -> i32 {
    ((n >> 1) as i32) ^ (-((n & 1) as i32))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_points() {
        let mut points = Vector3DTilePoints::new();
        points.add_point(DVec3::new(1.0, 2.0, 3.0), 0);
        points.add_point(DVec3::new(4.0, 5.0, 6.0), 1);

        assert_eq!(points.points_length(), 2);
        assert!(points.geometry_byte_length() > 0);
    }

    #[test]
    fn test_vector_polylines() {
        let mut polylines = Vector3DTilePolylines::new();
        polylines.add_polyline(
            &[DVec3::ZERO, DVec3::ONE, DVec3::new(2.0, 0.0, 0.0)],
            0,
            2.0,
        );
        polylines.add_polyline(&[DVec3::ZERO, DVec3::new(0.0, 1.0, 0.0)], 1, 1.0);

        assert_eq!(polylines.polylines_length(), 2);
        assert_eq!(polylines.triangles_length(), 6); // (2 + 1) 段 * 2 三角形
        assert_eq!(polylines.get_polyline(0).unwrap().len(), 3);
        assert_eq!(polylines.get_polyline(1).unwrap().len(), 2);
        assert!(polylines.get_polyline(5).is_none());
    }

    #[test]
    fn test_vector_polygons() {
        let mut polygons = Vector3DTilePolygons::new();
        // 三角形
        polygons.add_polygon(
            &[
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.5, 1.0, 0.0),
            ],
            &[0, 1, 2],
            0,
            0.0,
            10.0,
        );

        assert_eq!(polygons.polygons_length(), 1);
        assert_eq!(polygons.triangles_length(), 1);
        assert!(polygons.geometry_byte_length() > 0);
    }

    #[test]
    fn test_vector_tile_content() {
        let mut content = Vector3DTileContent::new();
        assert!(content.content_types().is_empty());

        content.points = Some(Vector3DTilePoints::new());
        content.polygons = Some(Vector3DTilePolygons::new());

        let types = content.content_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&Vector3DTileType::Points));
        assert!(types.contains(&Vector3DTileType::Polygons));
    }

    #[test]
    fn test_mvt_layer() {
        let mut layer = MvtLayer::new("buildings");
        assert_eq!(layer.name, "buildings");
        assert_eq!(layer.version, 2);
        assert_eq!(layer.extent, 4096);

        let mut feature = MvtFeature::new(MvtGeometryType::Polygon);
        feature.id = Some(42);
        layer.features.push(feature);

        assert_eq!(layer.features.len(), 1);
    }

    #[test]
    fn test_zigzag_decode() {
        assert_eq!(zigzag_decode(0), 0);
        assert_eq!(zigzag_decode(1), -1);
        assert_eq!(zigzag_decode(2), 1);
        assert_eq!(zigzag_decode(3), -2);
        assert_eq!(zigzag_decode(4), 2);
    }

    #[test]
    fn test_decode_mvt_geometry_point() {
        // MoveTo(1) with count=1, then parameters (25, 17) -> zigzag(12, 8)
        let commands = vec![
            (1 << 3) | 1, // MoveTo, count=1
            24,           // zigzag(12)
            16,           // zigzag(8)
        ];
        let rings = decode_mvt_geometry(&commands, 4096);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 1);
        assert!((rings[0][0].x - 12.0 / 4096.0).abs() < 1e-10);
        assert!((rings[0][0].y - 8.0 / 4096.0).abs() < 1e-10);
    }

    #[test]
    fn test_decode_mvt_geometry_linestring() {
        // MoveTo(1) count=1, LineTo(2) count=2
        let commands = vec![
            (1 << 3) | 1, // MoveTo, count=1
            2,            // zigzag(1)
            2,            // zigzag(1)
            (2 << 3) | 2, // LineTo, count=2
            4,            // zigzag(2)
            0,            // zigzag(0)
            0,            // zigzag(0)
            4,            // zigzag(2)
        ];
        let rings = decode_mvt_geometry(&commands, 4096);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 3); // 1 个 moveto + 2 个 lineto
    }

    #[test]
    fn test_decode_mvt_geometry_polygon() {
        // MoveTo(1), LineTo(2), ClosePath(7)
        let commands = vec![
            (1 << 3) | 1, // MoveTo, count=1
            0,            // x=0
            0,            // y=0
            (2 << 3) | 2, // LineTo, count=2
            2,            // dx=1
            0,            // dy=0
            0,            // dx=0
            2,            // dy=1
            15,           // ClosePath (7 | (1 << 3))
        ];
        let rings = decode_mvt_geometry(&commands, 4096);
        assert_eq!(rings.len(), 1);
        // 应为闭合（first == last）
        assert_eq!(rings[0].first(), rings[0].last());
    }
}
