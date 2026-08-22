//! Ported from `packages/engine/Source/Core/S2Cell.js`.
//!
//! S2 cell decomposition for the unit sphere.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use std::sync::OnceLock;

const S2_MAX_LEVEL: u32 = 30;
const S2_LIMIT_IJ: u32 = 1 << S2_MAX_LEVEL;
const S2_MAX_SITI: u64 = (1u64 << (S2_MAX_LEVEL + 1)) as u64;
const S2_POSITION_BITS: u32 = 2 * S2_MAX_LEVEL + 1;
const S2_LOOKUP_BITS: u32 = 4;
const S2_SWAP_MASK: u32 = 1;
const S2_INVERT_MASK: u32 = 2;

static S2_LOOKUP_POSITIONS: OnceLock<Vec<u32>> = OnceLock::new();
static S2_LOOKUP_IJ: OnceLock<Vec<u32>> = OnceLock::new();

static S2_POSITION_TO_IJ: [[u32; 4]; 4] = [
    [0, 1, 3, 2],
    [0, 2, 3, 1],
    [3, 2, 0, 1],
    [3, 1, 0, 2],
];

static S2_POSITION_TO_ORIENTATION_MASK: [u32; 4] = [
    S2_SWAP_MASK,
    0,
    0,
    S2_SWAP_MASK | S2_INVERT_MASK,
];

/// Lookup table for trailing zero bit positions.
static MOD67_BIT_POSITION: [u32; 68] = [
    64, 0, 1, 39, 2, 15, 40, 23, 3, 12, 16, 59, 41, 19, 24, 54, 4, 64, 13, 10, 17, 62, 60, 28,
    42, 30, 20, 51, 25, 44, 55, 47, 5, 32, 65, 38, 14, 22, 11, 58, 18, 53, 63, 9, 61, 27, 29,
    50, 43, 46, 31, 37, 21, 57, 52, 8, 26, 49, 45, 36, 56, 7, 48, 35, 6, 34, 33, 0,
];

/// Represents a cell in the S2 geometry library.
pub struct S2Cell {
    cell_id: u64,
    level: u32,
}

impl S2Cell {
    /// Creates a new S2Cell from a 64-bit cell ID.
    pub fn new(cell_id: u64) -> Self {
        assert!(Self::is_valid_id(cell_id), "Invalid S2 cell ID");
        let level = Self::get_level(cell_id);
        Self { cell_id, level }
    }

    /// Creates an S2Cell from a hex token.
    pub fn from_token(token: &str) -> Self {
        assert!(Self::is_valid_token(token), "Invalid S2 token");
        Self::new(Self::get_id_from_token(token))
    }

    /// Creates from face, Hilbert position, and level.
    pub fn from_face_position_level(face: u32, position: u64, level: u32) -> Self {
        assert!(face <= 5, "Invalid S2 Face (must be within 0-5)");
        assert!(level <= S2_MAX_LEVEL, "Invalid level (must be within 0-30)");

        let face_bits = face as u64;
        let _position_prefix_padding = 2 * level - count_bits(position);
        let position_suffix_padding = S2_POSITION_BITS - 2 * level;

        let cell_id = (face_bits << S2_POSITION_BITS)
            | (position << position_suffix_padding)
            | (1u64 << position_suffix_padding);

        Self::new(cell_id)
    }

    /// Validates an S2 cell ID.
    pub fn is_valid_id(cell_id: u64) -> bool {
        if cell_id == 0 {
            return false;
        }
        if cell_id >> S2_POSITION_BITS > 5 {
            return false;
        }
        let lowest_set_bit = cell_id & cell_id.wrapping_neg();
        if lowest_set_bit & 0x1555555555555555u64 == 0 {
            return false;
        }
        true
    }

    /// Validates an S2 cell token.
    pub fn is_valid_token(token: &str) -> bool {
        if token.is_empty() || token.len() > 16 {
            return false;
        }
        if !token.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        Self::is_valid_id(Self::get_id_from_token(token))
    }

