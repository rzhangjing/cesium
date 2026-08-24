//! Ported from `packages/engine/Source/DataSources/Property.js`.
//!
//! A property value that may vary over time.

use cesium_core::event::Event;

/// A property value that may vary over time.
///
/// Properties are used to define entity attributes that can change
/// over time, such as position, color, scale, etc.
///
/// In CesiumJS, Property is an abstract base class with `getValue`,
/// `isConstant`, and `definitionChanged` event. This trait mirrors that API.
pub trait Property {
    /// Returns the value at the given time.
    fn get_value(&self, time: f64) -> PropertyResult;

    /// Returns whether this property is constant.
    fn is_constant(&self) -> bool;

    /// Returns whether this property has been destroyed.
    fn is_destroyed(&self) -> bool;

    /// Returns whether this property equals another.
    ///
    /// In CesiumJS, this is `equals(other)`. Default implementation
    /// compares by pointer identity (always false for trait objects).
    fn equals(&self, _other: &dyn Property) -> bool {
        false
    }

    /// Gets the event that is raised whenever the definition of this
    /// property changes.
    ///
    /// Port of the `Property.prototype.definitionChanged` getter. The
    /// definition is considered to have changed if a call to `get_value`
    /// would return a different result for the same time.
    ///
    /// DEVIATION: CesiumJS exposes the `Event` on every Property
    /// implementation; the Rust trait returns `Option` with a `None`
    /// default so implementations without a mutable definition (and the
    /// not-yet-materialized `SampledProperty` /
    /// `TimeIntervalCollectionProperty` ports) stay compatible. The JS
    /// event payload is the property itself (`raiseEvent(this)`); the Rust
    /// event carries `()` since self-referential payloads are not
    /// expressible. See docs/deviations.md.
    fn definition_changed(&self) -> Option<&Event<()>> {
        None
    }
}

/// The result of evaluating a property.
///
/// In CesiumJS, properties can return any JavaScript value.
/// This enum covers the common Cesium property types.
#[derive(Debug, Clone)]
pub enum PropertyResult {
    /// A boolean value.
    Boolean(bool),
    /// A numeric value.
    Number(f64),
    /// A string value.
    String(String),
    /// A color value (RGBA, each 0.0–1.0).
    Color(f64, f64, f64, f64),
    /// A 3D position (x, y, z).
    Position(f64, f64, f64),
    /// A Cartesian3 position (mirrors `Position` but semantically distinct).
    Cartesian3(f64, f64, f64),
    /// A quaternion orientation (x, y, z, w).
    Quaternion(f64, f64, f64, f64),
    /// A near-far scalar (near distance, near value, far distance, far value).
    NearFarScalar(f64, f64, f64, f64),
    /// A bounding rectangle (west, south, east, north in radians).
    Rectangle(f64, f64, f64, f64),
    /// A height reference enum value.
    HeightReference(u32),
    /// A label style enum value.
    LabelStyle(u32),
    /// A horizontal/vertical origin pair.
    Origin(u32, u32),
    /// An arbitrary JSON object or array (mirrors CZML `object`/`array`
    /// constant custom properties).
    Json(serde_json::Value),
    /// No value / undefined.
    None,
}

impl PropertyResult {
    /// Returns the value as an f64, if it is a Number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PropertyResult::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as a bool, if it is a Boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyResult::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as a string reference, if it is a String.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PropertyResult::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a (r, g, b, a) tuple, if it is a Color.
    pub fn as_color(&self) -> Option<(f64, f64, f64, f64)> {
        match self {
            PropertyResult::Color(r, g, b, a) => Some((*r, *g, *b, *a)),
            _ => None,
        }
    }

    /// Returns the value as a (x, y, z) tuple, if it is a Position or Cartesian3.
    pub fn as_position(&self) -> Option<(f64, f64, f64)> {
        match self {
            PropertyResult::Position(x, y, z) | PropertyResult::Cartesian3(x, y, z) => {
                Some((*x, *y, *z))
            }
            _ => None,
        }
    }

    /// Returns whether this is `None` (undefined).
    pub fn is_none(&self) -> bool {
        matches!(self, PropertyResult::None)
    }
}

impl PartialEq for PropertyResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Color(r1, g1, b1, a1), Self::Color(r2, g2, b2, a2)) => {
                r1 == r2 && g1 == g2 && b1 == b2 && a1 == a2
            }
            (Self::Position(x1, y1, z1), Self::Position(x2, y2, z2))
            | (Self::Cartesian3(x1, y1, z1), Self::Cartesian3(x2, y2, z2)) => {
                x1 == x2 && y1 == y2 && z1 == z2
            }
            (Self::Quaternion(x1, y1, z1, w1), Self::Quaternion(x2, y2, z2, w2)) => {
                x1 == x2 && y1 == y2 && z1 == z2 && w1 == w2
            }
            (Self::Json(a), Self::Json(b)) => a == b,
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

/// Creates a definition-changed event for property implementations.
///
/// In CesiumJS, each property has a `definitionChanged` Event that fires
/// when the property's definition changes (not its value over time).
pub fn create_definition_changed_event() -> Event {
    Event::new()
}
