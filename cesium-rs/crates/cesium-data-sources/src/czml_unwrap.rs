//! Ported from the CZML interval unwrapping helpers of
//! `packages/engine/Source/DataSources/CzmlDataSource.js`:
//! `unwrapColorInterval`, `unwrapUriInterval`, `unwrapRectangleInterval`,
//! `convertUnitSphericalToCartesian`, `convertSphericalToCartesian`,
//! `convertCartographicRadiansToCartesian`,
//! `convertCartographicDegreesToCartesian`, `unwrapCartesianInterval`,
//! `normalizePackedCartesianArray`, `unwrapUnitCartesianInterval`,
//! `normalizePackedQuaternionArray`, `unwrapQuaternionInterval` and the
//! `unwrapInterval` dispatcher.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::quaternion::Quaternion;
use cesium_core::spherical::Spherical;
use serde_json::Value;

use crate::czml_data_source::resolve_uri;
use crate::czml_property::{CzmlPropertyType, CzmlValue};

/// The payload produced by [`unwrap_interval`] (mirrors the heterogeneous JS
/// return values of `unwrapInterval`).
#[derive(Debug, Clone)]
pub enum Unwrapped {
    /// An already-unpacked constant scalar value.
    Scalar(CzmlValue),
    /// A packed `f64` array (constant needs unpacking; longer arrays are
    /// sampled data).
    Packed(Vec<f64>),
    /// A string payload (never sampled, mirrors `typeof === "string"`).
    Text(String),
    /// An arbitrary JSON payload (`array`/`object` types).
    Json(Value),
}

/// Extracts a numeric JSON array (dropping non-numeric entries, which never
/// occur in valid CZML).
fn f64_array(value: Option<&Value>) -> Option<Vec<f64>> {
    value
        .and_then(|v| v.as_array())
        .map(|array| array.iter().filter_map(|v| v.as_f64()).collect())
}

// ============================================================================
// unwrapColorInterval / unwrapUriInterval / unwrapRectangleInterval
// ============================================================================

/// Mirror of `unwrapColorInterval(czmlInterval)`: returns the packed rgba
/// float array. Byte-encoded `rgba` values are converted with
/// `Color.byteToFloat`; packed sampled arrays keep the time components
/// untouched (`i += 5`).
pub fn unwrap_color_interval(czml_interval: &Value) -> Option<Vec<f64>> {
    if let Some(rgbaf) = f64_array(czml_interval.get("rgbaf")) {
        return Some(rgbaf);
    }

    let rgba = f64_array(czml_interval.get("rgba"))?;
    let length = rgba.len();
    if length == 4 {
        return Some(vec![
            color_byte_to_float(rgba[0]),
            color_byte_to_float(rgba[1]),
            color_byte_to_float(rgba[2]),
            color_byte_to_float(rgba[3]),
        ]);
    }

    let mut rgbaf = vec![0.0; length];
    let mut i = 0;
    while i < length {
        rgbaf[i] = rgba[i];
        rgbaf[i + 1] = color_byte_to_float(rgba[i + 1]);
        rgbaf[i + 2] = color_byte_to_float(rgba[i + 2]);
        rgbaf[i + 3] = color_byte_to_float(rgba[i + 3]);
        rgbaf[i + 4] = color_byte_to_float(rgba[i + 4]);
        i += 5;
    }
    Some(rgbaf)
}

/// Mirror of `Color.byteToFloat`.
fn color_byte_to_float(byte: f64) -> f64 {
    byte / 255.0
}

/// Mirror of the JS `Number`/`Rotation` branches of `unwrapInterval`
/// (`czmlInterval.number ?? czmlInterval`): the payload may be a constant
/// scalar or a packed sampled array `[time, value, time, value, ...]`.
fn unwrap_number_interval(czml_interval: &Value) -> Option<Unwrapped> {
    let payload = czml_interval
        .get("number")
        .unwrap_or(czml_interval);
    if let Some(numbers) = payload.as_array() {
        let packed: Vec<f64> = numbers.iter().filter_map(|v| v.as_f64()).collect();
        if packed.is_empty() {
            return None;
        }
        return Some(Unwrapped::Packed(packed));
    }
    Some(Unwrapped::Scalar(CzmlValue::Number(payload.as_f64()?)))
}

