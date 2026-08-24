//! Ported from the CZML property pipeline of
//! `packages/engine/Source/DataSources/CzmlDataSource.js`.
//!
//! This module models the time-dynamic property objects that CesiumJS builds
//! while processing CZML packets (`ConstantProperty`, `SampledProperty`,
//! `TimeIntervalCollectionProperty`, `CompositeProperty`, `ReferenceProperty`)
//! as the [`CzmlProperty`] enum, plus the shared helpers `intervalFromString`,
//! `wrapPropertyInInfiniteInterval`, `convertPropertyToComposite`,
//! `removePropertyData` and `updateInterpolationSettings`.
//!
//! DEVIATION (storage): CesiumJS stores these properties directly on the
//! entity / graphics objects (`entity.box.dimensions`, ...). The Rust
//! `Entity` only carries the constant value model, so the time-dynamic
//! values live in the sidecar store owned by [`crate::czml_data_source`].
//!
//! DEVIATION (events): `definitionChanged` is not raised; the event system
//! for CZML properties is owned by a separate work item.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::extrapolation_type::ExtrapolationType;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::quaternion::Quaternion;
use cesium_core::time_interval::TimeInterval;
use serde_json::Value;

use crate::property::{Property, PropertyResult};
use crate::sampled_property::{InterpolationAlgorithmKind, PackableType, SampledProperty};

// ============================================================================
// Time conversion
// ============================================================================

/// Converts a [`JulianDate`] to the crate-wide `f64` seconds representation
/// used by [`SampledProperty`]. The mapping is monotonic and consistent with
/// `JulianDate.secondsDifference`.
pub fn julian_to_seconds(date: &JulianDate) -> f64 {
    date.day_number as f64 * 86400.0 + date.seconds_of_day
}

// ============================================================================
// CzmlPropertyType (the JS `type` constructor parameters of processProperty)
// ============================================================================

/// The CZML property value type (mirror of the JS constructor `type`
/// parameters handed to `processProperty`, see `getPropertyType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CzmlPropertyType {
    Boolean,
    Number,
    String,
    Array,
    BoundingRectangle,
    Cartesian2,
    Cartesian3,
    UnitCartesian3,
    Color,
    ArcType,
    ClassificationType,
    ColorBlendMode,
    CornerType,
    HeightReference,
    HorizontalOrigin,
    JulianDate,
    LabelStyle,
    NearFarScalar,
    DistanceDisplayCondition,
    Object,
    Quaternion,
    PathMode,
    ShadowMode,
    StripeOrientation,
    Rectangle,
    Uri,
    VerticalOrigin,
    /// The special `Rotation` type (a Number that is never unpacked).
    Rotation,
    /// The special `Image` type (alias of Uri, used by materials).
    Image,
}

impl CzmlPropertyType {
    /// Mirrors `type.packedLength ?? 1` from `processProperty`.
    pub fn packed_length(self) -> usize {
        match self {
            CzmlPropertyType::Cartesian2 => 2,
            CzmlPropertyType::DistanceDisplayCondition => 2,
            CzmlPropertyType::Cartesian3 | CzmlPropertyType::UnitCartesian3 => 3,
            CzmlPropertyType::Color
            | CzmlPropertyType::Quaternion
            | CzmlPropertyType::NearFarScalar
            | CzmlPropertyType::Rectangle
            | CzmlPropertyType::BoundingRectangle => 4,
            CzmlPropertyType::JulianDate => 2,
            _ => 1,
        }
    }

    /// The [`PackableType`] used when a sampled property of this type is
    /// created. Returns `None` for types the Rust [`SampledProperty`] does
    /// not support (sampled values of those types are skipped).
    ///
    /// DEVIATION (sampled types): CesiumJS can sample `Cartesian2`,
    /// `BoundingRectangle` and `DistanceDisplayCondition` values; the Rust
    /// `SampledProperty` has no matching packable type, so sampled
    /// definitions of those types are not ingested.
    pub fn packable_type(self) -> Option<PackableType> {
        match self {
            CzmlPropertyType::Number | CzmlPropertyType::Rotation => Some(PackableType::Number),
            CzmlPropertyType::Color => Some(PackableType::Color),
            CzmlPropertyType::Cartesian3 | CzmlPropertyType::UnitCartesian3 => {
                Some(PackableType::Cartesian3)
            }
            CzmlPropertyType::Quaternion => Some(PackableType::Quaternion),
            CzmlPropertyType::NearFarScalar => Some(PackableType::NearFarScalar),
            CzmlPropertyType::Rectangle => Some(PackableType::Rectangle),
            _ => None,
        }
    }

