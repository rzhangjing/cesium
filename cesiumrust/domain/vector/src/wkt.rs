//! WKT (Well-Known Text) geometry parser.
//!
//! Implements OGC WKT specification for geometry representation.

use glam::DVec2;

/// A WKT geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum WktGeometry {
    /// A single point.
    Point(DVec2),
    /// A line string (polyline).
    LineString(Vec<DVec2>),
    /// A polygon with exterior ring and optional holes.
    Polygon {
        /// Exterior ring.
        exterior: Vec<DVec2>,
        /// Interior rings (holes).
        interiors: Vec<Vec<DVec2>>,
    },
    /// Multiple points.
    MultiPoint(Vec<DVec2>),
    /// Multiple line strings.
    MultiLineString(Vec<Vec<DVec2>>),
    /// Multiple polygons.
    MultiPolygon(Vec<WktGeometry>),
    /// A collection of geometries.
    GeometryCollection(Vec<WktGeometry>),
}

/// Parses a WKT string into a geometry.
pub fn parse_wkt(wkt: &str) -> Result<WktGeometry, WktError> {
    let wkt = wkt.trim();
    let upper = wkt.to_uppercase();

    if upper.starts_with("POINT") {
        parse_point(wkt)
    } else if upper.starts_with("LINESTRING") {
        parse_linestring(wkt)
    } else if upper.starts_with("POLYGON") {
        parse_polygon(wkt)
    } else if upper.starts_with("MULTIPOINT") {
        parse_multipoint(wkt)
    } else if upper.starts_with("MULTILINESTRING") {
        parse_multilinestring(wkt)
    } else if upper.starts_with("MULTIPOLYGON") {
        parse_multipolygon(wkt)
    } else if upper.starts_with("GEOMETRYCOLLECTION") {
        parse_geometry_collection(wkt)
    } else {
        Err(WktError::UnknownType(wkt[..20.min(wkt.len())].to_string()))
    }
}

/// WKT parsing errors.
#[derive(Debug, Clone, PartialEq)]
pub enum WktError {
    /// Unknown geometry type.
    UnknownType(String),
    /// Invalid coordinate format.
    InvalidCoordinate(String),
    /// Missing parentheses.
    MissingParenthesis,
    /// Unexpected end of input.
    UnexpectedEnd,
    /// Invalid number format.
    InvalidNumber(String),
}

impl std::fmt::Display for WktError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownType(t) => write!(f, "Unknown WKT type: {}", t),
            Self::InvalidCoordinate(c) => write!(f, "Invalid coordinate: {}", c),
            Self::MissingParenthesis => write!(f, "Missing parenthesis"),
            Self::UnexpectedEnd => write!(f, "Unexpected end of input"),
            Self::InvalidNumber(n) => write!(f, "Invalid number: {}", n),
        }
    }
}

fn parse_point(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    let coords = parse_coordinate_list(&inner)?;
    if coords.is_empty() {
        return Err(WktError::UnexpectedEnd);
    }
    Ok(WktGeometry::Point(coords[0]))
}

fn parse_linestring(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    let coords = parse_coordinate_list(&inner)?;
    Ok(WktGeometry::LineString(coords))
}

fn parse_polygon(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    let rings = parse_ring_list(&inner)?;
    if rings.is_empty() {
        return Err(WktError::UnexpectedEnd);
    }
    let exterior = rings[0].clone();
    let interiors = rings[1..].to_vec();
    Ok(WktGeometry::Polygon { exterior, interiors })
}

fn parse_multipoint(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    // MultiPoint can be ((x y), (x y)) or (x y, x y)
    if inner.contains('(') {
        let rings = parse_ring_list(&inner)?;
        let points: Vec<DVec2> = rings.iter().filter_map(|r| r.first().copied()).collect();
        Ok(WktGeometry::MultiPoint(points))
    } else {
        let coords = parse_coordinate_list(&inner)?;
        Ok(WktGeometry::MultiPoint(coords))
    }
}

fn parse_multilinestring(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    let rings = parse_ring_list(&inner)?;
    Ok(WktGeometry::MultiLineString(rings))
}

