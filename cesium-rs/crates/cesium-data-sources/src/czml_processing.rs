//! Ported from the CZML packet processing functions of
//! `packages/engine/Source/DataSources/CzmlDataSource.js`:
//! `processProperty`, `processPacketData`, `createReferenceProperty`,
//! `createSpecializedProperty`, `processPositionProperty`,
//! `processPositionPacketData`, `processShapePacketData`/`processShape`,
//! `processArrayPacketData`/`processArray`,
//! `processPositionArrayPacketData`/`processPositionArray`,
//! `unpackCartographicRadiansArray`/`unpackCartographicDegreesArray`,
//! `processPositionArrayOfArraysPacketData`/
//! `processPositionArrayOfArrays`, `processReferencesArrayPacketData`,
//! `processMaterialProperty`/`processMaterialPacketData` and
//! `processAlignedAxis`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::reference_frame::ReferenceFrame;
use cesium_core::time_interval::TimeInterval;
use serde_json::Value;

use crate::czml_property::{
    interval_from_string, julian_to_seconds, remove_property_data,
    update_interpolation_settings, wrap_in_infinite_interval, CzmlCompositeEntry,
    CzmlInnerProperty, CzmlIntervalEntry, CzmlProperty, CzmlPropertyType, CzmlValue,
};
use crate::czml_unwrap::{
    unpack_constant_value, unwrap_cartesian_interval, unwrap_interval, Unwrapped,
};
use crate::sampled_property::{PackableType, SampledProperty};

// ============================================================================
// Helpers
// ============================================================================

/// Computes the combined interval of `packetData.interval` and
/// `constrainedInterval` (mirrors the shared preamble of `processProperty`,
/// `processPositionProperty` and `processMaterialProperty`).
pub(crate) fn compute_combined_interval(
    packet_data: &Value,
    constrained_interval: Option<&TimeInterval>,
) -> Option<TimeInterval> {
    let mut combined = interval_from_string(packet_data.get("interval").and_then(|v| v.as_str()));
    if let Some(constrained) = constrained_interval {
        combined = Some(match combined {
            Some(combined) => TimeInterval::intersect(&combined, constrained),
            None => constrained.clone(),
        });
    }
    combined
}

/// Mirrors `hasInterval`: the interval is defined and not the maximum
/// (infinite) interval.
pub(crate) fn has_interval(combined: Option<&TimeInterval>) -> bool {
    combined.is_some_and(|interval| !TimeInterval::equals(interval, Iso8601::maximum_interval()))
}

/// Mirrors `TimeIntervalCollection.findInterval({ start, stop })` for the
/// material composite: matches by both endpoints.
pub(crate) fn endpoints_equal(interval: &TimeInterval, other: &TimeInterval) -> bool {
    JulianDate::equals(&interval.start, &other.start)
        && JulianDate::equals(&interval.stop, &other.stop)
}

// ============================================================================
// createReferenceProperty / createSpecializedProperty
// ============================================================================

/// Mirror of `createReferenceProperty(entityCollection, referenceString)`:
/// expands the `#propertyPath` shorthand with the current packet id.
pub fn create_reference_property(reference_string: &str, current_id: Option<&str>) -> CzmlProperty {
    let reference = if let Some(path) = reference_string.strip_prefix('#') {
        format!("{}{}", current_id.unwrap_or(""), path)
    } else {
        reference_string.to_string()
    };
    CzmlProperty::Reference(reference)
}

/// Expands a reference string the same way as [`create_reference_property`]
/// but returns the plain resolved string.
pub(crate) fn resolve_reference_string(reference: &str, current_id: Option<&str>) -> String {
    if let Some(path) = reference.strip_prefix('#') {
        format!("{}{}", current_id.unwrap_or(""), path)
    } else {
        reference.to_string()
    }
}

/// Mirror of `createSpecializedProperty(type, entityCollection, packetData)`.
pub fn create_specialized_property(
    r#type: CzmlPropertyType,
    packet_data: &Value,
    current_id: Option<&str>,
) -> CzmlProperty {
    if let Some(reference) = packet_data.get("reference").and_then(|v| v.as_str()) {
        return create_reference_property(reference, current_id);
    }

    if let Some(reference) = packet_data.get("velocityReference").and_then(|v| v.as_str()) {
        let reference = resolve_reference_string(reference, current_id);
        match r#type {
            CzmlPropertyType::Cartesian3
            | CzmlPropertyType::UnitCartesian3
            | CzmlPropertyType::Quaternion => return CzmlProperty::VelocityReference(reference),
            _ => {
                debug_assert!(false, "velocityReference is not valid CZML for this type");
                return CzmlProperty::Reference(reference);
            }
        }
    }

    debug_assert!(false, "packet data is not valid CZML");
    CzmlProperty::Constant(CzmlValue::Json(Value::Null))
}

// ============================================================================
// processProperty
// ============================================================================