    /// Mirrors `typeof type.unpack === "function" && type !== Rotation`
    /// (`needsUnpacking` in `processProperty`).
    pub fn needs_unpacking(self) -> bool {
        matches!(
            self,
            CzmlPropertyType::BoundingRectangle
                | CzmlPropertyType::Cartesian2
                | CzmlPropertyType::Cartesian3
                | CzmlPropertyType::UnitCartesian3
                | CzmlPropertyType::Color
                | CzmlPropertyType::NearFarScalar
                | CzmlPropertyType::DistanceDisplayCondition
                | CzmlPropertyType::Quaternion
                | CzmlPropertyType::Rectangle
        )
    }
}

/// Mirror of `getPropertyType(czmlInterval)`: determines the property type
/// from the JSON shape of a CZML packet value. The checked keys and their
/// ordering match the JS implementation exactly.
pub fn get_property_type(czml_interval: &Value) -> CzmlPropertyType {
    match czml_interval {
        Value::Bool(_) => CzmlPropertyType::Boolean,
        Value::Number(_) => CzmlPropertyType::Number,
        Value::String(_) => CzmlPropertyType::String,
        Value::Object(map) => {
            if map.contains_key("array") {
                CzmlPropertyType::Array
            } else if map.contains_key("boolean") {
                CzmlPropertyType::Boolean
            } else if map.contains_key("boundingRectangle") {
                CzmlPropertyType::BoundingRectangle
            } else if map.contains_key("cartesian2") {
                CzmlPropertyType::Cartesian2
            } else if map.contains_key("cartesian")
                || map.contains_key("spherical")
                || map.contains_key("cartographicRadians")
                || map.contains_key("cartographicDegrees")
            {
                CzmlPropertyType::Cartesian3
            } else if map.contains_key("unitCartesian") || map.contains_key("unitSpherical") {
                CzmlPropertyType::UnitCartesian3
            } else if map.contains_key("rgba") || map.contains_key("rgbaf") {
                CzmlPropertyType::Color
            } else if map.contains_key("arcType") {
                CzmlPropertyType::ArcType
            } else if map.contains_key("classificationType") {
                CzmlPropertyType::ClassificationType
            } else if map.contains_key("colorBlendMode") {
                CzmlPropertyType::ColorBlendMode
            } else if map.contains_key("cornerType") {
                CzmlPropertyType::CornerType
            } else if map.contains_key("heightReference") {
                CzmlPropertyType::HeightReference
            } else if map.contains_key("horizontalOrigin") {
                CzmlPropertyType::HorizontalOrigin
            } else if map.contains_key("date") {
                CzmlPropertyType::JulianDate
            } else if map.contains_key("labelStyle") {
                CzmlPropertyType::LabelStyle
            } else if map.contains_key("number") {
                CzmlPropertyType::Number
            } else if map.contains_key("nearFarScalar") {
                CzmlPropertyType::NearFarScalar
            } else if map.contains_key("distanceDisplayCondition") {
                CzmlPropertyType::DistanceDisplayCondition
            } else if map.contains_key("object") || map.contains_key("value") {
                CzmlPropertyType::Object
            } else if map.contains_key("unitQuaternion") {
                CzmlPropertyType::Quaternion
            } else if map.contains_key("pathMode") {
                CzmlPropertyType::PathMode
            } else if map.contains_key("shadowMode") {
                CzmlPropertyType::ShadowMode
            } else if map.contains_key("string") {
                CzmlPropertyType::String
            } else if map.contains_key("stripeOrientation") {
                CzmlPropertyType::StripeOrientation
            } else if map.contains_key("wsen") || map.contains_key("wsenDegrees") {
                CzmlPropertyType::Rectangle
            } else if map.contains_key("uri") {
                CzmlPropertyType::Uri
            } else if map.contains_key("verticalOrigin") {
                CzmlPropertyType::VerticalOrigin
            } else {
                // fallback case
                CzmlPropertyType::Object
            }
        }
        // Arrays / null never occur as a single packet value; mirror the JS
        // fallback.
        _ => CzmlPropertyType::Object,
    }
}

