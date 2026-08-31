//! Ported from `packages/engine/Source/DataSources/VelocityOrientationProperty.js`.
//!
//! A [`Property`] which evaluates to a [`Quaternion`] rotation based on the
//! velocity of the provided [`PositionProperty`].

use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::{Event, RemoveCallback};
use cesium_core::matrix3::Matrix3;
use cesium_core::quaternion::Quaternion;
use cesium_core::transforms;

use crate::position_property::PositionProperty;
use crate::property::{property_equals, Property, PropertyResult};
use crate::velocity_vector_property::VelocityVectorProperty;

/// A [`Property`] which evaluates to a [`Quaternion`] rotation based on the
/// velocity of the provided [`PositionProperty`].
pub struct VelocityOrientationProperty {
    velocity_vector_property: VelocityVectorProperty,
    subscription: Option<RemoveCallback<()>>,
    ellipsoid: Ellipsoid,
    definition_changed: Rc<Event<()>>,
}

impl VelocityOrientationProperty {
    /// Port of `new VelocityOrientationProperty(position, ellipsoid)`.
    /// `ellipsoid` defaults to WGS84 (JS `Ellipsoid.default`) when `None`.
    pub fn new(position: Option<Box<dyn PositionProperty>>, ellipsoid: Option<Ellipsoid>) -> Self {
        let velocity_vector_property = VelocityVectorProperty::new(position, Some(true));
        let definition_changed = Rc::new(Event::new());

        let mut result = Self {
            velocity_vector_property,
            subscription: None,
            ellipsoid: Ellipsoid::WGS84,
            definition_changed,
        };

        // JS sets `this.ellipsoid = ellipsoid ?? Ellipsoid.default` through
        // the setter, raising `definitionChanged` (old value `undefined`).
        result.set_ellipsoid(ellipsoid.unwrap_or(Ellipsoid::WGS84));

        let raised = Rc::clone(&result.definition_changed);
        if let Some(event) = result.velocity_vector_property.definition_changed() {
            result.subscription = Some(event.add_listener(move |_| {
                raised.raise_event(&());
            }));
        }
        result
    }

    /// Port of the `position` getter.
    pub fn position(&self) -> Option<&dyn PositionProperty> {
        self.velocity_vector_property.position()
    }

    /// Port of the `position` setter.
    pub fn set_position(&mut self, value: Option<Box<dyn PositionProperty>>) {
        self.velocity_vector_property.set_position(value);
    }

    /// Port of the `ellipsoid` getter.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Port of the `ellipsoid` setter: raises `definitionChanged` when the
    /// value actually changes.
    pub fn set_ellipsoid(&mut self, value: Ellipsoid) {
        if self.ellipsoid.equals(&value) {
            return;
        }
        self.ellipsoid = value;
        self.definition_changed.raise_event(&());
    }

    /// Port of `getValue(time, result)`: computes the orientation
    /// quaternion at `time` from the position property's velocity, or
    /// `None` when the velocity is unavailable.
    pub fn get_value_quaternion<'a>(
        &self,
        time: f64,
        result: &'a mut Quaternion,
    ) -> Option<&'a Quaternion> {
        let mut velocity = Cartesian3::default();
        let mut position = Cartesian3::default();
        let velocity = self.velocity_vector_property.get_value_with_position(
            time,
            &mut velocity,
            Some(&mut position),
        )?;

        let mut rotation = Matrix3::default();
        transforms::rotation_matrix_from_position_velocity(
            &position,
            velocity,
            Some(&self.ellipsoid),
            &mut rotation,
        );
        Quaternion::from_rotation_matrix(&rotation, result);
        Some(result)
    }
}

impl Property for VelocityOrientationProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        let mut result = Quaternion::default();
        match self.get_value_quaternion(time, &mut result) {
            Some(q) => PropertyResult::Quaternion(q.x, q.y, q.z, q.w),
            None => PropertyResult::None,
        }
    }

    fn is_constant(&self) -> bool {
        // JS `Property.isConstant(this._velocityVectorProperty)`.
        self.velocity_vector_property.is_constant()
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        let Some(other) = other
            .as_any()
            .and_then(|any| any.downcast_ref::<VelocityOrientationProperty>())
        else {
            return false;
        };
        property_equals(
            &self.velocity_vector_property,
            &other.velocity_vector_property,
        ) && self.ellipsoid.equals(&other.ellipsoid)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
