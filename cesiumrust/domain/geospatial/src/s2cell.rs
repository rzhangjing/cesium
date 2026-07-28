//! S2Cell - S2 geometry library cell representation.
//! Maps to CesiumJS `Core/S2Cell.js`
//!
//! Based on the S2 C++ reference implementation: https://github.com/google/s2geometry

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use glam::DVec3;

const S2_MAX_LEVEL: u32 = 30;
const S2_LIMIT_IJ: u32 = 1 << S2_MAX_LEVEL; // 2^30
const S2_MAX_SITI: u32 = 1 << (S2_MAX_LEVEL + 1); // 2^31
const S2_POSITION_BITS: u32 = 2 * S2_MAX_LEVEL + 1; // 61
const S2_LOOKUP_BITS: u32 = 4;
const S2_SWAP_MASK: u32 = 1;
const S2_INVERT_MASK: u32 = 2;

const S2_POSITION_TO_IJ: [[u32; 4]; 4] = [
    [0, 1, 3, 2], // 0: Normal order
    [0, 2, 3, 1], // 1: Swap bit set
    [3, 2, 0, 1], // 2: Invert bit set
    [3, 1, 0, 2], // 3: Swap and invert
];

const S2_POSITION_TO_ORIENTATION_MASK: [u32; 4] = [S2_SWAP_MASK, 0, 0, S2_SWAP_MASK | S2_INVERT_MASK];

/// Lookup tables (generated lazily)
struct LookupTables {
    positions: Vec<u32>, // 1024 entries
    ij: Vec<u32>,        // 1024 entries
}

static LOOKUP_TABLES: std::sync::LazyLock<LookupTables> = std::sync::LazyLock::new(|| {
    let mut positions = vec![0u32; 1024];
    let mut ij = vec![0u32; 1024];
    generate_lookup_cell(0, 0, 0, 0, 0, 0, &mut positions, &mut ij);
    generate_lookup_cell(0, 0, 0, S2_SWAP_MASK, 0, S2_SWAP_MASK, &mut positions, &mut ij);
    generate_lookup_cell(0, 0, 0, S2_INVERT_MASK, 0, S2_INVERT_MASK, &mut positions, &mut ij);
    generate_lookup_cell(
        0, 0, 0, S2_SWAP_MASK | S2_INVERT_MASK, 0, S2_SWAP_MASK | S2_INVERT_MASK,
        &mut positions, &mut ij,
    );
    LookupTables { positions, ij }
});

fn generate_lookup_cell(
    level: u32,
    i: u32,
    j: u32,
    original_orientation: u32,
    position: u32,
    orientation: u32,
    lookup_positions: &mut [u32],
    lookup_ij: &mut [u32],
) {
    if level == S2_LOOKUP_BITS {
        let ij_val = (i << S2_LOOKUP_BITS) + j;
        lookup_positions[((ij_val << 2) + original_orientation) as usize] =
            (position << 2) + orientation;
        lookup_ij[((position << 2) + original_orientation) as usize] =
            (ij_val << 2) + orientation;
    } else {
        let level = level + 1;
        let i = i << 1;
        let j = j << 1;
        let position = position << 2;
        let r = &S2_POSITION_TO_IJ[orientation as usize];
        generate_lookup_cell(level, i + (r[0] >> 1), j + (r[0] & 1), original_orientation, position, orientation ^ S2_POSITION_TO_ORIENTATION_MASK[0], lookup_positions, lookup_ij);
        generate_lookup_cell(level, i + (r[1] >> 1), j + (r[1] & 1), original_orientation, position + 1, orientation ^ S2_POSITION_TO_ORIENTATION_MASK[1], lookup_positions, lookup_ij);
        generate_lookup_cell(level, i + (r[2] >> 1), j + (r[2] & 1), original_orientation, position + 2, orientation ^ S2_POSITION_TO_ORIENTATION_MASK[2], lookup_positions, lookup_ij);
        generate_lookup_cell(level, i + (r[3] >> 1), j + (r[3] & 1), original_orientation, position + 3, orientation ^ S2_POSITION_TO_ORIENTATION_MASK[3], lookup_positions, lookup_ij);
    }
}