// ============================================================================
// intervalFromString
// ============================================================================

/// Mirror of `intervalFromString(intervalString)`.
pub fn interval_from_string(interval_string: Option<&str>) -> Option<TimeInterval> {
    interval_string.and_then(|s| TimeInterval::from_iso8601(s, None, None))
}

// ============================================================================
// CzmlValue (the unpacked value payload)
// ============================================================================

/// An unpacked CZML value (what CesiumJS stores inside `ConstantProperty` or
/// as `TimeInterval.data`).
#[derive(Debug, Clone)]
pub enum CzmlValue {
    Boolean(bool),
    Number(f64),
    Text(String),
    Color(f64, f64, f64, f64),
    Cartesian3(Cartesian3),
    /// A normalized Cartesian3 (CZML `unitCartesian`/`unitSpherical`).
    UnitCartesian3(Cartesian3),
    Cartesian2(Cartesian2),
    Quaternion(Quaternion),
    NearFarScalar(f64, f64, f64, f64),
    DistanceDisplayCondition(f64, f64),
    Rectangle(f64, f64, f64, f64),
    BoundingRectangle(f64, f64, f64, f64),
    Date(JulianDate),
    /// A plain number array (CZML `array`, wall heights, ...).
    NumberArray(Vec<f64>),
    Cartesian3Array(Vec<Cartesian3>),
    Cartesian2Array(Vec<Cartesian2>),
    Cartesian3ArrayOfArrays(Vec<Vec<Cartesian3>>),
    ReferenceArray(Vec<String>),
    ReferenceArrayOfArrays(Vec<Vec<String>>),
    Json(Value),
}

impl CzmlValue {
    /// Converts a [`PropertyResult`] (as returned by [`SampledProperty`])
    /// into the equivalent [`CzmlValue`].
    pub fn from_property_result(result: PropertyResult) -> CzmlValue {
        match result {
            PropertyResult::Boolean(b) => CzmlValue::Boolean(b),
            PropertyResult::Number(n) => CzmlValue::Number(n),
            PropertyResult::String(s) => CzmlValue::Text(s),
            PropertyResult::Color(r, g, b, a) => CzmlValue::Color(r, g, b, a),
            PropertyResult::Position(x, y, z) | PropertyResult::Cartesian3(x, y, z) => {
                CzmlValue::Cartesian3(Cartesian3::new(x, y, z))
            }
            PropertyResult::Quaternion(x, y, z, w) => {
                CzmlValue::Quaternion(Quaternion::new(x, y, z, w))
            }
            PropertyResult::NearFarScalar(a, b, c, d) => CzmlValue::NearFarScalar(a, b, c, d),
            PropertyResult::Rectangle(a, b, c, d) => CzmlValue::Rectangle(a, b, c, d),
            PropertyResult::HeightReference(v)
            | PropertyResult::LabelStyle(v) => CzmlValue::Number(v as f64),
            PropertyResult::Origin(h, v) => CzmlValue::NumberArray(vec![h as f64, v as f64]),
            PropertyResult::Json(json) => CzmlValue::Json(json),
            PropertyResult::None => CzmlValue::Json(Value::Null),
        }
    }

    /// Returns the value as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CzmlValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as a number, if it is one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CzmlValue::Number(n) => Some(*n),
            _ => None,
        }
    }
}

// ============================================================================
// CzmlProperty
// ============================================================================

/// A property stored inside a [`CzmlProperty::Composite`] interval (mirror of
/// the `TimeInterval.data` property objects of `CompositeProperty`).
pub enum CzmlInnerProperty {
    Constant(CzmlValue),
    Sampled(SampledProperty),
    Reference(String),
}

