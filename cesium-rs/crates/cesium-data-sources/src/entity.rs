//! Ported from `packages/engine/Source/DataSources/Entity.js`.
//!
//! An entity is a visual element in the scene with associated properties.

use std::collections::HashMap;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::Event;
use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::TimeInterval;

use crate::billboard_graphics::BillboardGraphics;
use crate::label_graphics::LabelGraphics;
use crate::point_graphics::PointGraphics;
use crate::polyline_graphics::PolylineGraphics;
use crate::polygon_graphics::PolygonGraphics;
use crate::rectangle_graphics::RectangleGraphics;
use crate::model_graphics::ModelGraphics;
use crate::property::PropertyResult;
use crate::property_bag::PropertyBag;

/// The payload of [`Entity::definition_changed`].
///
/// Port of the CesiumJS `definitionChanged.raiseEvent(this, name, newValue,
/// oldValue)` argument list. DEVIATION: the JS event passes the entity
/// itself as the first argument; Rust listeners subscribe per-entity (or via
/// [`crate::entity_collection::EntityCollection`], which captures the entity
/// id at subscription time), so the payload carries only the property name
/// and the old/new values. Values are projected onto [`PropertyResult`];
/// graphics sub-objects and other non-scalar values are reported as
/// [`PropertyResult::None`]. See docs/deviations.md.
#[derive(Debug, Clone)]
pub struct EntityDefinitionChangedArgs {
    /// The name of the property that changed (e.g. `"show"`, `"position"`).
    pub property_name: String,
    /// The new value of the property.
    pub new_value: PropertyResult,
    /// The previous value of the property.
    pub old_value: PropertyResult,
}

/// The names of all properties registered on every Entity instance.
///
/// Port of the `Entity` constructor `_propertyNames` array (built-in
/// subset; graphics not yet ported to the Rust value model are omitted).
const BUILTIN_PROPERTY_NAMES: &[&str] = &[
    "billboard",
    "description",
    "label",
    "model",
    "orientation",
    "point",
    "polygon",
    "polyline",
    "position",
    "properties",
    "rectangle",
    "viewFrom",
];

/// Reserved (non-property) member names of the Rust `Entity` value model.
///
/// DEVIATION: CesiumJS checks `propertyName in this` against the live
/// object; the Rust port checks against the static list of built-in fields.
/// See docs/deviations.md.
const RESERVED_PROPERTY_NAMES: &[&str] = &[
    "id",
    "name",
    "show",
    "availability",
    "parent",
    "children",
    "definitionChanged",
    "propertyNames",
];

fn cartesian3_to_result(value: &Option<Cartesian3>) -> PropertyResult {
    match value {
        Some(v) => PropertyResult::Cartesian3(v.x, v.y, v.z),
        None => PropertyResult::None,
    }
}

fn option_string_to_result(value: &Option<String>) -> PropertyResult {
    match value {
        Some(v) => PropertyResult::String(v.clone()),
        None => PropertyResult::None,
    }
}

