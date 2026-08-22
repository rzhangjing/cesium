//! Cache reordering: `reorderForPreVertexCache`, `reorderForPostVertexCache`.

use crate::geometry::Geometry;

/// Reorders attributes and indices for better GPU pre-vertex-shader cache performance.
///
/// TODO: full implementation.
pub fn reorder_for_pre_vertex_cache(geometry: &mut Geometry) {
    // TODO: implement — requires IndexDatatype.createTypedArray equivalent
    let _ = geometry;
}

/// Reorders indices for better GPU post-vertex-shader cache using Tipsify.
///
/// TODO: full implementation — requires Tipsify port.
pub fn reorder_for_post_vertex_cache(geometry: &mut Geometry, cache_capacity: Option<usize>) {
    let _ = (geometry, cache_capacity);
}