/// Mirror of `unwrapUriInterval(czmlInterval, sourceUri)`: resolves `uri`
/// (or a plain string) against the source uri.
pub fn unwrap_uri_interval(czml_interval: &Value, source_uri: Option<&str>) -> Option<String> {
    let uri = czml_interval
        .get("uri")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| czml_interval.as_str().map(|s| s.to_string()))?;
    Some(resolve_uri(source_uri, &uri))
}

/// Mirror of `unwrapRectangleInterval(czmlInterval)`: returns the packed wsen
/// (radians) array, converting `wsenDegrees` when present (`i += 5` for
/// packed sampled arrays).
pub fn unwrap_rectangle_interval(czml_interval: &Value) -> Option<Vec<f64>> {
    if let Some(wsen) = f64_array(czml_interval.get("wsen")) {
        return Some(wsen);
    }

    let wsen_degrees = f64_array(czml_interval.get("wsenDegrees"))?;
    let length = wsen_degrees.len();
    if length == 4 {
        return Some(vec![
            CesiumMath::to_radians(wsen_degrees[0]),
            CesiumMath::to_radians(wsen_degrees[1]),
            CesiumMath::to_radians(wsen_degrees[2]),
            CesiumMath::to_radians(wsen_degrees[3]),
        ]);
    }

    let mut wsen = vec![0.0; length];
    let mut i = 0;
    while i < length {
        wsen[i] = wsen_degrees[i];
        wsen[i + 1] = CesiumMath::to_radians(wsen_degrees[i + 1]);
        wsen[i + 2] = CesiumMath::to_radians(wsen_degrees[i + 2]);
        wsen[i + 3] = CesiumMath::to_radians(wsen_degrees[i + 3]);
        wsen[i + 4] = CesiumMath::to_radians(wsen_degrees[i + 4]);
        i += 5;
    }
    Some(wsen)
}

// ============================================================================
// convert*ToCartesian
// ============================================================================

/// Mirror of `convertUnitSphericalToCartesian(unitSpherical)`.
pub fn convert_unit_spherical_to_cartesian(unit_spherical: &[f64]) -> Vec<f64> {
    let length = unit_spherical.len();
    let mut spherical = Spherical {
        clock: 0.0,
        cone: 0.0,
        magnitude: 1.0,
    };
    let mut cartesian = Cartesian3::default();

    if length == 2 {
        spherical.clock = unit_spherical[0];
        spherical.cone = unit_spherical[1];
        Cartesian3::from_spherical(&spherical, &mut cartesian);
        return vec![cartesian.x, cartesian.y, cartesian.z];
    }

    let mut result = vec![0.0; (length / 3) * 4];
    let mut i = 0;
    let mut j = 0;
    while i < length {
        result[j] = unit_spherical[i];

        spherical.clock = unit_spherical[i + 1];
        spherical.cone = unit_spherical[i + 2];
        Cartesian3::from_spherical(&spherical, &mut cartesian);

        result[j + 1] = cartesian.x;
        result[j + 2] = cartesian.y;
        result[j + 3] = cartesian.z;

        i += 3;
        j += 4;
    }
    result
}

/// Mirror of `convertSphericalToCartesian(spherical)`.
pub fn convert_spherical_to_cartesian(spherical: &[f64]) -> Vec<f64> {
    let length = spherical.len();
    let mut scratch = Spherical {
        clock: 0.0,
        cone: 0.0,
        magnitude: 0.0,
    };
    let mut cartesian = Cartesian3::default();

    if length == 3 {
        scratch.clock = spherical[0];
        scratch.cone = spherical[1];
        scratch.magnitude = spherical[2];
        Cartesian3::from_spherical(&scratch, &mut cartesian);
        return vec![cartesian.x, cartesian.y, cartesian.z];
    }

    let mut result = vec![0.0; length];
    let mut i = 0;
    while i < length {
        result[i] = spherical[i];

        scratch.clock = spherical[i + 1];
        scratch.cone = spherical[i + 2];
        scratch.magnitude = spherical[i + 3];
        Cartesian3::from_spherical(&scratch, &mut cartesian);

        result[i + 1] = cartesian.x;
        result[i + 2] = cartesian.y;
        result[i + 3] = cartesian.z;

        i += 4;
    }
    result
}

