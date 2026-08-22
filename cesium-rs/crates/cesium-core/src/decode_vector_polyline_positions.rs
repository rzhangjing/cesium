//! Ported from `packages/engine/Source/Core/decodeVectorPolylinePositions.js`.
//!
//! Decodes compressed vector polyline positions.

use crate::attribute_compression::AttributeCompression;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;

const MAX_SHORT: f64 = 32767.0;

/// Decodes compressed polyline positions into Cartesian3 coordinates.
pub fn decode_vector_polyline_positions(
    positions: &[f64],
    rectangle: &Rectangle,
    minimum_height: f64,
    maximum_height: f64,
    ellipsoid: &Ellipsoid,
) -> Vec<f64> {
    let positions_length = positions.len() / 3;

    // Convert f64 positions to u16 buffers for decoding
    let mut u_buffer: Vec<u16> = positions[..positions_length]
        .iter()
        .map(|&v| v as u16)
        .collect();
    let mut v_buffer: Vec<u16> = positions[positions_length..2 * positions_length]
        .iter()
        .map(|&v| v as u16)
        .collect();
    let mut height_buffer: Vec<u16> = positions[2 * positions_length..]
        .iter()
        .map(|&v| v as u16)
        .collect();

    AttributeCompression::zig_zag_delta_decode(
        &mut u_buffer,
        &mut v_buffer,
        Some(&mut height_buffer),
    );

    let mut decoded = vec![0.0; positions.len()];
    for i in 0..positions_length {
        let u = u_buffer[i] as f64;
        let v = v_buffer[i] as f64;
        let h = height_buffer[i] as f64;

        let lon = CesiumMath::lerp(rectangle.west, rectangle.east, u / MAX_SHORT);
        let lat = CesiumMath::lerp(rectangle.south, rectangle.north, v / MAX_SHORT);
        let alt = CesiumMath::lerp(minimum_height, maximum_height, h / MAX_SHORT);

        let cartographic = Cartographic::from_radians_new(lon, lat, Some(alt));
        let mut decoded_position = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&cartographic, &mut decoded_position);

        decoded[i * 3] = decoded_position.x;
        decoded[i * 3 + 1] = decoded_position.y;
        decoded[i * 3 + 2] = decoded_position.z;
    }

    decoded
}
