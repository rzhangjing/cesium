//! Ported from `packages/engine/Source/Core/TileEdge.js`.

/// Identifies edges and corners of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TileEdge {
    West = 0,
    North = 1,
    East = 2,
    South = 3,
    Northwest = 4,
    Northeast = 5,
    Southwest = 6,
    Southeast = 7,
}