impl std::fmt::Debug for CzmlInnerProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CzmlInnerProperty::Constant(value) => {
                f.debug_tuple("Constant").field(value).finish()
            }
            // `SampledProperty` carries no Debug implementation.
            CzmlInnerProperty::Sampled(_) => f.debug_tuple("Sampled").finish(),
            CzmlInnerProperty::Reference(reference) => {
                f.debug_tuple("Reference").field(reference).finish()
            }
        }
    }
}

impl Clone for CzmlInnerProperty {
    fn clone(&self) -> Self {
        match self {
            CzmlInnerProperty::Constant(value) => {
                CzmlInnerProperty::Constant(value.clone())
            }
            CzmlInnerProperty::Reference(reference) => {
                CzmlInnerProperty::Reference(reference.clone())
            }
            CzmlInnerProperty::Sampled(_) => {
                // DEVIATION: `SampledProperty` is not Clone; cloned
                // remainders of a partially removed sampled interval keep no
                // samples, so an empty constant is the closest
                // representation.
                CzmlInnerProperty::Constant(CzmlValue::Json(Value::Null))
            }
        }
    }
}

impl CzmlInnerProperty {
    /// Evaluates the inner property at `time`.
    pub fn get_value(&self, time: &JulianDate) -> Option<CzmlValue> {
        match self {
            CzmlInnerProperty::Constant(value) => Some(value.clone()),
            CzmlInnerProperty::Sampled(property) => property
                .get_value_option(julian_to_seconds(time))
                .map(CzmlValue::from_property_result),
            CzmlInnerProperty::Reference(_) => None,
        }
    }
}

/// An interval entry of a [`CzmlProperty::TimeIntervalCollection`] (mirror of
/// `TimeIntervalCollectionProperty.intervals` holding raw values).
#[derive(Debug, Clone)]
pub struct CzmlIntervalEntry {
    pub interval: TimeInterval,
    pub value: CzmlValue,
}

/// An interval entry of a [`CzmlProperty::Composite`] (mirror of
/// `CompositeProperty.intervals` holding property objects).
#[derive(Debug, Clone)]
pub struct CzmlCompositeEntry {
    pub interval: TimeInterval,
    pub inner: CzmlInnerProperty,
}

/// A time-dynamic CZML property.
///
/// Mirrors the family of property objects CesiumJS creates in
/// `processProperty`: `ConstantProperty`, `SampledProperty`,
/// `TimeIntervalCollectionProperty`, `CompositeProperty`, `ReferenceProperty`,
/// velocity-derived references and the `CallbackProperty` adapter used for
/// the legacy `followSurface` → `arcType` mapping.
pub enum CzmlProperty {
    /// A `ConstantProperty`.
    Constant(CzmlValue),
    /// An infinite `SampledProperty`.
    Sampled(SampledProperty),
    /// A `TimeIntervalCollectionProperty` (interval-constrained constants).
    TimeIntervalCollection(Vec<CzmlIntervalEntry>),
    /// A `CompositeProperty` (interval-constrained sampled/constant mix).
    Composite(Vec<CzmlCompositeEntry>),
    /// A `ReferenceProperty`.
    Reference(String),
    /// A `VelocityVectorProperty`/`VelocityOrientationProperty` built from a
    /// `velocityReference`. The string is the resolved reference target.
    VelocityReference(String),
    /// The `CallbackProperty` adapter created by `createAdapterProperty`
    /// (legacy `followSurface` → `arcType`).
    FollowSurfaceAdapter(Box<CzmlProperty>),
}

impl std::fmt::Debug for CzmlProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CzmlProperty::Constant(value) => f.debug_tuple("Constant").field(value).finish(),
            // `SampledProperty` carries no Debug implementation.
            CzmlProperty::Sampled(_) => f.debug_tuple("Sampled").finish(),
            CzmlProperty::TimeIntervalCollection(entries) => f
                .debug_tuple("TimeIntervalCollection")
                .field(entries)
                .finish(),
            CzmlProperty::Composite(entries) => {
                f.debug_tuple("Composite").field(entries).finish()
            }
            CzmlProperty::Reference(reference) => {
                f.debug_tuple("Reference").field(reference).finish()
            }
            CzmlProperty::VelocityReference(reference) => f
                .debug_tuple("VelocityReference")
                .field(reference)
                .finish(),
            CzmlProperty::FollowSurfaceAdapter(inner) => f
                .debug_tuple("FollowSurfaceAdapter")
                .field(inner)
                .finish(),
        }
    }
}

