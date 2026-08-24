//! Ported specs from `packages/engine/Specs/DataSources/EntityClusterSpec.js`.
//!
//! The original Jasmine suite is tagged `WebGL` (it needs a live scene,
//! canvas projection and primitive collections). This file ports the
//! pure-logic portion of each `it(...)` against the substantive Rust
//! clustering implementation: option/property handling, index bookkeeping,
//! the declutter algorithm (fed with precomputed screen-space inputs in
//! place of the gpu-limited projection/bbox calls) and the cluster event.
//! The render-surface-dependent assertions (e.g. `_clusterLabelCollection`
//! contents after `scene.renderForSpecs`) map to `cluster_count` /
//! `has_cluster_collections` on the port.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_data_sources::entity_cluster::{
    get_screen_space_positions, ClusterCandidate, ClusterFrame, ClusterItemKind, EntityCluster,
    EntityClusterOptions, ScreenSpacePoint,
};
use cesium_test_utils::assert_approx_eq_f64;

/// Builds a screen-space point with a 10x10 bbox centered on the coord.
fn make_point(entity_id: &str, x: f64, y: f64, position: Cartesian3) -> ScreenSpacePoint {
    ScreenSpacePoint {
        entity_id: entity_id.to_string(),
        kind: ClusterItemKind::Point,
        position,
        coord: Cartesian2 { x, y },
        bbox: BoundingRectangle::new(x - 5.0, y - 5.0, 10.0, 10.0),
        label_bbox: None,
        index: 0,
        clustered: false,
        cluster_show: false,
    }
}

fn make_frame<'a>(points: Vec<ScreenSpacePoint>, current_height: f64) -> ClusterFrame<'a> {
    ClusterFrame {
        points,
        current_height,
        project: None,
    }
}

fn two_nearby_points() -> Vec<ScreenSpacePoint> {
    vec![
        make_point(
            "a",
            0.0,
            0.0,
            Cartesian3 {
                x: 100.0,
                y: 0.0,
                z: 0.0,
            },
        ),
        make_point(
            "b",
            10.0,
            10.0,
            Cartesian3 {
                x: 200.0,
                y: 0.0,
                z: 0.0,
            },
        ),
    ]
}

// ============================================================================
// Constructor / property specs (mirror EntityClusterSpec.js)
// ============================================================================

#[test]
fn constructor_sets_default_properties() {
    let mut cluster = EntityCluster::new();
    assert!(!cluster.enabled());
    assert!(cluster.show);
    assert_eq!(cluster.pixel_range(), 80.0);
    assert_eq!(cluster.minimum_cluster_size(), 2);
    assert!(cluster.cluster_billboards());
    assert!(cluster.cluster_labels());
    assert!(cluster.cluster_points());

    cluster.set_enabled(true);
    assert!(cluster.enabled());

    cluster.set_pixel_range(30.0);
    assert_eq!(cluster.pixel_range(), 30.0);

    cluster.set_minimum_cluster_size(5);
    assert_eq!(cluster.minimum_cluster_size(), 5);
}

#[test]
fn constructor_sets_expected_properties() {
    let cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        show: Some(false),
        pixel_range: Some(30.0),
        minimum_cluster_size: Some(5),
        cluster_billboards: Some(false),
        cluster_labels: Some(false),
        cluster_points: Some(false),
    });
    assert!(cluster.enabled());
    assert!(!cluster.show);
    assert_eq!(cluster.pixel_range(), 30.0);
    assert_eq!(cluster.minimum_cluster_size(), 5);
    assert!(!cluster.cluster_billboards());
    assert!(!cluster.cluster_labels());
    assert!(!cluster.cluster_points());
}

#[test]
fn setters_track_dirty_flags() {
    let mut cluster = EntityCluster::new();
    assert!(cluster.ready());

    // Setting the same value does not mark dirty (JS `|| value !== this._x`).
    cluster.set_pixel_range(80.0);
    cluster.set_minimum_cluster_size(2);
    cluster.set_cluster_billboards(true);
    cluster.set_cluster_labels(true);
    cluster.set_cluster_points(true);
    assert!(!cluster.cluster_dirty());

    cluster.set_pixel_range(30.0);
    assert!(cluster.cluster_dirty());
    assert!(!cluster.ready());

    let mut cluster = EntityCluster::new();
    cluster.set_enabled(true);
    assert!(cluster.enabled_dirty());
    cluster.set_enabled(true);
    assert!(!cluster.enabled_dirty());
}

