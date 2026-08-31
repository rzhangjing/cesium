//! Ported from `packages/engine/Source/DataSources/ReferenceProperty.js`.
//!
//! A [`Property`] which transparently links to another property on an
//! entity in a provided [`crate::entity_collection::EntityCollection`].

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use cesium_core::event::{Event, RemoveCallback};

use crate::entity::Entity;
use crate::entity_collection::{CollectionChangedArgs, EntityCollection};
use crate::property::{Property, PropertyResult};

/// A [`Property`] which transparently links to another property on a
/// provided entity collection.
///
/// DEVIATION (structural): CesiumJS holds a direct `EntityCollection`
/// reference and resolves to the live target `Property` instance; the Rust
/// port keeps a weak handle (the collection owns its entities) and resolves
/// against the entity value model on every lookup instead of caching the
/// target property instance.
pub struct ReferenceProperty {
    target_collection: Weak<RefCell<EntityCollection>>,
    target_id: String,
    target_property_names: Vec<String>,
    definition_changed: Rc<Event<()>>,
    /// Mirrors the JS `_targetEntity` defined/undefined cache flag.
    target_resolved: Rc<Cell<bool>>,
    _collection_subscription: Option<RemoveCallback<CollectionChangedArgs>>,
    target_subscription: RefCell<Option<RemoveCallback<crate::entity::EntityDefinitionChangedArgs>>>,
}

impl ReferenceProperty {
    /// Port of `new ReferenceProperty(targetCollection, targetId, targetPropertyNames)`.
    ///
    /// # Panics
    ///
    /// Debug builds panic with a `DeveloperError` when `target_id` is empty
    /// or `target_property_names` is empty / contains empty names.
    pub fn new(
        target_collection: &Rc<RefCell<EntityCollection>>,
        target_id: &str,
        target_property_names: Vec<String>,
    ) -> Self {
        if cfg!(debug_assertions) {
            if target_id.is_empty() {
                panic!("DeveloperError: targetId is required.");
            }
            if target_property_names.is_empty() {
                panic!("DeveloperError: targetPropertyNames is required.");
            }
            for item in &target_property_names {
                if item.is_empty() {
                    panic!("DeveloperError: reference contains invalid properties.");
                }
            }
        }

        let definition_changed = Rc::new(Event::new());
        let target_resolved = Rc::new(Cell::new(false));

        // Port of `_onCollectionChanged` subscription.
        let collection_subscription = {
            let target_id = target_id.to_string();
            let definition_changed = Rc::clone(&definition_changed);
            let target_resolved = Rc::clone(&target_resolved);
            let weak_collection = Rc::downgrade(target_collection);
            target_collection.borrow().collection_changed().add_listener(move |args| {
                reference_on_collection_changed(
                    args,
                    &target_id,
                    &target_resolved,
                    &definition_changed,
                    &weak_collection,
                );
            })
        };

        Self {
            target_collection: Rc::downgrade(target_collection),
            target_id: target_id.to_string(),
            target_property_names,
            definition_changed,
            target_resolved,
            _collection_subscription: Some(collection_subscription),
            target_subscription: RefCell::new(None),
        }
    }

    /// Port of `ReferenceProperty.fromString(targetCollection, referenceString)`.
    ///
    /// The format of the string is `"objectId#foo.bar"`, where `#` separates
    /// the id from the property path and `.` separates sub-properties. If
    /// the reference identifier or any sub-property contains a `#`, `.` or
    /// `\`, it must be escaped with a backslash.
    ///
    /// Returns `None` when `reference_string` is empty (the JS
    /// `DeveloperError` for a missing string is debug-gated; an empty
    /// string yields an unusable identifier).
    pub fn from_string(
        target_collection: &Rc<RefCell<EntityCollection>>,
        reference_string: &str,
    ) -> Option<Self> {
        if reference_string.is_empty() {
            return None;
        }

        let mut identifier: Option<String> = None;
        let mut values: Vec<String> = Vec::new();

        let mut in_identifier = true;
        let mut is_escaped = false;
        let mut token = String::new();
        for c in reference_string.chars() {
            if is_escaped {
                token.push(c);
                is_escaped = false;
            } else if c == '\\' {
                is_escaped = true;
            } else if in_identifier && c == '#' {
                identifier = Some(std::mem::take(&mut token));
                in_identifier = false;
            } else if !in_identifier && c == '.' {
                values.push(std::mem::take(&mut token));
            } else {
                token.push(c);
            }
        }
        values.push(token);

        identifier.map(|identifier| Self::new(target_collection, &identifier, values))
    }

    /// Port of the `targetId` getter.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Port of the `targetPropertyNames` getter.
    pub fn target_property_names(&self) -> &[String] {
        &self.target_property_names
    }

    /// Port of the `resolvedProperty` getter: resolves the referenced
    /// property and returns its current value in the entity value model.
    pub fn resolved_value(&self) -> Option<PropertyResult> {
        let collection = self.target_collection.upgrade()?;
        let collection = collection.borrow();
        let entity = collection.get_by_id(&self.target_id)?;

        self.ensure_entity_subscription(entity);
        self.target_resolved.set(true);

        entity_walk(entity, &self.target_property_names)
    }

