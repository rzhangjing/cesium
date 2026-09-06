//! Ported from `packages/engine/Source/Scene/ImplicitSubdivisionScheme.js`.

/// The subdivision scheme for an implicit tileset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitSubdivisionScheme {
    /// A quadtree divides a parent tile into four children.
    Quadtree,
    /// An octree divides a parent tile into eight children.
    Octree,
}

impl ImplicitSubdivisionScheme {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Quadtree => "QUADTREE",
            Self::Octree => "OCTREE",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "QUADTREE" => Some(Self::Quadtree),
            "OCTREE" => Some(Self::Octree),
            _ => None,
        }
    }

    /// Get the branching factor for the given subdivision scheme.
    /// Returns 4 for QUADTREE or 8 for OCTREE.
    pub fn get_branching_factor(&self) -> u32 {
        match self {
            Self::Quadtree => 4,
            Self::Octree => 8,
        }
    }
}
