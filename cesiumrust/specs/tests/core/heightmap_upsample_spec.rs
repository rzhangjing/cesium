//! Ported from CesiumJS `Core/HeightmapTerrainDataSpec.js` (upsample with stride/big-endian).
//!
//! A-class tests: stride, big-endian stride, stride + eastern child, clamp.

use cesium_terrain::heightmap::{
    get_height_from_buffer, set_height_in_buffer, HeightmapStructure, HeightmapTerrainData,
};

/// Helper: create a 4x4 heightmap with dummy heights (actual data is in the raw buffer).
fn make_4x4() -> HeightmapTerrainData {
    let heights = vec![0.0; 16];
    HeightmapTerrainData::new(heights, 4, 4, 0.0, 0.0)
}

// ---------------------------------------------------------------------------
// get_height_from_buffer / set_height_in_buffer unit tests
// ---------------------------------------------------------------------------

#[test]
fn get_height_little_endian() {
    // buffer[0]=1, buffer[1]=1 → LE: 1*256 + 1 = 257
    let buffer = [1u8, 1, 10];
    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        ..Default::default()
    };
    let h = get_height_from_buffer(&buffer, &structure, 0);
    assert_eq!(h, 257.0);
}

#[test]
fn get_height_big_endian() {
    // buffer[0]=1, buffer[1]=1 → BE: 1*256 + 1 = 257
    let buffer = [1u8, 1, 10];
    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        is_big_endian: true,
        ..Default::default()
    };
    let h = get_height_from_buffer(&buffer, &structure, 0);
    assert_eq!(h, 257.0);
}

#[test]
fn set_height_roundtrip_little_endian() {
    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        ..Default::default()
    };
    let mut buffer = vec![0u8; 6]; // 2 vertices
    set_height_in_buffer(&mut buffer, &structure, 0, 257.0);
    assert_eq!(buffer[0], 1); // low byte
    assert_eq!(buffer[1], 1); // high byte
    assert_eq!(buffer[2], 0); // padding

    let h = get_height_from_buffer(&buffer, &structure, 0);
    assert_eq!(h, 257.0);
}

#[test]
fn set_height_roundtrip_big_endian() {
    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        is_big_endian: true,
        ..Default::default()
    };
    let mut buffer = vec![0u8; 6];
    set_height_in_buffer(&mut buffer, &structure, 0, 257.0);
    assert_eq!(buffer[0], 1); // high byte
    assert_eq!(buffer[1], 1); // low byte
    assert_eq!(buffer[2], 0); // padding

    let h = get_height_from_buffer(&buffer, &structure, 0);
    assert_eq!(h, 257.0);
}

// ---------------------------------------------------------------------------
// upsample_with_structure: stride (little-endian)
// Ported from "upsample works with a stride"
// ---------------------------------------------------------------------------

#[test]
fn upsample_works_with_stride() {
    let data = make_4x4();

    // Input: heights 1..16 encoded as [val, 1, 10] per vertex (LE, stride=3, eph=2)
    // height N → bytes [N, 1] → decoded = 1*256 + N = 256+N
    let buffer: Vec<u8> = (1..=16u8)
        .flat_map(|n| [n, 1u8, 10])
        .collect();

    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        ..Default::default()
    };

    // Upsample SW child (descendant 0,0 at level 1)
    let result = data.upsample_with_structure(
        &buffer, &structure,
        0, 0, 0, // thisX, thisY, thisLevel
        0, 0, 1, // descendantX, descendantY, descendantLevel
    );

    // Expected from CesiumJS spec (raw bytes):
    let expected: Vec<u8> = vec![
        1, 1, 0, 1, 1, 0, 2, 1, 0, 2, 1, 0, 3, 1, 0, 3, 1, 0, 4, 1, 0, 4, 1, 0,
        5, 1, 0, 5, 1, 0, 6, 1, 0, 6, 1, 0, 7, 1, 0, 7, 1, 0, 8, 1, 0, 8, 1, 0,
    ];

    assert_eq!(result.len(), expected.len());
    for (i, (got, exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "mismatch at byte {}: got {}, expected {}",
            i, got, exp
        );
    }
}

// ---------------------------------------------------------------------------
// upsample_with_structure: big-endian stride
// Ported from "upsample works with a big endian stride"
// ---------------------------------------------------------------------------

#[test]
fn upsample_works_with_big_endian_stride() {
    let data = make_4x4();

    // Input: heights 1..16 encoded as [1, val, 10] per vertex (BE, stride=3, eph=2)
    // height N → bytes [1, N] → decoded = 1*256 + N = 256+N
    let buffer: Vec<u8> = (1..=16u8)
        .flat_map(|n| [1u8, n, 10])
        .collect();

    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        is_big_endian: true,
        ..Default::default()
    };

    // Upsample SW child
    let result = data.upsample_with_structure(
        &buffer, &structure,
        0, 0, 0,
        0, 0, 1,
    );

    // Expected from CesiumJS spec:
    let expected: Vec<u8> = vec![
        1, 1, 0, 1, 1, 0, 1, 2, 0, 1, 2, 0, 1, 3, 0, 1, 3, 0, 1, 4, 0, 1, 4, 0,
        1, 5, 0, 1, 5, 0, 1, 6, 0, 1, 6, 0, 1, 7, 0, 1, 7, 0, 1, 8, 0, 1, 8, 0,
    ];

    assert_eq!(result.len(), expected.len());
    for (i, (got, exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "mismatch at byte {}: got {}, expected {}",
            i, got, exp
        );
    }
}

