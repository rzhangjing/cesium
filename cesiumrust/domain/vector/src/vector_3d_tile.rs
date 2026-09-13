//! Vector 3D Tile content types.
//!
//! Maps to CesiumJS:
//! - `Scene/Vector3DTileContent.js`
//! - `Scene/Vector3DTilePoints.js`
//! - `Scene/Vector3DTilePolylines.js`
//! - `Scene/Vector3DTilePolygons.js`
//! - `Scene/Vector3DTileClampedPolylines.js`

use glam::DVec3;

// ============================================================================
// Vector3DTileType
// ============================================================================

/// Type of vector geometry in a 3D Tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vector3DTileType {
    /// Point features.
    Points,
    /// Polyline features.
    Polylines,
    /// Polygon features.
    Polygons,
}

// ============================================================================
// Vector3DTilePoints
// ============================================================================

/// Point features in a vector 3D tile.
///
/// Maps to CesiumJS `Scene/Vector3DTilePoints.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePoints {
    /// Positions of points (world coordinates).
    pub positions: Vec<DVec3>,
    /// Batch IDs for each point (maps to batch table).
    pub batch_ids: Vec<u32>,
    /// Point colors (RGBA, 0-1).
    pub colors: Vec<[f64; 4]>,
    /// Point sizes in pixels.
    pub sizes: Vec<f64>,
    /// Whether points are clamped to ground.
    pub clamp_to_ground: bool,
}

impl Vector3DTilePoints {
    /// Create empty points.
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            batch_ids: Vec::new(),
            colors: Vec::new(),
            sizes: Vec::new(),
            clamp_to_ground: false,
        }
    }

    /// Get the number of points.
    pub fn points_length(&self) -> usize {
        self.positions.len()
    }

    /// Get the byte length of geometry data.
    pub fn geometry_byte_length(&self) -> usize {
        // 3 f64 per position + 1 u32 per batch_id
        self.positions.len() * 24 + self.batch_ids.len() * 4
    }

    /// Add a point.
    pub fn add_point(&mut self, position: DVec3, batch_id: u32) {
        self.positions.push(position);
        self.batch_ids.push(batch_id);
    }
}

impl Default for Vector3DTilePoints {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTilePolylines
// ============================================================================

/// Polyline features in a vector 3D tile.
///
/// Maps to CesiumJS `Scene/Vector3DTilePolylines.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePolylines {
    /// Positions for all polylines (flattened).
    pub positions: Vec<DVec3>,
    /// Start index of each polyline in positions.
    pub polyline_starts: Vec<usize>,
    /// Number of vertices in each polyline.
    pub polyline_counts: Vec<usize>,
    /// Batch IDs for each polyline.
    pub batch_ids: Vec<u32>,
    /// Polyline widths in meters.
    pub widths: Vec<f64>,
    /// Polyline colors (RGBA).
    pub colors: Vec<[f64; 4]>,
    /// Whether polylines are clamped to ground.
    pub clamp_to_ground: bool,
}

impl Vector3DTilePolylines {
    /// Create empty polylines.
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

    /// Get the number of polylines.
    pub fn polylines_length(&self) -> usize {
        self.polyline_starts.len()
    }

    /// Get the number of triangles (for rendering as quads).
    pub fn triangles_length(&self) -> usize {
        // Each segment becomes 2 triangles
        let segments: usize = self.polyline_counts.iter().map(|c| c.saturating_sub(1)).sum();
        segments * 2
    }

    /// Get the byte length of geometry data.
    pub fn geometry_byte_length(&self) -> usize {
        self.positions.len() * 24
    }

    /// Add a polyline.
    pub fn add_polyline(&mut self, positions: &[DVec3], batch_id: u32, width: f64) {
        let start = self.positions.len();
        self.positions.extend_from_slice(positions);
        self.polyline_starts.push(start);
        self.polyline_counts.push(positions.len());
        self.batch_ids.push(batch_id);
        self.widths.push(width);
    }

    /// Get positions for a specific polyline.
    pub fn get_polyline(&self, index: usize) -> Option<&[DVec3]> {
        if index >= self.polyline_starts.len() {
            return None;
        }
        let start = self.polyline_starts[index];
        let count = self.polyline_counts[index];
        Some(&self.positions[start..start + count])
    }
}

impl Default for Vector3DTilePolylines {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTilePolygons
// ============================================================================

/// Polygon features in a vector 3D tile.
///
/// Maps to CesiumJS `Scene/Vector3DTilePolygons.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector3DTilePolygons {
    /// Positions for all polygons (flattened).
    pub positions: Vec<DVec3>,
    /// Polygon indices (triangulated).
    pub indices: Vec<u32>,
    /// Start index of each polygon's indices.
    pub polygon_index_starts: Vec<usize>,
    /// Number of indices for each polygon.
    pub polygon_index_counts: Vec<usize>,
    /// Batch IDs for each polygon.
    pub batch_ids: Vec<u32>,
    /// Polygon colors (RGBA).
    pub colors: Vec<[f64; 4]>,
    /// Polygon heights (for extrusion).
    pub heights: Vec<f64>,
    /// Polygon extruded heights.
    pub extruded_heights: Vec<f64>,
    /// Whether polygons are clamped to ground.
    pub clamp_to_ground: bool,
}