/// Mirror of `convertCartographicRadiansToCartesian(cartographicRadians)`.
pub fn convert_cartographic_radians_to_cartesian(cartographic_radians: &[f64]) -> Vec<f64> {
    let length = cartographic_radians.len();
    let ellipsoid = &Ellipsoid::WGS84;
    let mut cartographic = Cartographic::default();
    let mut cartesian = Cartesian3::default();

    if length == 3 {
        cartographic.longitude = cartographic_radians[0];
        cartographic.latitude = cartographic_radians[1];
        cartographic.height = cartographic_radians[2];
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);
        return vec![cartesian.x, cartesian.y, cartesian.z];
    }

    let mut result = vec![0.0; length];
    let mut i = 0;
    while i < length {
        result[i] = cartographic_radians[i];

        cartographic.longitude = cartographic_radians[i + 1];
        cartographic.latitude = cartographic_radians[i + 2];
        cartographic.height = cartographic_radians[i + 3];
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);

        result[i + 1] = cartesian.x;
        result[i + 2] = cartesian.y;
        result[i + 3] = cartesian.z;

        i += 4;
    }
    result
}

/// Mirror of `convertCartographicDegreesToCartesian(cartographicDegrees)`.
pub fn convert_cartographic_degrees_to_cartesian(cartographic_degrees: &[f64]) -> Vec<f64> {
    let length = cartographic_degrees.len();
    let ellipsoid = &Ellipsoid::WGS84;
    let mut cartographic = Cartographic::default();
    let mut cartesian = Cartesian3::default();

    if length == 3 {
        cartographic.longitude = CesiumMath::to_radians(cartographic_degrees[0]);
        cartographic.latitude = CesiumMath::to_radians(cartographic_degrees[1]);
        cartographic.height = cartographic_degrees[2];
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);
        return vec![cartesian.x, cartesian.y, cartesian.z];
    }

    let mut result = vec![0.0; length];
    let mut i = 0;
    while i < length {
        result[i] = cartographic_degrees[i];

        cartographic.longitude = CesiumMath::to_radians(cartographic_degrees[i + 1]);
        cartographic.latitude = CesiumMath::to_radians(cartographic_degrees[i + 2]);
        cartographic.height = cartographic_degrees[i + 3];
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);

        result[i + 1] = cartesian.x;
        result[i + 2] = cartesian.y;
        result[i + 3] = cartesian.z;

        i += 4;
    }
    result
}

// ============================================================================
// unwrapCartesianInterval & friends
// ============================================================================

/// Mirror of `unwrapCartesianInterval(czmlInterval)`: returns the packed
/// cartesian array for any supported encoding. Returns `None` when no known
/// encoding is present (JS throws a `RuntimeError`).
pub fn unwrap_cartesian_interval(czml_interval: &Value) -> Option<Vec<f64>> {
    if let Some(cartesian) = f64_array(czml_interval.get("cartesian")) {
        return Some(cartesian);
    }
    if let Some(cartesian_velocity) = f64_array(czml_interval.get("cartesianVelocity")) {
        return Some(cartesian_velocity);
    }
    if let Some(unit_cartesian) = f64_array(czml_interval.get("unitCartesian")) {
        return Some(unit_cartesian);
    }
    if let Some(unit_spherical) = f64_array(czml_interval.get("unitSpherical")) {
        return Some(convert_unit_spherical_to_cartesian(&unit_spherical));
    }
    if let Some(spherical) = f64_array(czml_interval.get("spherical")) {
        return Some(convert_spherical_to_cartesian(&spherical));
    }
    if let Some(radians) = f64_array(czml_interval.get("cartographicRadians")) {
        return Some(convert_cartographic_radians_to_cartesian(&radians));
    }
    if let Some(degrees) = f64_array(czml_interval.get("cartographicDegrees")) {
        return Some(convert_cartographic_degrees_to_cartesian(&degrees));
    }
    None
}

