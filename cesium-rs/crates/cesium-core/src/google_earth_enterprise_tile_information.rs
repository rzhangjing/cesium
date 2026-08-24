//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTileInformation.js`.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `childrenBitmasks` / `anyChildBitmask` / `cacheFlagBitmask` / `imageBitmask` / `terrainBitmask` | module constants | identical values |
//! | `GoogleEarthEnterpriseTileInformation` constructor | [`GoogleEarthEnterpriseTileInformation::new`] | identical |
//! | `GoogleEarthEnterpriseTileInformation.clone` | [`GoogleEarthEnterpriseTileInformation::clone_info`] | DEVIATION: no `result` parameter (returns a fresh copy) |
//! | `setParent` | [`GoogleEarthEnterpriseTileInformation::set_parent`] | identical |
//! | `hasSubtree` | [`GoogleEarthEnterpriseTileInformation::has_subtree`] | identical |
//! | `hasImagery` | [`GoogleEarthEnterpriseTileInformation::has_imagery`] | identical |
//! | `hasTerrain` | [`GoogleEarthEnterpriseTileInformation::has_terrain`] | identical |
//! | `hasChildren` | [`GoogleEarthEnterpriseTileInformation::has_children`] | identical |
//! | `hasChild` | [`GoogleEarthEnterpriseTileInformation::has_child`] | identical |
//! | `getChildBitmask` | [`GoogleEarthEnterpriseTileInformation::get_child_bitmask`] | identical |
//!
//! # DEVIATIONS
//!
//! 1. `clone(info, result)` has no `result` parameter; the Rust port always
//!    returns a fresh copy (`clone_info`).

use crate::is_bit_set::is_bit_set;

// Bitmask for checking tile properties
const CHILDREN_BITMASKS: [u32; 4] = [0x01, 0x02, 0x04, 0x08];
const ANY_CHILD_BITMASK: u32 = 0x0f;
const CACHE_FLAG_BITMASK: u32 = 0x10; // True if there is a child subtree
const IMAGE_BITMASK: u32 = 0x40;
const TERRAIN_BITMASK: u32 = 0x80;

/// Contains information about each tile from a Google Earth Enterprise
/// server.
#[derive(Debug, Clone)]
pub struct GoogleEarthEnterpriseTileInformation {
    bits: u32,
    /// Version of the request for subtree metadata.
    pub cnode_version: u32,
    /// Version of the request for imagery tile.
    pub imagery_version: u32,
    /// Version of the request for terrain tile.
    pub terrain_version: u32,
    /// Id of imagery provider.
    pub imagery_provider: u32,
    /// Id of terrain provider.
    pub terrain_provider: u32,
    /// Set it later once we find its parent.
    pub ancestor_has_terrain: bool,
    /// Terrain state tracked by the terrain provider (module-private
    /// `TerrainState` values; `None` mirrors the JS `undefined`).
    pub terrain_state: Option<u32>,
}

impl GoogleEarthEnterpriseTileInformation {
    /// Mirrors the JS constructor.
    pub fn new(
        bits: u32,
        cnode_version: u32,
        imagery_version: u32,
        terrain_version: u32,
        imagery_provider: u32,
        terrain_provider: u32,
    ) -> Self {
        Self {
            bits,
            cnode_version,
            imagery_version,
            terrain_version,
            imagery_provider,
            terrain_provider,
            ancestor_has_terrain: false,
            terrain_state: None,
        }
    }

    /// Creates a `GoogleEarthEnterpriseTileInformation` from another
    /// instance.
    ///
    /// Mirrors `GoogleEarthEnterpriseTileInformation.clone` (DEVIATION 1).
    pub fn clone_info(info: &GoogleEarthEnterpriseTileInformation) -> Self {
        let mut result = Self::new(
            info.bits,
            info.cnode_version,
            info.imagery_version,
            info.terrain_version,
            info.imagery_provider,
            info.terrain_provider,
        );
        result.ancestor_has_terrain = info.ancestor_has_terrain;
        result.terrain_state = info.terrain_state;
        result
    }

    /// The raw bitmask containing the type of data and available children.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// ORs additional bits into the raw bitmask (mirrors the direct
    /// `info._bits |= other` writes performed by
    /// `GoogleEarthEnterpriseMetadata#getQuadTreePacket` and the specs).
    pub fn or_bits(&mut self, other: u32) {
        self.bits |= other;
    }

    /// Replaces the raw bitmask (mirrors direct `info._bits = value` writes
    /// in the specs).
    pub fn set_bits(&mut self, bits: u32) {
        self.bits = bits;
    }

    /// Sets the parent for the tile.
    ///
    /// Mirrors `setParent(parent)`.
    pub fn set_parent(&mut self, parent: &GoogleEarthEnterpriseTileInformation) {
        self.ancestor_has_terrain = parent.ancestor_has_terrain || self.has_terrain();
    }

    /// Gets whether a subtree is available.
    ///
    /// Mirrors `hasSubtree()`.
    pub fn has_subtree(&self) -> bool {
        is_bit_set(self.bits, CACHE_FLAG_BITMASK)
    }

    /// Gets whether imagery is available.
    ///
    /// Mirrors `hasImagery()`.
    pub fn has_imagery(&self) -> bool {
        is_bit_set(self.bits, IMAGE_BITMASK)
    }

    /// Gets whether terrain is available.
    ///
    /// Mirrors `hasTerrain()`.
    pub fn has_terrain(&self) -> bool {
        is_bit_set(self.bits, TERRAIN_BITMASK)
    }

    /// Gets whether any children are present.
    ///
    /// Mirrors `hasChildren()`.
    pub fn has_children(&self) -> bool {
        is_bit_set(self.bits, ANY_CHILD_BITMASK)
    }

    /// Gets whether a specified child is available.
    ///
    /// Mirrors `hasChild(index)`.
    pub fn has_child(&self, index: usize) -> bool {
        is_bit_set(self.bits, CHILDREN_BITMASKS[index])
    }

    /// Gets bitmask containing children.
    ///
    /// Mirrors `getChildBitmask()`.
    pub fn get_child_bitmask(&self) -> u32 {
        self.bits & ANY_CHILD_BITMASK
    }
}