    /// Converts a token to a cell ID.
    pub fn get_id_from_token(token: &str) -> u64 {
        let padded = format!("{:0>16}", token);
        u64::from_str_radix(&padded, 16).unwrap_or(0)
    }

    /// Converts a cell ID to a token.
    pub fn get_token_from_id(cell_id: u64) -> String {
        let trailing_zero_hex = count_trailing_zero_bits(cell_id) / 4;
        let hex_string = format!("{:x}", cell_id);
        let hex_trimmed = hex_string.trim_end_matches('0');
        let zero_padding = 16usize.saturating_sub(trailing_zero_hex as usize + hex_trimmed.len());
        format!("{}{}", "0".repeat(zero_padding), hex_trimmed)
    }

    /// Gets the level from a cell ID.
    pub fn get_level(cell_id: u64) -> u32 {
        let mut lsb_position = 0u32;
        let mut id = cell_id;
        while id != 0 {
            if id & 1 != 0 {
                break;
            }
            lsb_position += 1;
            id >>= 1;
        }
        S2_MAX_LEVEL - (lsb_position / 2)
    }

    /// Gets the cell ID.
    pub fn cell_id(&self) -> u64 {
        self.cell_id
    }

    /// Gets the level.
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Gets the child cell at the given index (0-3).
    pub fn get_child(&self, index: u32) -> S2Cell {
        assert!(index <= 3, "child index must be in the range [0-3]");
        assert!(self.level < 30, "cannot get child of leaf cell");

        let new_lsb = lsb(self.cell_id) >> 2;
        let child_cell_id = self.cell_id.wrapping_add((2 * index + 1).wrapping_sub(4) as u64 * new_lsb);
        S2Cell::new(child_cell_id)
    }

    /// Gets the parent cell.
    pub fn get_parent(&self) -> S2Cell {
        assert!(self.level > 0, "cannot get parent of root cell");
        let new_lsb = lsb(self.cell_id) << 2;
        S2Cell::new((self.cell_id & new_lsb.wrapping_neg()) | new_lsb)
    }

    /// Gets the parent at a specific level.
    pub fn get_parent_at_level(&self, level: u32) -> S2Cell {
        assert!(
            self.level > 0 && level <= self.level,
            "cannot get parent at invalid level"
        );
        let new_lsb = lsb_for_level(level);
        S2Cell::new((self.cell_id & new_lsb.wrapping_neg()) | new_lsb)
    }

    /// Gets the center of the cell on the given ellipsoid.
    pub fn get_center(&self, ellipsoid: Option<&Ellipsoid>) -> Cartesian3 {
        let ellipsoid = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let center = get_s2_center(self.cell_id, self.level);
        let normalized = Cartesian3::normalize_new(&center);
        let mut carto = Cartographic::default();
        Cartographic::from_cartesian(&normalized, Some(&ellipsoid.ellipsoid_params()), &mut carto);
        Cartographic::to_cartesian_new(&carto)
    }

    /// Gets a vertex (0-3) of the cell on the given ellipsoid.
    pub fn get_vertex(&self, index: u32, ellipsoid: Option<&Ellipsoid>) -> Cartesian3 {
        assert!(index <= 3, "vertex index must be in the range [0-3]");
        let ellipsoid = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let vertex = get_s2_vertex(self.cell_id, self.level, index);
        let normalized = Cartesian3::normalize_new(&vertex);
        let mut carto = Cartographic::default();
        Cartographic::from_cartesian(&normalized, Some(&ellipsoid.ellipsoid_params()), &mut carto);
        Cartographic::to_cartesian_new(&carto)
    }
}

// --- Internal helpers ---

fn lsb(cell_id: u64) -> u64 {
    cell_id & cell_id.wrapping_neg()
}

fn lsb_for_level(level: u32) -> u64 {
    1u64 << (2 * (S2_MAX_LEVEL - level))
}