impl CzmlProperty {
    /// Mirror of `property.getValue(time)`; returns `None` for undefined
    /// values and unresolvable references.
    pub fn get_value(&self, time: &JulianDate) -> Option<CzmlValue> {
        match self {
            CzmlProperty::Constant(value) => Some(value.clone()),
            CzmlProperty::Sampled(property) => property
                .get_value_option(julian_to_seconds(time))
                .map(CzmlValue::from_property_result),
            CzmlProperty::TimeIntervalCollection(entries) => {
                for entry in entries {
                    if entry.interval.contains(time) {
                        return Some(entry.value.clone());
                    }
                }
                None
            }
            CzmlProperty::Composite(entries) => {
                for entry in entries {
                    if entry.interval.contains(time) {
                        return entry.inner.get_value(time);
                    }
                }
                None
            }
            CzmlProperty::Reference(_) | CzmlProperty::VelocityReference(_) => None,
            CzmlProperty::FollowSurfaceAdapter(inner) => {
                // Mirrors adaptFollowSurfaceToArcType: boolean -> ArcType.
                inner
                    .get_value(time)
                    .and_then(|value| value.as_bool())
                    .map(|follow_surface| {
                        CzmlValue::Number(if follow_surface {
                            cesium_core::arc_type::ArcType::Geodesic as i32 as f64
                        } else {
                            cesium_core::arc_type::ArcType::None as i32 as f64
                        })
                    })
            }
        }
    }

    /// Mirror of `property.isConstant`.
    pub fn is_constant(&self) -> bool {
        match self {
            CzmlProperty::Constant(_) => true,
            CzmlProperty::Sampled(property) => property.is_constant(),
            CzmlProperty::TimeIntervalCollection(entries) => {
                // Mirrors TimeIntervalCollectionProperty.isConstant.
                entries.is_empty()
                    || (entries.len() == 1
                        && TimeInterval::equals(
                            &entries[0].interval,
                            Iso8601::maximum_interval(),
                        ))
            }
            // CompositeProperty has no isConstant in CesiumJS (undefined is
            // falsy).
            CzmlProperty::Composite(_) => false,
            CzmlProperty::Reference(_)
            | CzmlProperty::VelocityReference(_)
            | CzmlProperty::FollowSurfaceAdapter(_) => false,
        }
    }
}

// ============================================================================
// wrapPropertyInInfiniteInterval / convertPropertyToComposite
// ============================================================================

/// Mirror of `wrapPropertyInInfiniteInterval`: wraps `inner` in a clone of
/// `Iso8601.MAXIMUM_INTERVAL`.
pub fn wrap_in_infinite_interval(inner: CzmlInnerProperty) -> CzmlCompositeEntry {
    CzmlCompositeEntry {
        interval: Iso8601::maximum_interval().clone(),
        inner,
    }
}

/// Converts an existing property into a [`CzmlProperty::Composite`] that
/// preserves the old data wrapped in an infinite interval (mirror of
/// `convertPropertyToComposite`).
pub fn into_composite(property: CzmlProperty) -> Vec<CzmlCompositeEntry> {
    let inner = match property {
        CzmlProperty::Constant(value) => CzmlInnerProperty::Constant(value),
        CzmlProperty::Sampled(property) => CzmlInnerProperty::Sampled(property),
        CzmlProperty::Reference(reference) => CzmlInnerProperty::Reference(reference),
        CzmlProperty::TimeIntervalCollection(entries) => {
            // A collection folded into a composite keeps its entries intact.
            return entries
                .into_iter()
                .map(|entry| CzmlCompositeEntry {
                    interval: entry.interval,
                    inner: CzmlInnerProperty::Constant(entry.value),
                })
                .collect();
        }
        CzmlProperty::Composite(entries) => return entries,
        CzmlProperty::VelocityReference(reference) => CzmlInnerProperty::Reference(reference),
        CzmlProperty::FollowSurfaceAdapter(_) => {
            // Adapters are only produced after composite handling finished;
            // folding them keeps the adapted value as a constant.
            CzmlInnerProperty::Constant(CzmlValue::Json(Value::Null))
        }
    };
    vec![wrap_in_infinite_interval(inner)]
}

