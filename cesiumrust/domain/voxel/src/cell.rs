//! Voxel cell metadata access.
//!
//! Maps to CesiumJS `Scene/VoxelCell.js`.

use std::collections::HashMap;

use crate::shape::OrientedBoundingBox;

/// Metadata value types for voxel cells.
#[derive(Debug, Clone, PartialEq)]
pub enum VoxelMetadataValue {
    /// Single f32 value.
    Float(f32),
    /// Single f64 value.
    Double(f64),
    /// Single i32 value.
    Int(i32),
    /// Single u32 value.
    Uint(u32),
    /// Vector of f32 values.
    VecF32(Vec<f32>),
    /// Vector of f64 values.
    VecF64(Vec<f64>),
    /// String value.
    String(String),
}

/// A cell from a voxel primitive, providing access to metadata and spatial info.
///
/// Maps to CesiumJS `VoxelCell`.
#[derive(Debug, Clone)]
pub struct VoxelCell {
    /// Index of the tile containing this cell.
    tile_index: u32,
    /// Index of the sample within the tile.
    sample_index: u32,
    /// Metadata property map (name -> value).
    metadata: HashMap<String, VoxelMetadataValue>,
    /// Oriented bounding box of the cell.
    oriented_bounding_box: OrientedBoundingBox,
}

impl VoxelCell {
    /// Create a new voxel cell.
    pub fn new(tile_index: u32, sample_index: u32) -> Self {
        Self {
            tile_index,
            sample_index,
            metadata: HashMap::new(),
            oriented_bounding_box: OrientedBoundingBox::default(),
        }
    }

    /// Create a cell with metadata and bounding box.
    pub fn with_data(
        tile_index: u32,
        sample_index: u32,
        metadata: HashMap<String, VoxelMetadataValue>,
        obb: OrientedBoundingBox,
    ) -> Self {
        Self {
            tile_index,
            sample_index,
            metadata,
            oriented_bounding_box: obb,
        }
    }

    /// Get the tile index.
    pub fn tile_index(&self) -> u32 {
        self.tile_index
    }

    /// Get the sample index within the tile.
    pub fn sample_index(&self) -> u32 {
        self.sample_index
    }

    /// Get the oriented bounding box.
    pub fn oriented_bounding_box(&self) -> &OrientedBoundingBox {
        &self.oriented_bounding_box
    }

    /// Check if the cell has a property with the given name.
    pub fn has_property(&self, name: &str) -> bool {
        self.metadata.contains_key(name)
    }

    /// Get all property names.
    pub fn get_names(&self) -> Vec<&str> {
        self.metadata.keys().map(|s| s.as_str()).collect()
    }

    /// Get a property value by name.
    pub fn get_property(&self, name: &str) -> Option<&VoxelMetadataValue> {
        self.metadata.get(name)
    }

    /// Get a float property value.
    pub fn get_float(&self, name: &str) -> Option<f64> {
        match self.metadata.get(name) {
            Some(VoxelMetadataValue::Float(v)) => Some(*v as f64),
            Some(VoxelMetadataValue::Double(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get an integer property value.
    pub fn get_int(&self, name: &str) -> Option<i64> {
        match self.metadata.get(name) {
            Some(VoxelMetadataValue::Int(v)) => Some(*v as i64),
            Some(VoxelMetadataValue::Uint(v)) => Some(*v as i64),
            _ => None,
        }
    }

    /// Set a property value.
    pub fn set_property(&mut self, name: String, value: VoxelMetadataValue) {
        self.metadata.insert(name, value);
    }

    /// Get the number of metadata properties.
    pub fn property_count(&self) -> usize {
        self.metadata.len()
    }

    /// Convert a sample index to 3D tile coordinates given padded dimensions.
    ///
    /// Returns (x, y, z) indices within the padded tile.
    pub fn sample_index_to_tile_coordinate(
        sample_index: u32,
        padded_dim_x: u32,
        padded_dim_y: u32,
    ) -> (u32, u32, u32) {
        let slice_size = padded_dim_x * padded_dim_y;
        let z = sample_index / slice_size;
        let index_in_slice = sample_index - z * slice_size;
        let y = index_in_slice / padded_dim_x;
        let x = index_in_slice - y * padded_dim_x;
        (x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_cell_new() {
        let cell = VoxelCell::new(5, 42);
        assert_eq!(cell.tile_index(), 5);
        assert_eq!(cell.sample_index(), 42);
        assert_eq!(cell.property_count(), 0);
    }

    #[test]
    fn test_voxel_cell_metadata() {
        let mut cell = VoxelCell::new(0, 0);
        cell.set_property("temperature".to_string(), VoxelMetadataValue::Float(25.5));
        cell.set_property("density".to_string(), VoxelMetadataValue::Double(1.225));
        cell.set_property("class_id".to_string(), VoxelMetadataValue::Int(3));

        assert!(cell.has_property("temperature"));
        assert!(cell.has_property("density"));
        assert!(!cell.has_property("pressure"));

        assert_eq!(cell.property_count(), 3);
        assert!((cell.get_float("temperature").unwrap() - 25.5).abs() < 1e-5);
        assert!((cell.get_float("density").unwrap() - 1.225).abs() < 1e-10);
        assert_eq!(cell.get_int("class_id"), Some(3));
    }

    #[test]
    fn test_voxel_cell_get_names() {
        let mut cell = VoxelCell::new(0, 0);
        cell.set_property("a".to_string(), VoxelMetadataValue::Int(1));
        cell.set_property("b".to_string(), VoxelMetadataValue::Int(2));

        let names = cell.get_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_voxel_cell_with_data() {
        let mut metadata = HashMap::new();
        metadata.insert("value".to_string(), VoxelMetadataValue::Float(42.0));

        let obb = OrientedBoundingBox::default();
        let cell = VoxelCell::with_data(1, 10, metadata, obb);

        assert_eq!(cell.tile_index(), 1);
        assert_eq!(cell.sample_index(), 10);
        assert!(cell.has_property("value"));
    }

    #[test]
    fn test_sample_index_to_tile_coordinate() {
        // 4x4x4 padded dimensions
        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(0, 4, 4);
        assert_eq!((x, y, z), (0, 0, 0));

        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(5, 4, 4);
        assert_eq!((x, y, z), (1, 1, 0));

        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(63, 4, 4);
        assert_eq!((x, y, z), (3, 3, 3));
    }

    #[test]
    fn test_voxel_metadata_value_types() {
        let f = VoxelMetadataValue::Float(1.5);
        let d = VoxelMetadataValue::Double(2.5);
        let i = VoxelMetadataValue::Int(-3);
        let u = VoxelMetadataValue::Uint(4);
        let s = VoxelMetadataValue::String("hello".to_string());
        let v = VoxelMetadataValue::VecF32(vec![1.0, 2.0, 3.0]);

        assert_eq!(f, VoxelMetadataValue::Float(1.5));
        assert_eq!(d, VoxelMetadataValue::Double(2.5));
        assert_eq!(i, VoxelMetadataValue::Int(-3));
        assert_eq!(u, VoxelMetadataValue::Uint(4));
        assert_eq!(s, VoxelMetadataValue::String("hello".to_string()));
        assert_eq!(v, VoxelMetadataValue::VecF32(vec![1.0, 2.0, 3.0]));
    }
}