/// Mirror of `normalizePackedCartesianArray(array, startingIndex)`.
pub fn normalize_packed_cartesian_array(array: &mut [f64], starting_index: usize) {
    let input = Cartesian3::new(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
    );
    let mut normalized = Cartesian3::default();
    Cartesian3::normalize(&input, &mut normalized);
    array[starting_index] = normalized.x;
    array[starting_index + 1] = normalized.y;
    array[starting_index + 2] = normalized.z;
}

/// Mirror of `unwrapUnitCartesianInterval(czmlInterval)`: like
/// `unwrapCartesianInterval` but normalizes every value.
pub fn unwrap_unit_cartesian_interval(czml_interval: &Value) -> Option<Vec<f64>> {
    let mut cartesian = unwrap_cartesian_interval(czml_interval)?;
    if cartesian.len() == 3 {
        normalize_packed_cartesian_array(&mut cartesian, 0);
        return Some(cartesian);
    }

    let mut i = 1;
    while i < cartesian.len() {
        normalize_packed_cartesian_array(&mut cartesian, i);
        i += 4;
    }
    Some(cartesian)
}

/// Mirror of `normalizePackedQuaternionArray(array, startingIndex)`.
pub fn normalize_packed_quaternion_array(array: &mut [f64], starting_index: usize) {
    let input = Quaternion::new(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    );
    let mut normalized = Quaternion::default();
    Quaternion::normalize(&input, &mut normalized);
    array[starting_index] = normalized.x;
    array[starting_index + 1] = normalized.y;
    array[starting_index + 2] = normalized.z;
    array[starting_index + 3] = normalized.w;
}

/// Mirror of `unwrapQuaternionInterval(czmlInterval)`: unpacks and normalizes
/// `unitQuaternion` data (packed stride 5 for sampled arrays).
pub fn unwrap_quaternion_interval(czml_interval: &Value) -> Option<Vec<f64>> {
    let mut unit_quaternion = f64_array(czml_interval.get("unitQuaternion"))?;
    if unit_quaternion.len() == 4 {
        normalize_packed_quaternion_array(&mut unit_quaternion, 0);
    } else {
        let mut i = 1;
        while i < unit_quaternion.len() {
            normalize_packed_quaternion_array(&mut unit_quaternion, i);
            i += 5;
        }
    }
    Some(unit_quaternion)
}

// ============================================================================
// CZML enum name mapping (mirrors the `EnumType[name]` lookups of
// unwrapInterval / getPropertyType)
// ============================================================================

/// Maps a CZML `arcType` name to the enum discriminant.
pub fn arc_type_from_name(name: &str) -> Option<i32> {
    match name {
        "NONE" => Some(cesium_core::arc_type::ArcType::None as i32),
        "GEODESIC" => Some(cesium_core::arc_type::ArcType::Geodesic as i32),
        "RHUMB" => Some(cesium_core::arc_type::ArcType::Rhumb as i32),
        _ => None,
    }
}

/// Maps a CZML `classificationType` name to the enum discriminant.
pub fn classification_type_from_name(name: &str) -> Option<i32> {
    match name {
        "CESIUM_3D_TILE" => Some(cesium_scene::classification_type::ClassificationType::Cesium3DTiles as i32),
        "TERRAIN" => Some(cesium_scene::classification_type::ClassificationType::Terrain as i32),
        "BOTH" => Some(cesium_scene::classification_type::ClassificationType::Both as i32),
        _ => None,
    }
}

/// Maps a CZML `colorBlendMode` name to the enum discriminant.
pub fn color_blend_mode_from_name(name: &str) -> Option<i32> {
    match name {
        "HIGHLIGHT" => Some(cesium_scene::color_blend_mode::ColorBlendMode::Highlight as i32),
        "REPLACE" => Some(cesium_scene::color_blend_mode::ColorBlendMode::Replace as i32),
        "MIX" => Some(cesium_scene::color_blend_mode::ColorBlendMode::Mix as i32),
        _ => None,
    }
}