// ============================================================================
// removePropertyData
// ============================================================================

/// Removes the samples/entries of `interval` from an inner composite property
/// (mirror of the recursive `removePropertyData` on `interval.data`).
fn remove_inner_property_data(inner: &mut CzmlInnerProperty, interval: &TimeInterval) {
    if let CzmlInnerProperty::Sampled(property) = inner {
        let start = julian_to_seconds(&interval.start);
        let stop = julian_to_seconds(&interval.stop);
        property.remove_samples_interval(
            start,
            stop,
            interval.is_start_included,
            interval.is_stop_included,
        );
    }
}

/// Trims or removes the entries covered by `interval` from a sorted entry
/// list, mirroring `TimeIntervalCollection.removeInterval`.
fn remove_entries(entries: &mut Vec<CzmlIntervalEntry>, interval: &TimeInterval) {
    let mut index = 0;
    while index < entries.len() {
        let intersection = TimeInterval::intersect(&entries[index].interval, interval);
        if intersection.is_empty() {
            index += 1;
            continue;
        }

        let entry_interval = entries[index].interval.clone();
        let value = entries[index].value.clone();

        // Left remainder: [entry.start, interval.start)
        let mut remainders: Vec<CzmlIntervalEntry> = Vec::new();
        let left_cmp = JulianDate::compare(&entry_interval.start, &interval.start);
        if left_cmp < 0 {
            remainders.push(CzmlIntervalEntry {
                interval: TimeInterval::new(
                    Some(entry_interval.start.clone()),
                    Some(interval.start.clone()),
                    Some(entry_interval.is_start_included),
                    Some(false),
                ),
                value: value.clone(),
            });
        }
        // Right remainder: (interval.stop, entry.stop]
        let right_cmp = JulianDate::compare(&interval.stop, &entry_interval.stop);
        if right_cmp < 0 {
            remainders.push(CzmlIntervalEntry {
                interval: TimeInterval::new(
                    Some(interval.stop.clone()),
                    Some(entry_interval.stop.clone()),
                    Some(false),
                    Some(entry_interval.is_stop_included),
                ),
                value,
            });
        }

        entries.splice(index..index + 1, remainders.iter().cloned());
        index += remainders.len();
    }
}

/// Removes `interval` data from `entries`, then removes the entries covered
/// by the interval (the `CompositeProperty` half of `removePropertyData`).
fn remove_composite_entries(entries: &mut Vec<CzmlCompositeEntry>, interval: &TimeInterval) {
    let mut index = 0;
    while index < entries.len() {
        let intersection = TimeInterval::intersect(&entries[index].interval, interval);
        if !intersection.is_empty() {
            remove_inner_property_data(&mut entries[index].inner, interval);
        }
        index += 1;
    }

    // Mirrors intervals.removeInterval(interval): drop covered intervals,
    // keep remainders for partially covered ones.
    let mut index = 0;
    while index < entries.len() {
        let intersection = TimeInterval::intersect(&entries[index].interval, interval);
        if intersection.is_empty() {
            index += 1;
            continue;
        }

        let entry_interval = entries[index].interval.clone();
        let inner = entries.splice(index..index + 1, []).next().unwrap().inner;

        let mut remainders: Vec<CzmlCompositeEntry> = Vec::new();
        if JulianDate::compare(&entry_interval.start, &interval.start) < 0 {
            remainders.push(CzmlCompositeEntry {
                interval: TimeInterval::new(
                    Some(entry_interval.start.clone()),
                    Some(interval.start.clone()),
                    Some(entry_interval.is_start_included),
                    Some(false),
                ),
                inner: clone_inner(&inner),
            });
        }
        if JulianDate::compare(&interval.stop, &entry_interval.stop) < 0 {
            remainders.push(CzmlCompositeEntry {
                interval: TimeInterval::new(
                    Some(interval.stop.clone()),
                    Some(entry_interval.stop.clone()),
                    Some(false),
                    Some(entry_interval.is_stop_included),
                ),
                inner,
            });
        }

        for (offset, remainder) in remainders.iter().enumerate() {
            entries.insert(index + offset, clone_composite_entry(remainder));
        }
        index += remainders.len();
    }
}