fn count_trailing_zero_bits(x: u64) -> u32 {
    if x == 0 {
        return 64;
    }
    let isolated = x & x.wrapping_neg();
    MOD67_BIT_POSITION[(isolated % 67) as usize]
}

fn count_bits(v: u64) -> u32 {
    if v == 0 {
        return 0;
    }
    64 - v.leading_zeros()
}

fn generate_lookup_cell(
    level: u32,
    mut i: u32,
    mut j: u32,
    original_orientation: u32,
    mut position: u32,
    orientation: u32,
    lookup_positions: &mut Vec<u32>,
    lookup_ij: &mut Vec<u32>,
) {
    if level == S2_LOOKUP_BITS {
        let ij = (i << S2_LOOKUP_BITS) + j;
        let idx = ((ij << 2) + original_orientation) as usize;
        if idx >= lookup_positions.len() {
            lookup_positions.resize(idx + 1, 0);
            lookup_ij.resize(idx + 1, 0);
        }
        lookup_positions[idx] = (position << 2) + orientation;
        lookup_ij[((position << 2) + original_orientation) as usize] = (ij << 2) + orientation;
    } else {
        let lvl = level + 1;
        i <<= 1;
        j <<= 1;
        position <<= 2;
        let r = S2_POSITION_TO_IJ[orientation as usize];

        for k in 0..4u32 {
            generate_lookup_cell(
                lvl,
                i + (r[k as usize] >> 1),
                j + (r[k as usize] & 1),
                original_orientation,
                position + k,
                orientation ^ S2_POSITION_TO_ORIENTATION_MASK[k as usize],
                lookup_positions,
                lookup_ij,
            );
        }
    }
}

fn ensure_lookup_tables() {
    S2_LOOKUP_POSITIONS.get_or_init(|| {
        let mut positions = Vec::new();
        let mut ij = Vec::new();
        generate_lookup_cell(0, 0, 0, 0, 0, 0, &mut positions, &mut ij);
        generate_lookup_cell(0, 0, 0, S2_SWAP_MASK, 0, S2_SWAP_MASK, &mut positions, &mut ij);
        generate_lookup_cell(0, 0, 0, S2_INVERT_MASK, 0, S2_INVERT_MASK, &mut positions, &mut ij);
        generate_lookup_cell(
            0, 0, 0,
            S2_SWAP_MASK | S2_INVERT_MASK,
            0,
            S2_SWAP_MASK | S2_INVERT_MASK,
            &mut positions,
            &mut ij,
        );
        positions
    });
    S2_LOOKUP_IJ.get_or_init(|| {
        let mut positions = Vec::new();
        let mut ij = Vec::new();
        generate_lookup_cell(0, 0, 0, 0, 0, 0, &mut positions, &mut ij);
        generate_lookup_cell(0, 0, 0, S2_SWAP_MASK, 0, S2_SWAP_MASK, &mut positions, &mut ij);
        generate_lookup_cell(0, 0, 0, S2_INVERT_MASK, 0, S2_INVERT_MASK, &mut positions, &mut ij);
        generate_lookup_cell(
            0, 0, 0,
            S2_SWAP_MASK | S2_INVERT_MASK,
            0,
            S2_SWAP_MASK | S2_INVERT_MASK,
            &mut positions,
            &mut ij,
        );
        ij
    });
}

fn convert_cell_id_to_face_ij(cell_id: u64) -> [u32; 3] {
    ensure_lookup_tables();
    let lookup_ij = S2_LOOKUP_IJ.get().unwrap();

    let face = (cell_id >> S2_POSITION_BITS) as u32;
    let mut bits = face & S2_SWAP_MASK;
    let lookup_mask = (1u32 << S2_LOOKUP_BITS) - 1;

    let mut i: u32 = 0;
    let mut j: u32 = 0;

    for k in (0..8u32).rev() {
        let number_of_bits = if k == 7 {
            S2_MAX_LEVEL - 7 * S2_LOOKUP_BITS
        } else {
            S2_LOOKUP_BITS
        };
        let extract_mask = (1u64 << (2 * number_of_bits)) - 1;
        bits += ((cell_id >> (k * 2 * S2_LOOKUP_BITS + 1)) & extract_mask) as u32;
        bits = lookup_ij[bits as usize];

        let offset = k * S2_LOOKUP_BITS;
        i += (bits >> (S2_LOOKUP_BITS + 2)) << offset;
        j += ((bits >> 2) & lookup_mask) << offset;
        bits &= S2_SWAP_MASK | S2_INVERT_MASK;
    }

    [face, i, j]
}