/// Maps a CZML `cornerType` name to the enum discriminant.
pub fn corner_type_from_name(name: &str) -> Option<i32> {
    match name {
        "ROUNDED" => Some(cesium_core::corner_type::CornerType::Rounded as i32),
        "MITRED" => Some(cesium_core::corner_type::CornerType::Mitered as i32),
        "BEVELED" => Some(cesium_core::corner_type::CornerType::Beveled as i32),
        _ => None,
    }
}

/// Maps a CZML `heightReference` name to the enum discriminant.
pub fn height_reference_from_name(name: &str) -> Option<i32> {
    match name {
        "NONE" => Some(cesium_scene::height_reference::HeightReference::None as i32),
        "CLAMP_TO_GROUND" => {
            Some(cesium_scene::height_reference::HeightReference::ClampToGround as i32)
        }
        "RELATIVE_TO_GROUND" => {
            Some(cesium_scene::height_reference::HeightReference::RelativeToGround as i32)
        }
        _ => None,
    }
}

/// Maps a CZML `horizontalOrigin` name to the enum discriminant.
pub fn horizontal_origin_from_name(name: &str) -> Option<i32> {
    match name {
        "CENTER" => Some(cesium_scene::horizontal_origin::HorizontalOrigin::Center as i32),
        "LEFT" => Some(cesium_scene::horizontal_origin::HorizontalOrigin::Left as i32),
        "RIGHT" => Some(cesium_scene::horizontal_origin::HorizontalOrigin::Right as i32),
        _ => None,
    }
}

/// Maps a CZML `labelStyle` name to the enum discriminant.
pub fn label_style_from_name(name: &str) -> Option<i32> {
    match name {
        "FILL" => Some(cesium_scene::label_style::LabelStyle::Fill as i32),
        "OUTLINE" => Some(cesium_scene::label_style::LabelStyle::Outline as i32),
        "FILL_AND_OUTLINE" => Some(cesium_scene::label_style::LabelStyle::FillAndOutline as i32),
        _ => None,
    }
}

/// Maps a CZML `pathMode` name to the enum discriminant.
pub fn path_mode_from_name(name: &str) -> Option<i32> {
    match name {
        "FIXED" => Some(crate::path_mode::PathMode::Fixed as i32),
        "INERTIAL" => Some(crate::path_mode::PathMode::Inertial as i32),
        "VELOCITY_ORIENTATION" => Some(crate::path_mode::PathMode::VelocityOrientation as i32),
        _ => None,
    }
}

/// Maps a CZML `shadowMode`/`shadows` name to the enum discriminant.
pub fn shadow_mode_from_name(name: &str) -> Option<i32> {
    match name {
        "DISABLED" => Some(cesium_scene::shadow_mode::ShadowMode::Disabled as i32),
        "ENABLED" => Some(cesium_scene::shadow_mode::ShadowMode::Enabled as i32),
        "CAST_ONLY" => Some(cesium_scene::shadow_mode::ShadowMode::CastOnly as i32),
        "RECEIVE_ONLY" => Some(cesium_scene::shadow_mode::ShadowMode::ReceiveOnly as i32),
        _ => None,
    }
}

/// Maps a CZML `stripeOrientation` name to the enum discriminant.
pub fn stripe_orientation_from_name(name: &str) -> Option<i32> {
    match name {
        "HORIZONTAL" => Some(crate::stripe_orientation::StripeOrientation::Horizontal as i32),
        "VERTICAL" => Some(crate::stripe_orientation::StripeOrientation::Vertical as i32),
        _ => None,
    }
}

/// Maps a CZML `verticalOrigin` name to the enum discriminant.
pub fn vertical_origin_from_name(name: &str) -> Option<i32> {
    match name {
        "CENTER" => Some(cesium_scene::vertical_origin::VerticalOrigin::Center as i32),
        "BOTTOM" => Some(cesium_scene::vertical_origin::VerticalOrigin::Bottom as i32),
        "TOP" => Some(cesium_scene::vertical_origin::VerticalOrigin::Top as i32),
        _ => None,
    }
}