// ---------------------------------------------------------------------------
// upsample_with_structure: stride + eastern child
// Ported from "upsample works with a stride for an eastern child"
// ---------------------------------------------------------------------------

#[test]
fn upsample_works_with_stride_eastern_child() {
    let data = make_4x4();

    // Same input as stride test
    let buffer: Vec<u8> = (1..=16u8)
        .flat_map(|n| [n, 1u8, 10])
        .collect();

    let structure = HeightmapStructure {
        stride: 3,
        elements_per_height: 2,
        ..Default::default()
    };

    // Upsample EASTERN child (descendant 1,0 at level 1)
    let result = data.upsample_with_structure(
        &buffer, &structure,
        0, 0, 0,
        1, 0, 1, // eastern child
    );

    // Expected from CesiumJS spec:
    let expected: Vec<u8> = vec![
        2, 1, 0, 3, 1, 0, 3, 1, 0, 4, 1, 0, 4, 1, 0, 5, 1, 0, 5, 1, 0, 6, 1, 0,
        6, 1, 0, 7, 1, 0, 7, 1, 0, 8, 1, 0, 8, 1, 0, 9, 1, 0, 9, 1, 0, 10, 1, 0,
    ];

    assert_eq!(result.len(), expected.len());
    for (i, (got, exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "mismatch at byte {}: got {}, expected {}",
            i, got, exp
        );
    }
}

// ---------------------------------------------------------------------------
// upsample_with_structure: clamp out of range
// Ported from "upsample clamps out of range data"
// ---------------------------------------------------------------------------

#[test]
fn upsample_clamps_out_of_range() {
    let data = make_4x4();

    // Input: heights [-1,-2,-3,-4, 5,6,7,8, 9,10,11,12, 13,14,15,16]
    // With stride=1, elementsPerHeight=1, these are stored as raw f64 in our
    // simplified model. But CesiumJS uses Float32Array with structure stride=1.
    // For this test, we use a Float32-like approach: store as i8 values in bytes.
    // Actually CesiumJS uses Float32Array for this test with lowestEncodedHeight=1,
    // highestEncodedHeight=7. The heights are decoded from mesh (float), then clamped.
    //
    // In our implementation, we decode from buffer using structure. With stride=1,
    // elementsPerHeight=1, buffer values ARE the heights.
    // But the input has negative values which can't be stored as u8.
    // CesiumJS uses Float32Array here, so let's simulate: the decoded heights
    // from the mesh would be [-1,-2,-3,-4, 5,6,7,8, 9,10,11,12, 13,14,15,16].
    // After clamping to [1,7]: [1,1,1,1, 5,6,7,7, 7,7,7,7, 7,7,7,7]
    // Then upsampled for SW child and clamped again.
    //
    // Since our upsample_with_structure works with u8 buffers, we'll test the
    // clamp logic separately using heights that fit in u8.
    // Use heights [0,0,0,0, 5,6,7,8, 9,10,11,12, 13,14,15,16] with clamp [1,7].
    let buffer: Vec<u8> = vec![
        0, 0, 0, 0, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    ];

    let structure = HeightmapStructure {
        stride: 1,
        elements_per_height: 1,
        lowest_encoded_height: Some(1.0),
        highest_encoded_height: Some(7.0),
        ..Default::default()
    };

    // Upsample SW child
    let result = data.upsample_with_structure(
        &buffer, &structure,
        0, 0, 0,
        0, 0, 1,
    );

    // The SW child covers the bottom-left quadrant.
    // Source grid (4x4, rows north→south):
    //   row0(north): 13, 14, 15, 16
    //   row1:         9, 10, 11, 12
    //   row2:         5,  6,  7,  8
    //   row3(south):  0,  0,  0,  0
    //
    // SW child: west half, south half → rows 2-3, cols 0-1
    // After clamp to [1,7]:
    //   row2: 5, 6, 7, 7
    //   row3: 1, 1, 1, 1
    //
    // Interpolated 4x4 output (north→south):
    //   j=0 (parent_v=0.5): between row2 and row1 (clamped)
    //   j=3 (parent_v=0.0): row3 (clamped)
    //
    // All values should be clamped to [1, 7]
    for (i, &val) in result.iter().enumerate() {
        assert!(
            val >= 1 && val <= 7,
            "byte {} = {} not in [1,7]",
            i,
            val
        );
    }
}
