//! Ported from `packages/engine/Source/DataSources/PositionPropertyArray.js`.
//!
//! A [`Property`] whose value is an array whose items are the computed
//! value of other [`PositionProperty`] instances.

use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;

use crate::position_property::{PositionProperty, PositionReferenceFrame};
use crate::property::{Property, PropertyResult};

/// A [`Property`] whose value is an array whose items are the computed
/// value of other [`PositionProperty`] instances.
pub struct PositionPropertyArray {
    value: Option<Vec<Rc<dyn PositionProperty>>>,
    definition_changed: Rc<Event<()>>,
    /// EventHelper removal tokens (port of `_eventHelper`): each closure
    /// unsubscribes one inner property's `definitionChanged` listener.
    event_helper_removals: Vec<Box<dyn FnMut()>>,
    reference_frame: PositionReferenceFrame,
}

impl PositionPropertyArray {
    /// Port of `new PositionPropertyArray(value, referenceFrame)` with
    /// `referenceFrame ?? ReferenceFrame.FIXED`.
    pub fn new(
        value: Option<Vec<Rc<dyn PositionProperty>>>,
        reference_frame: Option<PositionReferenceFrame>,
    ) -> Self {
        let mut this = Self {
            value: None,
            definition_changed: Rc::new(Event::new()),
            event_helper_removals: Vec::new(),
            reference_frame: reference_frame.unwrap_or(PositionReferenceFrame::Fixed),
        };
        // JS constructor: `this.setValue(value)`.
        this.set_value(value);
        this
    }

    /// Port of the `referenceFrame` getter.
    pub fn reference_frame(&self) -> PositionReferenceFrame {
        self.reference_frame
    }

    /// Port of `getValueInReferenceFrame(time, referenceFrame, result)`:
    /// evaluates every inner position property in the requested reference
    /// frame, skipping (JS) undefined item values and compacting the
    /// result. Returns `None` when the array value itself is undefined.
    pub fn get_value_in_reference_frame(
        &self,
        time: f64,
        reference_frame: PositionReferenceFrame,
    ) -> Option<Vec<Cartesian3>> {
        let value = self.value.as_ref()?;

        let mut result: Vec<Cartesian3> = Vec::with_capacity(value.len());
        for property in value {
            let mut scratch = Cartesian3::default();
            let item_value = property.get_value_in_reference_frame(time, reference_frame, &mut scratch);
            if let Some(item) = item_value {
                result.push(*item);
            }
        }
        Some(result)
    }

    /// Port of `setValue(value)`: replaces the array (JS `value.slice()`),
    /// resubscribes to every defined item's `definitionChanged` and raises
    /// this property's own `definitionChanged`.
    pub fn set_value(&mut self, value: Option<Vec<Rc<dyn PositionProperty>>>) {
        // eventHelper.removeAll()
        for removal in self.event_helper_removals.drain(..) {
            let mut removal = removal;
            removal();
        }

        if let Some(value) = value {
            for property in &value {
                if let Some(event) = property.definition_changed() {
                    let raised = Rc::clone(&self.definition_changed);
                    let remove = event.add_listener(move |_| {
                        raised.raise_event(&());
                    });
                    let id = remove.id();
                    let property = Rc::clone(property);
                    self.event_helper_removals.push(Box::new(move || {
                        if let Some(event) = property.definition_changed() {
                            event.remove_listener(id);
                        }
                    }));
                }
            }
            self.value = Some(value);
        } else {
            self.value = None;
        }
        self.definition_changed.raise_event(&());
    }

    /// Port of `equals(other)` for two [`PositionPropertyArray`] instances
    /// (mirrors `this._referenceFrame === other._referenceFrame &&
    /// Property.arrayEquals(this._value, other._value)`).
    pub fn equals_position_property_array(&self, other: &PositionPropertyArray) -> bool {
        if self.reference_frame != other.reference_frame {
            return false;
        }
        // Property.arrayEquals over the two (possibly undefined) arrays.
        match (self.value.as_ref(), other.value.as_ref()) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(left), Some(right)) => {
                left.len() == right.len()
                    && left.iter().zip(right.iter()).all(|(l, r)| {
                        crate::property::property_equals(l.as_ref(), r.as_ref())
                    })
            }
        }
    }
}

impl Default for PositionPropertyArray {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl Drop for PositionPropertyArray {
    fn drop(&mut self) {
        // eventHelper.removeAll(): unsubscribe from all inner properties.
        for removal in self.event_helper_removals.drain(..) {
            let mut removal = removal;
            removal();
        }
    }
}

impl Property for PositionPropertyArray {
    /// Port of `getValue(time, result)`: delegates to
    /// `getValueInReferenceFrame(time, ReferenceFrame.FIXED, result)`.
    ///
    /// DEVIATION: the JS return value is a `Cartesian3[]`; the Rust
    /// `PropertyResult` has no array-of-positions variant, so the fixed
    /// frame positions are returned as a JSON array of `[x, y, z]`
    /// triples (and `None` mirrors JS `undefined`).
    fn get_value(&self, time: f64) -> PropertyResult {
        match self.get_value_in_reference_frame(time, PositionReferenceFrame::Fixed) {
            Some(positions) => PropertyResult::Json(serde_json::Value::Array(
                positions
                    .iter()
                    .map(|p| {
                        serde_json::Value::Array(vec![
                            serde_json::Value::from(p.x),
                            serde_json::Value::from(p.y),
                            serde_json::Value::from(p.z),
                        ])
                    })
                    .collect(),
            )),
            None => PropertyResult::None,
        }
    }

    /// Port of the `isConstant` getter: constant when the value is
    /// undefined or every item is constant (`Property.isConstant` treats
    /// undefined items as constant).
    fn is_constant(&self) -> bool {
        match self.value.as_ref() {
            None => true,
            Some(value) => value.iter().all(|item| item.is_constant()),
        }
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<PositionPropertyArray>())
            .map(|other| self.equals_position_property_array(other))
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
