//! Ported from `packages/engine/Source/Core/CylinderGeometryLibrary.js`.

use crate::math::CesiumMath;

/// Computes positions for a cylinder's side surface (and optionally top/bottom caps).
pub fn compute_positions(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: usize,
    fill: bool,
) -> Vec<f64> {
    let top_z = length * 0.5;
    let bottom_z = -top_z;

    let two_slice = slices + slices;
    let size = if fill { 2 * two_slice } else { two_slice };
    let mut positions = vec![0.0f64; size * 3];

    let bottom_offset = if fill { two_slice * 3 } else { 0 };
    let top_offset = if fill { (two_slice + slices) * 3 } else { slices * 3 };

    let mut i_pos = 0;
    let mut tb_index = 0;

    for i in 0..slices {
        let angle = (i as f64 / slices as f64) * CesiumMath::TWO_PI;
        let x = angle.cos();
        let y = angle.sin();
        let bottom_x = x * bottom_radius;
        let bottom_y = y * bottom_radius;
        let top_x = x * top_radius;
        let top_y = y * top_radius;

        positions[tb_index + bottom_offset] = bottom_x;
        positions[tb_index + bottom_offset + 1] = bottom_y;
        positions[tb_index + bottom_offset + 2] = bottom_z;

        positions[tb_index + top_offset] = top_x;
        positions[tb_index + top_offset + 1] = top_y;
        positions[tb_index + top_offset + 2] = top_z;
        tb_index += 3;

        if fill {
            positions[i_pos] = bottom_x;
            positions[i_pos + 1] = bottom_y;
            positions[i_pos + 2] = bottom_z;
            positions[i_pos + 3] = top_x;
            positions[i_pos + 4] = top_y;
            positions[i_pos + 5] = top_z;
            i_pos += 6;
        }
    }

    positions
}