/// An S2 cell on the unit sphere.
/// Maps to CesiumJS `Core/S2Cell`
#[derive(Debug, Clone, PartialEq)]
pub struct S2Cell {
    cell_id: u128,
    level: u32,
}

impl S2Cell {
    /// Creates a new S2Cell from a 64-bit cell ID.
    pub fn new(cell_id: u128) -> Self {
        debug_assert!(Self::is_valid_id(cell_id), "cell ID is invalid");
        let level = Self::get_level(cell_id);
        Self { cell_id, level }
    }

    /// Creates a new S2Cell from a token (hex representation).
    /// Maps to `S2Cell.fromToken`
    pub fn from_token(token: &str) -> Self {
        debug_assert!(Self::is_valid_token(token), "token is invalid");
        Self::new(Self::get_id_from_token(token))
    }

    /// Creates an S2Cell from face, position along Hilbert curve, and level.
    /// Maps to `S2Cell.fromFacePositionLevel`
    pub fn from_face_position_level(face: u32, position: u128, level: u32) -> Self {
        debug_assert!(face <= 5, "Invalid S2 Face (must be within 0-5)");
        debug_assert!(level <= S2_MAX_LEVEL, "Invalid level (must be within 0-30)");
        let max_position: u128 = 1u128 << (2 * level);
        debug_assert!(position < max_position, "Invalid Hilbert position for level");

        // Build cell ID: face (3 bits) + position (2*level bits) + sentinel (1 bit) + padding
        let face_bits = (face as u128) << S2_POSITION_BITS;
        let position_shift = S2_POSITION_BITS - 2 * level - 1;
        let position_bits = position << (position_shift + 1);
        let sentinel_bit: u128 = 1 << position_shift;
        let cell_id = face_bits | position_bits | sentinel_bit;
        Self::new(cell_id)
    }

    /// The cell ID.
    pub fn cell_id(&self) -> u128 {
        self.cell_id
    }

    /// The level of this cell.
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Validates an S2 cell ID.
    /// Maps to `S2Cell.isValidId`
    pub fn is_valid_id(cell_id: u128) -> bool {
        if cell_id == 0 {
            return false;
        }
        // Check face bits [0-5]
        if cell_id >> S2_POSITION_BITS > 5 {
            return false;
        }
        // Check trailing 1 bit is in valid even position
        let lowest_set_bit = cell_id & (!cell_id + 1);
        if lowest_set_bit & 0x1555555555555555u128 == 0 {
            return false;
        }
        true
    }

    /// Validates an S2 cell token.
    /// Maps to `S2Cell.isValidToken`
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
    /// Maps to `S2Cell.getIdFromToken`
    pub fn get_id_from_token(token: &str) -> u128 {
        let padded = format!("{}{}", token, "0".repeat(16 - token.len()));
        u128::from_str_radix(&padded, 16).unwrap_or(0)
    }

    /// Converts a cell ID to a token.
    /// Maps to `S2Cell.getTokenFromId`
    pub fn get_token_from_id(cell_id: u128) -> String {
        let trailing_zero_bits = cell_id.trailing_zeros();
        let trailing_zero_hex = trailing_zero_bits / 4;
        let hex_string = format!("{:x}", cell_id);
        let hex_string = hex_string.trim_end_matches('0');
        let hex_string = if hex_string.is_empty() { "0" } else { hex_string };
        let zero_count = 16usize.saturating_sub(trailing_zero_hex as usize).saturating_sub(hex_string.len());
        format!("{}{}", "0".repeat(zero_count), hex_string)
    }

    /// Gets the level from a cell ID.
    /// Maps to `S2Cell.getLevel`
    pub fn get_level(cell_id: u128) -> u32 {
        let lsb_position = cell_id.trailing_zeros();
        S2_MAX_LEVEL - (lsb_position >> 1)
    }