/// Mirror of `processProperty(type, object, propertyName, packetData,
/// constrainedInterval, sourceUri, entityCollection)`: unpacks `packetData`
/// and upserts it into `slot` following the JS four-quadrant logic
/// (constant/sampled × with/without interval).
pub fn process_property(
    slot: &mut Option<CzmlProperty>,
    r#type: CzmlPropertyType,
    packet_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let combined_interval = compute_combined_interval(packet_data, constrained_interval);

    let is_value = packet_data.get("reference").is_none()
        && packet_data.get("velocityReference").is_none();
    let is_interval_constrained = has_interval(combined_interval.as_ref());

    if packet_data.get("delete").and_then(|v| v.as_bool()) == Some(true) {
        // If deleting this property for all time, simply clear it.
        if !is_interval_constrained {
            *slot = None;
            return;
        }
        // Deleting depends on the type of property we have.
        remove_property_data(slot.as_mut(), combined_interval.as_ref().unwrap());
        return;
    }

    let mut is_sampled = false;
    let mut unwrapped: Option<Unwrapped> = None;

    if is_value {
        unwrapped = unwrap_interval(r#type, packet_data, source_uri);
        let Some(ref data) = unwrapped else {
            // Not a known value type, bail.
            return;
        };
        let packed_length = r#type.packed_length();
        let unwrapped_length = match data {
            Unwrapped::Packed(packed) => packed.len(),
            _ => 1,
        };
        is_sampled = packet_data.get("array").is_none()
            && !matches!(data, Unwrapped::Text(_))
            && unwrapped_length > packed_length
            && r#type != CzmlPropertyType::Object;
    }

    // Any time a constant value is assigned, it completely blows away
    // anything else.
    if !is_sampled && !is_interval_constrained {
        if is_value {
            let value = unpack_constant_value(r#type, unwrapped.unwrap());
            *slot = Some(CzmlProperty::Constant(value));
        } else {
            *slot = Some(create_specialized_property(r#type, packet_data, current_id));
        }
        return;
    }

    let epoch = packet_data
        .get("epoch")
        .and_then(|v| v.as_str())
        .and_then(JulianDate::from_iso8601)
        .as_ref()
        .map(julian_to_seconds);

    // Without an interval, any sampled value is infinite, meaning it
    // completely replaces any non-sampled property that may exist.
    if is_sampled && !is_interval_constrained {
        if let Some(packable) = r#type.packable_type() {
            if !matches!(slot, Some(CzmlProperty::Sampled(_))) {
                *slot = Some(CzmlProperty::Sampled(SampledProperty::new(packable)));
            }
            if let (Some(CzmlProperty::Sampled(property)), Some(Unwrapped::Packed(packed))) =
                (slot.as_mut(), unwrapped)
            {
                property.add_samples_packed_array(&packed, epoch);
                update_interpolation_settings(packet_data, property);
            }
        }
        // DEVIATION (sampled types): types without a Rust PackableType are
        // not ingested as samples (see CzmlPropertyType::packable_type).
        return;
    }

    let combined_interval = combined_interval.unwrap();

    // A constant value with an interval is normally part of a
    // TimeIntervalCollection; if the current property is not one, turn it
    // into a Composite, preserving the old data with the new interval.
    if !is_sampled {
        let value: CzmlInnerProperty = if is_value {
            CzmlInnerProperty::Constant(unpack_constant_value(r#type, unwrapped.unwrap()))
        } else {
            match create_specialized_property(r#type, packet_data, current_id) {
                CzmlProperty::Reference(reference) => CzmlInnerProperty::Reference(reference),
                CzmlProperty::VelocityReference(reference) => {
                    CzmlInnerProperty::Reference(reference)
                }
                other => {
                    debug_assert!(false, "unexpected specialized property");
                    let _ = other;
                    CzmlInnerProperty::Constant(CzmlValue::Json(Value::Null))
                }
            }
        };

        let take_old = slot.take();
        match take_old {
            None => {
                if is_value {
                    if let CzmlInnerProperty::Constant(value) = value {
                        *slot = Some(CzmlProperty::TimeIntervalCollection(vec![
                            CzmlIntervalEntry {
                                interval: combined_interval,
                                value,
                            },
                        ]));
                    }
                } else {
                    *slot = Some(CzmlProperty::Composite(vec![CzmlCompositeEntry {
                        interval: combined_interval,
                        inner: value,
                    }]));
                }
            }
            Some(CzmlProperty::TimeIntervalCollection(mut entries)) if is_value => {
                if let CzmlInnerProperty::Constant(value) = value {
                    entries.push(CzmlIntervalEntry {
                        interval: combined_interval,
                        value,
                    });
                }
                *slot = Some(CzmlProperty::TimeIntervalCollection(entries));
            }
            Some(CzmlProperty::Composite(mut entries)) => {
                entries.push(CzmlCompositeEntry {
                    interval: combined_interval,
                    inner: value,
                });
                *slot = Some(CzmlProperty::Composite(entries));
            }
            Some(old) => {
                // Otherwise, create a CompositeProperty but preserve the
                // existing data.
                let mut entries = crate::czml_property::into_composite(old);
                entries.push(CzmlCompositeEntry {
                    interval: combined_interval,
                    inner: value,
                });
                *slot = Some(CzmlProperty::Composite(entries));
            }
        }
        return;
    }

    // isSampled && hasInterval
    let Some(packable) = r#type.packable_type() else {
        // DEVIATION (sampled types): unsupported packable type; skip.
        return;
    };

    let take_old = slot.take();
    let mut entries = match take_old {
        None => Vec::new(),
        Some(CzmlProperty::Composite(entries)) => entries,
        Some(old) => crate::czml_property::into_composite(old),
    };

    // Check if the interval already exists in the composite.
    let existing = entries
        .iter_mut()
        .find(|entry| TimeInterval::equals(&entry.interval, &combined_interval));
    let sampled: &mut SampledProperty = match existing {
        Some(entry) if matches!(entry.inner, CzmlInnerProperty::Sampled(_)) => {
            if let CzmlInnerProperty::Sampled(property) = &mut entry.inner {
                property
            } else {
                unreachable!()
            }
        }
        _ => {
            entries.push(CzmlCompositeEntry {
                interval: combined_interval,
                inner: CzmlInnerProperty::Sampled(SampledProperty::new(packable)),
            });
            let last = entries.last_mut().unwrap();
            if let CzmlInnerProperty::Sampled(property) = &mut last.inner {
                property
            } else {
                unreachable!()
            }
        }
    };

    if let Some(Unwrapped::Packed(packed)) = unwrapped {
        sampled.add_samples_packed_array(&packed, epoch);
    }
    update_interpolation_settings(packet_data, sampled);

    *slot = Some(CzmlProperty::Composite(entries));
}

/// Mirror of `processPacketData`: accepts either a single packet or an array
/// of interval packets.
pub fn process_packet_data(
    slot: &mut Option<CzmlProperty>,
    r#type: CzmlPropertyType,
    packet_data: Option<&Value>,
    interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };

    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_property(slot, r#type, packet, interval, source_uri, current_id);
        }
    } else {
        process_property(slot, r#type, packet_data, interval, source_uri, current_id);
    }
}

// ============================================================================
// processPositionProperty
// ============================================================================

/// The inner value of a position property interval entry.
pub enum CzmlPositionInner {
    Constant(Cartesian3),
    Sampled(SampledProperty),
    Reference(String),
}

impl std::fmt::Debug for CzmlPositionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CzmlPositionInner::Constant(value) => {
                f.debug_tuple("Constant").field(value).finish()
            }
            // `SampledProperty` carries no Debug implementation.
            CzmlPositionInner::Sampled(_) => f.debug_tuple("Sampled").finish(),
            CzmlPositionInner::Reference(reference) => {
                f.debug_tuple("Reference").field(reference).finish()
            }
        }
    }
}

