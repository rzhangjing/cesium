//! Ported from `packages/engine/Source/Core/WallOutlineGeometry.js`.
//!
//! A description of a wall outline.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;
use crate::wall_geometry_library;

/// A description of a wall outline.
#[derive(Debug, Clone)]
pub struct WallOutlineGeometry {
    positions: Vec<Cartesian3>,
    maximum_heights: Option<Vec<f64>>,
    minimum_heights: Option<Vec<f64>>,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl WallOutlineGeometry {
    /// Creates a new `WallOutlineGeometry`.
    ///
    /// # Panics (debug)
    /// Mirrors the JS `DeveloperError` checks behind `debug_assertions`.
    pub fn new(
        positions: Vec<Cartesian3>,
        maximum_heights: Option<Vec<f64>>,
        minimum_heights: Option<Vec<f64>>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        if cfg!(debug_assertions) {
            if positions.len() < 2 {
                crate::developer_error::throw_developer_error(
                    "options.positions length must be greater than or equal to 2.",
                );
            }
            if let Some(maximum_heights) = &maximum_heights {
                if maximum_heights.len() != positions.len() {
                    crate::developer_error::throw_developer_error(
                        "options.positions and options.maximumHeights must have the same length.",
                    );
                }
            }
            if let Some(minimum_heights) = &minimum_heights {
                if minimum_heights.len() != positions.len() {
                    crate::developer_error::throw_developer_error(
                        "options.positions and options.minimumHeights must have the same length.",
                    );
                }
            }
        }

        Self {
            positions,
            maximum_heights,
            minimum_heights,
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// Creates a wall outline from constant min/max heights (JS
    /// `WallOutlineGeometry.fromConstantHeights`).
    pub fn from_constant_heights(
        positions: Vec<Cartesian3>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let len = positions.len();
        let min_heights = minimum_height.map(|h| vec![h; len]);
        let max_heights = maximum_height.map(|h| vec![h; len]);
        Self::new(positions, max_heights, min_heights, None, ellipsoid)
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    /// The minimum heights (JS `_minimumHeights`).
    pub fn minimum_heights(&self) -> Option<&Vec<f64>> {
        self.minimum_heights.as_ref()
    }

    /// The maximum heights (JS `_maximumHeights`).
    pub fn maximum_heights(&self) -> Option<&Vec<f64>> {
        self.maximum_heights.as_ref()
    }

    /// The granularity (JS `_granularity`).
    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    /// The ellipsoid (JS `_ellipsoid`).
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS exposes `packedLength` as an instance property computed
    /// in the constructor; Rust computes it on demand (always equal).
    pub fn packed_length(&self) -> usize {
        let mut num_components = 1 + self.positions.len() * Cartesian3::PACKED_LENGTH + 2;
        if let Some(minimum_heights) = &self.minimum_heights {
            num_components += minimum_heights.len();
        }
        if let Some(maximum_heights) = &self.maximum_heights {
            num_components += maximum_heights.len();
        }
        num_components + Ellipsoid::PACKED_LENGTH + 1
    }

    /// Stores this instance into `array` (JS `WallOutlineGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        let length = self.positions.len();
        array[si] = length as f64;
        si += 1;

        for position in &self.positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        let min_length = self.minimum_heights.as_ref().map_or(0, |h| h.len());
        array[si] = min_length as f64;
        si += 1;
        if let Some(minimum_heights) = &self.minimum_heights {
            for height in minimum_heights {
                array[si] = *height;
                si += 1;
            }
        }

        let max_length = self.maximum_heights.as_ref().map_or(0, |h| h.len());
        array[si] = max_length as f64;
        si += 1;
        if let Some(maximum_heights) = &self.maximum_heights {
            for height in maximum_heights {
                array[si] = *height;
                si += 1;
            }
        }

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        array[si] = self.granularity;
    }

    /// Retrieves an instance from a packed array (JS
    /// `WallOutlineGeometry.unpack`).
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let mut length = array[si] as usize;
        si += 1;
        let mut positions = Vec::with_capacity(length);
        for _ in 0..length {
            positions.push(Cartesian3::unpack_new(array, Some(si)));
            si += Cartesian3::PACKED_LENGTH;
        }

        length = array[si] as usize;
        si += 1;
        let mut minimum_heights: Option<Vec<f64>> = None;
        if length > 0 {
            let mut heights = Vec::with_capacity(length);
            for _ in 0..length {
                heights.push(array[si]);
                si += 1;
            }
            minimum_heights = Some(heights);
        }

        length = array[si] as usize;
        si += 1;
        let mut maximum_heights: Option<Vec<f64>> = None;
        if length > 0 {
            let mut heights = Vec::with_capacity(length);
            for _ in 0..length {
                heights.push(array[si]);
                si += 1;
            }
            maximum_heights = Some(heights);
        }

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let granularity = array[si];

        match result {
            None => Self::new(
                positions,
                maximum_heights,
                minimum_heights,
                Some(granularity),
                Some(ellipsoid),
            ),
            Some(r) => {
                r.positions = positions;
                r.minimum_heights = minimum_heights;
                r.maximum_heights = maximum_heights;
                r.ellipsoid = ellipsoid;
                r.granularity = granularity;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of a wall outline, including its
    /// vertices, indices, and a bounding sphere (JS
    /// `WallOutlineGeometry.createGeometry`).
    pub fn create_geometry(&self) -> Option<Geometry> {
        let wall_positions = &self.positions;
        let ellipsoid = &self.ellipsoid;

        let pos = wall_geometry_library::compute_positions(
            ellipsoid,
            wall_positions,
            self.maximum_heights.as_deref(),
            self.minimum_heights.as_deref(),
            self.granularity,
            false,
        )?;

        let bottom_positions = &pos.bottom_positions;
        let top_positions = &pos.top_positions;

        let mut length = top_positions.len();
        let size = length * 2;

        let mut positions = vec![0.0f64; size];
        let mut position_index = 0usize;

        // Add lower and upper points one after the other, lower
        // points being even and upper points being odd.
        length /= 3;
        for i in 0..length {
            let i3 = i * 3;
            let top_position = Cartesian3::from_array_new(top_positions, Some(i3));
            let bottom_position = Cartesian3::from_array_new(bottom_positions, Some(i3));

            // insert the lower point
            positions[position_index] = bottom_position.x;
            position_index += 1;
            positions[position_index] = bottom_position.y;
            position_index += 1;
            positions[position_index] = bottom_position.z;
            position_index += 1;

            // insert the upper point
            positions[position_index] = top_position.x;
            position_index += 1;
            positions[position_index] = top_position.y;
            position_index += 1;
            positions[position_index] = top_position.z;
            position_index += 1;
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
        );

        let num_vertices = size / 3;
        let index_count = 2 * num_vertices - 4 + num_vertices;
        let mut indices: IndexStorage =
            IndexDatatype::create_typed_array(num_vertices, index_count);

        let mut i = 0usize;
        while i + 2 < num_vertices {
            let ll = i;
            let lr = i + 2;
            let pl = Cartesian3::from_array_new(&positions, Some(ll * 3));
            let pr = Cartesian3::from_array_new(&positions, Some(lr * 3));
            if !Cartesian3::equals_epsilon(
                Some(&pl),
                Some(&pr),
                Some(CesiumMath::EPSILON10),
                Some(CesiumMath::EPSILON10),
            ) {
                let ul = i + 1;
                let ur = i + 3;

                indices.push(ul as u32);
                indices.push(ll as u32);
                indices.push(ul as u32);
                indices.push(ur as u32);
                indices.push(ll as u32);
                indices.push(lr as u32);
            }
            i += 2;
        }

        indices.push((num_vertices - 2) as u32);
        indices.push((num_vertices - 1) as u32);

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Lines),
            Some(BoundingSphere::from_vertices(&positions, None, None, None)),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}