/// Resolves `czmlInterval[key] ?? czmlInterval` as a string and maps it with
/// `from_name` (mirrors `EnumType[czmlInterval.key ?? czmlInterval]`).
fn enum_value(
    czml_interval: &Value,
    key: &str,
    from_name: fn(&str) -> Option<i32>,
) -> Option<Unwrapped> {
    let name = czml_interval
        .get(key)
        .and_then(|v| v.as_str())
        .or_else(|| czml_interval.as_str())?;
    Some(Unwrapped::Scalar(CzmlValue::Number(from_name(name)? as f64)))
}

// ============================================================================
// unwrapInterval
// ============================================================================

/// Mirror of `unwrapInterval(type, czmlInterval, sourceUri)`. Returns `None`
/// when the payload does not carry a value of the given type (JS returns
/// `undefined`, or throws for invalid cartesian intervals; the Rust port
/// skips both).
pub fn unwrap_interval(
    r#type: CzmlPropertyType,
    czml_interval: &Value,
    source_uri: Option<&str>,
) -> Option<Unwrapped> {
    match r#type {
        CzmlPropertyType::ArcType => enum_value(czml_interval, "arcType", arc_type_from_name),
        CzmlPropertyType::Array => czml_interval
            .get("array")
            .map(|array| Unwrapped::Json(array.clone())),
        CzmlPropertyType::Boolean => {
            let boolean = czml_interval
                .get("boolean")
                .and_then(|v| v.as_bool())
                .or_else(|| czml_interval.as_bool())?;
            Some(Unwrapped::Scalar(CzmlValue::Boolean(boolean)))
        }
        CzmlPropertyType::BoundingRectangle => f64_array(czml_interval.get("boundingRectangle"))
            .filter(|array| array.len() == 4)
            .map(Unwrapped::Packed),
        CzmlPropertyType::Cartesian2 => f64_array(czml_interval.get("cartesian2"))
            .filter(|array| array.len() == 2)
            .map(Unwrapped::Packed),
        CzmlPropertyType::Cartesian3 => {
            unwrap_cartesian_interval(czml_interval).map(Unwrapped::Packed)
        }
        CzmlPropertyType::UnitCartesian3 => {
            unwrap_unit_cartesian_interval(czml_interval).map(Unwrapped::Packed)
        }
        CzmlPropertyType::Color => unwrap_color_interval(czml_interval).map(Unwrapped::Packed),
        CzmlPropertyType::ClassificationType => enum_value(
            czml_interval,
            "classificationType",
            classification_type_from_name,
        ),
        CzmlPropertyType::ColorBlendMode => enum_value(
            czml_interval,
            "colorBlendMode",
            color_blend_mode_from_name,
        ),
        CzmlPropertyType::CornerType => {
            enum_value(czml_interval, "cornerType", corner_type_from_name)
        }
        CzmlPropertyType::HeightReference => enum_value(
            czml_interval,
            "heightReference",
            height_reference_from_name,
        ),
        CzmlPropertyType::HorizontalOrigin => enum_value(
            czml_interval,
            "horizontalOrigin",
            horizontal_origin_from_name,
        ),
        CzmlPropertyType::Image | CzmlPropertyType::Uri => {
            unwrap_uri_interval(czml_interval, source_uri).map(Unwrapped::Text)
        }
        CzmlPropertyType::JulianDate => {
            let iso8601 = czml_interval
                .get("date")
                .and_then(|v| v.as_str())
                .or_else(|| czml_interval.as_str())?;
            let date = cesium_core::julian_date::JulianDate::from_iso8601(iso8601)?;
            Some(Unwrapped::Scalar(CzmlValue::Date(date)))
        }
        CzmlPropertyType::LabelStyle => {
            let name = czml_interval
                .get("labelStyle")
                .and_then(|v| v.as_str())
                .or_else(|| czml_interval.as_str())?;
            Some(Unwrapped::Scalar(CzmlValue::Number(
                label_style_from_name(name)? as f64,
            )))
        }
        CzmlPropertyType::Number => unwrap_number_interval(czml_interval),
        CzmlPropertyType::NearFarScalar => f64_array(czml_interval.get("nearFarScalar"))
            .filter(|array| array.len() == 4)
            .map(Unwrapped::Packed),
        CzmlPropertyType::DistanceDisplayCondition => {
            f64_array(czml_interval.get("distanceDisplayCondition"))
                .filter(|array| array.len() == 2)
                .map(Unwrapped::Packed)
        }
        CzmlPropertyType::Object => {
            let payload = czml_interval
                .get("object")
                .or_else(|| czml_interval.get("value"))
                .unwrap_or(czml_interval);
            Some(Unwrapped::Json(payload.clone()))
        }
        CzmlPropertyType::PathMode => enum_value(czml_interval, "pathMode", path_mode_from_name),
        CzmlPropertyType::Quaternion => {
            unwrap_quaternion_interval(czml_interval).map(Unwrapped::Packed)
        }
        CzmlPropertyType::Rotation => unwrap_number_interval(czml_interval),
        CzmlPropertyType::ShadowMode => {
            let name = czml_interval
                .get("shadowMode")
                .and_then(|v| v.as_str())
                .or_else(|| czml_interval.get("shadows").and_then(|v| v.as_str()))
                .or_else(|| czml_interval.as_str())?;
            Some(Unwrapped::Scalar(CzmlValue::Number(
                shadow_mode_from_name(name)? as f64,
            )))
        }
        CzmlPropertyType::String => {
            let string = czml_interval
                .get("string")
                .and_then(|v| v.as_str())
                .or_else(|| czml_interval.as_str())?;
            Some(Unwrapped::Text(string.to_string()))
        }
        CzmlPropertyType::StripeOrientation => enum_value(
            czml_interval,
            "stripeOrientation",
            stripe_orientation_from_name,
        ),
        CzmlPropertyType::Rectangle => {
            unwrap_rectangle_interval(czml_interval).map(Unwrapped::Packed)
        }
        CzmlPropertyType::VerticalOrigin => enum_value(
            czml_interval,
            "verticalOrigin",
            vertical_origin_from_name,
        ),
    }
}