impl CzmlPositionInner {
    fn get_value(&self, time: &JulianDate) -> Option<Cartesian3> {
        match self {
            CzmlPositionInner::Constant(value) => Some(*value),
            CzmlPositionInner::Sampled(property) => property
                .get_value_option(julian_to_seconds(time))
                .and_then(|result| result.as_position())
                .map(|(x, y, z)| Cartesian3::new(x, y, z)),
            CzmlPositionInner::Reference(_) => None,
        }
    }
}

/// An interval entry of a position property collection/composite.
#[derive(Debug)]
pub struct CzmlPositionIntervalEntry {
    pub interval: TimeInterval,
    pub inner: CzmlPositionInner,
}

/// A time-dynamic CZML position (mirror of `ConstantPositionProperty`,
/// `SampledPositionProperty`, `TimeIntervalCollectionPositionProperty`,
/// `CompositePositionProperty` and `ReferenceProperty` for positions).
pub enum CzmlPositionProperty {
    Constant {
        value: Cartesian3,
        reference_frame: ReferenceFrame,
    },
    Sampled {
        property: SampledProperty,
        reference_frame: ReferenceFrame,
        number_of_derivatives: u32,
    },
    TimeIntervalCollection {
        entries: Vec<CzmlPositionIntervalEntry>,
        reference_frame: ReferenceFrame,
    },
    Composite {
        entries: Vec<CzmlPositionIntervalEntry>,
        reference_frame: ReferenceFrame,
    },
    Reference(String),
}

impl std::fmt::Debug for CzmlPositionProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CzmlPositionProperty::Constant { value, reference_frame } => f
                .debug_struct("Constant")
                .field("value", value)
                .field("reference_frame", reference_frame)
                .finish(),
            // `SampledProperty` carries no Debug implementation.
            CzmlPositionProperty::Sampled { reference_frame, number_of_derivatives, .. } => f
                .debug_struct("Sampled")
                .field("reference_frame", reference_frame)
                .field("number_of_derivatives", number_of_derivatives)
                .finish(),
            CzmlPositionProperty::TimeIntervalCollection { entries, reference_frame } => f
                .debug_struct("TimeIntervalCollection")
                .field("entries", entries)
                .field("reference_frame", reference_frame)
                .finish(),
            CzmlPositionProperty::Composite { entries, reference_frame } => f
                .debug_struct("Composite")
                .field("entries", entries)
                .field("reference_frame", reference_frame)
                .finish(),
            CzmlPositionProperty::Reference(reference) => {
                f.debug_tuple("Reference").field(reference).finish()
            }
        }
    }
}

impl CzmlPositionProperty {
    /// Evaluates the position at `time` (mirror of `getValue`).
    pub fn get_value(&self, time: &JulianDate) -> Option<Cartesian3> {
        match self {
            CzmlPositionProperty::Constant { value, .. } => Some(*value),
            CzmlPositionProperty::Sampled { property, .. } => property
                .get_value_option(julian_to_seconds(time))
                .and_then(|result| result.as_position())
                .map(|(x, y, z)| Cartesian3::new(x, y, z)),
            CzmlPositionProperty::TimeIntervalCollection { entries, .. }
            | CzmlPositionProperty::Composite { entries, .. } => {
                for entry in entries {
                    if entry.interval.contains(time) {
                        return entry.inner.get_value(time);
                    }
                }
                None
            }
            CzmlPositionProperty::Reference(_) => None,
        }
    }

    /// The reference frame of this position property.
    pub fn reference_frame(&self) -> ReferenceFrame {
        match self {
            CzmlPositionProperty::Constant { reference_frame, .. }
            | CzmlPositionProperty::Sampled { reference_frame, .. }
            | CzmlPositionProperty::TimeIntervalCollection { reference_frame, .. }
            | CzmlPositionProperty::Composite { reference_frame, .. } => *reference_frame,
            CzmlPositionProperty::Reference(_) => ReferenceFrame::Fixed,
        }
    }
}

/// Maps a CZML `referenceFrame` name.
fn reference_frame_from_name(name: &str) -> Option<ReferenceFrame> {
    match name {
        "FIXED" => Some(ReferenceFrame::Fixed),
        "INERTIAL" => Some(ReferenceFrame::Inertial),
        _ => None,
    }
}

/// Mirror of `removePositionPropertyData(property, interval)`.
pub fn remove_position_property_data(
    property: Option<&mut CzmlPositionProperty>,
    interval: &TimeInterval,
) {
    let Some(property) = property else {
        return;
    };
    let start = julian_to_seconds(&interval.start);
    let stop = julian_to_seconds(&interval.stop);
    match property {
        CzmlPositionProperty::Sampled { property, .. } => {
            property.remove_samples_interval(
                start,
                stop,
                interval.is_start_included,
                interval.is_stop_included,
            );
        }
        CzmlPositionProperty::TimeIntervalCollection { entries, .. }
        | CzmlPositionProperty::Composite { entries, .. } => {
            for entry in entries.iter_mut() {
                let intersection = TimeInterval::intersect(&entry.interval, interval);
                if !intersection.is_empty() {
                    if let CzmlPositionInner::Sampled(sampled) = &mut entry.inner {
                        sampled.remove_samples_interval(
                            start,
                            stop,
                            interval.is_start_included,
                            interval.is_stop_included,
                        );
                    }
                }
            }
            remove_position_entries(entries, interval);
        }
        _ => {}
    }
}

/// Mirrors `TimeIntervalCollection.removeInterval` for position entries.
fn remove_position_entries(
    entries: &mut Vec<CzmlPositionIntervalEntry>,
    interval: &TimeInterval,
) {
    let mut index = 0;
    while index < entries.len() {
        let intersection = TimeInterval::intersect(&entries[index].interval, interval);
        if intersection.is_empty() {
            index += 1;
            continue;
        }

        let entry_interval = entries[index].interval.clone();
        let inner = entries
            .splice(index..index + 1, [])
            .next()
            .unwrap()
            .inner;

        let mut remainders: Vec<CzmlPositionIntervalEntry> = Vec::new();
        if JulianDate::compare(&entry_interval.start, &interval.start) < 0 {
            remainders.push(CzmlPositionIntervalEntry {
                interval: TimeInterval::new(
                    Some(entry_interval.start.clone()),
                    Some(interval.start.clone()),
                    Some(entry_interval.is_start_included),
                    Some(false),
                ),
                inner: constant_remainder(&inner),
            });
        }
        if JulianDate::compare(&interval.stop, &entry_interval.stop) < 0 {
            remainders.push(CzmlPositionIntervalEntry {
                interval: TimeInterval::new(
                    Some(interval.stop.clone()),
                    Some(entry_interval.stop.clone()),
                    Some(false),
                    Some(entry_interval.is_stop_included),
                ),
                inner,
            });
        }

        let mut offset = 0;
        for remainder in remainders {
            entries.insert(index + offset, remainder);
            offset += 1;
        }
        index += offset;
    }
}