impl Vector3DTilePolygons {
    /// Create empty polygons.
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

    /// Get the number of polygons.
    pub fn polygons_length(&self) -> usize {
        self.polygon_index_starts.len()
    }

    /// Get the number of triangles.
    pub fn triangles_length(&self) -> usize {
        self.indices.len() / 3
    }

    /// Get the byte length of geometry data.
    pub fn geometry_byte_length(&self) -> usize {
        self.positions.len() * 24 + self.indices.len() * 4
    }

    /// Add a polygon with triangulated indices.
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
        // Offset indices by vertex offset
        self.indices.extend(indices.iter().map(|i| i + vertex_offset));

        self.polygon_index_starts.push(index_start);
        self.polygon_index_counts.push(indices.len());
        self.batch_ids.push(batch_id);
        self.heights.push(height);
        self.extruded_heights.push(extruded_height);
    }
}

impl Default for Vector3DTilePolygons {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vector3DTileContent
// ============================================================================

/// Complete vector 3D tile content.
///
/// Maps to CesiumJS `Scene/Vector3DTileContent.js`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vector3DTileContent {
    /// Point features.
    pub points: Option<Vector3DTilePoints>,
    /// Polyline features.
    pub polylines: Option<Vector3DTilePolylines>,
    /// Polygon features.
    pub polygons: Option<Vector3DTilePolygons>,
    /// Feature count from batch table.
    pub features_length: usize,
}

impl Vector3DTileContent {
    /// Create empty vector tile content.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the total number of points.
    pub fn points_length(&self) -> usize {
        self.points.as_ref().map_or(0, |p| p.points_length())
    }

    /// Get the total number of triangles.
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

    /// Get the total geometry byte length.
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

    /// Get the content types present.
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
// MVT (Mapbox Vector Tile) support
// ============================================================================

/// MVT geometry types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvtGeometryType {
    /// Unknown geometry.
    Unknown,
    /// Point geometry.
    Point,
    /// LineString geometry.
    LineString,
    /// Polygon geometry.
    Polygon,
}

/// An MVT layer.
#[derive(Debug, Clone, PartialEq)]
pub struct MvtLayer {
    /// Layer name.
    pub name: String,
    /// Layer version.
    pub version: u32,
    /// Tile extent (typically 4096).
    pub extent: u32,
    /// Features in this layer.
    pub features: Vec<MvtFeature>,
    /// Keys (property names).
    pub keys: Vec<String>,
    /// Values (property values).
    pub values: Vec<MvtValue>,
}

impl MvtLayer {
    /// Create a new MVT layer.
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

/// An MVT feature.
#[derive(Debug, Clone, PartialEq)]
pub struct MvtFeature {
    /// Feature ID.
    pub id: Option<u64>,
    /// Geometry type.
    pub geometry_type: MvtGeometryType,
    /// Geometry commands (encoded).
    pub geometry: Vec<u32>,
    /// Property tags (key_idx, value_idx pairs).
    pub tags: Vec<u32>,
}

impl MvtFeature {
    /// Create a new feature.
    pub fn new(geometry_type: MvtGeometryType) -> Self {
        Self {
            id: None,
            geometry_type,
            geometry: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// MVT property value.
#[derive(Debug, Clone, PartialEq)]
pub enum MvtValue {
    /// String value.
    String(String),
    /// Float value.
    Float(f64),
    /// Double value.
    Double(f64),
    /// Integer value.
    Int(i64),
    /// Unsigned integer value.
    Uint(u64),
    /// Signed integer value.
    Sint(i64),
    /// Boolean value.
    Bool(bool),
}

/// Decode MVT geometry commands into positions.
///
/// MVT uses a command-based encoding:
/// - MoveTo (command_id = 1)
/// - LineTo (command_id = 2)
/// - ClosePath (command_id = 7)
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

/// Zigzag decode an unsigned integer to signed.
fn zigzag_decode(n: u32) -> i32 {
    ((n >> 1) as i32) ^ (-((n & 1) as i32))
}

// ============================================================================
// Tests
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
        assert_eq!(polylines.triangles_length(), 6); // (2 + 1) segments * 2 triangles
        assert_eq!(polylines.get_polyline(0).unwrap().len(), 3);
        assert_eq!(polylines.get_polyline(1).unwrap().len(), 2);
        assert!(polylines.get_polyline(5).is_none());
    }

    #[test]
    fn test_vector_polygons() {
        let mut polygons = Vector3DTilePolygons::new();
        // Triangle
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
        assert_eq!(rings[0].len(), 3); // 1 moveto + 2 lineto
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
        // Should be closed (first == last)
        assert_eq!(rings[0].first(), rings[0].last());
    }
}
