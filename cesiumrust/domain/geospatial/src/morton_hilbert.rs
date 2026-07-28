//! Morton Order (Z-Order Curve) and Hilbert Order helper functions.
//! Maps to CesiumJS `Core/MortonOrder.js` and `Core/HilbertOrder.js`

// =============================================================================
// MortonOrder
// =============================================================================

/// Inserts one 0 bit of spacing between a number's bits.
/// Input: 16-bit unsigned integer → Output: 32-bit unsigned integer.
fn insert_one_spacing(v: u32) -> u32 {
    let mut v = v;
    v = (v ^ (v << 8)) & 0x00ff00ff;
    v = (v ^ (v << 4)) & 0x0f0f0f0f;
    v = (v ^ (v << 2)) & 0x33333333;
    v = (v ^ (v << 1)) & 0x55555555;
    v
}

/// Inserts two 0 bits of spacing between a number's bits.
/// Input: 10-bit unsigned integer → Output: 30-bit unsigned integer.
fn insert_two_spacing(v: u32) -> u32 {
    let mut v = v;
    v = (v ^ (v << 16)) & 0x030000ff;
    v = (v ^ (v << 8)) & 0x0300f00f;
    v = (v ^ (v << 4)) & 0x030c30c3;
    v = (v ^ (v << 2)) & 0x09249249;
    v
}

/// Removes one bit of spacing between bits.
/// Input: 32-bit unsigned integer → Output: 16-bit unsigned integer.
fn remove_one_spacing(v: u32) -> u32 {
    let mut v = v;
    v &= 0x55555555;
    v = (v ^ (v >> 1)) & 0x33333333;
    v = (v ^ (v >> 2)) & 0x0f0f0f0f;
    v = (v ^ (v >> 4)) & 0x00ff00ff;
    v = (v ^ (v >> 8)) & 0x0000ffff;
    v
}

/// Removes two bits of spacing between bits.
/// Input: 30-bit unsigned integer → Output: 10-bit unsigned integer.
fn remove_two_spacing(v: u32) -> u32 {
    let mut v = v;
    v &= 0x09249249;
    v = (v ^ (v >> 2)) & 0x030c30c3;
    v = (v ^ (v >> 4)) & 0x0300f00f;
    v = (v ^ (v >> 8)) & 0xff0000ff;
    v = (v ^ (v >> 16)) & 0x000003ff;
    v
}

/// Computes the Morton index from 2D coordinates (bit interleaving).
/// Inputs must be 16-bit unsigned integers [0, 65535].
/// Maps to `MortonOrder.encode2D`
pub fn morton_encode_2d(x: u32, y: u32) -> u32 {
    debug_assert!(x <= 65535 && y <= 65535, "inputs must be 16-bit unsigned integers");
    insert_one_spacing(x) | (insert_one_spacing(y) << 1)
}

/// Computes the 2D coordinates from a Morton index (bit deinterleaving).
/// Input must be a 32-bit unsigned integer [0, 4294967295].
/// Maps to `MortonOrder.decode2D`
pub fn morton_decode_2d(morton_index: u32) -> (u32, u32) {
    let x = remove_one_spacing(morton_index);
    let y = remove_one_spacing(morton_index >> 1);
    (x, y)
}

/// Computes the Morton index from 3D coordinates (bit interleaving).
/// Inputs must be 10-bit unsigned integers [0, 1023].
/// Maps to `MortonOrder.encode3D`
pub fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
    debug_assert!(x <= 1023 && y <= 1023 && z <= 1023, "inputs must be 10-bit unsigned integers");
    insert_two_spacing(x) | (insert_two_spacing(y) << 1) | (insert_two_spacing(z) << 2)
}

/// Computes the 3D coordinates from a Morton index (bit deinterleaving).
/// Input must be a 30-bit unsigned integer [0, 1073741823].
/// Maps to `MortonOrder.decode3D`
pub fn morton_decode_3d(morton_index: u32) -> (u32, u32, u32) {
    let x = remove_two_spacing(morton_index);
    let y = remove_two_spacing(morton_index >> 1);
    let z = remove_two_spacing(morton_index >> 2);
    (x, y, z)
}

// =============================================================================
// HilbertOrder
// =============================================================================

/// Rotate/flip a quadrant appropriately for Hilbert curve traversal.
fn hilbert_rotate(n: u32, x: &mut u32, y: &mut u32, rx: u32, ry: u32) {
    if ry != 0 {
        return;
    }
    if rx == 1 {
        *x = n - 1 - *x;
        *y = n - 1 - *y;
    }
    let t = *x;
    *x = *y;
    *y = t;
}

/// Computes the Hilbert index at the given level from 2D coordinates.
/// Maps to `HilbertOrder.encode2D`
pub fn hilbert_encode_2d(level: u32, x: u32, y: u32) -> u128 {
    let n: u32 = 1 << level;
    debug_assert!(level >= 1, "Hilbert level cannot be less than 1");
    debug_assert!(x < n && y < n, "Invalid coordinates for given level");

    let mut px = x;
    let mut py = y;
    let mut index: u128 = 0;

    let mut s = n / 2;
    while s > 0 {
        let rx = if (px & s) > 0 { 1u32 } else { 0u32 };
        let ry = if (py & s) > 0 { 1u32 } else { 0u32 };
        index += ((3 * rx) ^ ry) as u128 * (s as u128) * (s as u128);
        hilbert_rotate(n, &mut px, &mut py, rx, ry);
        s /= 2;
    }

    index
}

/// Computes the 2D coordinates from the Hilbert index at the given level.
/// Maps to `HilbertOrder.decode2D`
pub fn hilbert_decode_2d(level: u32, index: u128) -> (u32, u32) {
    debug_assert!(level >= 1, "Hilbert level cannot be less than 1");
    let n: u32 = 1 << level;
    let max_index: u128 = 1u128 << (2 * level);
    debug_assert!(index < max_index, "Hilbert index exceeds valid maximum for given level");

    let mut px: u32 = 0;
    let mut py: u32 = 0;
    let mut t = index;

    let mut s: u32 = 1;
    while s < n {
        let rx = 1u32 & ((t / 2) as u32);
        let ry = 1u32 & ((t ^ (rx as u128)) as u32);
        hilbert_rotate(s, &mut px, &mut py, rx, ry);
        px += s * rx;
        py += s * ry;
        t /= 4;
        s *= 2;
    }

    (px, py)
}
