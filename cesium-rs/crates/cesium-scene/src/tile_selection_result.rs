//! Ported from `packages/engine/Source/Scene/TileSelectionResult.js`.
//!
//! Indicates what happened the last time a tile was visited for selection.
//!
//! | CesiumJS                            | Rust port                                        |
//! | ----------------------------------- | ------------------------------------------------ |
//! | `NONE` / `CULLED` / `RENDERED` / `REFINED` | associated constants with the same values |
//! | `RENDERED_AND_KICKED` (`2 \| 4`)     | [`TileSelectionResult::RENDERED_AND_KICKED`]     |
//! | `REFINED_AND_KICKED` (`3 \| 4`)      | [`TileSelectionResult::REFINED_AND_KICKED`]      |
//! | `CULLED_BUT_NEEDED` (`1 \| 8`)       | [`TileSelectionResult::CULLED_BUT_NEEDED`]       |
//! | `wasKicked(value)`                   | [`TileSelectionResult::was_kicked`]              |
//! | `originalResult(value)`              | [`TileSelectionResult::original_result`]         |
//! | `kick(value)`                        | [`TileSelectionResult::kick`]                    |
//!
//! # Why a newtype and not an `enum`
//!
//! CesiumJS models this as a bag of integers plus bit-twiddling helpers, so the
//! value domain is closed under those helpers rather than under the seven named
//! constants: `kick(CULLED_BUT_NEEDED)` is `9 | 4 == 13`, which no enumeration
//! can name. `was_kicked` is likewise a *comparison* (`value >= 6`), not a
//! membership test, so CesiumJS deliberately reports `true` for
//! `CULLED_BUT_NEEDED` (`9 >= 6`) — `TerrainFillMesh.js` L248 depends on that.
//! A newtype over `i32` reproduces both quirks bit-exactly.
//!
//! # `TileSelectionResult.KICKED` does not exist
//!
//! `QuadtreePrimitive.js` L914 guards its ancestor kick loop with
//! `workTile._lastSelectionResult !== TileSelectionResult.KICKED`, but the
//! module never defines `KICKED`, so the right-hand side is `undefined` and the
//! comparison is *always* true. The loop therefore only stops at
//! `workTile === tile` or `workTile === undefined`. Because `kick` is
//! idempotent (`value | 4` applied twice equals applied once), re-walking an
//! already-kicked ancestor is a no-op and the observable behaviour matches a
//! guard that was intended to short-circuit. The Rust port mirrors the
//! *effective* behaviour and omits the dead guard; see
//! `docs/deviations.md`.

/// Indicates what happened the last time this tile was visited for selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileSelectionResult(pub i32);

impl TileSelectionResult {
    /// There was no selection result, perhaps because the tile wasn't visited
    /// last frame.
    pub const NONE: Self = Self(0);

    /// This tile was deemed not visible and culled.
    pub const CULLED: Self = Self(1);

    /// The tile was selected for rendering.
    pub const RENDERED: Self = Self(2);

    /// This tile did not meet the required screen-space error and was refined.
    pub const REFINED: Self = Self(3);

    /// This tile was originally rendered, but it got kicked out of the render
    /// list in favor of an ancestor because it is not yet renderable.
    ///
    /// CesiumJS spells this `2 | 4`.
    pub const RENDERED_AND_KICKED: Self = Self(2 | 4);

    /// This tile was originally refined, but its rendered descendants got
    /// kicked out of the render list in favor of an ancestor because it is not
    /// yet renderable.
    ///
    /// CesiumJS spells this `3 | 4`.
    pub const REFINED_AND_KICKED: Self = Self(3 | 4);

    /// This tile was culled because it was not visible, but it still needs to
    /// be loaded and any heights on it need to be updated because the camera's
    /// position or the camera's reference frame's origin falls inside this
    /// tile. Loading this tile could affect the position of the camera if the
    /// camera is currently below terrain or if it is tracking an object whose
    /// height is referenced to terrain. And a change in the camera position
    /// may, in turn, affect what is culled.
    ///
    /// CesiumJS spells this `1 | 8`.
    pub const CULLED_BUT_NEEDED: Self = Self(1 | 8);

    /// Returns the underlying integer, matching the CesiumJS constant value.
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Wraps a raw integer as a selection result. Unlike an enum conversion
    /// this is total: CesiumJS's `kick`/`originalResult` helpers routinely
    /// produce values outside the seven named constants.
    pub const fn from_i32(value: i32) -> Self {
        Self(value)
    }

    /// Determines if a selection result indicates that this tile or its
    /// descendants were kicked from the render list. In other words, if it is
    /// `RENDERED_AND_KICKED` or `REFINED_AND_KICKED`.
    ///
    /// Mirrors `wasKicked: value >= TileSelectionResult.RENDERED_AND_KICKED`.
    /// Being a comparison, this also returns `true` for `CULLED_BUT_NEEDED`
    /// (9 >= 6), exactly as CesiumJS does.
    pub const fn was_kicked(self) -> bool {
        self.0 >= Self::RENDERED_AND_KICKED.0
    }

    /// Determines the original selection result prior to being kicked or
    /// `CULLED_BUT_NEEDED`. If the tile wasn't kicked or `CULLED_BUT_NEEDED`,
    /// the original value is returned.
    ///
    /// Mirrors `originalResult: value & 3`.
    pub const fn original_result(self) -> Self {
        Self(self.0 & 3)
    }

    /// Converts this selection result to a kick.
    ///
    /// Mirrors `kick: value | 4`. Idempotent, which is what makes the missing
    /// `KICKED` guard in CesiumJS's kick loop harmless (see the module docs).
    pub const fn kick(self) -> Self {
        Self(self.0 | 4)
    }
}

impl Default for TileSelectionResult {
    /// Mirrors the `QuadtreeTile` constructor's
    /// `this._lastSelectionResult = TileSelectionResult.NONE`.
    fn default() -> Self {
        Self::NONE
    }
}