fn parse_multipolygon(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    // Split at top-level parentheses: each polygon is ((rings))
    let mut polygons = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    for c in inner.chars() {
        match c {
            '(' => {
                depth += 1;
                if depth == 1 {
                    current = String::new();
                } else {
                    current.push(c);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        let full = format!("POLYGON({})", trimmed);
                        polygons.push(parse_polygon(&full)?);
                    }
                } else {
                    current.push(c);
                }
            }
            _ => {
                if depth >= 1 {
                    current.push(c);
                }
            }
        }
    }

    Ok(WktGeometry::MultiPolygon(polygons))
}

fn parse_geometry_collection(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    let parts = split_top_level(&inner, ',')?;
    let mut geometries = Vec::new();
    for part in parts {
        let part = part.trim();
        if !part.is_empty() {
            geometries.push(parse_wkt(part)?);
        }
    }
    Ok(WktGeometry::GeometryCollection(geometries))
}

fn extract_parentheses(wkt: &str) -> Result<String, WktError> {
    let start = wkt.find('(').ok_or(WktError::MissingParenthesis)?;
    let end = wkt.rfind(')').ok_or(WktError::MissingParenthesis)?;
    if start >= end {
        return Err(WktError::MissingParenthesis);
    }
    Ok(wkt[start + 1..end].to_string())
}

fn parse_coordinate_list(s: &str) -> Result<Vec<DVec2>, WktError> {
    let mut coords = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let nums: Vec<&str> = part.split_whitespace().collect();
        if nums.len() < 2 {
            return Err(WktError::InvalidCoordinate(part.to_string()));
        }
        let x: f64 = nums[0].parse().map_err(|_| WktError::InvalidNumber(nums[0].to_string()))?;
        let y: f64 = nums[1].parse().map_err(|_| WktError::InvalidNumber(nums[1].to_string()))?;
        coords.push(DVec2::new(x, y));
    }
    Ok(coords)
}

fn parse_ring_list(s: &str) -> Result<Vec<Vec<DVec2>>, WktError> {
    let mut rings = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                if depth == 1 {
                    current = String::new();
                } else {
                    current.push(c);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        rings.push(parse_coordinate_list(trimmed)?);
                    }
                } else {
                    current.push(c);
                }
            }
            _ => {
                if depth >= 1 {
                    current.push(c);
                }
            }
        }
    }

    // Handle case where there are no inner parentheses (e.g., MultiPoint without parens)
    if rings.is_empty() && !s.trim().is_empty() && !s.contains('(') {
        rings.push(parse_coordinate_list(s)?);
    }

    Ok(rings)
}

fn split_top_level(s: &str, delimiter: char) -> Result<Vec<String>, WktError> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                if depth == 0 && delimiter == ')' {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    current = String::new();
                } else {
                    depth -= 1;
                    current.push(c);
                }
            }
            ',' if depth == 0 && delimiter == ',' => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                }
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    Ok(parts)
}

/// Serializes a geometry to WKT string.
pub fn to_wkt(geometry: &WktGeometry) -> String {
    match geometry {
        WktGeometry::Point(p) => format!("POINT ({} {})", p.x, p.y),
        WktGeometry::LineString(coords) => {
            format!("LINESTRING ({})", coords_to_string(coords))
        }
        WktGeometry::Polygon { exterior, interiors } => {
            let mut rings = vec![format!("({})", coords_to_string(exterior))];
            for interior in interiors {
                rings.push(format!("({})", coords_to_string(interior)));
            }
            format!("POLYGON ({})", rings.join(", "))
        }
        WktGeometry::MultiPoint(points) => {
            let pts: Vec<String> = points.iter().map(|p| format!("({} {})", p.x, p.y)).collect();
            format!("MULTIPOINT ({})", pts.join(", "))
        }
        WktGeometry::MultiLineString(lines) => {
            let ls: Vec<String> = lines.iter().map(|l| format!("({})", coords_to_string(l))).collect();
            format!("MULTILINESTRING ({})", ls.join(", "))
        }
        WktGeometry::MultiPolygon(polys) => {
            let ps: Vec<String> = polys.iter().map(|p| {
                let wkt = to_wkt(p);
                wkt.strip_prefix("POLYGON ").unwrap_or(&wkt).to_string()
            }).collect();
            format!("MULTIPOLYGON ({})", ps.join(", "))
        }
        WktGeometry::GeometryCollection(geoms) => {
            let gs: Vec<String> = geoms.iter().map(to_wkt).collect();
            format!("GEOMETRYCOLLECTION ({})", gs.join(", "))
        }
    }
}