#[test]
fn initialize_marks_cluster_initialized() {
    let mut cluster = EntityCluster::new();
    assert!(!cluster.is_initialized());
    cluster.initialize();
    assert!(cluster.is_initialized());
}

// ============================================================================
// Index bookkeeping specs (createGetEntity / removeX family)
// ============================================================================

#[test]
fn records_entity_collection_indices_on_getting_billboard_label_and_point() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    let entity = "entity-1";
    cluster.get_billboard(entity);
    cluster.get_label(entity);
    cluster.get_point(entity);

    let indices = cluster.collection_indices(entity).expect("indices recorded");
    assert!(indices.billboard_index.is_some());
    assert!(indices.label_index.is_some());
    assert!(indices.point_index.is_some());
    assert!(cluster.has_label_index(entity));
}

#[test]
fn removes_entity_collection_indices_when_billboard_label_and_point_have_been_removed() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    let entity = "entity-1";
    cluster.get_billboard(entity);
    cluster.get_label(entity);
    cluster.get_point(entity);

    cluster.remove_billboard(entity);
    cluster.remove_label(entity);
    cluster.remove_point(entity);

    assert!(cluster.collection_indices(entity).is_none());
    assert!(!cluster.has_label_index(entity));
}

#[test]
fn does_not_remove_entity_collection_indices_when_at_least_one_remains() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    let entity = "entity-1";
    cluster.get_billboard(entity);
    cluster.get_label(entity);
    cluster.get_point(entity);

    cluster.remove_billboard(entity);
    cluster.remove_label(entity);

    assert!(cluster.collection_indices(entity).is_some());
    assert!(!cluster.has_label_index(entity));
}

#[test]
fn get_returns_existing_index_and_reuses_removed_indices() {
    let mut cluster = EntityCluster::new();

    let first = cluster.get_label("a");
    // Same entity gets the same index back (JS returns collection.get(index)).
    assert_eq!(cluster.get_label("a"), first);

    let second = cluster.get_label("b");
    assert_ne!(first, second);

    cluster.remove_label("a");
    // The freed index is handed out again (JS unusedIndices.shift()).
    assert_eq!(cluster.get_label("c"), first);
}

#[test]
fn getting_items_marks_cluster_dirty() {
    let mut cluster = EntityCluster::new();
    assert!(!cluster.cluster_dirty());
    cluster.get_point("a");
    assert!(cluster.cluster_dirty());
}

#[test]
fn can_destroy_cluster_and_re_add_entities() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    cluster.get_billboard("entity-1");
    cluster.get_label("entity-2");
    cluster.get_point("entity-3");

    cluster.destroy();
    assert!(cluster.is_destroyed());
    assert!(cluster.collection_indices("entity-1").is_none());

    // Per CesiumJS the instance remains reusable after destroy.
    cluster.get_billboard("entity-1");
    cluster.get_label("entity-2");
    cluster.get_point("entity-3");
    assert!(cluster.collection_indices("entity-1").is_some());
    assert!(cluster.collection_indices("entity-2").is_some());
    assert!(cluster.collection_indices("entity-3").is_some());
}

// ============================================================================
// Clustering algorithm specs (pure-logic port of the WebGL specs)
// ============================================================================

#[test]
fn clusters_points() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    // JS spec obtains the primitives first (getBillboard), which marks the
    // cluster dirty; the port models the same via get_point.
    cluster.get_point("a");
    cluster.get_point("b");

    // Clustering disabled: update keeps everything unclustered.
    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());
    assert_eq!(cluster.cluster_count(), 0);

    cluster.set_enabled(true);
    cluster.update(&mut frame);

    assert!(cluster.has_cluster_collections());
    assert_eq!(cluster.cluster_count(), 1);
    assert!(frame.points.iter().all(|p| p.clustered));
    assert!(frame.points.iter().all(|p| !p.cluster_show));

    // Disabling point clustering removes the cluster again
    // (JS `cluster.clusterPoints = false; cluster.update(frameState)`).
    cluster.set_cluster_points(false);
    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());
}

