//! e2e skeleton: `priority_function` consumption through the scheduler and
//! (eventually) `HttpTileFetcher`.
//!
//! The pure `PriorityFunction` / `RequestScheduler::update_with_context`
//! machinery is implemented in `cesium-resource` and exercised here. Wiring the
//! computed priority into `HttpTileFetcher::fetch(url, priority)` — which
//! currently ignores `_priority` (`adapters/network/src/lib.rs:186`) — is the
//! joint M8.3 (#66) + M11.1 deliverable.

use cesium_resource::priority::{
    DistanceDecayPriority, FrameContext, PriorityFunction, PriorityKey, SsedPriority,
};
use cesium_resource::{Request, RequestScheduler, RequestType};

/// Skeleton: SSED priority orders a near tile ahead of a far tile.
#[test]
#[ignore = "M8-wiremock skeleton: needs HttpTileFetcher to consume `_priority` (M8.3/#66 + M11.1). See docs/deferred.md."]
fn ssed_priority_orders_near_before_far() {
    let ssed = SsedPriority::new();
    let ctx = FrameContext::new().with_camera(0.0, 0.0, 6_378_137.0);

    let near = PriorityKey::new(0, 0, 18)
        .with_center(0.0, 0.0, 6_378_137.0 - 1_000.0)
        .with_geometric_error(20.0);
    let far = PriorityKey::new(0, 0, 10)
        .with_center(0.0, 0.0, 6_378_137.0 - 500_000.0)
        .with_geometric_error(20.0);

    let p_near = ssed.compute_priority(&near, &ctx);
    let p_far = ssed.compute_priority(&far, &ctx);
    // Lower value = higher priority; the near tile must sort first.
    assert!(p_near <= p_far, "near={p_near} far={p_far}");
}

/// Skeleton: the scheduler recomputes pending priorities from frame context.
#[test]
#[ignore = "M8-wiremock skeleton: needs HttpTileFetcher to consume `_priority` (M8.3/#66 + M11.1). See docs/deferred.md."]
fn scheduler_recomputes_priority_with_context() {
    let mut scheduler = RequestScheduler::new();
    scheduler.maximum_requests = 0; // keep the request pending
    let pf: Box<dyn PriorityFunction> = Box::new(SsedPriority::new());
    scheduler.set_priority_function(pf);
    assert_eq!(scheduler.priority_function_name(), Some("SSED"));

    let key = PriorityKey::new(1, 1, 14)
        .with_center(0.0, 0.0, 6_378_137.0 - 10_000.0)
        .with_geometric_error(75.0);

    let id = scheduler
        .schedule(
            Request::throttled(
                "https://tiles.example.com/1/1/14.b3dm".to_string(),
                RequestType::Tiles3D,
                12345.0, // sentinel priority that the function must overwrite
            )
            .with_priority_key(key),
        )
        .unwrap();

    let ctx = FrameContext::new().with_camera(0.0, 0.0, 6_378_137.0);
    scheduler.update_with_context(&ctx);

    let request = scheduler.get_request(id).expect("request is pending");
    assert!(
        (request.priority - 12345.0).abs() > f64::EPSILON,
        "priority function should have recomputed the sentinel value"
    );
}

/// Skeleton: distance-decay priority is a valid alternative signal.
#[test]
#[ignore = "M8-wiremock skeleton: needs HttpTileFetcher to consume `_priority` (M8.3/#66 + M11.1). See docs/deferred.md."]
fn distance_decay_priority_is_monotonic() {
    let decay = DistanceDecayPriority::new();
    let ctx = FrameContext::new().with_camera(0.0, 0.0, 6_378_137.0);

    let near = PriorityKey::new(0, 0, 16).with_center(0.0, 0.0, 6_378_137.0 - 500.0);
    let far = PriorityKey::new(0, 0, 8).with_center(0.0, 0.0, 6_378_137.0 - 900_000.0);

    let p_near = decay.compute_priority(&near, &ctx);
    let p_far = decay.compute_priority(&far, &ctx);
    assert!(p_near <= p_far, "near={p_near} far={p_far}");
}
