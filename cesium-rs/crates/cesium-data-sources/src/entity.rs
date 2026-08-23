//! Ported from `packages/engine/Source/DataSources/Entity.js`.
//!
//! An entity is a visual element in the scene with associated properties.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;

use crate::billboard_graphics::BillboardGraphics;
use crate::label_graphics::LabelGraphics;
use crate::point_graphics::PointGraphics;
use crate::polyline_graphics::PolylineGraphics;
use crate::polygon_graphics::PolygonGraphics;
use crate::model_graphics::ModelGraphics;
use crate::property_bag::PropertyBag;

/// An entity is a visual element in the scene with associated properties.
///
/// Entities are the primary way to add visual elements to a scene.
/// They can represent points, labels, billboards, models, polylines,
/// polygons, and more.
///
/// In CesiumJS, Entity.js is ~1200 lines with full property change tracking,
/// availability intervals, and merge/clone semantics.
pub struct Entity {
    /// The unique identifier for this entity.
    pub id: String,
    /// The name of this entity (displayed in the selection indicator).
    pub name: Option<String>,
    /// Whether this entity is shown.
    pub show: bool,
    /// The description (HTML) for this entity (shown in the info box).
    pub description: Option<String>,
    /// The position of this entity.
    pub position: Option<Cartesian3>,
    /// The orientation of this entity.
    pub orientation: Option<cesium_core::quaternion::Quaternion>,
    /// The billboard graphics.
    pub billboard: Option<BillboardGraphics>,
    /// The label graphics.
    pub label: Option<LabelGraphics>,
    /// The point graphics.
    pub point: Option<PointGraphics>,
    /// The polyline graphics.
    pub polyline: Option<PolylineGraphics>,
    /// The polygon graphics.
    pub polygon: Option<PolygonGraphics>,
    /// The model graphics.
    pub model: Option<ModelGraphics>,
    /// The parent entity ID.
    pub parent_id: Option<String>,
    /// Arbitrary user-defined properties.
    pub properties: PropertyBag,
    /// The availability interval for this entity.
    pub availability: Option<TimeInterval>,
    /// Fired when a property or sub-property changes.
    pub definition_changed: Event,
}

/// A simple time interval (start, end) in Julian day numbers.
#[derive(Debug, Clone, Copy)]
pub struct TimeInterval {
    /// The start time (Julian day number).
    pub start: f64,
    /// The end time (Julian day number).
    pub end: f64,
    /// Whether the start is included.
    pub is_start_included: bool,
    /// Whether the end is included.
    pub is_end_included: bool,
}

impl TimeInterval {
    /// Creates a new time interval.
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            start,
            end,
            is_start_included: true,
            is_end_included: true,
        }
    }

    /// Returns whether the given time is within this interval.
    pub fn contains(&self, time: f64) -> bool {
        let after_start = if self.is_start_included {
            time >= self.start
        } else {
            time > self.start
        };
        let before_end = if self.is_end_included {
            time <= self.end
        } else {
            time < self.end
        };
        after_start && before_end
    }

    /// Returns whether this interval is empty.
    pub fn is_empty(&self) -> bool {
        if self.start == self.end {
            !(self.is_start_included && self.is_end_included)
        } else {
            self.start > self.end
        }
    }
}

impl Entity {
    /// Creates a new entity with the given ID.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            show: true,
            description: None,
            position: None,
            orientation: None,
            billboard: None,
            label: None,
            point: None,
            polyline: None,
            polygon: None,
            model: None,
            parent_id: None,
            properties: PropertyBag::new(),
            availability: None,
            definition_changed: Event::new(),
        }
    }

    /// Merges another entity's properties into this one.
    ///
    /// In CesiumJS, `Entity.merge(source)` copies all defined properties
    /// from the source entity to this entity. Existing values are overwritten.
    pub fn merge(&mut self, other: &Entity) {
        if let Some(ref name) = other.name {
            self.name = Some(name.clone());
        }
        if other.show {
            self.show = true;
        }
        if let Some(ref desc) = other.description {
            self.description = Some(desc.clone());
        }
        if let Some(ref pos) = other.position {
            self.position = Some(*pos);
        }
        if let Some(ref orient) = other.orientation {
            self.orientation = Some(*orient);
        }
        if other.billboard.is_some() {
            self.billboard = other.billboard.clone();
        }
        if other.label.is_some() {
            self.label = other.label.clone();
        }
        if other.point.is_some() {
            self.point = other.point.clone();
        }
        if other.polyline.is_some() {
            self.polyline = other.polyline.clone();
        }
        if other.polygon.is_some() {
            self.polygon = other.polygon.clone();
        }
        if other.model.is_some() {
            self.model = other.model.clone();
        }
        // Merge user-defined properties
        for key in other.properties.keys() {
            if let Some(val) = other.properties.get(key) {
                self.properties.set(key, val.clone());
            }
        }
        if other.availability.is_some() {
            self.availability = other.availability;
        }
    }

    /// Returns whether this entity is available at the given time.
    ///
    /// In CesiumJS, this checks the `availability` TimeInterval.
    pub fn is_available(&self, time: f64) -> bool {
        match self.availability {
            Some(ref interval) => interval.contains(time),
            None => true, // No availability constraint
        }
    }

    /// Returns whether this entity has any visual representation.
    pub fn has_visuals(&self) -> bool {
        self.billboard.is_some()
            || self.label.is_some()
            || self.point.is_some()
            || self.polyline.is_some()
            || self.polygon.is_some()
            || self.model.is_some()
    }
}