fn convert_cell_id_to_face_si_ti(cell_id: u64, level: u32) -> [u32; 3] {
    let face_ij = convert_cell_id_to_face_ij(cell_id);
    let face = face_ij[0];
    let i = face_ij[1];
    let j = face_ij[2];

    let is_leaf = level == 30;
    let should_correct = !is_leaf && ((i as u64 ^ (cell_id >> 2)) & 1) != 0;
    let correction = if is_leaf { 1 } else if should_correct { 2 } else { 0 };
    let si = (i << 1) + correction;
    let ti = (j << 1) + correction;
    [face, si, ti]
}

fn convert_si_ti_to_st(si: u32) -> f64 {
    (1.0 / S2_MAX_SITI as f64) * si as f64
}

fn convert_st_to_uv(s: f64) -> f64 {
    if s >= 0.5 {
        (1.0 / 3.0) * (4.0 * s * s - 1.0)
    } else {
        (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s))
    }
}

fn convert_face_uv_to_xyz(face: u32, u: f64, v: f64) -> Cartesian3 {
    match face {
        0 => Cartesian3::new(1.0, u, v),
        1 => Cartesian3::new(-u, 1.0, v),
        2 => Cartesian3::new(-u, -v, 1.0),
        3 => Cartesian3::new(-1.0, -v, -u),
        4 => Cartesian3::new(v, -1.0, -u),
        _ => Cartesian3::new(v, u, -1.0),
    }
}

fn convert_face_si_ti_to_xyz(face: u32, si: u32, ti: u32) -> Cartesian3 {
    let s = convert_si_ti_to_st(si);
    let t = convert_si_ti_to_st(ti);
    let u = convert_st_to_uv(s);
    let v = convert_st_to_uv(t);
    convert_face_uv_to_xyz(face, u, v)
}

fn get_size_ij(level: u32) -> u32 {
    1 << (S2_MAX_LEVEL - level)
}

fn convert_ij_to_st_minimum(i: u32) -> f64 {
    (1.0 / S2_LIMIT_IJ as f64) * i as f64
}

fn convert_ij_level_to_bound_uv(ij: &[u32; 2], level: u32) -> [[f64; 2]; 2] {
    let cell_size = get_size_ij(level);
    let mut result = [[0.0; 2]; 2];
    for d in 0..2 {
        let ij_low = ij[d] & cell_size.wrapping_neg();
        let ij_high = ij_low + cell_size;
        result[d][0] = convert_st_to_uv(convert_ij_to_st_minimum(ij_low));
        result[d][1] = convert_st_to_uv(convert_ij_to_st_minimum(ij_high));
    }
    result
}

fn get_s2_center(cell_id: u64, level: u32) -> Cartesian3 {
    let face_si_ti = convert_cell_id_to_face_si_ti(cell_id, level);
    convert_face_si_ti_to_xyz(face_si_ti[0], face_si_ti[1], face_si_ti[2])
}

fn get_s2_vertex(cell_id: u64, level: u32, index: u32) -> Cartesian3 {
    let face_ij = convert_cell_id_to_face_ij(cell_id);
    let uv = convert_ij_level_to_bound_uv(&[face_ij[1], face_ij[2]], level);
    let y = ((index >> 1) & 1) as usize;
    convert_face_uv_to_xyz(
        face_ij[0],
        uv[0][y ^ (index as usize & 1)],
        uv[1][y],
    )
}
