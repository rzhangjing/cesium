//! Ported from `packages/engine/Source/DataSources/VelocityVectorProperty.js`.
//!
//! A [`Property`] which evaluates to a [`Cartesian3`] vector based on the
//! velocity of the provided [`PositionProperty`].

use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::{Event, RemoveCallback};

use crate::position_property::PositionProperty;
use crate::property::{property_equals, Property, PropertyResult};

/// The finite-difference step (seconds) used to derive velocity from
/// position samples (JS `step = 1.0 / 60.0`).
const STEP: f64 = 1.0 / 60.0;

/// A [`Property`] which evaluates to a [`Cartesian3`] vector based on the
/// velocity of the provided [`PositionProperty`].
pub struct VelocityVectorProperty {
    position: Option<Box<dyn PositionProperty>>,
    position_subscription: Option<RemoveCallback<()>>,
    definition_changed: Rc<Event<()>>,
    normalize: bool,
}

impl VelocityVectorProperty {
    /// Port of `new VelocityVectorProperty(position, normalize)`.
    /// `normalize` defaults to `true` when `None` (JS `?? true`).
    pub fn new(position: Option<Box<dyn PositionProperty>>, normalize: Option<bool>) -> Self {
        let mut result = Self {
            position: None,
            position_subscription: None,
            definition_changed: Rc::new(Event::new()),
            normalize: normalize.unwrap_or(true),
        };
        result.set_position(position);
        result
    }

    /// Port of the `position` getter.
    pub fn position(&self) -> Option<&dyn PositionProperty> {
        self.position.as_deref()
    }

    /// Port of the `position` setter: unsubscribes from the previous
    /// position property's `definitionChanged`, subscribes to the new one,
    /// and raises this property's `definitionChanged`.
    pub fn set_position(&mut self, value: Option<Box<dyn PositionProperty>>) {
        let was_none = self.position.is_none();
        if let Some(old) = &self.position {
            if let Some(remove) = self.position_subscription.take() {
                if let Some(event) = old.definition_changed() {
                    event.remove_listener(remove.id());
                }
            }
        }

        self.position = value;

        if let Some(position) = &self.position {
            if let Some(event) = position.definition_changed() {
                let definition_changed = Rc::clone(&self.definition_changed);
                self.position_subscription = Some(event.add_listener(move |_| {
                    definition_changed.raise_event(&());
                }));
            }
        }

        // JS `oldValue !== value`: identity is not expressible for trait
        // objects; `None` -> `None` is treated as unchanged.
        if !(was_none && self.position.is_none()) {
            self.definition_changed.raise_event(&());
        }
    }

    /// Port of the `normalize` getter.
    pub fn normalize(&self) -> bool {
        self.normalize
    }

    /// Port of the `normalize` setter: raises `definitionChanged` when the
    /// value actually changes.
    pub fn set_normalize(&mut self, value: bool) {
        if self.normalize == value {
            return;
        }
        self.normalize = value;
        self.definition_changed.raise_event(&());
    }

    /// Port of the (private) `_getValue(time, velocityResult, positionResult)`.
    ///
    /// Computes the velocity vector of the wrapped position property at
    /// `time` using a centered finite difference; stores the sampled
    /// position into `position_result` when provided. Returns `None` when
    /// the velocity is unavailable (or a zero/normalized-zero vector is
    /// undefined in CesiumJS semantics).
    pub fn get_value_with_position<'a>(
        &self,
        time: f64,
        velocity_result: &'a mut Cartesian3,
        position_result: Option<&mut Cartesian3>,
    ) -> Option<&'a Cartesian3> {
        let property = self.position.as_deref()?;
        self_get_value_impl(
            property,
            self.normalize,
            time,
            velocity_result,
            position_result,
        )
    }

    /// Static form of the velocity computation (mirrors calling `_getValue`
    /// on a property with the given `position` and `normalize`).
    pub fn compute_velocity<'a>(
        position: &dyn PositionProperty,
        normalize: bool,
        time: f64,
        velocity_result: &'a mut Cartesian3,
        position_result: Option<&mut Cartesian3>,
    ) -> Option<&'a Cartesian3> {
        self_get_value_impl(position, normalize, time, velocity_result, position_result)
    }
}

/// Shared implementation of `VelocityVectorProperty.prototype._getValue`.
fn self_get_value_impl<'a>(
    property: &dyn PositionProperty,
    normalize: bool,
    time: f64,
    velocity_result: &'a mut Cartesian3,
    mut position_result: Option<&mut Cartesian3>,
) -> Option<&'a Cartesian3> {
    // JS `Property.isConstant(property)`: undefined or constant positions
    // yield no usable velocity direction.
    if property.is_constant() {
        if normalize {
            return None;
        }
        *velocity_result = Cartesian3::ZERO;
        return Some(velocity_result);
    }

    let mut position1_scratch = Cartesian3::default();
    let mut position2_scratch = Cartesian3::default();

    let mut have_position1 = property
        .position_value(time, &mut position1_scratch)
        .is_some();
    let have_position2 = property
        .position_value(time + STEP, &mut position2_scratch)
        .is_some();

    // If we don't have a position for now, return undefined.
    if !have_position1 {
        return None;
    }

    // If we don't have a position for now + step, see if we have a
    // position for now - step.
    if !have_position2 {
        position2_scratch = position1_scratch;
        have_position1 = property
            .position_value(time - STEP, &mut position1_scratch)
            .is_some();
        if !have_position1 {
            return None;
        }
    }

    if position1_scratch == position2_scratch {
        if normalize {
            return None;
        }
        *velocity_result = Cartesian3::ZERO;
        return Some(velocity_result);
    }

    if let Some(position_result) = position_result.as_deref_mut() {
        *position_result = position1_scratch;
    }

    let mut velocity = Cartesian3::default();
    Cartesian3::subtract(&position2_scratch, &position1_scratch, &mut velocity);
    if normalize {
        Cartesian3::normalize(&velocity, velocity_result);
    } else {
        Cartesian3::divide_by_scalar(&velocity, STEP, velocity_result);
    }
    Some(velocity_result)
}

impl Property for VelocityVectorProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        // JS: without a position property `getValue` returns
        // `Cartesian3.clone(Cartesian3.ZERO, result)`.
        let Some(property) = self.position.as_deref() else {
            return PropertyResult::Cartesian3(0.0, 0.0, 0.0);
        };
        let mut result = Cartesian3::default();
        match self_get_value_impl(property, self.normalize, time, &mut result, None) {
            Some(v) => PropertyResult::Cartesian3(v.x, v.y, v.z),
            None => PropertyResult::None,
        }
    }

    fn is_constant(&self) -> bool {
        // JS `Property.isConstant(this._position)`.
        match &self.position {
            None => true,
            Some(position) => position.is_constant(),
        }
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        let Some(other) = other
            .as_any()
            .and_then(|any| any.downcast_ref::<VelocityVectorProperty>())
        else {
            return false;
        };
        match (&self.position, &other.position) {
            (None, None) => true,
            (Some(left), Some(right)) => property_equals(left.as_ref(), right.as_ref()),
            _ => false,
        }
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