#[test]
fn clusters_on_first_update() {
    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");
    cluster.get_point("b");

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);

    assert!(cluster.has_cluster_collections());
    assert_eq!(cluster.cluster_count(), 1);
}

#[test]
fn cluster_event_fires_with_clustered_entities_and_default_styling() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    let recorded = Rc::new(RefCell::new(None));
    let sink = recorded.clone();
    cluster.cluster_event().add_listener(move |payload| {
        *sink.borrow_mut() = Some((
            payload.clustered_entities.clone(),
            payload.cluster.label.show(),
            payload.cluster.label.text(),
            payload.cluster.billboard.show(),
            payload.cluster.point.show(),
            payload.cluster.label.position(),
        ));
    });

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.set_enabled(true);
    cluster.update(&mut frame);

    let (ids, label_show, label_text, billboard_show, point_show, position) = recorded
        .borrow()
        .clone()
        .expect("cluster event raised");
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
    assert!(label_show);
    assert_eq!(label_text, "2");
    assert!(!billboard_show);
    assert!(!point_show);
    // Cluster position is the centroid of the member world positions.
    assert_approx_eq_f64!(position.x, 150.0);
    assert_approx_eq_f64!(position.y, 0.0);
    assert_approx_eq_f64!(position.z, 0.0);
}

#[test]
fn custom_cluster_styling() {
    let mut cluster = EntityCluster::new();
    cluster.initialize();

    cluster.cluster_event().add_listener(|payload| {
        payload.cluster.billboard.set_show(true);
        payload.cluster.label.set_text("cluster");
    });

    let recorded_text = Rc::new(RefCell::new(String::new()));
    let recorded_billboard = Rc::new(RefCell::new(false));
    let text_sink = recorded_text.clone();
    let billboard_sink = recorded_billboard.clone();
    cluster.cluster_event().add_listener(move |payload| {
        *text_sink.borrow_mut() = payload.cluster.label.text();
        *billboard_sink.borrow_mut() = payload.cluster.billboard.show();
    });

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.set_enabled(true);
    cluster.update(&mut frame);

    assert_eq!(cluster.cluster_count(), 1);
    assert_eq!(*recorded_text.borrow(), "cluster");
    assert!(*recorded_billboard.borrow());
}

#[test]
fn pixel_range() {
    // Two points 80 px apart: joined at the default pixelRange 80 (the
    // seed's expanded box reaches x = 5 + 80 = 85) but separated at
    // pixelRange 1 — mirroring the JS "pixel range" spec.
    let points = vec![
        make_point(
            "a",
            0.0,
            0.0,
            Cartesian3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ),
        make_point(
            "b",
            80.0,
            0.0,
            Cartesian3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ),
    ];

    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");
    cluster.get_point("b");

    // pixelRange 80: boxes extend to [-85, 95] and [-5, 165] → overlap.
    let mut frame = make_frame(points.clone(), 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);

    // pixelRange 1: boxes extend to [-6, 16] and [74, 86] → disjoint.
    cluster.set_pixel_range(1.0);
    let mut frame = make_frame(points, 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());
    assert!(frame.points.iter().all(|p| p.cluster_show));
}

#[test]
fn minimum_cluster_size() {
    // Four points at the corners of a 10x10 canvas (JS spec layout).
    fn corner_points() -> Vec<ScreenSpacePoint> {
        let zero = Cartesian3::default();
        vec![
            make_point("a", 0.0, 0.0, zero),
            make_point("b", 10.0, 0.0, zero),
            make_point("c", 0.0, 10.0, zero),
            make_point("d", 10.0, 10.0, zero),
        ]
    }

    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");

    let mut frame = make_frame(corner_points(), 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);

    cluster.set_minimum_cluster_size(5);
    let mut frame = make_frame(corner_points(), 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());
    assert!(frame.points.iter().all(|p| p.cluster_show));
}

