//! Ported from `packages/engine/Source/Workers/createGeometry.js`.
//!
//! Dispatches geometry creation to the appropriate worker function.

/// The type of geometry to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    Box,
    BoxOutline,
    Circle,
    CircleOutline,
    CoplanarPolygon,
    CoplanarPolygonOutline,
    Corridor,
    CorridorOutline,
    Cylinder,
    CylinderOutline,
    Ellipse,
    EllipseOutline,
    Ellipsoid,
    EllipsoidOutline,
    Frustum,
    FrustumOutline,
    GroundPolyline,
    Plane,
    PlaneOutline,
    Polygon,
    PolygonOutline,
    Polyline,
    PolylineVolume,
    PolylineVolumeOutline,
    Rectangle,
    RectangleOutline,
    SimplePolyline,
    Sphere,
    SphereOutline,
    Wall,
    WallOutline,
}

/// Dispatches geometry creation to the appropriate worker function.
///
/// This is the main entry point called by [`TaskProcessor`](crate::task_processor::TaskProcessor)
/// when creating geometry on a background thread.
/// Mirrors CesiumJS `createGeometry` (272 lines).
///
/// In CesiumJS, this deserializes the geometry subtask name and packed
/// geometry, then delegates to the matching `create*Geometry` module.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error instead of a silent empty result; in-process callers should use
/// the typed `create_*_geometry_unpacked` entry points directly.
pub fn create_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createGeometry"))
}

/// Returns the geometry type name from a string.
pub fn geometry_type_from_str(name: &str) -> Option<GeometryType> {
    match name {
        "box" => Some(GeometryType::Box),
        "boxOutline" => Some(GeometryType::BoxOutline),
        "circle" => Some(GeometryType::Circle),
        "circleOutline" => Some(GeometryType::CircleOutline),
        "coplanarPolygon" => Some(GeometryType::CoplanarPolygon),
        "coplanarPolygonOutline" => Some(GeometryType::CoplanarPolygonOutline),
        "corridor" => Some(GeometryType::Corridor),
        "corridorOutline" => Some(GeometryType::CorridorOutline),
        "cylinder" => Some(GeometryType::Cylinder),
        "cylinderOutline" => Some(GeometryType::CylinderOutline),
        "ellipse" => Some(GeometryType::Ellipse),
        "ellipseOutline" => Some(GeometryType::EllipseOutline),
        "ellipsoid" => Some(GeometryType::Ellipsoid),
        "ellipsoidOutline" => Some(GeometryType::EllipsoidOutline),
        "frustum" => Some(GeometryType::Frustum),
        "frustumOutline" => Some(GeometryType::FrustumOutline),
        "groundPolyline" => Some(GeometryType::GroundPolyline),
        "plane" => Some(GeometryType::Plane),
        "planeOutline" => Some(GeometryType::PlaneOutline),
        "polygon" => Some(GeometryType::Polygon),
        "polygonOutline" => Some(GeometryType::PolygonOutline),
        "polyline" => Some(GeometryType::Polyline),
        "polylineVolume" => Some(GeometryType::PolylineVolume),
        "polylineVolumeOutline" => Some(GeometryType::PolylineVolumeOutline),
        "rectangle" => Some(GeometryType::Rectangle),
        "rectangleOutline" => Some(GeometryType::RectangleOutline),
        "simplePolyline" => Some(GeometryType::SimplePolyline),
        "sphere" => Some(GeometryType::Sphere),
        "sphereOutline" => Some(GeometryType::SphereOutline),
        "wall" => Some(GeometryType::Wall),
        "wallOutline" => Some(GeometryType::WallOutline),
        _ => None,
    }
}