fn constant_remainder(inner: &CzmlPositionInner) -> CzmlPositionInner {
    match inner {
        CzmlPositionInner::Constant(value) => CzmlPositionInner::Constant(*value),
        CzmlPositionInner::Reference(reference) => {
            CzmlPositionInner::Reference(reference.clone())
        }
        CzmlPositionInner::Sampled(_) => CzmlPositionInner::Constant(Cartesian3::default()),
    }
}

/// Mirror of `processPositionProperty`.
pub fn process_position_property(
    slot: &mut Option<CzmlPositionProperty>,
    packet_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let _ = source_uri;
    let combined_interval = compute_combined_interval(packet_data, constrained_interval);

    let number_of_derivatives: u32 = if packet_data.get("cartesianVelocity").is_some() {
        1
    } else {
        0
    };
    let packed_length = Cartesian3::PACKED_LENGTH * (number_of_derivatives as usize + 1);
    let is_value = packet_data.get("reference").is_none();
    let is_interval_constrained = has_interval(combined_interval.as_ref());

    if packet_data.get("delete").and_then(|v| v.as_bool()) == Some(true) {
        if !is_interval_constrained {
            *slot = None;
            return;
        }
        remove_position_property_data(slot.as_mut(), combined_interval.as_ref().unwrap());
        return;
    }

    let mut reference_frame = ReferenceFrame::Fixed;
    let mut is_sampled = false;
    let mut unwrapped: Option<Vec<f64>> = None;

    if is_value {
        if let Some(name) = packet_data.get("referenceFrame").and_then(|v| v.as_str()) {
            if let Some(frame) = reference_frame_from_name(name) {
                reference_frame = frame;
            }
        }
        let Some(data) = unwrap_cartesian_interval(packet_data) else {
            return;
        };
        is_sampled = data.len() > packed_length;
        unwrapped = Some(data);
    }

    // Any time a constant value is assigned, it completely blows away
    // anything else.
    if !is_sampled && !is_interval_constrained {
        if is_value {
            let packed = unwrapped.unwrap();
            *slot = Some(CzmlPositionProperty::Constant {
                value: Cartesian3::new(packed[0], packed[1], packed[2]),
                reference_frame,
            });
        } else {
            let reference = packet_data.get("reference").and_then(|v| v.as_str()).unwrap();
            *slot = Some(CzmlPositionProperty::Reference(resolve_reference_string(
                reference, current_id,
            )));
        }
        return;
    }

    let epoch = packet_data
        .get("epoch")
        .and_then(|v| v.as_str())
        .and_then(JulianDate::from_iso8601)
        .as_ref()
        .map(julian_to_seconds);

    // Without an interval, any sampled value completely replaces any
    // non-sampled property that may exist.
    if is_sampled && !is_interval_constrained {
        let reusable = matches!(
            slot,
            Some(CzmlPositionProperty::Sampled {
                reference_frame: frame,
                ..
            }) if *frame == reference_frame
        );
        if !reusable {
            *slot = Some(CzmlPositionProperty::Sampled {
                property: SampledProperty::with_derivative_types(
                    PackableType::Position,
                    if number_of_derivatives > 0 {
                        Some(vec![PackableType::Position; number_of_derivatives as usize])
                    } else {
                        None
                    },
                ),
                reference_frame,
                number_of_derivatives,
            });
        }
        if let Some(CzmlPositionProperty::Sampled { property, .. }) = slot.as_mut() {
            property.add_samples_packed_array(&unwrapped.unwrap(), epoch);
            update_interpolation_settings(packet_data, property);
        }
        return;
    }

    let combined_interval = combined_interval.unwrap();

    if !is_sampled {
        let inner: CzmlPositionInner = if is_value {
            let packed = unwrapped.unwrap();
            CzmlPositionInner::Constant(Cartesian3::new(packed[0], packed[1], packed[2]))
        } else {
            let reference = packet_data.get("reference").and_then(|v| v.as_str()).unwrap();
            CzmlPositionInner::Reference(resolve_reference_string(reference, current_id))
        };

        let take_old = slot.take();
        match take_old {
            None => {
                let entries = vec![CzmlPositionIntervalEntry {
                    interval: combined_interval,
                    inner,
                }];
                *slot = Some(if is_value {
                    CzmlPositionProperty::TimeIntervalCollection {
                        entries,
                        reference_frame,
                    }
                } else {
                    CzmlPositionProperty::Composite {
                        entries,
                        reference_frame,
                    }
                });
            }
            Some(CzmlPositionProperty::TimeIntervalCollection {
                mut entries,
                reference_frame: existing_frame,
            }) if is_value && existing_frame == reference_frame => {
                entries.push(CzmlPositionIntervalEntry {
                    interval: combined_interval,
                    inner,
                });
                *slot = Some(CzmlPositionProperty::TimeIntervalCollection {
                    entries,
                    reference_frame,
                });
            }
            Some(CzmlPositionProperty::Composite {
                mut entries,
                reference_frame: existing_frame,
            }) => {
                entries.push(CzmlPositionIntervalEntry {
                    interval: combined_interval,
                    inner,
                });
                *slot = Some(CzmlPositionProperty::Composite {
                    entries,
                    reference_frame: existing_frame,
                });
            }
            Some(old) => {
                let mut entries = convert_position_property_to_composite_entries(old);
                entries.push(CzmlPositionIntervalEntry {
                    interval: combined_interval,
                    inner,
                });
                *slot = Some(CzmlPositionProperty::Composite {
                    entries,
                    reference_frame,
                });
            }
        }
        return;
    }

    // isSampled && hasInterval
    let take_old = slot.take();
    let (mut entries, composite_frame) = match take_old {
        None => (Vec::new(), reference_frame),
        Some(CzmlPositionProperty::Composite {
            entries,
            reference_frame: existing_frame,
        }) => (entries, existing_frame),
        Some(old) => {
            let frame = old.reference_frame();
            (convert_position_property_to_composite_entries(old), frame)
        }
    };

    // Check if the interval already exists in the composite.
    let existing_index = entries
        .iter()
        .position(|entry| TimeInterval::equals(&entry.interval, &combined_interval));
    let reusable = existing_index
        .map(|index| matches!(entries[index].inner, CzmlPositionInner::Sampled(_)))
        .unwrap_or(false);

    if !reusable {
        entries.push(CzmlPositionIntervalEntry {
            interval: combined_interval,
            inner: CzmlPositionInner::Sampled(SampledProperty::with_derivative_types(
                PackableType::Position,
                if number_of_derivatives > 0 {
                    Some(vec![PackableType::Position; number_of_derivatives as usize])
                } else {
                    None
                },
            )),
        });
    }
    let index = existing_index.unwrap_or(entries.len() - 1);
    if let CzmlPositionInner::Sampled(property) = &mut entries[index].inner {
        property.add_samples_packed_array(&unwrapped.unwrap(), epoch);
        update_interpolation_settings(packet_data, property);
    }

    *slot = Some(CzmlPositionProperty::Composite {
        entries,
        reference_frame: composite_frame,
    });
}