#[test]
fn clusters_around_the_same_point_on_zoom_in() {
    // JS "clusters around the same point": after moving the camera forward,
    // the previous cluster keeps its world position via the reuse pass.
    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");
    cluster.get_point("b");

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);
    let position = cluster.previous_clusters()[0].position;

    // Zoom in (current height < previous height) with a projection callback
    // standing in for Billboard._computeScreenSpacePosition. The pixelRange
    // tweak mirrors the JS spec and marks the cluster dirty.
    cluster.set_pixel_range(cluster.pixel_range() - 1.0);
    let project = |_p: &Cartesian3| Some(Cartesian2 { x: 5.0, y: 5.0 });
    let mut frame = ClusterFrame {
        points: two_nearby_points(),
        current_height: 9999.0,
        project: Some(&project),
    };
    cluster.update(&mut frame);

    assert_eq!(cluster.cluster_count(), 1);
    let new_position = cluster.previous_clusters()[0].position;
    assert_eq!(new_position, position);
}

#[test]
fn zoom_in_without_projection_rebuilds_clusters_from_points() {
    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);

    // No projection callback: the reuse pass cannot project previous
    // clusters, so the main pass recomputes the cluster (same members).
    cluster.set_pixel_range(cluster.pixel_range() - 1.0);
    let mut frame = make_frame(two_nearby_points(), 9999.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);
    assert_eq!(cluster.previous_clusters()[0].position.x, 150.0);
}

#[test]
fn disabling_enabled_clears_clusters_and_restores_visibility() {
    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);

    // JS: setting enabled = false then update() runs updateEnable(), which
    // destroys the cluster collections and restores clusterShow.
    cluster.set_enabled(false);
    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());
    assert!(frame.points.iter().all(|p| p.cluster_show));

    // Re-enabling clusters again on the next update.
    cluster.set_enabled(true);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);
}

#[test]
fn update_skipped_when_show_is_false() {
    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        show: Some(false),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    cluster.get_point("a");

    let mut frame = make_frame(two_nearby_points(), 10000.0);
    cluster.update(&mut frame);

    // show=false short-circuits update(): dirty flags remain, no clusters.
    assert!(cluster.cluster_dirty());
    assert!(!cluster.has_cluster_collections());
}

#[test]
fn get_screen_space_positions_filters_candidates() {
    let zero = Cartesian3::default();
    let candidates = vec![
        ClusterCandidate {
            entity_id: "visible".to_string(),
            kind: ClusterItemKind::Point,
            show: true,
            visible: true,
            has_billboard: false,
            has_label: false,
            has_point: true,
            position: zero,
            screen_position: Some(Cartesian2 { x: 1.0, y: 2.0 }),
            bbox: BoundingRectangle::new(0.0, 0.0, 2.0, 2.0),
            label_bbox: None,
            index: 0,
        },
        // Hidden item (item.show = false).
        ClusterCandidate {
            entity_id: "hidden".to_string(),
            kind: ClusterItemKind::Point,
            show: false,
            visible: true,
            has_billboard: false,
            has_label: false,
            has_point: true,
            position: zero,
            screen_position: Some(Cartesian2 { x: 3.0, y: 4.0 }),
            bbox: BoundingRectangle::default(),
            label_bbox: None,
            index: 1,
        },
        // Occluded item (SCENE3D ellipsoidal occluder said not visible).
        ClusterCandidate {
            entity_id: "occluded".to_string(),
            kind: ClusterItemKind::Point,
            show: true,
            visible: false,
            has_billboard: false,
            has_label: false,
            has_point: true,
            position: zero,
            screen_position: Some(Cartesian2 { x: 5.0, y: 6.0 }),
            bbox: BoundingRectangle::default(),
            label_bbox: None,
            index: 2,
        },
        // Label item whose entity is also shown as billboard: skipped
        // (JS: canClusterLabels && canClusterBillboards → continue).
        ClusterCandidate {
            entity_id: "label-with-billboard".to_string(),
            kind: ClusterItemKind::Label,
            show: true,
            visible: true,
            has_billboard: true,
            has_label: true,
            has_point: false,
            position: zero,
            screen_position: Some(Cartesian2 { x: 7.0, y: 8.0 }),
            bbox: BoundingRectangle::default(),
            label_bbox: None,
            index: 3,
        },
        // Projection failed (computeScreenSpacePosition returned undefined).
        ClusterCandidate {
            entity_id: "unprojectable".to_string(),
            kind: ClusterItemKind::Point,
            show: true,
            visible: true,
            has_billboard: false,
            has_label: false,
            has_point: true,
            position: zero,
            screen_position: None,
            bbox: BoundingRectangle::default(),
            label_bbox: None,
            index: 4,
        },
    ];

    let mut points = Vec::new();
    get_screen_space_positions(true, true, true, &candidates, &mut points);

    assert_eq!(points.len(), 1);
    assert_eq!(points[0].entity_id, "visible");
    assert!(!points[0].cluster_show);
    assert!(!points[0].clustered);
}

