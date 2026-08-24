//! Ported from `packages/engine/Source/Workers/gaussianSplatSorter.js`.
//!
//! Worker entry point for sorting Gaussian splats by depth.
//! Gaussian splatting requires front-to-back sorting for correct alpha blending.

/// Sorts Gaussian splats.
///
/// In CesiumJS, this receives Gaussian splat data (positions, colors, opacities)
/// and the current view matrix, then sorts the splats by depth for correct
/// rendering order.
pub fn gaussian_splat_sorter(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("gaussianSplatSorter"))
}

/// Sorts Gaussian splats by depth (for in-process use).
///
/// # Arguments
/// * `splat_positions` - Flat array of splat center positions (x,y,z triplets).
/// * `view_matrix` - 4×4 column-major view matrix (16 f64 values).
///
/// Returns sorted indices as `Vec<u32>`.
pub fn gaussian_splat_sorter_unpacked(
    _splat_positions: &[f64],
    _view_matrix: &[f64; 16],
) -> Vec<u32> {
    // DEVIATION: Depth sorting not yet implemented
    Vec::new()
}
