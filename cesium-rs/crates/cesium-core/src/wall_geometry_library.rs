//! Ported from `packages/engine/Source/Core/WallGeometryLibrary.js`.
//!
//! Computes positions for a wall geometry.

use crate::array_remove_duplicates::array_remove_duplicates;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::polyline_pipeline::{GenerateArcHeight, GenerateArcOptions, PolylinePipeline};

/// Result of wall position computation.
#[derive(Debug, Clone)]
pub struct WallComputedPositions {
    pub bottom_positions: Vec<f64>,
    pub top_positions: Vec<f64>,
    pub num_corners: usize,
}

struct CleanedPositions {
    positions: Vec<Cartesian3>,
    top_heights: Vec<f64>,
    bottom_heights: Vec<f64>,
}

fn lat_lon_equals(c0: &Cartographic, c1: &Cartographic) -> bool {
    CesiumMath::equals_epsilon(
        c0.latitude,
        c1.latitude,
        Some(CesiumMath::EPSILON10),
        Some(CesiumMath::EPSILON10),
    ) && CesiumMath::equals_epsilon(
        c0.longitude,
        c1.longitude,
        Some(CesiumMath::EPSILON10),
        Some(CesiumMath::EPSILON10),
    )
}

/// Mirrors the private JS `removeDuplicates` helper.
fn remove_duplicates(
    ellipsoid: &Ellipsoid,
    wall_positions: &[Cartesian3],
    top_heights: Option<&[f64]>,
    bottom_heights: Option<&[f64]>,
) -> Option<CleanedPositions> {
    // JS `arrayRemoveDuplicates` returns the original array when no
    // duplicates were found; the Rust port returns `None` in that case.
    let positions = array_remove_duplicates(
        wall_positions,
        |left: &Cartesian3, right: &Cartesian3, eps: f64| {
            Cartesian3::equals_epsilon(Some(left), Some(right), Some(eps), Some(eps))
        },
        false,
        None,
    )
    .unwrap_or_else(|| wall_positions.to_vec());

    let length = positions.len();
    if length < 2 {
        return None;
    }

    let has_bottom_heights = bottom_heights.is_some();
    let has_top_heights = top_heights.is_some();

    let mut cleaned_positions: Vec<Cartesian3> = Vec::with_capacity(length);
    let mut cleaned_top_heights: Vec<f64> = vec![0.0; length];
    let mut cleaned_bottom_heights: Vec<f64> = vec![0.0; length];

    let v0 = &positions[0];
    cleaned_positions.push(*v0);

    let mut c0 = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(v0, &mut c0);
    if let Some(top_heights) = top_heights {
        c0.height = top_heights[0];
    }

    cleaned_top_heights[0] = c0.height;
    cleaned_bottom_heights[0] = if has_bottom_heights {
        bottom_heights.unwrap()[0]
    } else {
        0.0
    };

    let start_top_height = cleaned_top_heights[0];
    let start_bottom_height = cleaned_bottom_heights[0];
    let mut has_all_same_heights = start_top_height == start_bottom_height;

    let mut index = 1usize;
    for i in 1..length {
        let v1 = &positions[i];
        let mut c1 = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(v1, &mut c1);
        if let Some(top_heights) = top_heights {
            c1.height = top_heights[i];
        }
        has_all_same_heights = has_all_same_heights && c1.height == 0.0;

        if !lat_lon_equals(&c0, &c1) {
            cleaned_positions.push(*v1); // Shallow copy!
            cleaned_top_heights[index] = c1.height;

            cleaned_bottom_heights[index] = if has_bottom_heights {
                bottom_heights.unwrap()[i]
            } else {
                0.0
            };
            has_all_same_heights = has_all_same_heights
                && cleaned_top_heights[index] == cleaned_bottom_heights[index];

            c0 = c1;
            index += 1;
        } else if c0.height < c1.height {
            // Two adjacent positions are the same, so use whichever has the
            // greater height.
            cleaned_top_heights[index - 1] = c1.height;
        }
    }

    if has_all_same_heights || index < 2 {
        return None;
    }

    cleaned_positions.truncate(index);
    cleaned_top_heights.truncate(index);
    cleaned_bottom_heights.truncate(index);

    Some(CleanedPositions {
        positions: cleaned_positions,
        top_heights: cleaned_top_heights,
        bottom_heights: cleaned_bottom_heights,
    })
}

/// Port of `WallGeometryLibrary.computePositions`.
pub fn compute_positions(
    ellipsoid: &Ellipsoid,
    wall_positions: &[Cartesian3],
    maximum_heights: Option<&[f64]>,
    minimum_heights: Option<&[f64]>,
    granularity: f64,
    duplicate_corners: bool,
) -> Option<WallComputedPositions> {
    let o = remove_duplicates(ellipsoid, wall_positions, maximum_heights, minimum_heights)?;

    let wall_positions = &o.positions;
    let maximum_heights = &o.top_heights;
    let minimum_heights = &o.bottom_heights;

    let length = wall_positions.len();
    let num_corners = length - 2;

    let min_distance = CesiumMath::chord_length(granularity, ellipsoid.maximum_radius());

    let mut generate_arc_options = GenerateArcOptions::default();
    generate_arc_options.min_distance = Some(min_distance);
    generate_arc_options.ellipsoid = Some(ellipsoid.clone());

    let (top_positions, bottom_positions) = if duplicate_corners {
        let mut count = 0usize;
        for i in 0..length - 1 {
            count += PolylinePipeline::number_of_points(
                &wall_positions[i],
                &wall_positions[i + 1],
                min_distance,
            ) + 1;
        }

        let mut top_positions = vec![0.0f64; count * 3];
        let mut bottom_positions = vec![0.0f64; count * 3];

        let mut offset = 0usize;
        for i in 0..length - 1 {
            generate_arc_options.positions =
                vec![wall_positions[i], wall_positions[i + 1]];

            generate_arc_options.height = Some(GenerateArcHeight::Array(vec![
                maximum_heights[i],
                maximum_heights[i + 1],
            ]));
            let pos = PolylinePipeline::generate_arc(Some(&generate_arc_options));
            top_positions[offset..offset + pos.len()].copy_from_slice(&pos);

            generate_arc_options.height = Some(GenerateArcHeight::Array(vec![
                minimum_heights[i],
                minimum_heights[i + 1],
            ]));
            let bottom_pos = PolylinePipeline::generate_arc(Some(&generate_arc_options));
            bottom_positions[offset..offset + bottom_pos.len()].copy_from_slice(&bottom_pos);

            offset += pos.len();
        }

        (top_positions, bottom_positions)
    } else {
        generate_arc_options.positions = wall_positions.clone();
        generate_arc_options.height = Some(GenerateArcHeight::Array(maximum_heights.clone()));
        let top_positions = PolylinePipeline::generate_arc(Some(&generate_arc_options));

        generate_arc_options.height = Some(GenerateArcHeight::Array(minimum_heights.clone()));
        let bottom_positions = PolylinePipeline::generate_arc(Some(&generate_arc_options));

        (top_positions, bottom_positions)
    };

    Some(WallComputedPositions {
        bottom_positions,
        top_positions,
        num_corners,
    })
}