    /// Gets the child cell at the given index [0-3].
    /// Maps to `S2Cell.prototype.getChild`
    pub fn get_child(&self, index: u32) -> Self {
        debug_assert!(index <= 3, "child index must be in the range [0-3]");
        debug_assert!(self.level != 30, "cannot get child of leaf cell");
        let new_lsb = lsb(self.cell_id) >> 2;
        // Formula: cellId + (2*index + 1 - 4) * newLsb
        // Use i128 to handle negative multiplier for index 0,1
        let multiplier = (2 * index as i128 + 1 - 4) as i128;
        let child_cell_id = (self.cell_id as i128 + multiplier * new_lsb as i128) as u128;
        Self::new(child_cell_id)
    }

    /// Gets the parent cell.
    /// Maps to `S2Cell.prototype.getParent`
    pub fn get_parent(&self) -> Self {
        debug_assert!(self.level != 0, "cannot get parent of root cell");
        let new_lsb = lsb(self.cell_id) << 2;
        Self::new((self.cell_id & (!new_lsb + 1)) | new_lsb)
    }

    /// Gets the parent cell at the given level.
    /// Maps to `S2Cell.prototype.getParentAtLevel`
    pub fn get_parent_at_level(&self, level: u32) -> Self {
        debug_assert!(self.level != 0 && level <= self.level, "cannot get parent at invalid level");
        let new_lsb = lsb_for_level(level);
        // In u128 arithmetic: cellId & -newLsb is equivalent to cellId & (!newLsb + 1)
        Self::new((self.cell_id & (!new_lsb + 1)) | new_lsb)
    }

    /// Gets the center of the cell as a Cartesian3 on the given ellipsoid.
    /// Maps to `S2Cell.prototype.getCenter`
    pub fn get_center(&self, ellipsoid: &Ellipsoid) -> DVec3 {
        let center = get_s2_center(self.cell_id, self.level);
        let center = center.normalize();
        let cartographic = Cartographic::from_cartesian(center, &Ellipsoid::UNIT_SPHERE).unwrap();
        Cartographic::to_cartesian(&cartographic, ellipsoid)
    }

    /// Gets a vertex of the cell (CCW order, index [0-3]).
    /// Maps to `S2Cell.prototype.getVertex`
    pub fn get_vertex(&self, index: u32, ellipsoid: &Ellipsoid) -> DVec3 {
        debug_assert!(index <= 3, "vertex index must be in the range [0-3]");
        let vertex = get_s2_vertex(self.cell_id, self.level, index);
        let vertex = vertex.normalize();
        let cartographic = Cartographic::from_cartesian(vertex, &Ellipsoid::UNIT_SPHERE).unwrap();
        Cartographic::to_cartesian(&cartographic, ellipsoid)
    }
}

// =============================================================================
// Internal coordinate conversion functions
// =============================================================================

fn lsb(cell_id: u128) -> u128 {
    cell_id & (!cell_id + 1)
}

fn lsb_for_level(level: u32) -> u128 {
    1u128 << (2 * (S2_MAX_LEVEL - level))
}

fn get_s2_center(cell_id: u128, level: u32) -> DVec3 {
    let (face, si, ti) = convert_cell_id_to_face_siti(cell_id, level);
    convert_face_siti_to_xyz(face, si, ti)
}

fn get_s2_vertex(cell_id: u128, level: u32, index: u32) -> DVec3 {
    let (face, i, j) = convert_cell_id_to_face_ij(cell_id);
    let uv = convert_ij_level_to_bound_uv(i, j, level);
    // CCW ordering
    let y = (index >> 1) & 1;
    let u_idx = y ^ (index & 1);
    convert_face_uv_to_xyz(face, uv[0][u_idx as usize], uv[1][y as usize])
}

