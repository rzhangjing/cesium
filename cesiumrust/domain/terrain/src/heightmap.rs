//! Heightmap terrain data.
//! Maps to CesiumJS `Core/HeightmapTerrainData.js`

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::terrain_mesh::TerrainMesh;

/// Describes the layout of height data in a raw buffer.
///
/// Maps to CesiumJS `HeightmapTessellator.DEFAULT_STRUCTURE` and the
/// `structure` option of `HeightmapTerrainData`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeightmapStructure {
    /// Number of elements to skip to get from one height to the next.
    pub stride: usize,
    /// Number of elements that make up a single height value.
    pub elements_per_height: usize,
    /// Multiplier between elements (default 256).
    pub element_multiplier: f64,
    /// Whether multi-element heights are big-endian.
    pub is_big_endian: bool,
    /// Scale applied after decoding.
    pub height_scale: f64,
    /// Offset added after scaling.
    pub height_offset: f64,
    /// Optional lowest clamped value (encoded units).
    pub lowest_encoded_height: Option<f64>,
    /// Optional highest clamped value (encoded units).
    pub highest_encoded_height: Option<f64>,
}

impl Default for HeightmapStructure {
    fn default() -> Self {
        Self {
            stride: 1,
            elements_per_height: 1,
            element_multiplier: 256.0,
            is_big_endian: false,
            height_scale: 1.0,
            height_offset: 0.0,
            lowest_encoded_height: None,
            highest_encoded_height: None,
        }
    }
}

/// Reads a height value from a raw buffer at the given vertex index.
///
/// Maps to CesiumJS `getHeight` in HeightmapTerrainData.js.
pub fn get_height_from_buffer(
    buffer: &[u8],
    structure: &HeightmapStructure,
    index: usize,
) -> f64 {
    let offset = index * structure.stride;
    let mut height = 0.0f64;

    if structure.is_big_endian {
        for i in 0..structure.elements_per_height {
            height = height * structure.element_multiplier + buffer[offset + i] as f64;
        }
    } else {
        for i in (0..structure.elements_per_height).rev() {
            height = height * structure.element_multiplier + buffer[offset + i] as f64;
        }
    }

    height
}

/// Writes a height value into a raw buffer at the given vertex index.
///
/// Maps to CesiumJS `setHeight` in HeightmapTerrainData.js.
pub fn set_height_in_buffer(
    buffer: &mut [u8],
    structure: &HeightmapStructure,
    index: usize,
    mut height: f64,
) {
    let offset = index * structure.stride;
    let divisor = structure
        .element_multiplier
        .powi(structure.elements_per_height as i32 - 1);
    let mut div = divisor;

    if structure.is_big_endian {
        for i in 0..structure.elements_per_height - 1 {
            let val = (height / div).floor() as u8;
            buffer[offset + i] = val;
            height -= val as f64 * div;
            div /= structure.element_multiplier;
        }
        // Last element gets remainder
        buffer[offset + structure.elements_per_height - 1] = height as u8;
    } else {
        for i in (1..structure.elements_per_height).rev() {
            let val = (height / div).floor() as u8;
            buffer[offset + i] = val;
            height -= val as f64 * div;
            div /= structure.element_multiplier;
        }
        // First element (index 0) gets remainder
        buffer[offset] = height as u8;
    }
}

