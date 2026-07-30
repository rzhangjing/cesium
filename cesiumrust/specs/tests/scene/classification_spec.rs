//! Classification specs - ported from ClassificationPrimitiveSpec.js
//! and ClassificationTypeSpec.js
//!
//! Tests Classification/ClassificationCollection/FeatureMetadata:
//! creation, builder pattern, feature/batch filtering, color blending,
//! collection management, metadata properties.

use cesium_styling::{
    Classification, ClassificationCollection, ClassificationType, FeatureMetadata, MetadataValue,
};

// ─── Classification Creation ───────────────────────────────────────────────

#[test]
fn classification_default_values() {
    let c = Classification::new("test");
    assert_eq!(c.id, "test");
    assert_eq!(c.classification_type, ClassificationType::Both);
    assert!(c.show);
    assert_eq!(c.color, [1.0, 1.0, 0.0, 0.5]);
    assert!(c.feature_ids.is_empty());
    assert!(c.batch_ids.is_empty());
}

#[test]
fn classification_builder_type() {
    let c = Classification::new("c1").with_type(ClassificationType::Terrain);
    assert_eq!(c.classification_type, ClassificationType::Terrain);
}

#[test]
fn classification_builder_color() {
    let c = Classification::new("c1").with_color([0.0, 0.0, 1.0, 0.8]);
    assert_eq!(c.color, [0.0, 0.0, 1.0, 0.8]);
}

#[test]
fn classification_builder_feature_ids() {
    let c = Classification::new("c1").with_feature_ids(vec![10, 20, 30]);
    assert_eq!(c.feature_ids, vec![10, 20, 30]);
}

#[test]
fn classification_builder_batch_ids() {
    let c = Classification::new("c1").with_batch_ids(vec![0, 5, 10]);
    assert_eq!(c.batch_ids, vec![0, 5, 10]);
}

// ─── Contains Feature / Batch ──────────────────────────────────────────────

#[test]
fn contains_feature_empty_means_all() {
    let c = Classification::new("c1");
    // Empty feature_ids → classifies ALL features
    assert!(c.contains_feature(0));
    assert!(c.contains_feature(999999));
}

#[test]
fn contains_feature_specific_ids() {
    let c = Classification::new("c1").with_feature_ids(vec![1, 2, 3]);
    assert!(c.contains_feature(1));
    assert!(c.contains_feature(2));
    assert!(c.contains_feature(3));
    assert!(!c.contains_feature(4));
    assert!(!c.contains_feature(0));
}

#[test]
fn contains_batch_empty_means_all() {
    let c = Classification::new("c1");
    assert!(c.contains_batch(0));
    assert!(c.contains_batch(100));
}

#[test]
fn contains_batch_specific_ids() {
    let c = Classification::new("c1").with_batch_ids(vec![5, 10, 15]);
    assert!(c.contains_batch(5));
    assert!(c.contains_batch(10));
    assert!(!c.contains_batch(6));
}

// ─── ClassificationType ────────────────────────────────────────────────────

#[test]
fn classification_type_default_is_both() {
    assert_eq!(ClassificationType::default(), ClassificationType::Both);
}

#[test]
fn classification_type_variants() {
    let terrain = ClassificationType::Terrain;
    let tile = ClassificationType::Cesium3DTile;
    let both = ClassificationType::Both;
    assert_ne!(terrain, tile);
    assert_ne!(terrain, both);
    assert_ne!(tile, both);
}

// ─── ClassificationCollection ──────────────────────────────────────────────

#[test]
fn collection_new_is_empty() {
    let collection = ClassificationCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

#[test]
fn collection_add_and_len() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("c1"));
    collection.add(Classification::new("c2"));
    collection.add(Classification::new("c3"));
    assert_eq!(collection.len(), 3);
    assert!(!collection.is_empty());
}

#[test]
fn collection_get_by_id() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("alpha").with_color([1.0, 0.0, 0.0, 1.0]));
    collection.add(Classification::new("beta").with_color([0.0, 1.0, 0.0, 1.0]));

    let found = collection.get("alpha").unwrap();
    assert_eq!(found.color, [1.0, 0.0, 0.0, 1.0]);

    assert!(collection.get("gamma").is_none());
}

#[test]
fn collection_remove() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("c1"));
    collection.add(Classification::new("c2"));

    let removed = collection.remove("c1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "c1");
    assert_eq!(collection.len(), 1);
    assert!(collection.get("c1").is_none());
}

#[test]
fn collection_remove_nonexistent() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("c1"));
    let removed = collection.remove("nonexistent");
    assert!(removed.is_none());
    assert_eq!(collection.len(), 1);
}

// ─── Get For Feature / Batch ───────────────────────────────────────────────