fn convert_cell_id_to_face_siti(cell_id: u128, level: u32) -> (u32, u32, u32) {
    let (face, i, j) = convert_cell_id_to_face_ij(cell_id);
    let is_leaf = level == 30;
    let should_correct = !is_leaf && (((i as u128) ^ (cell_id >> 2)) & 1) != 0;
    let correction = if is_leaf { 1 } else if should_correct { 2 } else { 0 };
    let si = (i << 1) + correction;
    let ti = (j << 1) + correction;
    (face, si, ti)
}

fn convert_cell_id_to_face_ij(cell_id: u128) -> (u32, u32, u32) {
    let tables = &LOOKUP_TABLES;
    let face = (cell_id >> S2_POSITION_BITS) as u32;
    let mut bits = face & S2_SWAP_MASK;
    let lookup_mask = (1u32 << S2_LOOKUP_BITS) - 1;

    let mut i: u32 = 0;
    let mut j: u32 = 0;

    for k in (0..8).rev() {
        let number_of_bits = if k == 7 {
            S2_MAX_LEVEL - 7 * S2_LOOKUP_BITS
        } else {
            S2_LOOKUP_BITS
        };
        let extract_mask = (1u128 << (2 * number_of_bits)) - 1;
        bits += (((cell_id >> (k * 2 * S2_LOOKUP_BITS + 1)) & extract_mask) as u32) << 2;

        bits = tables.ij[bits as usize];

        let offset = k * S2_LOOKUP_BITS;
        i += (bits >> (S2_LOOKUP_BITS + 2)) << offset;
        j += ((bits >> 2) & lookup_mask) << offset;

        bits &= S2_SWAP_MASK | S2_INVERT_MASK;
    }

    (face, i, j)
}

fn convert_face_siti_to_xyz(face: u32, si: u32, ti: u32) -> DVec3 {
    let s = convert_siti_to_st(si);
    let t = convert_siti_to_st(ti);
    let u = convert_st_to_uv(s);
    let v = convert_st_to_uv(t);
    convert_face_uv_to_xyz(face, u, v)
}

fn convert_face_uv_to_xyz(face: u32, u: f64, v: f64) -> DVec3 {
    match face {
        0 => DVec3::new(1.0, u, v),
        1 => DVec3::new(-u, 1.0, v),
        2 => DVec3::new(-u, -v, 1.0),
        3 => DVec3::new(-1.0, -v, -u),
        4 => DVec3::new(v, -1.0, -u),
        _ => DVec3::new(v, u, -1.0),
    }
}

/// Quadratic ST to UV transform.
fn convert_st_to_uv(s: f64) -> f64 {
    if s >= 0.5 {
        (1.0 / 3.0) * (4.0 * s * s - 1.0)
    } else {
        (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s))
    }
}

fn convert_siti_to_st(si: u32) -> f64 {
    (1.0 / S2_MAX_SITI as f64) * si as f64
}

fn convert_ij_level_to_bound_uv(i: u32, j: u32, level: u32) -> [[f64; 2]; 2] {
    let cell_size = get_size_ij(level);
    let mut result = [[0.0f64; 2]; 2];

    let ij_low = i & (!cell_size + 1); // i & -cellSize (unsigned wrap)
    let ij_high = ij_low + cell_size;
    result[0][0] = convert_st_to_uv(convert_ij_to_st_minimum(ij_low));
    result[0][1] = convert_st_to_uv(convert_ij_to_st_minimum(ij_high));

    let ij_low = j & (!cell_size + 1);
    let ij_high = ij_low + cell_size;
    result[1][0] = convert_st_to_uv(convert_ij_to_st_minimum(ij_low));
    result[1][1] = convert_st_to_uv(convert_ij_to_st_minimum(ij_high));

    result
}

fn get_size_ij(level: u32) -> u32 {
    1u32 << (S2_MAX_LEVEL - level)
}

fn convert_ij_to_st_minimum(i: u32) -> f64 {
    (1.0 / S2_LIMIT_IJ as f64) * i as f64
}