    /// Port of `getValueInReferenceFrame`: in the Rust value model the
    /// referenced values are already expressed in the fixed frame, so this
    /// returns the resolved value unchanged when it is a position.
    ///
    /// DEVIATION: CesiumJS delegates to the target property's own
    /// `getValueInReferenceFrame` (with frame conversion); the value model
    /// only supports the fixed frame.
    pub fn get_value_in_reference_frame(&self, _time: f64) -> Option<PropertyResult> {
        match self.resolved_value()? {
            value @ PropertyResult::Position(_, _, _) | value @ PropertyResult::Cartesian3(_, _, _) => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Port of `getType(time)`: only meaningful for material references in
    /// CesiumJS; the entity value model has no material type metadata.
    pub fn get_type(&self, _time: f64) -> Option<String> {
        None
    }

    /// Subscribes to the target entity's `definitionChanged` event the
    /// first time the entity resolves (port of the subscription inside
    /// `resolve`).
    fn ensure_entity_subscription(&self, entity: &Entity) {
        if self.target_subscription.borrow().is_some() {
            return;
        }
        let first_name = match self.target_property_names.first() {
            Some(name) => name.clone(),
            None => return,
        };
        let definition_changed = Rc::clone(&self.definition_changed);
        let remove = entity.definition_changed.add_listener(move |args| {
            // Port of `_onTargetEntityDefinitionChanged`: only changes to
            // the first property in the path invalidate the reference.
            if args.property_name == first_name {
                definition_changed.raise_event(&());
            }
        });
        *self.target_subscription.borrow_mut() = Some(remove);
    }

    /// Whether the target collection is the same collection as `other`'s
    /// (port of the JS `this._targetCollection !== other._targetCollection`
    /// identity check).
    fn same_collection(&self, other: &ReferenceProperty) -> bool {
        self.target_collection.ptr_eq(&other.target_collection)
    }
}

/// Port of `ReferenceProperty.prototype._onCollectionChanged`.
///
/// DEVIATION (defensive): CesiumJS re-queries the collection via
/// `this._targetCollection.getById(targetId)` inside the `added` branch;
/// the Rust `collectionChanged` listeners fire while the collection's
/// `RefCell` borrow is still held, so the resolution is driven purely
/// from the `added` id list (which carries the same information).
fn reference_on_collection_changed(
    args: &CollectionChangedArgs,
    target_id: &str,
    target_resolved: &Rc<Cell<bool>>,
    definition_changed: &Rc<Event<()>>,
    _weak_collection: &Weak<RefCell<EntityCollection>>,
) {
    if args.removed.iter().any(|id| id == target_id) {
        // JS additionally unsubscribes from the entity's
        // `definitionChanged` here; the Rust subscription is owned by the
        // property and stays dormant while the entity is absent.
        target_resolved.set(false);
    } else if !target_resolved.get() && args.added.iter().any(|id| id == target_id) {
        target_resolved.set(true);
        definition_changed.raise_event(&());
    }
}

/// Walks the entity value model along the property name path (Rust stand-in
/// for the JS `targetProperty = targetProperty[targetPropertyNames[i]]`
/// member chain).
///
/// DEVIATION: CesiumJS walks live object members; the Rust value model maps
/// a fixed set of entity/graphics fields onto [`PropertyResult`].
fn entity_walk(entity: &Entity, names: &[String]) -> Option<PropertyResult> {
    let first = names.first()?.as_str();
    match first {
        "position" => {
            let position = entity.position?;
            Some(PropertyResult::Cartesian3(position.x, position.y, position.z))
        }
        "name" => entity.name.clone().map(PropertyResult::String),
        "description" => entity.description.clone().map(PropertyResult::String),
        "show" => Some(PropertyResult::Boolean(entity.show)),
        "viewFrom" => entity
            .view_from
            .map(|v| PropertyResult::Cartesian3(v.x, v.y, v.z)),
        "properties" => {
            let name = names.get(1)?;
            entity.properties.get(name).cloned()
        }
        "billboard" => billboard_walk(entity.billboard.as_ref()?, &names[1..]),
        "label" => label_walk(entity.label.as_ref()?, &names[1..]),
        "point" => point_walk(entity.point.as_ref()?, &names[1..]),
        "model" => model_walk(entity.model.as_ref()?, &names[1..]),
        "polyline" => polyline_walk(entity.polyline.as_ref()?, &names[1..]),
        "polygon" => polygon_walk(entity.polygon.as_ref()?, &names[1..]),
        _ => None,
    }
}

fn billboard_walk(graphics: &crate::billboard_graphics::BillboardGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "image" => graphics.image.clone().map(PropertyResult::String),
        "scale" => Some(PropertyResult::Number(graphics.scale)),
        "rotation" => Some(PropertyResult::Number(graphics.rotation)),
        "color" => graphics.color.map(|c| PropertyResult::Color(c.red, c.green, c.blue, c.alpha)),
        "horizontalOrigin" => Some(PropertyResult::Number(graphics.horizontal_origin as f64)),
        "verticalOrigin" => Some(PropertyResult::Number(graphics.vertical_origin as f64)),
        "width" => graphics.width.map(PropertyResult::Number),
        "height" => graphics.height.map(PropertyResult::Number),
        "pixelOffset" => graphics
            .pixel_offset
            .map(|(x, y)| PropertyResult::Cartesian3(x, y, 0.0)),
        "eyeOffset" => graphics
            .eye_offset
            .map(|v| PropertyResult::Cartesian3(v.x, v.y, v.z)),
        "alignedAxis" => graphics
            .aligned_axis
            .map(|v| PropertyResult::Cartesian3(v.x, v.y, v.z)),
        "sizeInMeters" => graphics.size_in_meters.map(PropertyResult::Boolean),
        "heightReference" => Some(PropertyResult::Number(graphics.height_reference as f64)),
        _ => None,
    }
}

fn label_walk(graphics: &crate::label_graphics::LabelGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "text" => graphics.text.clone().map(PropertyResult::String),
        "font" => graphics.font.clone().map(PropertyResult::String),
        "scale" => Some(PropertyResult::Number(graphics.scale)),
        "style" => Some(PropertyResult::Number(graphics.style as f64)),
        "fillColor" => Some(PropertyResult::Color(
            graphics.fill_color.red,
            graphics.fill_color.green,
            graphics.fill_color.blue,
            graphics.fill_color.alpha,
        )),
        "outlineColor" => Some(PropertyResult::Color(
            graphics.outline_color.red,
            graphics.outline_color.green,
            graphics.outline_color.blue,
            graphics.outline_color.alpha,
        )),
        "outlineWidth" => Some(PropertyResult::Number(graphics.outline_width)),
        "horizontalOrigin" => Some(PropertyResult::Number(graphics.horizontal_origin as f64)),
        "verticalOrigin" => Some(PropertyResult::Number(graphics.vertical_origin as f64)),
        _ => None,
    }
}