/// Mirror of `convertPositionPropertyToComposite` (entry form).
fn convert_position_property_to_composite_entries(
    property: CzmlPositionProperty,
) -> Vec<CzmlPositionIntervalEntry> {
    match property {
        CzmlPositionProperty::Composite { entries, .. } => entries,
        CzmlPositionProperty::TimeIntervalCollection { entries, .. } => entries,
        CzmlPositionProperty::Constant { value, .. } => {
            vec![CzmlPositionIntervalEntry {
                interval: Iso8601::maximum_interval().clone(),
                inner: CzmlPositionInner::Constant(value),
            }]
        }
        CzmlPositionProperty::Sampled { property, .. } => {
            vec![CzmlPositionIntervalEntry {
                interval: Iso8601::maximum_interval().clone(),
                inner: CzmlPositionInner::Sampled(property),
            }]
        }
        CzmlPositionProperty::Reference(reference) => {
            vec![CzmlPositionIntervalEntry {
                interval: Iso8601::maximum_interval().clone(),
                inner: CzmlPositionInner::Reference(reference),
            }]
        }
    }
}

/// Mirror of `processPositionPacketData`.
pub fn process_position_packet_data(
    slot: &mut Option<CzmlPositionProperty>,
    packet_data: Option<&Value>,
    interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };

    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_position_property(slot, packet, interval, source_uri, current_id);
        }
    } else {
        process_position_property(slot, packet_data, interval, source_uri, current_id);
    }
}

// ============================================================================
// processReferencesArrayPacketData
// ============================================================================

/// Mirror of `processReferencesArrayPacketData`: resolves every reference
/// string and stores a `PropertyArray` (constant) or, when `interval` is
/// defined, a `CompositeProperty` whose old data is preserved wrapped in an
/// infinite interval.
pub fn process_references_array_packet_data(
    slot: &mut Option<CzmlProperty>,
    references: &[Value],
    interval_string: Option<&str>,
    current_id: Option<&str>,
) {
    let resolved: Vec<String> = references
        .iter()
        .filter_map(|value| value.as_str())
        .map(|reference| resolve_reference_string(reference, current_id))
        .collect();

    if let Some(interval) = interval_string.and_then(|s| interval_from_string(Some(s))) {
        let mut entries = match slot.take() {
            Some(CzmlProperty::Composite(entries)) => entries,
            Some(old) => into_reference_composite(old),
            None => Vec::new(),
        };
        entries.push(CzmlCompositeEntry {
            interval,
            inner: CzmlInnerProperty::Constant(CzmlValue::ReferenceArray(resolved)),
        });
        *slot = Some(CzmlProperty::Composite(entries));
    } else {
        *slot = Some(CzmlProperty::Constant(CzmlValue::ReferenceArray(resolved)));
    }
}

/// Wraps an existing array property into composite entries (mirror of
/// `wrapPropertyInInfiniteInterval(property)` used by
/// `processReferencesArrayPacketData`).
fn into_reference_composite(property: CzmlProperty) -> Vec<CzmlCompositeEntry> {
    match property {
        CzmlProperty::Composite(entries) => entries,
        CzmlProperty::TimeIntervalCollection(entries) => entries
            .into_iter()
            .map(|entry| CzmlCompositeEntry {
                interval: entry.interval,
                inner: CzmlInnerProperty::Constant(entry.value),
            })
            .collect(),
        other => vec![wrap_in_infinite_interval(match other {
            CzmlProperty::Constant(value) => CzmlInnerProperty::Constant(value),
            CzmlProperty::Sampled(property) => CzmlInnerProperty::Sampled(property),
            CzmlProperty::Reference(reference) => CzmlInnerProperty::Reference(reference),
            CzmlProperty::VelocityReference(reference) => {
                CzmlInnerProperty::Reference(reference)
            }
            CzmlProperty::FollowSurfaceAdapter(_) => {
                CzmlInnerProperty::Constant(CzmlValue::Json(Value::Null))
            }
            CzmlProperty::Composite(_) => unreachable!(),
            CzmlProperty::TimeIntervalCollection(_) => unreachable!(),
        })],
    }
}

// ============================================================================
// Generic array value property (the `processPacketData(Array, ...)` path)
// ============================================================================

/// The shared upsert used by the array packet-data functions. Mirrors the
/// constant paths of `processProperty` for the `Array` type (arrays are
/// never sampled: `isSampled` requires `!defined(packetData.array)`).
pub fn process_array_value_property(
    slot: &mut Option<CzmlProperty>,
    packet_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    unpack: fn(&Value) -> Option<CzmlValue>,
) {
    let combined_interval = compute_combined_interval(packet_data, constrained_interval);
    let is_interval_constrained = has_interval(combined_interval.as_ref());

    if packet_data.get("delete").and_then(|v| v.as_bool()) == Some(true) {
        if !is_interval_constrained {
            *slot = None;
            return;
        }
        remove_property_data(slot.as_mut(), combined_interval.as_ref().unwrap());
        return;
    }

    let Some(value) = unpack(packet_data) else {
        return;
    };

    // Any time a constant value is assigned, it completely blows away
    // anything else.
    if !is_interval_constrained {
        *slot = Some(CzmlProperty::Constant(value));
        return;
    }

    let combined_interval = combined_interval.unwrap();
    match slot.take() {
        None => {
            *slot = Some(CzmlProperty::TimeIntervalCollection(vec![
                CzmlIntervalEntry {
                    interval: combined_interval,
                    value,
                },
            ]));
        }
        Some(CzmlProperty::TimeIntervalCollection(mut entries)) => {
            entries.push(CzmlIntervalEntry {
                interval: combined_interval,
                value,
            });
            *slot = Some(CzmlProperty::TimeIntervalCollection(entries));
        }
        Some(CzmlProperty::Composite(mut entries)) => {
            entries.push(CzmlCompositeEntry {
                interval: combined_interval,
                inner: CzmlInnerProperty::Constant(value),
            });
            *slot = Some(CzmlProperty::Composite(entries));
        }
        Some(old) => {
            // Otherwise, create a CompositeProperty but preserve the
            // existing data.
            let mut entries = crate::czml_property::into_composite(old);
            entries.push(CzmlCompositeEntry {
                interval: combined_interval,
                inner: CzmlInnerProperty::Constant(value),
            });
            *slot = Some(CzmlProperty::Composite(entries));
        }
    }
}

