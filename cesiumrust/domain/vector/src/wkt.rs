//! WKT（Well-Known Text）几何解析器。
//!
//! 实现用于几何表示的 OGC WKT 规范：支持 Point、LineString、Polygon、
//! MultiPoint、MultiLineString、MultiPolygon 与 GeometryCollection 的解析
//! （parse_wkt）与序列化（to_wkt）；解析采用括号深度跟踪以处理嵌套结构。

use glam::DVec2;

/// 一个 WKT 几何。
///
/// 覆盖 OGC simple features 的七种基本几何类型，二维坐标以 DVec2 表示。
#[derive(Debug, Clone, PartialEq)]
pub enum WktGeometry {
    /// 单个点。
    ///
    /// 一对二维坐标 (x, y)。
    Point(DVec2),
    /// 一条线串（折线）。
    ///
    /// 至少含两个顶点的有序坐标序列。
    LineString(Vec<DVec2>),
    /// 一个带外环和可选内腔的多边形。
    ///
    /// 外环定义轮廓，内环（interiors）表示挖去的空洞。
    Polygon {
        /// 外环。
        exterior: Vec<DVec2>,
        /// 内环（内腔）。
        interiors: Vec<Vec<DVec2>>,
    },
    /// 多个点。
    ///
    /// 零个或多个 Point 的集合。
    MultiPoint(Vec<DVec2>),
    /// 多条线串。
    ///
    /// 零个或多个 LineString 的集合。
    MultiLineString(Vec<Vec<DVec2>>),
    /// 多个多边形。
    ///
    /// 每个元素本身是一个 Polygon 几何。
    MultiPolygon(Vec<WktGeometry>),
    /// 一个几何集合。
    ///
    /// 可混合容纳任意类型的子几何。
    GeometryCollection(Vec<WktGeometry>),
}

/// 将 WKT 字符串解析为几何。
pub fn parse_wkt(wkt: &str) -> Result<WktGeometry, WktError> {
    // 去除首尾空白，并以大写前缀判定几何类型（WKT 类型名大小写不敏感）
    let wkt = wkt.trim();
    let upper = wkt.to_uppercase();

    // 按类型前缀分派到对应的解析函数
    if upper.starts_with("POINT") {
        // 单点：括号内含一对坐标
        parse_point(wkt)
    } else if upper.starts_with("LINESTRING") {
        // 线串：一对括号内多个坐标
        parse_linestring(wkt)
    } else if upper.starts_with("POLYGON") {
        // 多边形：嵌套括号表示外环与内腔
        parse_polygon(wkt)
    } else if upper.starts_with("MULTIPOINT") {
        // 多点：兼容带/不带内层括号两种写法
        parse_multipoint(wkt)
    } else if upper.starts_with("MULTILINESTRING") {
        // 多线：每个括号环一条线
        parse_multilinestring(wkt)
    } else if upper.starts_with("MULTIPOLYGON") {
        // 多面：三层嵌套括号，逐面递归解析
        parse_multipolygon(wkt)
    } else if upper.starts_with("GEOMETRYCOLLECTION") {
        // 几何集合：顶层逗号拆分子几何
        parse_geometry_collection(wkt)
    } else {
        // 无法识别的类型：截取前 20 个字符作为错误提示
        Err(WktError::UnknownType(wkt[..20.min(wkt.len())].to_string()))
    }
}

/// WKT 解析错误。
///
/// 描述解析失败的具体原因，包括未知类型、坐标/数字格式错误、括号缺失与输入意外结束。
#[derive(Debug, Clone, PartialEq)]
pub enum WktError {
    /// 未知的几何类型。
    ///
    /// 前缀不匹配任何已知 WKT 关键字。
    UnknownType(String),
    /// 无效的坐标格式。
    ///
    /// 某个坐标片段缺少或携带过多分量。
    InvalidCoordinate(String),
    /// 缺少括号。
    ///
    /// 未找到成对的 '(' 与 ')'。
    MissingParenthesis,
    /// 输入意外结束。
    ///
    /// 需要至少一个坐标却得到空集。
    UnexpectedEnd,
    /// 无效的数字格式。
    ///
    /// 坐标分量无法解析为 f64。
    InvalidNumber(String),
}

impl std::fmt::Display for WktError {
    /// 以可读文本渲染错误变体，供错误信息展示使用。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 逐变体拼接英文提示信息，变量类错误携带具体入参
        match self {
            Self::UnknownType(t) => write!(f, "Unknown WKT type: {}", t),
            Self::InvalidCoordinate(c) => write!(f, "Invalid coordinate: {}", c),
            Self::MissingParenthesis => write!(f, "Missing parenthesis"),
            Self::UnexpectedEnd => write!(f, "Unexpected end of input"),
            Self::InvalidNumber(n) => write!(f, "Invalid number: {}", n),
        }
    }
}