fn point_walk(graphics: &crate::point_graphics::PointGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "pixelSize" => Some(PropertyResult::Number(graphics.pixel_size)),
        "color" => Some(PropertyResult::Color(
            graphics.color.red,
            graphics.color.green,
            graphics.color.blue,
            graphics.color.alpha,
        )),
        "outlineColor" => Some(PropertyResult::Color(
            graphics.outline_color.red,
            graphics.outline_color.green,
            graphics.outline_color.blue,
            graphics.outline_color.alpha,
        )),
        "outlineWidth" => Some(PropertyResult::Number(graphics.outline_width)),
        "heightReference" => Some(PropertyResult::Number(graphics.height_reference as f64)),
        _ => None,
    }
}

fn model_walk(graphics: &crate::model_graphics::ModelGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "uri" => graphics.uri.clone().map(PropertyResult::String),
        "scale" => Some(PropertyResult::Number(graphics.scale)),
        "minimumPixelSize" => Some(PropertyResult::Number(graphics.minimum_pixel_size)),
        "maximumScale" => Some(PropertyResult::Number(graphics.maximum_scale)),
        "showOutline" => Some(PropertyResult::Boolean(graphics.show_outline)),
        "shadows" => Some(PropertyResult::Number(graphics.shadows as f64)),
        _ => None,
    }
}

fn polyline_walk(graphics: &crate::polyline_graphics::PolylineGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "width" => Some(PropertyResult::Number(graphics.width)),
        "clampToGround" => Some(PropertyResult::Boolean(graphics.clamp_to_ground)),
        "materialColor" => Some(PropertyResult::Color(
            graphics.material_color.red,
            graphics.material_color.green,
            graphics.material_color.blue,
            graphics.material_color.alpha,
        )),
        _ => None,
    }
}

fn polygon_walk(graphics: &crate::polygon_graphics::PolygonGraphics, names: &[String]) -> Option<PropertyResult> {
    let name = names.first()?.as_str();
    match name {
        "show" => Some(PropertyResult::Boolean(graphics.show)),
        "height" => graphics.height.map(PropertyResult::Number),
        "extrudedHeight" => graphics.extruded_height.map(PropertyResult::Number),
        "fill" => Some(PropertyResult::Boolean(graphics.fill)),
        "outline" => Some(PropertyResult::Boolean(graphics.outline)),
        "outlineWidth" => Some(PropertyResult::Number(graphics.outline_width)),
        "materialColor" => Some(PropertyResult::Color(
            graphics.material_color.red,
            graphics.material_color.green,
            graphics.material_color.blue,
            graphics.material_color.alpha,
        )),
        _ => None,
    }
}

impl Property for ReferenceProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        self.resolved_value().unwrap_or(PropertyResult::None)
    }

    fn is_constant(&self) -> bool {
        // JS `Property.isConstant(resolve(this))`; the Rust entity value
        // model stores constant values only.
        true
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        let Some(other) = other
            .as_any()
            .and_then(|any| any.downcast_ref::<ReferenceProperty>())
        else {
            return false;
        };
        self.same_collection(other)
            && self.target_id == other.target_id
            && self.target_property_names == other.target_property_names
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