/// Converts an `array` payload into a [`CzmlValue`]: an all-numeric array
/// becomes a [`CzmlValue::NumberArray`], anything else is kept as JSON
/// (mirrors the raw-array storage of the JS `Array` type).
fn array_payload_to_value(payload: &Value) -> Option<CzmlValue> {
    let array = payload.as_array()?;
    let mut numbers = Vec::with_capacity(array.len());
    for element in array {
        let Some(number) = element.as_f64() else {
            return Some(CzmlValue::Json(payload.clone()));
        };
        numbers.push(number);
    }
    Some(CzmlValue::NumberArray(numbers))
}

fn unpack_plain_array(packet_data: &Value) -> Option<CzmlValue> {
    array_payload_to_value(packet_data.get("array")?)
}

// ============================================================================
// processArrayPacketData / processArray
// ============================================================================

/// Mirror of `processArrayPacketData`.
pub fn process_array_packet_data(
    slot: &mut Option<CzmlProperty>,
    packet_data: &Value,
    current_id: Option<&str>,
) {
    if let Some(references) = packet_data.get("references").and_then(|v| v.as_array()) {
        let interval = packet_data.get("interval").and_then(|v| v.as_str());
        process_references_array_packet_data(slot, references, interval, current_id);
        return;
    }
    process_array_value_property(slot, packet_data, None, unpack_plain_array);
}

/// Mirror of `processArray`.
pub fn process_array(
    slot: &mut Option<CzmlProperty>,
    packet_data: Option<&Value>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_array_packet_data(slot, packet, current_id);
        }
    } else {
        process_array_packet_data(slot, packet_data, current_id);
    }
}

// ============================================================================
// processShapePacketData / processShape
// ============================================================================

fn unpack_shape_array(packet_data: &Value) -> Option<CzmlValue> {
    let packed = packet_data
        .get("cartesian2")
        .or_else(|| packet_data.get("cartesian"))
        .and_then(|v| v.as_array())?;
    let numbers: Vec<f64> = packed.iter().filter_map(|v| v.as_f64()).collect();
    Some(CzmlValue::Cartesian2Array(Cartesian2::unpack_array(
        &numbers, None,
    )))
}

/// Mirror of `processShapePacketData` (also accepts the legacy `cartesian`
/// key for backwards compatibility, exactly like the JS port).
pub fn process_shape_packet_data(
    slot: &mut Option<CzmlProperty>,
    packet_data: &Value,
    current_id: Option<&str>,
) {
    if let Some(references) = packet_data.get("references").and_then(|v| v.as_array()) {
        let interval = packet_data.get("interval").and_then(|v| v.as_str());
        process_references_array_packet_data(slot, references, interval, current_id);
        return;
    }
    if packet_data.get("cartesian2").is_some() || packet_data.get("cartesian").is_some() {
        process_array_value_property(slot, packet_data, None, unpack_shape_array);
    }
}

/// Mirror of `processShape`.
pub fn process_shape(
    slot: &mut Option<CzmlProperty>,
    packet_data: Option<&Value>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_shape_packet_data(slot, packet, current_id);
        }
    } else {
        process_shape_packet_data(slot, packet_data, current_id);
    }
}

// ============================================================================
// processPositionArrayPacketData / processPositionArray
// ============================================================================

/// Mirror of `unpackCartesianArray`.
pub fn unpack_cartesian_array(array: &[f64]) -> Vec<Cartesian3> {
    Cartesian3::unpack_array(array, None)
}

/// Mirror of `unpackCartographicRadiansArray`.
pub fn unpack_cartographic_radians_array(array: &[f64]) -> Vec<Cartesian3> {
    // `None` selects the default (WGS84) radii, mirroring the JS default
    // `ellipsoid = Ellipsoid.WGS84`.
    Cartesian3::from_radians_array_heights(array, None, None)
}

/// Mirror of `unpackCartographicDegreesArray`.
pub fn unpack_cartographic_degrees_array(array: &[f64]) -> Vec<Cartesian3> {
    Cartesian3::from_degrees_array_heights(array, None, None)
}

fn unpack_position_array(packet_data: &Value) -> Option<CzmlValue> {
    if let Some(cartesian) = packet_data.get("cartesian").and_then(|v| v.as_array()) {
        let numbers: Vec<f64> = cartesian.iter().filter_map(|v| v.as_f64()).collect();
        return Some(CzmlValue::Cartesian3Array(unpack_cartesian_array(&numbers)));
    }
    if let Some(radians) = packet_data
        .get("cartographicRadians")
        .and_then(|v| v.as_array())
    {
        let numbers: Vec<f64> = radians.iter().filter_map(|v| v.as_f64()).collect();
        return Some(CzmlValue::Cartesian3Array(
            unpack_cartographic_radians_array(&numbers),
        ));
    }
    if let Some(degrees) = packet_data
        .get("cartographicDegrees")
        .and_then(|v| v.as_array())
    {
        let numbers: Vec<f64> = degrees.iter().filter_map(|v| v.as_f64()).collect();
        return Some(CzmlValue::Cartesian3Array(
            unpack_cartographic_degrees_array(&numbers),
        ));
    }
    None
}

/// Mirror of `processPositionArrayPacketData`.
pub fn process_position_array_packet_data(
    slot: &mut Option<CzmlProperty>,
    packet_data: &Value,
    current_id: Option<&str>,
) {
    if let Some(references) = packet_data.get("references").and_then(|v| v.as_array()) {
        let interval = packet_data.get("interval").and_then(|v| v.as_str());
        process_references_array_packet_data(slot, references, interval, current_id);
        return;
    }
    if packet_data.get("cartesian").is_some()
        || packet_data.get("cartographicRadians").is_some()
        || packet_data.get("cartographicDegrees").is_some()
    {
        process_array_value_property(slot, packet_data, None, unpack_position_array);
    }
}