/// 解析 POINT 几何。
fn parse_point(wkt: &str) -> Result<WktGeometry, WktError> {
    // 提取括号内内容并解析坐标列表，取首个点
    let inner = extract_parentheses(wkt)?;
    let coords = parse_coordinate_list(&inner)?;
    if coords.is_empty() {
        return Err(WktError::UnexpectedEnd);
    }
    Ok(WktGeometry::Point(coords[0]))
}

/// 解析 LINESTRING 几何。
fn parse_linestring(wkt: &str) -> Result<WktGeometry, WktError> {
    // 括号内即一组以逗号分隔的坐标点
    let inner = extract_parentheses(wkt)?;
    let coords = parse_coordinate_list(&inner)?;
    Ok(WktGeometry::LineString(coords))
}

/// 解析 POLYGON 几何。
fn parse_polygon(wkt: &str) -> Result<WktGeometry, WktError> {
    // 首个环为外环，其余为内腔（空洞）
    let inner = extract_parentheses(wkt)?;
    let rings = parse_ring_list(&inner)?;
    if rings.is_empty() {
        return Err(WktError::UnexpectedEnd);
    }
    let exterior = rings[0].clone();
    let interiors = rings[1..].to_vec();
    Ok(WktGeometry::Polygon { exterior, interiors })
}

/// 解析 MULTIPOINT 几何。
fn parse_multipoint(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    // MultiPoint 可以是 ((x y), (x y)) 或 (x y, x y)
    if inner.contains('(') {
        // 带内层括号：每个环取首点作为一个点
        let rings = parse_ring_list(&inner)?;
        let points: Vec<DVec2> = rings.iter().filter_map(|r| r.first().copied()).collect();
        Ok(WktGeometry::MultiPoint(points))
    } else {
        // 无内层括号：直接按扁平坐标列表解析
        let coords = parse_coordinate_list(&inner)?;
        Ok(WktGeometry::MultiPoint(coords))
    }
}

/// 解析 MULTILINESTRING 几何。
fn parse_multilinestring(wkt: &str) -> Result<WktGeometry, WktError> {
    // 每个括号环即一条折线
    let inner = extract_parentheses(wkt)?;
    let rings = parse_ring_list(&inner)?;
    Ok(WktGeometry::MultiLineString(rings))
}