// ============================================================================
// Constant unpacking (the `needsUnpacking` path of processProperty)
// ============================================================================

/// Unpacks a constant [`Unwrapped`] payload into a [`CzmlValue`] (mirror of
/// `type.unpack(unwrappedInterval, 0)` in the constant paths of
/// `processProperty`).
pub fn unpack_constant_value(r#type: CzmlPropertyType, unwrapped: Unwrapped) -> CzmlValue {
    match unwrapped {
        Unwrapped::Scalar(value) => value,
        Unwrapped::Text(text) => CzmlValue::Text(text),
        Unwrapped::Json(json) => CzmlValue::Json(json),
        Unwrapped::Packed(packed) => match r#type {
            CzmlPropertyType::Cartesian3 => CzmlValue::Cartesian3(Cartesian3::new(
                packed[0], packed[1], packed[2],
            )),
            CzmlPropertyType::UnitCartesian3 => {
                let mut result = Cartesian3::new(packed[0], packed[1], packed[2]);
                let input = result;
                Cartesian3::normalize(&input, &mut result);
                CzmlValue::UnitCartesian3(result)
            }
            CzmlPropertyType::Color => {
                CzmlValue::Color(packed[0], packed[1], packed[2], packed[3])
            }
            CzmlPropertyType::Quaternion => CzmlValue::Quaternion(Quaternion::new(
                packed[0], packed[1], packed[2], packed[3],
            )),
            CzmlPropertyType::Rectangle => {
                CzmlValue::Rectangle(packed[0], packed[1], packed[2], packed[3])
            }
            CzmlPropertyType::NearFarScalar => {
                CzmlValue::NearFarScalar(packed[0], packed[1], packed[2], packed[3])
            }
            CzmlPropertyType::DistanceDisplayCondition => {
                CzmlValue::DistanceDisplayCondition(packed[0], packed[1])
            }
            CzmlPropertyType::Cartesian2 => CzmlValue::Cartesian2(Cartesian2 {
                x: packed[0],
                y: packed[1],
            }),
            CzmlPropertyType::BoundingRectangle => {
                CzmlValue::BoundingRectangle(packed[0], packed[1], packed[2], packed[3])
            }
            // Non-packable types never produce Packed payloads.
            _ => CzmlValue::NumberArray(packed),
        },
    }
}