/// Interpolates a height from a grid using the CesiumJS triangle method.
///
/// The grid is stored row-major with rows going from NORTH to SOUTH
/// (row 0 = north, row height-1 = south), matching CesiumJS mesh layout.
/// `u` is west-to-east [0,1], `v` is south-to-north [0,1].
///
/// Maps to CesiumJS `interpolateHeight` / `interpolateMeshHeight` +
/// `triangleInterpolateHeight`.
fn interpolate_height_from_grid(
    heights: &[f64],
    width: usize,
    height: usize,
    u: f64,
    v: f64,
) -> f64 {
    // Convert u,v to grid coordinates (fromWest, fromSouth)
    let from_west = u * (width - 1) as f64;
    let from_south = v * (height - 1) as f64;

    let mut west_int = from_west as usize;
    let mut east_int = west_int + 1;
    if east_int >= width {
        east_int = width - 1;
        west_int = width - 2;
    }

    let mut south_int = from_south as usize;
    let mut north_int = south_int + 1;
    if north_int >= height {
        north_int = height - 1;
        south_int = height - 2;
    }

    let dx = from_west - west_int as f64;
    let dy = from_south - south_int as f64;

    // Flip row indices: grid rows go north-to-south, but south_int/north_int
    // are in south-to-north space.
    let south_row = height - 1 - south_int;
    let north_row = height - 1 - north_int;

    let sw = heights[south_row * width + west_int];
    let se = heights[south_row * width + east_int];
    let nw = heights[north_row * width + west_int];
    let ne = heights[north_row * width + east_int];

    // Triangle interpolation (CesiumJS bisects quad from SW to NE)
    if dy < dx {
        // Lower-right triangle
        sw + dx * (se - sw) + dy * (ne - se)
    } else {
        // Upper-left triangle
        sw + dx * (ne - nw) + dy * (nw - sw)
    }
}

/// Terrain data represented as a heightmap.
///
/// A heightmap is a regular grid of height values covering a rectangular region.
///
/// Maps to CesiumJS `HeightmapTerrainData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightmapTerrainData {
    /// Height values in row-major order (south to north, west to east)
    pub heights: Vec<f64>,

    /// Number of rows (latitude samples)
    pub width: usize,

    /// Number of columns (longitude samples)
    pub height: usize,

    /// Minimum height in the tile
    pub minimum_height: f64,

    /// Maximum height in the tile
    pub maximum_height: f64,

    /// Bounding sphere for the tile
    pub bounding_sphere: BoundingSphere,

    /// Bit mask indicating which children exist
    #[serde(default = "default_child_mask")]
    pub child_tile_mask: u8,

    /// Whether this was created by upsampling
    #[serde(default)]
    pub created_by_upsampling: bool,
}

fn default_child_mask() -> u8 {
    15
}

impl HeightmapTerrainData {
    /// Creates a new heightmap terrain data.
    pub fn new(
        heights: Vec<f64>,
        width: usize,
        height: usize,
        minimum_height: f64,
        maximum_height: f64,
    ) -> Self {
        let bounding_sphere = BoundingSphere::new(DVec3::ZERO, 0.0);
        Self {
            heights,
            width,
            height,
            minimum_height,
            maximum_height,
            bounding_sphere,
            child_tile_mask: 15,
            created_by_upsampling: false,
        }
    }

    /// Gets the height at a specific grid position.
    pub fn get_height(&self, col: usize, row: usize) -> Option<f64> {
        if col < self.width && row < self.height {
            Some(self.heights[row * self.width + col])
        } else {
            None
        }
    }

    /// Interpolates height at a fractional grid position.
    pub fn interpolate_height(&self, u: f64, v: f64) -> f64 {
        let col_f = u * (self.width - 1) as f64;
        let row_f = v * (self.height - 1) as f64;

        let col0 = col_f.floor() as usize;
        let row0 = row_f.floor() as usize;
        let col1 = (col0 + 1).min(self.width - 1);
        let row1 = (row0 + 1).min(self.height - 1);

        let du = col_f - col0 as f64;
        let dv = row_f - row0 as f64;

        let h00 = self.heights[row0 * self.width + col0];
        let h10 = self.heights[row0 * self.width + col1];
        let h01 = self.heights[row1 * self.width + col0];
        let h11 = self.heights[row1 * self.width + col1];

        // Bilinear interpolation
        let h0 = math_utils::lerp(h00, h10, du);
        let h1 = math_utils::lerp(h01, h11, du);
        math_utils::lerp(h0, h1, dv)
    }