/// Mirror of `processPositionArray`.
pub fn process_position_array(
    slot: &mut Option<CzmlProperty>,
    packet_data: Option<&Value>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_position_array_packet_data(slot, packet, current_id);
        }
    } else {
        process_position_array_packet_data(slot, packet_data, current_id);
    }
}

// ============================================================================
// processPositionArrayOfArraysPacketData / processPositionArrayOfArrays
// ============================================================================

fn unpack_position_array_of_arrays(packet_data: &Value) -> Option<CzmlValue> {
    let (key, unpack): (&str, fn(&[f64]) -> Vec<Cartesian3>) =
        if packet_data.get("cartesian").is_some() {
            ("cartesian", unpack_cartesian_array)
        } else if packet_data.get("cartographicRadians").is_some() {
            ("cartographicRadians", unpack_cartographic_radians_array)
        } else if packet_data.get("cartographicDegrees").is_some() {
            ("cartographicDegrees", unpack_cartographic_degrees_array)
        } else {
            return None;
        };

    let outer = packet_data.get(key)?.as_array()?;
    let mut result = Vec::with_capacity(outer.len());
    for inner in outer {
        let numbers: Vec<f64> = inner
            .as_array()?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        result.push(unpack(&numbers));
    }
    Some(CzmlValue::Cartesian3ArrayOfArrays(result))
}

/// Mirror of `processPositionArrayOfArraysPacketData`.
///
/// DEVIATION (hole references with intervals): CesiumJS applies the packet
/// interval to every inner reference array individually, producing a
/// `CompositePositionProperty` per hole. The Rust port resolves the inner
/// references into [`CzmlValue::ReferenceArrayOfArrays`] and does not model
/// per-hole interval composites.
pub fn process_position_array_of_arrays_packet_data(
    slot: &mut Option<CzmlProperty>,
    packet_data: &Value,
    current_id: Option<&str>,
) {
    if let Some(references) = packet_data.get("references").and_then(|v| v.as_array()) {
        let mut arrays = Vec::with_capacity(references.len());
        for reference_array in references {
            let Some(reference_array) = reference_array.as_array() else {
                continue;
            };
            arrays.push(
                reference_array
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(|reference| resolve_reference_string(reference, current_id))
                    .collect(),
            );
        }
        *slot = Some(CzmlProperty::Constant(CzmlValue::ReferenceArrayOfArrays(
            arrays,
        )));
        return;
    }
    process_array_value_property(slot, packet_data, None, unpack_position_array_of_arrays);
}

/// Mirror of `processPositionArrayOfArrays`.
pub fn process_position_array_of_arrays(
    slot: &mut Option<CzmlProperty>,
    packet_data: Option<&Value>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_position_array_of_arrays_packet_data(slot, packet, current_id);
        }
    } else {
        process_position_array_of_arrays_packet_data(slot, packet_data, current_id);
    }
}

// ============================================================================
// processAlignedAxis
// ============================================================================

/// Mirror of `processAlignedAxis`.
pub fn process_aligned_axis(
    slot: &mut Option<CzmlProperty>,
    packet_data: Option<&Value>,
    interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    process_packet_data(
        slot,
        CzmlPropertyType::UnitCartesian3,
        Some(packet_data),
        interval,
        source_uri,
        current_id,
    );
}

// ============================================================================
// processMaterialProperty / processMaterialPacketData
// ============================================================================

/// The material kind (mirror of the concrete `*MaterialProperty` classes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CzmlMaterialKind {
    SolidColor,
    Grid,
    Image,
    Stripe,
    PolylineOutline,
    PolylineGlow,
    PolylineArrow,
    PolylineDash,
    Checkerboard,
}

/// A single material definition: its kind plus the processed sub-properties
/// (mirror of a concrete `*MaterialProperty` instance).
#[derive(Debug, Default)]
pub struct CzmlMaterial {
    pub kind: CzmlMaterialKind,
    pub properties: std::collections::BTreeMap<String, Option<CzmlProperty>>,
}

impl CzmlMaterial {
    /// Processes one sub-property of this material (the per-field
    /// `processPacketData` calls of `processMaterialProperty`).
    pub fn process_property(
        &mut self,
        name: &str,
        r#type: CzmlPropertyType,
        packet_data: Option<&Value>,
        source_uri: Option<&str>,
        current_id: Option<&str>,
    ) {
        let Some(packet_data) = packet_data else {
            return;
        };
        let slot = self
            .properties
            .entry(name.to_string())
            .or_insert_with(|| None);
        // Mirrors JS: material sub-properties are processed with an
        // undefined constrained interval.
        process_packet_data(
            slot,
            r#type,
            Some(packet_data),
            None,
            source_uri,
            current_id,
        );
    }

    /// Reads a sub-property value at `time`.
    pub fn get_property(&self, name: &str, time: &JulianDate) -> Option<CzmlValue> {
        self.properties.get(name)?.as_ref()?.get_value(time)
    }
}

/// An interval entry of a [`CzmlMaterialProperty::Composite`].
#[derive(Debug)]
pub struct CzmlMaterialIntervalEntry {
    pub interval: TimeInterval,
    pub material: CzmlMaterial,
}

/// A CZML material property (mirror of the concrete material properties and
/// `CompositeMaterialProperty`).
#[derive(Debug)]
pub enum CzmlMaterialProperty {
    Single(CzmlMaterial),
    Composite(Vec<CzmlMaterialIntervalEntry>),
}

impl CzmlMaterialProperty {
    /// Returns the material in effect at `time`.
    pub fn get_material(&self, time: &JulianDate) -> Option<&CzmlMaterial> {
        match self {
            CzmlMaterialProperty::Single(material) => Some(material),
            CzmlMaterialProperty::Composite(entries) => entries
                .iter()
                .find(|entry| entry.interval.contains(time))
                .map(|entry| &entry.material),
        }
    }
}

impl Default for CzmlMaterialKind {
    fn default() -> Self {
        CzmlMaterialKind::SolidColor
    }
}

