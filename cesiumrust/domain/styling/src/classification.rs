//! Classification system for 3D Tiles and terrain.
//!
//! Maps to CesiumJS classification primitives:
//! - `Scene/ClassificationPrimitive.js`
//! - `Scene/ClassificationType.js`
//! - Feature ID-based classification

/// Classification type determines what geometry is affected.
///
/// Maps to CesiumJS `Scene/ClassificationType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassificationType {
    /// Classify both terrain and 3D Tiles.
    #[default]
    Both,
    /// Classify only terrain.
    Terrain,
    /// Classify only 3D Tiles.
    Cesium3DTile,
}

/// A classification definition for features.
#[derive(Debug, Clone)]
pub struct Classification {
    /// Unique identifier.
    pub id: String,
    /// Classification type.
    pub classification_type: ClassificationType,
    /// Whether the classification is shown.
    pub show: bool,
    /// Color to apply [r, g, b, a].
    pub color: [f64; 4],
    /// Feature IDs to classify (empty = all).
    pub feature_ids: Vec<u64>,
    /// Batch IDs to classify (for b3dm).
    pub batch_ids: Vec<u32>,
}

impl Classification {
    /// Creates a new classification.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            classification_type: ClassificationType::Both,
            show: true,
            color: [1.0, 1.0, 0.0, 0.5],
            feature_ids: Vec::new(),
            batch_ids: Vec::new(),
        }
    }

    /// Sets the classification type.
    pub fn with_type(mut self, classification_type: ClassificationType) -> Self {
        self.classification_type = classification_type;
        self
    }

    /// Sets the color.
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// Adds feature IDs to classify.
    pub fn with_feature_ids(mut self, ids: Vec<u64>) -> Self {
        self.feature_ids = ids;
        self
    }

    /// Adds batch IDs to classify.
    pub fn with_batch_ids(mut self, ids: Vec<u32>) -> Self {
        self.batch_ids = ids;
        self
    }

    /// Checks if a feature ID is classified.
    pub fn contains_feature(&self, feature_id: u64) -> bool {
        self.feature_ids.is_empty() || self.feature_ids.contains(&feature_id)
    }

    /// Checks if a batch ID is classified.
    pub fn contains_batch(&self, batch_id: u32) -> bool {
        self.batch_ids.is_empty() || self.batch_ids.contains(&batch_id)
    }
}

/// A collection of classifications.
#[derive(Debug, Default)]
pub struct ClassificationCollection {
    /// Classifications by ID.
    classifications: Vec<Classification>,
}

impl ClassificationCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a classification.
    pub fn add(&mut self, classification: Classification) {
        self.classifications.push(classification);
    }

    /// Removes a classification by ID.
    pub fn remove(&mut self, id: &str) -> Option<Classification> {
        if let Some(pos) = self.classifications.iter().position(|c| c.id == id) {
            Some(self.classifications.remove(pos))
        } else {
            None
        }
    }

    /// Gets a classification by ID.
    pub fn get(&self, id: &str) -> Option<&Classification> {
        self.classifications.iter().find(|c| c.id == id)
    }

    /// Returns the number of classifications.
    pub fn len(&self) -> usize {
        self.classifications.len()
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.classifications.is_empty()
    }

    /// Gets all classifications that affect a feature.
    pub fn get_for_feature(&self, feature_id: u64) -> Vec<&Classification> {
        self.classifications
            .iter()
            .filter(|c| c.show && c.contains_feature(feature_id))
            .collect()
    }

    /// Gets all classifications that affect a batch.
    pub fn get_for_batch(&self, batch_id: u32) -> Vec<&Classification> {
        self.classifications
            .iter()
            .filter(|c| c.show && c.contains_batch(batch_id))
            .collect()
    }

    /// Computes the blended color for a feature.
    pub fn compute_feature_color(&self, feature_id: u64, base_color: [f64; 4]) -> [f64; 4] {
        let classifications = self.get_for_feature(feature_id);
        Self::blend_colors(base_color, &classifications)
    }

    /// Blends multiple classification colors with a base color.
    fn blend_colors(base_color: [f64; 4], classifications: &[&Classification]) -> [f64; 4] {
        let mut result = base_color;

        for classification in classifications {
            let c = classification.color;
            let alpha = c[3];

            // Alpha blending: result = base * (1 - alpha) + overlay * alpha
            result[0] = result[0] * (1.0 - alpha) + c[0] * alpha;
            result[1] = result[1] * (1.0 - alpha) + c[1] * alpha;
            result[2] = result[2] * (1.0 - alpha) + c[2] * alpha;
            result[3] = result[3].max(alpha);
        }

        result
    }
}