/// An entity is a visual element in the scene with associated properties.
///
/// Entities are the primary way to add visual elements to a scene.
/// They can represent points, labels, billboards, models, polylines,
/// polygons, and more.
///
/// In CesiumJS, Entity.js is ~1200 lines with full property change tracking,
/// availability intervals, and merge/clone semantics.
///
/// DEVIATION: CesiumJS property descriptors intercept assignment and raise
/// `definitionChanged`; the Rust port keeps the fields public for the
/// existing pipeline code and provides `set_*` mutators that raise the
/// event. Direct field writes bypass the event (callers should prefer the
/// setters). The JS `show`/`parent` setters also cascade `isShowing`
/// updates to children; the Rust value model has no child hierarchy, so the
/// cascade is omitted. See docs/deviations.md.
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
    /// The rectangle graphics.
    pub rectangle: Option<RectangleGraphics>,
    /// The model graphics.
    pub model: Option<ModelGraphics>,
    /// The parent entity ID.
    pub parent_id: Option<String>,
    /// Arbitrary user-defined properties.
    pub properties: PropertyBag,
    /// The availability intervals for this entity (mirrors a
    /// `TimeIntervalCollection`; stored as a plain interval list in this
    /// simplified value model).
    pub availability: Vec<TimeInterval>,
    /// The preferred view offset when framing this entity (mirrors
    /// `viewFrom`, constant subset of the CZML value model).
    pub view_from: Option<Cartesian3>,
    /// Fired when a property or sub-property changes.
    ///
    /// Port of `Entity.prototype.definitionChanged`.
    pub definition_changed: Event<EntityDefinitionChangedArgs>,
    /// The names of all properties registered on this instance
    /// (port of `_propertyNames`).
    property_names: Vec<String>,
    /// Values of custom properties added with [`Entity::add_property`]
    /// (Rust stand-in for dynamically defined own properties).
    extra_properties: HashMap<String, PropertyResult>,
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
            rectangle: None,
            model: None,
            parent_id: None,
            properties: PropertyBag::new(),
            availability: Vec::new(),
            view_from: None,
            definition_changed: Event::new(),
            property_names: BUILTIN_PROPERTY_NAMES
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            extra_properties: HashMap::new(),
        }
    }

    /// Raises [`Entity::definition_changed`] with the given property name
    /// and old/new values.
    fn raise_definition_changed(
        &self,
        property_name: &str,
        new_value: PropertyResult,
        old_value: PropertyResult,
    ) {
        self.definition_changed.raise_event(&EntityDefinitionChangedArgs {
            property_name: property_name.to_string(),
            new_value,
            old_value,
        });
    }

    /// Gets the names of all properties registered on this instance
    /// (port of the `propertyNames` getter).
    pub fn property_names(&self) -> &[String] {
        &self.property_names
    }

    /// Sets the name of the entity.
    ///
    /// Port of the `name` setter created by `createRawPropertyDescriptor`:
    /// raises `definitionChanged` only when the value differs.
    pub fn set_name(&mut self, value: Option<String>) {
        if self.name != value {
            let old_value = option_string_to_result(&self.name);
            self.name = value;
            self.raise_definition_changed(
                "name",
                option_string_to_result(&self.name),
                old_value,
            );
        }
    }

    /// Sets whether this entity should be displayed.
    ///
    /// Port of the `show` setter: raises `definitionChanged(this, "show",
    /// value, !value)` when the value changes.
    ///
    /// DEVIATION: the CesiumJS setter also propagates `isShowing` events to
    /// children; the Rust value model has no child hierarchy.
    /// See docs/deviations.md.
    pub fn set_show(&mut self, value: bool) {
        if value == self.show {
            return;
        }
        self.show = value;
        self.raise_definition_changed(
            "show",
            PropertyResult::Boolean(value),
            PropertyResult::Boolean(!value),
        );
    }

    /// Sets the description of the entity (raw descriptor semantics:
    /// raises `definitionChanged` only when the value differs).
    pub fn set_description(&mut self, value: Option<String>) {
        if self.description != value {
            let old_value = option_string_to_result(&self.description);
            self.description = value;
            self.raise_definition_changed(
                "description",
                option_string_to_result(&self.description),
                old_value,
            );
        }
    }

    /// Sets the position of the entity (raw descriptor semantics: raises
    /// `definitionChanged` only when the value differs).
    pub fn set_position(&mut self, value: Option<Cartesian3>) {
        if self.position != value {
            let old_value = cartesian3_to_result(&self.position);
            self.position = value;
            self.raise_definition_changed(
                "position",
                cartesian3_to_result(&self.position),
                old_value,
            );
        }
    }

    /// Sets the orientation of the entity (raises `definitionChanged` only
    /// when the value differs).
    pub fn set_orientation(&mut self, value: Option<cesium_core::quaternion::Quaternion>) {
        if self.orientation != value {
            let old_value = self.orientation.map_or(PropertyResult::None, |q| {
                PropertyResult::Quaternion(q.x, q.y, q.z, q.w)
            });
            self.orientation = value;
            let new_value = self.orientation.map_or(PropertyResult::None, |q| {
                PropertyResult::Quaternion(q.x, q.y, q.z, q.w)
            });
            self.raise_definition_changed("orientation", new_value, old_value);
        }
    }

    /// Sets the parent entity ID.
    ///
    /// Port of the `parent` setter (value-model subset: raises
    /// `definitionChanged(this, "parent", value, oldValue)` when the parent
    /// changes). DEVIATION: the JS setter maintains a child list and
    /// cascades `isShowing`; the Rust value model stores only the parent
    /// id. See docs/deviations.md.
    pub fn set_parent_id(&mut self, value: Option<String>) {
        if self.parent_id != value {
            let old_value = option_string_to_result(&self.parent_id);
            self.parent_id = value;
            self.raise_definition_changed(
                "parent",
                option_string_to_result(&self.parent_id),
                old_value,
            );
        }
    }

    /// Sets the suggested view offset (raises `definitionChanged` only when
    /// the value differs).
    pub fn set_view_from(&mut self, value: Option<Cartesian3>) {
        if self.view_from != value {
            let old_value = cartesian3_to_result(&self.view_from);
            self.view_from = value;
            self.raise_definition_changed(
                "viewFrom",
                cartesian3_to_result(&self.view_from),
                old_value,
            );
        }
    }

    /// Sets the billboard graphics (property descriptor semantics: raises
    /// `definitionChanged` when the graphics object is replaced).
    pub fn set_billboard(&mut self, value: Option<BillboardGraphics>) {
        // DEVIATION: graphics objects are compared by replacement only
        // (no `equals` in the Rust value model); the old/new payload is
        // `PropertyResult::None`. See docs/deviations.md.
        let changed = match (&self.billboard, &value) {
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.billboard = value;
            self.raise_definition_changed("billboard", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the label graphics (raises `definitionChanged` on replacement).
    pub fn set_label(&mut self, value: Option<LabelGraphics>) {
        let changed = !matches!((&self.label, &value), (None, None));
        if changed {
            self.label = value;
            self.raise_definition_changed("label", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the point graphics (raises `definitionChanged` on replacement).
    pub fn set_point(&mut self, value: Option<PointGraphics>) {
        let changed = !matches!((&self.point, &value), (None, None));
        if changed {
            self.point = value;
            self.raise_definition_changed("point", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the polyline graphics (raises `definitionChanged` on replacement).
    pub fn set_polyline(&mut self, value: Option<PolylineGraphics>) {
        let changed = !matches!((&self.polyline, &value), (None, None));
        if changed {
            self.polyline = value;
            self.raise_definition_changed("polyline", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the polygon graphics (raises `definitionChanged` on replacement).
    pub fn set_polygon(&mut self, value: Option<PolygonGraphics>) {
        let changed = !matches!((&self.polygon, &value), (None, None));
        if changed {
            self.polygon = value;
            self.raise_definition_changed("polygon", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the rectangle graphics (raises `definitionChanged` on replacement).
    pub fn set_rectangle(&mut self, value: Option<RectangleGraphics>) {
        let changed = !matches!((&self.rectangle, &value), (None, None));
        if changed {
            self.rectangle = value;
            self.raise_definition_changed("rectangle", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Sets the model graphics (raises `definitionChanged` on replacement).
    pub fn set_model(&mut self, value: Option<ModelGraphics>) {
        let changed = !matches!((&self.model, &value), (None, None));
        if changed {
            self.model = value;
            self.raise_definition_changed("model", PropertyResult::None, PropertyResult::None);
        }
    }

    /// Adds a property to this object. Once a property is added, it can be
    /// observed with [`Entity::definition_changed`].
    ///
    /// Port of `Entity.prototype.addProperty` (the debug-only checks are
    /// gated on `debug_assertions` per the porting conventions).
    pub fn add_property(&mut self, property_name: &str) {
        // >>includeStart('debug', pragmas.debug)
        #[cfg(debug_assertions)]
        {
            if self.property_names.iter().any(|name| name == property_name) {
                throw_developer_error(&format!(
                    "{property_name} is already a registered property."
                ));
            }
            if RESERVED_PROPERTY_NAMES.iter().any(|name| *name == property_name) {
                throw_developer_error(&format!(
                    "{property_name} is a reserved property name."
                ));
            }
        }
        // >>includeEnd('debug');

        self.property_names.push(property_name.to_string());
        self.extra_properties
            .entry(property_name.to_string())
            .or_insert(PropertyResult::None);
    }

    /// Removes a property previously added with [`Entity::add_property`].
    ///
    /// Port of `Entity.prototype.removeProperty`.
    pub fn remove_property(&mut self, property_name: &str) {
        let index = self
            .property_names
            .iter()
            .position(|name| name == property_name);

        // >>includeStart('debug', pragmas.debug)
        #[cfg(debug_assertions)]
        {
            if index.is_none() {
                throw_developer_error(&format!(
                    "{property_name} is not a registered property."
                ));
            }
        }
        // >>includeEnd('debug');

        if let Some(index) = index {
            self.property_names.remove(index);
            self.extra_properties.remove(property_name);
        }
    }

    /// Gets the value of a custom property added with
    /// [`Entity::add_property`].
    pub fn get_custom_property(&self, property_name: &str) -> Option<&PropertyResult> {
        self.extra_properties.get(property_name)
    }

    /// Sets the value of a custom property (raw descriptor semantics:
    /// raises `definitionChanged` only when the value differs).
    pub fn set_custom_property(&mut self, property_name: &str, value: PropertyResult) {
        let old_value = self
            .extra_properties
            .get(property_name)
            .cloned()
            .unwrap_or(PropertyResult::None);
        if old_value != value {
            self.extra_properties
                .insert(property_name.to_string(), value.clone());
            self.raise_definition_changed(property_name, value, old_value);
        }
    }

    /// Merges another entity's properties into this one.
    ///
    /// In CesiumJS, `Entity.merge(source)` copies all defined properties
    /// from the source entity to this entity. Existing values are
    /// overwritten. Mutations go through the `set_*` mutators so that
    /// `definitionChanged` is raised for every effective change (CesiumJS
    /// merge assigns through the property descriptors).
    pub fn merge(&mut self, other: &Entity) {
        if let Some(ref name) = other.name {
            self.set_name(Some(name.clone()));
        }
        if other.show {
            self.set_show(true);
        }
        if let Some(ref desc) = other.description {
            self.set_description(Some(desc.clone()));
        }
        if let Some(ref pos) = other.position {
            self.set_position(Some(*pos));
        }
        if let Some(ref orient) = other.orientation {
            self.set_orientation(Some(*orient));
        }
        if other.billboard.is_some() {
            self.set_billboard(other.billboard.clone());
        }
        if other.label.is_some() {
            self.set_label(other.label.clone());
        }
        if other.point.is_some() {
            self.set_point(other.point.clone());
        }
        if other.polyline.is_some() {
            self.set_polyline(other.polyline.clone());
        }
        if other.polygon.is_some() {
            self.set_polygon(other.polygon.clone());
        }
        if other.rectangle.is_some() {
            self.set_rectangle(other.rectangle.clone());
        }
        if other.model.is_some() {
            self.set_model(other.model.clone());
        }
        // Merge user-defined properties
        for key in other.properties.keys() {
            if let Some(val) = other.properties.get(key) {
                self.properties.set(key, val.clone());
            }
        }
        if !other.availability.is_empty() {
            self.availability = other.availability.clone();
        }
        if let Some(ref view_from) = other.view_from {
            self.set_view_from(Some(*view_from));
        }
    }

    /// Returns whether this entity is available at the given time.
    ///
    /// In CesiumJS, this checks the `availability` TimeIntervalCollection;
    /// an entity without availability is available at all times.
    pub fn is_available(&self, time: &JulianDate) -> bool {
        if self.availability.is_empty() {
            return true; // No availability constraint
        }
        self.availability.iter().any(|interval| interval.contains(time))
    }

    /// Returns whether this entity has any visual representation.
    pub fn has_visuals(&self) -> bool {
        self.billboard.is_some()
            || self.label.is_some()
            || self.point.is_some()
            || self.polyline.is_some()
            || self.polygon.is_some()
            || self.rectangle.is_some()
            || self.model.is_some()
    }
}