/// Mirror of `processMaterialProperty`.
///
/// Note the JS semantics preserved here: when an interval is present and the
/// current property is not a `CompositeMaterialProperty`, a fresh composite
/// is created (the previous single material is not preserved), and
/// sub-properties are processed with an undefined constrained interval.
pub fn process_material_property(
    slot: &mut Option<CzmlMaterialProperty>,
    packet_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let combined_interval = compute_combined_interval(packet_data, constrained_interval);

    if let Some(ref combined) = combined_interval {
        // With an interval the property must be a CompositeMaterialProperty;
        // CesiumJS discards any non-composite property here.
        if !matches!(slot, Some(CzmlMaterialProperty::Composite(_))) {
            *slot = Some(CzmlMaterialProperty::Composite(Vec::new()));
        }
        let entries = if let Some(CzmlMaterialProperty::Composite(entries)) = slot.as_mut() {
            entries
        } else {
            unreachable!()
        };
        // See if we already have data at that interval (findInterval by
        // start/stop).
        let index = match entries
            .iter()
            .position(|entry| endpoints_equal(&entry.interval, combined))
        {
            Some(index) => index,
            None => {
                entries.push(CzmlMaterialIntervalEntry {
                    interval: combined.clone(),
                    material: CzmlMaterial::default(),
                });
                entries.len() - 1
            }
        };
        process_material_fields(&mut entries[index].material, packet_data, source_uri, current_id);
        return;
    }

    // Without an interval the existing single material is edited in place;
    // any other property shape is replaced by a fresh single material only
    // when a known material key is present (JS leaves the property
    // untouched otherwise).
    let existing_single = match slot.take() {
        Some(CzmlMaterialProperty::Single(material)) => Some(material),
        other => {
            if let Some(property) = other {
                *slot = Some(property);
            }
            None
        }
    };
    let mut material = existing_single.unwrap_or_default();
    if process_material_fields(&mut material, packet_data, source_uri, current_id) {
        *slot = Some(CzmlMaterialProperty::Single(material));
    }
}

/// Processes the material-kind dispatch and sub-fields of one material
/// packet (the `solidColor`/`grid`/... chain of `processMaterialProperty`).
/// Returns whether a known material key was present.
fn process_material_fields(
    material: &mut CzmlMaterial,
    packet_data: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) -> bool {
    macro_rules! field {
        ($data:expr, $name:expr, $type:expr, $uri:expr) => {
            material.process_property($name, $type, $data.get($name), $uri, current_id);
        };
    }
    macro_rules! set_kind {
        ($kind:expr) => {
            if material.kind != $kind {
                *material = CzmlMaterial {
                    kind: $kind,
                    ..Default::default()
                };
            }
        };
    }

    if let Some(data) = packet_data.get("solidColor") {
        set_kind!(CzmlMaterialKind::SolidColor);
        // JS passes an undefined sourceUri for solidColor.color.
        field!(data, "color", CzmlPropertyType::Color, None);
        true
    } else if let Some(data) = packet_data.get("grid") {
        set_kind!(CzmlMaterialKind::Grid);
        field!(data, "color", CzmlPropertyType::Color, source_uri);
        field!(data, "cellAlpha", CzmlPropertyType::Number, source_uri);
        field!(data, "lineCount", CzmlPropertyType::Cartesian2, source_uri);
        field!(
            data,
            "lineThickness",
            CzmlPropertyType::Cartesian2,
            source_uri
        );
        field!(data, "lineOffset", CzmlPropertyType::Cartesian2, source_uri);
        true
    } else if let Some(data) = packet_data.get("image") {
        set_kind!(CzmlMaterialKind::Image);
        field!(data, "image", CzmlPropertyType::Image, source_uri);
        field!(data, "repeat", CzmlPropertyType::Cartesian2, source_uri);
        field!(data, "color", CzmlPropertyType::Color, source_uri);
        field!(data, "transparent", CzmlPropertyType::Boolean, source_uri);
        true
    } else if let Some(data) = packet_data.get("stripe") {
        set_kind!(CzmlMaterialKind::Stripe);
        field!(
            data,
            "orientation",
            CzmlPropertyType::StripeOrientation,
            source_uri
        );
        field!(data, "evenColor", CzmlPropertyType::Color, source_uri);
        field!(data, "oddColor", CzmlPropertyType::Color, source_uri);
        field!(data, "offset", CzmlPropertyType::Number, source_uri);
        field!(data, "repeat", CzmlPropertyType::Number, source_uri);
        true
    } else if let Some(data) = packet_data.get("polylineOutline") {
        set_kind!(CzmlMaterialKind::PolylineOutline);
        field!(data, "color", CzmlPropertyType::Color, source_uri);
        field!(data, "outlineColor", CzmlPropertyType::Color, source_uri);
        field!(data, "outlineWidth", CzmlPropertyType::Number, source_uri);
        true
    } else if let Some(data) = packet_data.get("polylineGlow") {
        set_kind!(CzmlMaterialKind::PolylineGlow);
        field!(data, "color", CzmlPropertyType::Color, source_uri);
        field!(data, "glowPower", CzmlPropertyType::Number, source_uri);
        field!(data, "taperPower", CzmlPropertyType::Number, source_uri);
        true
    } else if let Some(data) = packet_data.get("polylineArrow") {
        set_kind!(CzmlMaterialKind::PolylineArrow);
        // JS passes an undefined sourceUri for polylineArrow.color.
        field!(data, "color", CzmlPropertyType::Color, None);
        true
    } else if let Some(data) = packet_data.get("polylineDash") {
        set_kind!(CzmlMaterialKind::PolylineDash);
        // JS passes an undefined sourceUri for color/gapColor.
        field!(data, "color", CzmlPropertyType::Color, None);
        field!(data, "gapColor", CzmlPropertyType::Color, None);
        field!(data, "dashLength", CzmlPropertyType::Number, source_uri);
        field!(data, "dashPattern", CzmlPropertyType::Number, source_uri);
        true
    } else if let Some(data) = packet_data.get("checkerboard") {
        set_kind!(CzmlMaterialKind::Checkerboard);
        field!(data, "evenColor", CzmlPropertyType::Color, source_uri);
        field!(data, "oddColor", CzmlPropertyType::Color, source_uri);
        field!(data, "repeat", CzmlPropertyType::Cartesian2, source_uri);
        true
    } else {
        false
    }
}

/// Mirror of `processMaterialPacketData`.
pub fn process_material_packet_data(
    slot: &mut Option<CzmlMaterialProperty>,
    packet_data: Option<&Value>,
    interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(packet_data) = packet_data else {
        return;
    };
    if let Some(packets) = packet_data.as_array() {
        for packet in packets {
            process_material_property(slot, packet, interval, source_uri, current_id);
        }
    } else {
        process_material_property(slot, packet_data, interval, source_uri, current_id);
    }
}