    /// Creates a terrain mesh from the heightmap.
    ///
    /// # Arguments
    /// * `rectangle` - The tile rectangle
    /// * `ellipsoid` - The ellipsoid
    pub fn create_mesh(&self, rectangle: &Rectangle, ellipsoid: &Ellipsoid) -> TerrainMesh {
        let mut positions = Vec::with_capacity(self.width * self.height);
        let mut uvs = Vec::with_capacity(self.width * self.height);
        let mut indices = Vec::new();

        // Generate vertices
        for row in 0..self.height {
            let v = row as f64 / (self.height - 1) as f64;
            let lat = math_utils::lerp(rectangle.south, rectangle.north, v);

            for col in 0..self.width {
                let u = col as f64 / (self.width - 1) as f64;
                let lon = math_utils::lerp(rectangle.west, rectangle.east, u);
                let height = self.heights[row * self.width + col];

                let carto = Cartographic::from_radians(lon, lat, height);
                let pos = ellipsoid.cartographic_to_cartesian(&carto);

                positions.push([pos.x, pos.y, pos.z]);
                uvs.push([u, v]);
            }
        }

        // Generate indices
        for row in 0..self.height - 1 {
            for col in 0..self.width - 1 {
                let i0 = (row * self.width + col) as u32;
                let i1 = i0 + 1;
                let i2 = i0 + self.width as u32;
                let i3 = i2 + 1;

                // Two triangles per quad
                indices.push(i0);
                indices.push(i2);
                indices.push(i1);

                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
            }
        }

        let mut mesh = TerrainMesh {
            positions,
            normals: None,
            tex_coords: Some(uvs),
            indices,
            minimum_height: self.minimum_height,
            maximum_height: self.maximum_height,
            bounding_sphere: self.bounding_sphere,
        };

        mesh.compute_normals();
        mesh
    }

    /// Checks if a specific child tile exists.
    pub fn is_child_available(&self, child: usize) -> bool {
        (self.child_tile_mask & (1 << child)) != 0
    }

    /// Upsamples this heightmap using a raw byte buffer with structure encoding.
    ///
    /// This is the faithful port of CesiumJS `HeightmapTerrainData.upsample` for
    /// multi-element/stride/big-endian buffers. It decodes heights from the raw
    /// buffer, interpolates, clamps, and re-encodes.
    ///
    /// # Arguments
    /// * `buffer` - Raw byte buffer containing encoded heights
    /// * `structure` - Height data layout description
    /// * `this_x/this_y/this_level` - This tile coordinates
    /// * `descendant_x/descendant_y/descendant_level` - Child tile coordinates
    pub fn upsample_with_structure(
        &self,
        buffer: &[u8],
        structure: &HeightmapStructure,
        this_x: u32,
        this_y: u32,
        this_level: u32,
        descendant_x: u32,
        descendant_y: u32,
        descendant_level: u32,
    ) -> Vec<u8> {
        let level_difference = descendant_level - this_level;
        assert!(level_difference == 1, "upsample can only cross one level");

        let width = self.width;
        let height = self.height;

        // Compute relative position of child within parent
        let relative_x = descendant_x - this_x * 2;
        let relative_y = descendant_y - this_y * 2;

        // Child covers [relative/2, (relative+1)/2] of parent
        let west_frac = relative_x as f64 / 2.0;
        let east_frac = (relative_x + 1) as f64 / 2.0;
        // CesiumJS tile Y increases southward; child row 0 = north
        let north_frac = relative_y as f64 / 2.0;
        let south_frac = (relative_y + 1) as f64 / 2.0;

        // Decode all source heights from buffer
        let mut source_heights = vec![0.0f64; width * height];
        for idx in 0..width * height {
            let h = get_height_from_buffer(buffer, structure, idx);
            source_heights[idx] = h * structure.height_scale + structure.height_offset;
        }

        // Output buffer
        let mut out_buffer = vec![0u8; width * height * structure.stride];

        for j in 0..height {
            // CesiumJS iterates rows from north to south
            let v = j as f64 / (height - 1) as f64;
            // j=0 → dest north → parent v = 1 - north_frac
            // j=height-1 → dest south → parent v = 1 - south_frac
            let parent_v = (1.0 - north_frac) + v * ((1.0 - south_frac) - (1.0 - north_frac));

            for i in 0..width {
                let u = i as f64 / (width - 1) as f64;
                let parent_u = west_frac + u * (east_frac - west_frac);

                // Interpolate using triangle method (faithful to CesiumJS)
                let h = interpolate_height_from_grid(
                    &source_heights,
                    width,
                    height,
                    parent_u,
                    parent_v,
                );

                // Clamp
                let mut h_clamped = h;
                if let Some(low) = structure.lowest_encoded_height {
                    if h_clamped < low {
                        h_clamped = low;
                    }
                }
                if let Some(high) = structure.highest_encoded_height {
                    if h_clamped > high {
                        h_clamped = high;
                    }
                }

                set_height_in_buffer(
                    &mut out_buffer,
                    structure,
                    j * width + i,
                    h_clamped,
                );
            }
        }

        out_buffer
    }