/// 解析 MULTIPOLYGON 几何。
fn parse_multipolygon(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    // 在顶层括号处拆分：每个多边形为 ((rings))
    let mut polygons = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    // 逐字符扫描，用 depth 追踪括号嵌套层级，仅在 depth 回到 0 时切出一个多边形
    for c in inner.chars() {
        match c {
            '(' => {
                depth += 1;
                if depth == 1 {
                    // 进入新多边形：重置累加器
                    current = String::new();
                } else {
                    current.push(c);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    // 一个完整多边形闭合：包装为 POLYGON 递归解析
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
                // 非括号字符：仅当处于环内部（depth>=1）时才累加
                if depth >= 1 {
                    current.push(c);
                }
            }
        }
    }

    Ok(WktGeometry::MultiPolygon(polygons))
}

/// 解析 GEOMETRYCOLLECTION 几何。
fn parse_geometry_collection(wkt: &str) -> Result<WktGeometry, WktError> {
    let inner = extract_parentheses(wkt)?;
    // 在顶层逗号处拆分，每个子串递归解析为单独几何
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

/// 提取首个 '(' 与末个 ')' 之间的内容（不含外层括号）。
fn extract_parentheses(wkt: &str) -> Result<String, WktError> {
    // 找不到开/闭括号或顺序颠倒均视为括号缺失错误
    let start = wkt.find('(').ok_or(WktError::MissingParenthesis)?;
    let end = wkt.rfind(')').ok_or(WktError::MissingParenthesis)?;
    // 取最外层括号之间的内容，供上层按环/点进一步解析
    if start >= end {
        return Err(WktError::MissingParenthesis);
    }
    Ok(wkt[start + 1..end].to_string())
}

/// 解析以逗号分隔的坐标列表为 DVec2 序列。
fn parse_coordinate_list(s: &str) -> Result<Vec<DVec2>, WktError> {
    let mut coords = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        // 跳过空片段（例如尾部多余逗号）
        if part.is_empty() {
            continue;
        }
        // 每个点由空白分隔的 x、y 两个数字组成，少于两个为无效坐标
        let nums: Vec<&str> = part.split_whitespace().collect();
        if nums.len() < 2 {
            return Err(WktError::InvalidCoordinate(part.to_string()));
        }
        // x、y 分别解析为 f64，任一不可解析则报无效数字错误
        let x: f64 = nums[0].parse().map_err(|_| WktError::InvalidNumber(nums[0].to_string()))?;
        let y: f64 = nums[1].parse().map_err(|_| WktError::InvalidNumber(nums[1].to_string()))?;
        coords.push(DVec2::new(x, y));
    }
    Ok(coords)
}

/// 解析括号环列表，每对括号内为一个坐标环。
fn parse_ring_list(s: &str) -> Result<Vec<Vec<DVec2>>, WktError> {
    let mut rings = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    // 逐字符扫描：depth 从 0 升到 1 开启一个新环，回到 0 时环闭合并解析
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                if depth == 1 {
                    // 顶层开括号：开始一个新的环，重置累加器
                    current = String::new();
                } else {
                    // 内层括号（如坐标分组）：作为普通字符保留
                    current.push(c);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    // 环闭合：将累加器内容解析为一个坐标环
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

    // 处理没有内层括号的情况（例如无括号的 MultiPoint）
    if rings.is_empty() && !s.trim().is_empty() && !s.contains('(') {
        rings.push(parse_coordinate_list(s)?);
    }

    Ok(rings)
}

/// 按顶层分隔符拆分字符串（嵌套括号内的分隔符不参与拆分）。
fn split_top_level(s: &str, delimiter: char) -> Result<Vec<String>, WktError> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    // 仅在 depth==0（顶层）时遇到分隔符才切分，否则归入当前片段
    for c in s.chars() {
        match c {
            // 遇开括号加深嵌套层级
            '(' => {
                depth += 1;
                current.push(c);
            }
            // 遇闭括号：若正好处于顶层且分隔符为 ')' 则在此切分
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
            // 顶层逗号：按分隔符切出一个片段
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
        // 收尾：将最后一个未被分隔符切出的片段加入结果
        parts.push(current.trim().to_string());
    }

    Ok(parts)
}

/// 将几何序列化为 WKT 字符串。
pub fn to_wkt(geometry: &WktGeometry) -> String {
    // 逐类型拼接 WKT 文本；坐标统一以空格分隔的 x y 形式输出
    match geometry {
        // POINT：单个括号内的 x y
        WktGeometry::Point(p) => format!("POINT ({} {})", p.x, p.y),
        // LINESTRING：坐标列表展平写入一对括号
        WktGeometry::LineString(coords) => {
            format!("LINESTRING ({})", coords_to_string(coords))
        }
        WktGeometry::Polygon { exterior, interiors } => {
            // POLYGON：外环在前、内腔依次排列，每个环各自包裹一对括号
            let mut rings = vec![format!("({})", coords_to_string(exterior))];
            for interior in interiors {
                rings.push(format!("({})", coords_to_string(interior)));
            }
            format!("POLYGON ({})", rings.join(", "))
        }
        // MULTIPOINT：每个点各自包裹一对括号
        WktGeometry::MultiPoint(points) => {
            let pts: Vec<String> = points.iter().map(|p| format!("({} {})", p.x, p.y)).collect();
            format!("MULTIPOINT ({})", pts.join(", "))
        }
        // MULTILINESTRING：每条线为一对括号内的坐标序列
        WktGeometry::MultiLineString(lines) => {
            let ls: Vec<String> = lines.iter().map(|l| format!("({})", coords_to_string(l))).collect();
            format!("MULTILINESTRING ({})", ls.join(", "))
        }
        WktGeometry::MultiPolygon(polys) => {
            // MULTIPOLYGON：递归序列化子面并去掉 POLYGON 前缀，仅保留环列表
            let ps: Vec<String> = polys.iter().map(|p| {
                let wkt = to_wkt(p);
                wkt.strip_prefix("POLYGON ").unwrap_or(&wkt).to_string()
            }).collect();
            format!("MULTIPOLYGON ({})", ps.join(", "))
        }
        // GEOMETRYCOLLECTION：子几何递归序列化后以逗号拼接
        WktGeometry::GeometryCollection(geoms) => {
            let gs: Vec<String> = geoms.iter().map(to_wkt).collect();
            format!("GEOMETRYCOLLECTION ({})", gs.join(", "))
        }
    }
}

/// 将坐标序列格式化为 WKT 的 "x y, x y, ..." 字符串。
fn coords_to_string(coords: &[DVec2]) -> String {
    // 每点写作 "x y"，点之间以逗号空格分隔
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