#[test]
fn label_bbox_extends_bounding_box_for_non_label_items() {
    // Two points 100 px apart with pixelRange 10: disjoint without the
    // associated-label union branch of getBoundingBox.
    let mut p1 = make_point(
        "a",
        0.0,
        0.0,
        Cartesian3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    );
    p1.kind = ClusterItemKind::Billboard;
    p1.label_bbox = Some(BoundingRectangle::new(0.0, -5.0, 150.0, 10.0));
    let mut p2 = make_point(
        "b",
        100.0,
        0.0,
        Cartesian3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    p2.kind = ClusterItemKind::Billboard;

    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        pixel_range: Some(10.0),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    // Getting the items marks the cluster dirty (JS microtask deferral).
    cluster.get_billboard("a");
    cluster.get_billboard("b");

    // Without a recorded label index the union branch does not apply.
    let mut frame = make_frame(vec![p1.clone(), p2.clone()], 10000.0);
    cluster.update(&mut frame);
    assert!(!cluster.has_cluster_collections());

    // Recording a label for the entity (JS: entity.label exists +
    // hasLabelIndex) unions the label bbox → the pair now clusters.
    cluster.get_label("a");
    let mut frame = make_frame(vec![p1, p2], 10000.0);
    cluster.update(&mut frame);
    assert_eq!(cluster.cluster_count(), 1);
}

#[test]
fn many_points_cluster_deterministically_via_kd_index() {
    // 200 points in two far-apart groups exercises the kd-tree build path
    // (nodeSize 64 → non-trivial partitioning) deterministically.
    let mut points = Vec::new();
    for i in 0..100usize {
        let jitter = (i % 7) as f64 - 3.0;
        points.push(make_point(
            &format!("g1-{i}"),
            jitter,
            jitter,
            Cartesian3 {
                x: jitter,
                y: 0.0,
                z: 0.0,
            },
        ));
    }
    for i in 0..100usize {
        let jitter = (i % 7) as f64 - 3.0;
        points.push(make_point(
            &format!("g2-{i}"),
            10000.0 + jitter,
            10000.0 + jitter,
            Cartesian3 {
                x: 10000.0 + jitter,
                y: 0.0,
                z: 0.0,
            },
        ));
    }

    let mut cluster = EntityCluster::with_options(&EntityClusterOptions {
        enabled: Some(true),
        ..EntityClusterOptions::default()
    });
    cluster.initialize();
    // A single allocation marks the cluster dirty (JS does this per item
    // via microtasks; the flag is shared).
    cluster.get_point("g1-0");

    let event_count = Rc::new(RefCell::new(0usize));
    let member_total = Rc::new(RefCell::new(0usize));
    let event_sink = event_count.clone();
    let member_sink = member_total.clone();
    cluster.cluster_event().add_listener(move |payload| {
        *event_sink.borrow_mut() += 1;
        *member_sink.borrow_mut() += payload.clustered_entities.len();
    });

    let mut frame = make_frame(points, 10000.0);
    cluster.update(&mut frame);

    assert_eq!(cluster.cluster_count(), 2);
    assert_eq!(*event_count.borrow(), 2);
    assert_eq!(*member_total.borrow(), 200);
    assert!(frame.points.iter().all(|p| p.clustered));
}