/// Feature metadata for classification.
#[derive(Debug, Clone, Default)]
pub struct FeatureMetadata {
    /// Feature ID.
    pub feature_id: u64,
    /// Batch ID (for b3dm).
    pub batch_id: Option<u32>,
    /// Property table index.
    pub property_table: Option<u32>,
    /// Custom properties.
    pub properties: Vec<(String, MetadataValue)>,
}

/// Metadata value types.
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// Array of floats.
    FloatArray(Vec<f64>),
}

impl FeatureMetadata {
    /// Creates new feature metadata.
    pub fn new(feature_id: u64) -> Self {
        Self {
            feature_id,
            ..Default::default()
        }
    }

    /// Gets a property value by name.
    pub fn get_property(&self, name: &str) -> Option<&MetadataValue> {
        self.properties
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }

    /// Sets a property value.
    pub fn set_property(&mut self, name: impl Into<String>, value: MetadataValue) {
        let name = name.into();
        if let Some(prop) = self.properties.iter_mut().find(|(n, _)| *n == name) {
            prop.1 = value;
        } else {
            self.properties.push((name, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_creation() {
        let classification = Classification::new("test")
            .with_type(ClassificationType::Terrain)
            .with_color([1.0, 0.0, 0.0, 0.5]);

        assert_eq!(classification.id, "test");
        assert_eq!(classification.classification_type, ClassificationType::Terrain);
        assert_eq!(classification.color, [1.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn test_classification_contains() {
        let classification = Classification::new("test")
            .with_feature_ids(vec![1, 2, 3]);

        assert!(classification.contains_feature(1));
        assert!(classification.contains_feature(2));
        assert!(!classification.contains_feature(4));
    }

    #[test]
    fn test_classification_empty_ids() {
        let classification = Classification::new("test");

        // Empty feature_ids means all features
        assert!(classification.contains_feature(999));
    }

    #[test]
    fn test_classification_collection() {
        let mut collection = ClassificationCollection::new();

        collection.add(Classification::new("c1").with_feature_ids(vec![1, 2]));
        collection.add(Classification::new("c2").with_feature_ids(vec![2, 3]));

        assert_eq!(collection.len(), 2);

        let for_feature_2 = collection.get_for_feature(2);
        assert_eq!(for_feature_2.len(), 2);

        let for_feature_1 = collection.get_for_feature(1);
        assert_eq!(for_feature_1.len(), 1);
    }

    #[test]
    fn test_classification_removal() {
        let mut collection = ClassificationCollection::new();
        collection.add(Classification::new("c1"));
        collection.add(Classification::new("c2"));

        let removed = collection.remove("c1");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_color_blending() {
        let mut collection = ClassificationCollection::new();
        collection.add(
            Classification::new("c1")
                .with_color([1.0, 0.0, 0.0, 0.5])
                .with_feature_ids(vec![1]),
        );

        let base = [0.0, 0.0, 1.0, 1.0]; // Blue
        let result = collection.compute_feature_color(1, base);

        // Red overlay at 50% alpha on blue base
        // result = blue * 0.5 + red * 0.5 = [0.5, 0.0, 0.5, 1.0]
        assert!((result[0] - 0.5).abs() < 0.01);
        assert!((result[1] - 0.0).abs() < 0.01);
        assert!((result[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_feature_metadata() {
        let mut metadata = FeatureMetadata::new(42);
        metadata.set_property("height", MetadataValue::Float(100.0));
        metadata.set_property("type", MetadataValue::String("building".to_string()));

        assert_eq!(metadata.feature_id, 42);
        assert_eq!(
            metadata.get_property("height"),
            Some(&MetadataValue::Float(100.0))
        );
        assert_eq!(
            metadata.get_property("type"),
            Some(&MetadataValue::String("building".to_string()))
        );
        assert_eq!(metadata.get_property("missing"), None);
    }

    #[test]
    fn test_classification_type_default() {
        assert_eq!(ClassificationType::default(), ClassificationType::Both);
    }

    #[test]
    fn test_batch_classification() {
        let classification = Classification::new("test")
            .with_batch_ids(vec![0, 5, 10]);

        assert!(classification.contains_batch(0));
        assert!(classification.contains_batch(5));
        assert!(!classification.contains_batch(3));
    }
}