fn clone_inner(inner: &CzmlInnerProperty) -> CzmlInnerProperty {
    inner.clone()
}

fn clone_composite_entry(entry: &CzmlCompositeEntry) -> CzmlCompositeEntry {
    CzmlCompositeEntry {
        interval: entry.interval.clone(),
        inner: clone_inner(&entry.inner),
    }
}

/// Mirror of `removePropertyData(property, interval)`.
pub fn remove_property_data(property: Option<&mut CzmlProperty>, interval: &TimeInterval) {
    let Some(property) = property else {
        return;
    };
    match property {
        CzmlProperty::Sampled(sampled) => {
            let start = julian_to_seconds(&interval.start);
            let stop = julian_to_seconds(&interval.stop);
            sampled.remove_samples_interval(
                start,
                stop,
                interval.is_start_included,
                interval.is_stop_included,
            );
        }
        CzmlProperty::TimeIntervalCollection(entries) => {
            remove_entries(entries, interval);
        }
        CzmlProperty::Composite(entries) => {
            remove_composite_entries(entries, interval);
        }
        _ => {}
    }
}

// ============================================================================
// updateInterpolationSettings
// ============================================================================

/// Maps a CZML `interpolationAlgorithm` name to the Rust interpolation kind
/// (mirror of the `interpolators` lookup).
pub fn interpolator_from_name(name: &str) -> Option<InterpolationAlgorithmKind> {
    match name {
        "HERMITE" => Some(InterpolationAlgorithmKind::Hermite),
        "LAGRANGE" => Some(InterpolationAlgorithmKind::Lagrange),
        "LINEAR" => Some(InterpolationAlgorithmKind::Linear),
        _ => None,
    }
}

/// Maps a CZML `forwardExtrapolationType`/`backwardExtrapolationType` name.
pub fn extrapolation_type_from_name(name: &str) -> Option<ExtrapolationType> {
    match name {
        "NONE" => Some(ExtrapolationType::None),
        "HOLD" => Some(ExtrapolationType::Hold),
        "EXTRAPOLATE" => Some(ExtrapolationType::Extrapolate),
        _ => None,
    }
}

/// Mirror of `updateInterpolationSettings(packetData, property)`.
pub fn update_interpolation_settings(packet_data: &Value, property: &mut SampledProperty) {
    let interpolation_algorithm = packet_data
        .get("interpolationAlgorithm")
        .and_then(|v| v.as_str());
    let interpolation_degree = packet_data.get("interpolationDegree").and_then(|v| v.as_f64());
    if interpolation_algorithm.is_some() || interpolation_degree.is_some() {
        property.set_interpolation_options(
            interpolation_algorithm.and_then(interpolator_from_name),
            interpolation_degree.map(|degree| degree as u32),
        );
    }

    if let Some(name) = packet_data
        .get("forwardExtrapolationType")
        .and_then(|v| v.as_str())
    {
        if let Some(extrapolation_type) = extrapolation_type_from_name(name) {
            property.set_forward_extrapolation_type(extrapolation_type);
        }
    }

    if let Some(duration) = packet_data
        .get("forwardExtrapolationDuration")
        .and_then(|v| v.as_f64())
    {
        property.set_forward_extrapolation_duration(duration);
    }

    if let Some(name) = packet_data
        .get("backwardExtrapolationType")
        .and_then(|v| v.as_str())
    {
        if let Some(extrapolation_type) = extrapolation_type_from_name(name) {
            property.set_backward_extrapolation_type(extrapolation_type);
        }
    }

    if let Some(duration) = packet_data
        .get("backwardExtrapolationDuration")
        .and_then(|v| v.as_f64())
    {
        property.set_backward_extrapolation_duration(duration);
    }
}