fn coords_to_string(coords: &[DVec2]) -> String {
    coords
        .iter()
        .map(|c| format!("{} {}", c.x, c.y))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_point() {
        let geom = parse_wkt("POINT (30 10)").unwrap();
        assert_eq!(geom, WktGeometry::Point(DVec2::new(30.0, 10.0)));
    }

    #[test]
    fn test_parse_linestring() {
        let geom = parse_wkt("LINESTRING (30 10, 10 30, 40 40)").unwrap();
        if let WktGeometry::LineString(coords) = geom {
            assert_eq!(coords.len(), 3);
            assert_eq!(coords[0], DVec2::new(30.0, 10.0));
        } else {
            panic!("Expected LineString");
        }
    }

    #[test]
    fn test_parse_polygon() {
        let geom = parse_wkt("POLYGON ((30 10, 40 40, 20 40, 10 20, 30 10))").unwrap();
        if let WktGeometry::Polygon { exterior, interiors } = geom {
            assert_eq!(exterior.len(), 5);
            assert!(interiors.is_empty());
        } else {
            panic!("Expected Polygon");
        }
    }

    #[test]
    fn test_parse_polygon_with_hole() {
        let geom = parse_wkt(
            "POLYGON ((35 10, 45 45, 15 40, 10 20, 35 10), (20 30, 35 35, 30 20, 20 30))",
        )
        .unwrap();
        if let WktGeometry::Polygon { exterior, interiors } = geom {
            assert_eq!(exterior.len(), 5);
            assert_eq!(interiors.len(), 1);
            assert_eq!(interiors[0].len(), 4);
        } else {
            panic!("Expected Polygon");
        }
    }

    #[test]
    fn test_parse_multipoint() {
        let geom = parse_wkt("MULTIPOINT ((10 40), (40 30), (20 20))").unwrap();
        if let WktGeometry::MultiPoint(points) = geom {
            assert_eq!(points.len(), 3);
        } else {
            panic!("Expected MultiPoint");
        }
    }

    #[test]
    fn test_parse_multilinestring() {
        let geom = parse_wkt("MULTILINESTRING ((10 10, 20 20, 10 40), (40 40, 30 30, 40 20, 30 10))").unwrap();
        if let WktGeometry::MultiLineString(lines) = geom {
            assert_eq!(lines.len(), 2);
            assert_eq!(lines[0].len(), 3);
            assert_eq!(lines[1].len(), 4);
        } else {
            panic!("Expected MultiLineString");
        }
    }

    #[test]
    fn test_parse_geometry_collection() {
        let geom = parse_wkt("GEOMETRYCOLLECTION (POINT (4 6), LINESTRING (4 6, 7 10))").unwrap();
        if let WktGeometry::GeometryCollection(geoms) = geom {
            assert_eq!(geoms.len(), 2);
        } else {
            panic!("Expected GeometryCollection");
        }
    }

    #[test]
    fn test_to_wkt_point() {
        let geom = WktGeometry::Point(DVec2::new(30.0, 10.0));
        assert_eq!(to_wkt(&geom), "POINT (30 10)");
    }

    #[test]
    fn test_to_wkt_linestring() {
        let geom = WktGeometry::LineString(vec![
            DVec2::new(30.0, 10.0),
            DVec2::new(10.0, 30.0),
        ]);
        assert_eq!(to_wkt(&geom), "LINESTRING (30 10, 10 30)");
    }

    #[test]
    fn test_roundtrip() {
        let original = "POINT (30 10)";
        let geom = parse_wkt(original).unwrap();
        let output = to_wkt(&geom);
        let geom2 = parse_wkt(&output).unwrap();
        assert_eq!(geom, geom2);
    }

    #[test]
    fn test_invalid_type() {
        let result = parse_wkt("INVALID (30 10)");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_parenthesis() {
        let result = parse_wkt("POINT 30 10");
        assert!(result.is_err());
    }
}
