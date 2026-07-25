//! Clipping & Cloud specs - ported from Scene/ClippingPlaneSpec, CloudCollectionSpec
//! Covers: ClippingPlane, ClippingPlaneCollection, CloudCollection, CumulusCloud, CloudType

use cesium_effects::{ClippingPlane, ClippingPlaneCollection, CloudCollection, CloudType};
use glam::DVec3;

// ─── ClippingPlane ──────────────────────────────────────────────────────────

#[test]
fn clipping_plane_creation() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 0.0, 1.0), 5.0);
    assert_eq!(plane.normal, DVec3::new(0.0, 0.0, 1.0));
    assert_eq!(plane.distance, 5.0);
}

#[test]
fn clipping_plane_negative_distance() {
    let plane = ClippingPlane::new(DVec3::X, -10.0);
    assert_eq!(plane.distance, -10.0);
}

// ─── ClippingPlaneCollection ────────────────────────────────────────────────

#[test]
fn clipping_plane_collection_default() {
    let collection = ClippingPlaneCollection::default();
    assert!(collection.enabled);
    assert!(!collection.union_clipping_regions);
}

#[test]
fn clipping_plane_collection_add() {
    let mut collection = ClippingPlaneCollection::default();
    collection.add(ClippingPlane::new(DVec3::Z, 0.0));
    collection.add(ClippingPlane::new(DVec3::new(0.0, 0.0, -1.0), 0.0));
    assert_eq!(collection.len(), 2);
}

#[test]
fn clipping_plane_collection_union_mode() {
    let mut collection = ClippingPlaneCollection::default();
    collection.union_clipping_regions = true;
    assert!(collection.union_clipping_regions);
}

// ─── CloudCollection ────────────────────────────────────────────────────────

#[test]
fn cloud_collection_default() {
    let clouds = CloudCollection::default();
    assert_eq!(clouds.len(), 0);
}

#[test]
fn cloud_type_default() {
    assert_eq!(CloudType::default(), CloudType::Cumulus);
}