#[test]
fn get_for_feature_filters_correctly() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("c1").with_feature_ids(vec![1, 2, 3]));
    collection.add(Classification::new("c2").with_feature_ids(vec![2, 3, 4]));
    collection.add(Classification::new("c3").with_feature_ids(vec![5, 6]));

    let for_2 = collection.get_for_feature(2);
    assert_eq!(for_2.len(), 2); // c1 and c2

    let for_5 = collection.get_for_feature(5);
    assert_eq!(for_5.len(), 1); // only c3

    let for_99 = collection.get_for_feature(99);
    assert_eq!(for_99.len(), 0);
}

#[test]
fn get_for_feature_respects_show_flag() {
    let mut collection = ClassificationCollection::new();
    let mut hidden = Classification::new("hidden").with_feature_ids(vec![1, 2]);
    hidden.show = false;
    collection.add(hidden);
    collection.add(Classification::new("visible").with_feature_ids(vec![1, 2]));

    let for_1 = collection.get_for_feature(1);
    assert_eq!(for_1.len(), 1); // only visible
    assert_eq!(for_1[0].id, "visible");
}

#[test]
fn get_for_batch_filters_correctly() {
    let mut collection = ClassificationCollection::new();
    collection.add(Classification::new("c1").with_batch_ids(vec![0, 1]));
    collection.add(Classification::new("c2").with_batch_ids(vec![1, 2]));

    let for_1 = collection.get_for_batch(1);
    assert_eq!(for_1.len(), 2);

    let for_0 = collection.get_for_batch(0);
    assert_eq!(for_0.len(), 1);
}

// ─── Color Blending ────────────────────────────────────────────────────────

#[test]
fn compute_feature_color_no_classifications() {
    let collection = ClassificationCollection::new();
    let base = [0.5, 0.5, 0.5, 1.0];
    let result = collection.compute_feature_color(1, base);
    // No classifications → base color unchanged
    assert_eq!(result, base);
}

#[test]
fn compute_feature_color_single_overlay() {
    let mut collection = ClassificationCollection::new();
    collection.add(
        Classification::new("c1")
            .with_color([1.0, 0.0, 0.0, 0.5])
            .with_feature_ids(vec![1]),
    );

    let base = [0.0, 0.0, 1.0, 1.0]; // blue
    let result = collection.compute_feature_color(1, base);

    // Alpha blend: base * (1-0.5) + overlay * 0.5
    assert!((result[0] - 0.5).abs() < 0.01); // 0*0.5 + 1*0.5
    assert!((result[1] - 0.0).abs() < 0.01);
    assert!((result[2] - 0.5).abs() < 0.01); // 1*0.5 + 0*0.5
}

#[test]
fn compute_feature_color_full_alpha_replaces() {
    let mut collection = ClassificationCollection::new();
    collection.add(
        Classification::new("c1")
            .with_color([0.0, 1.0, 0.0, 1.0]) // full alpha
            .with_feature_ids(vec![1]),
    );

    let base = [1.0, 0.0, 0.0, 1.0];
    let result = collection.compute_feature_color(1, base);

    // Full alpha overlay completely replaces base
    assert!((result[0] - 0.0).abs() < 0.01);
    assert!((result[1] - 1.0).abs() < 0.01);
    assert!((result[2] - 0.0).abs() < 0.01);
}

// ─── FeatureMetadata ───────────────────────────────────────────────────────

#[test]
fn feature_metadata_new() {
    let meta = FeatureMetadata::new(42);
    assert_eq!(meta.feature_id, 42);
    assert!(meta.batch_id.is_none());
    assert!(meta.property_table.is_none());
    assert!(meta.properties.is_empty());
}

#[test]
fn feature_metadata_set_get_property() {
    let mut meta = FeatureMetadata::new(1);
    meta.set_property("height", MetadataValue::Float(100.0));
    meta.set_property("name", MetadataValue::String("building".to_string()));
    meta.set_property("visible", MetadataValue::Bool(true));

    assert_eq!(meta.get_property("height"), Some(&MetadataValue::Float(100.0)));
    assert_eq!(
        meta.get_property("name"),
        Some(&MetadataValue::String("building".to_string()))
    );
    assert_eq!(meta.get_property("visible"), Some(&MetadataValue::Bool(true)));
    assert_eq!(meta.get_property("missing"), None);
}

#[test]
fn feature_metadata_overwrite_property() {
    let mut meta = FeatureMetadata::new(1);
    meta.set_property("height", MetadataValue::Float(50.0));
    meta.set_property("height", MetadataValue::Float(200.0));

    assert_eq!(meta.get_property("height"), Some(&MetadataValue::Float(200.0)));
    // Should not duplicate
    assert_eq!(meta.properties.len(), 1);
}

#[test]
fn feature_metadata_batch_id() {
    let mut meta = FeatureMetadata::new(10);
    meta.batch_id = Some(5);
    meta.property_table = Some(0);
    assert_eq!(meta.batch_id, Some(5));
    assert_eq!(meta.property_table, Some(0));
}