    /// Upsamples this heightmap to produce a child tile at the given position.
    ///
    /// Maps to CesiumJS `HeightmapTerrainData.upsample`.
    ///
    /// # Arguments
    /// * `this_x` - This tile's X coordinate
    /// * `this_y` - This tile's Y coordinate
    /// * `this_level` - This tile's level
    /// * `descendant_x` - Child tile's X coordinate
    /// * `descendant_y` - Child tile's Y coordinate
    /// * `descendant_level` - Child tile's level (must be this_level + 1)
    pub fn upsample(
        &self,
        this_x: u32,
        this_y: u32,
        this_level: u32,
        descendant_x: u32,
        descendant_y: u32,
        descendant_level: u32,
    ) -> HeightmapTerrainData {
        let level_difference = descendant_level - this_level;
        assert!(
            level_difference == 1,
            "upsample can only cross one level"
        );

        // Compute the child's position within the parent
        let tiles_at_this_level = 1u32 << level_difference;
        let relative_x = descendant_x - this_x * tiles_at_this_level;
        let relative_y = descendant_y - this_y * tiles_at_this_level;

        // The child covers [relative_x/tiles, (relative_x+1)/tiles] of the parent
        let west_fraction = relative_x as f64 / tiles_at_this_level as f64;
        let east_fraction = (relative_x + 1) as f64 / tiles_at_this_level as f64;
        let south_fraction = relative_y as f64 / tiles_at_this_level as f64;
        let north_fraction = (relative_y + 1) as f64 / tiles_at_this_level as f64;

        // Child has same dimensions as parent
        let child_width = self.width;
        let child_height = self.height;
        let mut child_heights = vec![0.0f64; child_width * child_height];

        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;

        for row in 0..child_height {
            let v = row as f64 / (child_height - 1) as f64;
            let parent_v = south_fraction + v * (north_fraction - south_fraction);

            for col in 0..child_width {
                let u = col as f64 / (child_width - 1) as f64;
                let parent_u = west_fraction + u * (east_fraction - west_fraction);

                let h = self.interpolate_height(parent_u, parent_v);
                child_heights[row * child_width + col] = h;
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }

        let mut child = HeightmapTerrainData::new(
            child_heights,
            child_width,
            child_height,
            min_h,
            max_h,
        );
        child.created_by_upsampling = true;
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_heightmap() -> HeightmapTerrainData {
        // 3x3 heightmap
        let heights = vec![
            0.0, 100.0, 0.0,
            100.0, 200.0, 100.0,
            0.0, 100.0, 0.0,
        ];
        HeightmapTerrainData::new(heights, 3, 3, 0.0, 200.0)
    }

    #[test]
    fn test_get_height() {
        let data = create_test_heightmap();
        assert_eq!(data.get_height(0, 0), Some(0.0));
        assert_eq!(data.get_height(1, 1), Some(200.0));
        assert_eq!(data.get_height(2, 2), Some(0.0));
        assert_eq!(data.get_height(3, 0), None);
    }

    #[test]
    fn test_interpolate_height() {
        let data = create_test_heightmap();
        // Center should be 200
        assert!((data.interpolate_height(0.5, 0.5) - 200.0).abs() < 0.01);
        // Corner should be 0
        assert!((data.interpolate_height(0.0, 0.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_create_mesh() {
        let data = create_test_heightmap();
        let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
        let ellipsoid = Ellipsoid::WGS84;

        let mesh = data.create_mesh(&rectangle, &ellipsoid);

        assert_eq!(mesh.positions.len(), 9); // 3x3
        assert_eq!(mesh.indices.len(), 24); // 4 quads * 2 triangles * 3 indices
        assert!(mesh.normals.is_some());
    }

    #[test]
    fn test_child_availability() {
        let data = create_test_heightmap();
        assert!(data.is_child_available(0));
        assert!(data.is_child_available(3));
    }
}
