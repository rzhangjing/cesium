//! A pick result: which element a cursor hit, on which part, with enough
//! context to rank competing candidates (plan §7).
//!
//! `PickHit` is plain data so the whole hit → selection ranking can be unit-
//! tested headless; the bridge only projects geometry to screen and calls the
//! pure [`crate::geom::hit`] primitives, then folds every candidate through
//! [`pick_best`].

use serde::{Deserialize, Serialize};

use super::ids::{ElementId, LayerId};

/// The part of an element a cursor landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Part {
    /// The filled interior of a face (lowest pick priority).
    Body,
    /// A vertex handle at index `i`.
    Vertex(usize),
    /// The `i`-th edge / segment (a line stroke or a polygon boundary).
    Edge(usize),
}

/// Candidate ranking buckets (§7: markers before lines before polygon edges
/// before polygon fills; smaller wins).
pub const RANK_MARKER: u8 = 0;
pub const RANK_LINE: u8 = 1;
pub const RANK_POLY_EDGE: u8 = 2;
pub const RANK_POLY_BODY: u8 = 3;

/// One element a cursor hit, ready to be ranked against the others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickHit {
    /// The element under the cursor.
    pub element: ElementId,
    /// Which part was hit.
    pub part: Part,
    /// The layer it lives in (for §9 permission checks downstream).
    pub layer: LayerId,
    /// The element's draw / pick order within the layer (higher wins ties).
    pub z_order: i32,
    /// Ranking bucket (one of the `RANK_*` constants).
    pub rank: u8,
    /// Screen distance from the cursor to the hit feature (pixels); tiebreak.
    pub screen_dist: f64,
}

impl PickHit {
    /// Sort key: rank ascending, then `z_order` descending, then screen distance
    /// ascending. Lexicographic so it composes into a single `min`.
    fn key(&self) -> (u8, i32, ordering::F64) {
        (self.rank, -self.z_order, ordering::F64(self.screen_dist))
    }
}

/// Choose the winner among candidates (plan §7 priority), or `None` if empty.
pub fn pick_best(cands: &[PickHit]) -> Option<PickHit> {
    cands
        .iter()
        .min_by(|a, b| a.key().cmp(&b.key()))
        .copied()
}

/// Small total-ordering wrapper so `f64` (not `Ord`) can sit in a sort key.
mod ordering {
    /// A wrapper giving `f64` a deterministic `Ord` (NaN sorts last).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct F64(pub f64);
    impl Eq for F64 {}
    impl PartialOrd for F64 {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for F64 {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
        }
    }
}
