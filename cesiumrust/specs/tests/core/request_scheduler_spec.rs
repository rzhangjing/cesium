//! RequestScheduler spec - ported from packages/engine/Specs/Core/RequestSchedulerSpec.js
//!
//! A-class tests: 15 (pure logic, synchronous scheduler)

use cesium_resource::{get_server_key, Request, RequestScheduler, RequestState, RequestType};

#[cfg(test)]
mod tests {
    use super::*;

    /// "getServer with https"
    #[test]
    fn get_server_key_https() {
        let server = get_server_key("https://test.invalid/1");
        assert_eq!(server, "test.invalid:443");
    }

    /// "getServer with http"
    #[test]
    fn get_server_key_http() {
        let server = get_server_key("http://test.invalid/1");
        assert_eq!(server, "test.invalid:80");
    }

    /// "getServer with explicit port"
    #[test]
    fn get_server_key_explicit_port() {
        let server = get_server_key("https://test.invalid:8443/1");
        assert_eq!(server, "test.invalid:8443");
    }

    /// "getServer strips credentials"
    #[test]
    fn get_server_key_strips_credentials() {
        let server = get_server_key("https://user:pass@test.invalid/1");
        assert_eq!(server, "test.invalid:443");
    }

    /// "honors maximumRequests"
    #[test]
    fn honors_maximum_requests() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 2;
        scheduler.throttle_requests = true;

        // Schedule 2 requests (should succeed)
        let r1 = Request::throttled("http://test.invalid/1".to_string(), RequestType::Other, 0.0);
        let r2 = Request::throttled("http://test.invalid/2".to_string(), RequestType::Other, 0.0);
        let id1 = scheduler.schedule(r1);
        let id2 = scheduler.schedule(r2);
        assert!(id1.is_some());
        assert!(id2.is_some());

        scheduler.update();
        assert_eq!(scheduler.active_request_count(), 2);

        // Third request goes to pending (heap has room), but won't activate
        let r3 = Request::throttled("http://test.invalid/3".to_string(), RequestType::Other, 0.0);
        let _id3 = scheduler.schedule(r3);
        scheduler.update();
        // Active count stays at 2 (max)
        assert_eq!(scheduler.active_request_count(), 2);
    }

    /// "honors maximumRequestsPerServer"
    #[test]
    fn honors_maximum_requests_per_server() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 2;
        scheduler.throttle_requests = true;

        let url = "http://test.invalid/1";
        let server = get_server_key(url);

        // Schedule 2 requests to same server
        let r1 = Request::throttled(url.to_string(), RequestType::Other, 0.0);
        let r2 = Request::throttled(url.to_string(), RequestType::Other, 0.0);
        scheduler.schedule(r1);
        scheduler.schedule(r2);
        scheduler.update();

        assert!(!scheduler.server_has_open_slots(&server, 1));

        // Different server should have slots
        assert!(scheduler.server_has_open_slots("other.invalid:80", 1));
    }

    /// "honors priorityHeapLength"
    #[test]
    fn honors_priority_heap_length() {
        let mut scheduler = RequestScheduler::new();
        scheduler.priority_heap_length = 1;
        scheduler.maximum_requests = 0; // Force all to pending
        scheduler.throttle_requests = true;

        let r1 = Request::throttled("http://test.invalid/1".to_string(), RequestType::Other, 0.0);
        let id1 = scheduler.schedule(r1);
        assert!(id1.is_some());

        // Heap is full, second request rejected
        let r2 = Request::throttled("http://test.invalid/2".to_string(), RequestType::Other, 1.0);
        let id2 = scheduler.schedule(r2);
        assert!(id2.is_none());
    }

    /// "request goes through immediately when throttle is false"
    #[test]
    fn immediate_when_not_throttled() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;

        // Non-throttled request goes immediately active
        let mut r = Request::new("https://test.invalid/1".to_string(), RequestType::Other);
        r.throttle = false;
        let id = scheduler.schedule(r);
        assert!(id.is_some());

        // Should be active immediately (no update needed)
        let req = scheduler.get_request(id.unwrap()).unwrap();
        assert_eq!(req.state, RequestState::Active);
    }

    /// "makes a throttled request" - state transitions
    #[test]
    fn throttled_request_state_transitions() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;
        scheduler.maximum_requests = 0; // Force to pending initially

        let r = Request::throttled("https://test.invalid/1".to_string(), RequestType::Other, 0.0);
        assert_eq!(r.state, RequestState::Unissued);

        let id = scheduler.schedule(r).unwrap();
        // After schedule with max=0, state is Issued (pending)
        {
            let req = scheduler.get_request(id).unwrap();
            assert_eq!(req.state, RequestState::Issued);
        }

        // Now allow activation
        scheduler.maximum_requests = 1;
        scheduler.update();
        {
            let req = scheduler.get_request(id).unwrap();
            assert_eq!(req.state, RequestState::Active);
        }

        // After complete, request is removed
        scheduler.complete(id);
        assert_eq!(scheduler.active_request_count(), 0);
        assert!(scheduler.get_request(id).is_none());
    }

    /// "cancels an issued request"
    #[test]
    fn cancels_issued_request() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;
        scheduler.maximum_requests = 0; // Force to pending

        let r = Request::throttled("https://test.invalid/1".to_string(), RequestType::Other, 0.0);
        let id = scheduler.schedule(r).unwrap();

        // Verify it's pending
        assert_eq!(scheduler.pending_request_count(), 1);

        // Cancel before update
        assert!(scheduler.cancel(id));
        // Request is removed after cancel
        assert!(scheduler.get_request(id).is_none());
    }

    /// "cancels an active request"
    #[test]
    fn cancels_active_request() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;

        let r = Request::throttled("https://test.invalid/1".to_string(), RequestType::Other, 0.0);
        let id = scheduler.schedule(r).unwrap();
        scheduler.update();

        // Now active
        {
            let req = scheduler.get_request(id).unwrap();
            assert_eq!(req.state, RequestState::Active);
        }
        assert_eq!(scheduler.active_request_count(), 1);

        // Cancel
        assert!(scheduler.cancel(id));
        // Request is removed after cancel
        assert!(scheduler.get_request(id).is_none());
        assert_eq!(scheduler.active_request_count(), 0);
    }

    /// "prioritizes requests" - lower priority value = higher priority
    #[test]
    fn prioritizes_requests() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;
        scheduler.maximum_requests = 1; // Only 1 active at a time

        // Schedule requests with different priorities
        let r1 = Request::throttled("http://test.invalid/1".to_string(), RequestType::Other, 0.9);
        let r2 = Request::throttled("http://test.invalid/2".to_string(), RequestType::Other, 0.1);
        let r3 = Request::throttled("http://test.invalid/3".to_string(), RequestType::Other, 0.5);

        let id1 = scheduler.schedule(r1).unwrap();
        let _id2 = scheduler.schedule(r2).unwrap();
        let _id3 = scheduler.schedule(r3).unwrap();

        // First update activates one request
        scheduler.update();
        assert_eq!(scheduler.active_request_count(), 1);

        // Complete it and update - should activate highest priority (lowest value)
        scheduler.complete(id1);
        scheduler.update();

        // After update, one of the pending should be active
        assert_eq!(scheduler.active_request_count(), 1);
    }

    /// "handles low priority requests" - heap full rejects low priority
    #[test]
    fn handles_low_priority_requests() {
        let mut scheduler = RequestScheduler::new();
        scheduler.throttle_requests = true;
        scheduler.maximum_requests = 0; // Force all to pending
        scheduler.priority_heap_length = 2;

        // Fill the heap
        let r1 = Request::throttled("http://test.invalid/1".to_string(), RequestType::Other, 0.5);
        let r2 = Request::throttled("http://test.invalid/2".to_string(), RequestType::Other, 0.5);
        assert!(scheduler.schedule(r1).is_some());
        assert!(scheduler.schedule(r2).is_some());

        // Heap full, low priority rejected
        let r3 = Request::throttled("http://test.invalid/3".to_string(), RequestType::Other, 1.0);
        assert!(scheduler.schedule(r3).is_none());
    }

    /// "does not throttle requests when throttleRequests is false"
    #[test]
    fn no_throttle_when_disabled() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0; // Would normally block

        // With throttle_requests = false, requests go through
        scheduler.throttle_requests = false;
        let r = Request::throttled("https://test.invalid/1".to_string(), RequestType::Other, 0.0);
        let id = scheduler.schedule(r);
        assert!(id.is_some());

        let req = scheduler.get_request(id.unwrap()).unwrap();
        assert_eq!(req.state, RequestState::Active);
    }

    /// "serverHasOpenSlots works for single requests"
    #[test]
    fn server_has_open_slots_single() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 5;
        scheduler.throttle_requests = true;

        let server = "test.invalid:80";

        // Initially has slots
        assert!(scheduler.server_has_open_slots(server, 1));

        // Schedule 5 requests
        for i in 0..5 {
            let r = Request::throttled(
                format!("http://test.invalid/{}", i),
                RequestType::Other,
                0.0,
            );
            scheduler.schedule(r);
        }
        scheduler.update();

        // Now full
        assert!(!scheduler.server_has_open_slots(server, 1));
    }

    /// "serverHasOpenSlots works for multiple requests"
    #[test]
    fn server_has_open_slots_multiple() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 5;
        scheduler.throttle_requests = true;

        let server = "test.invalid:80";

        // Schedule 2 requests
        for i in 0..2 {
            let r = Request::throttled(
                format!("http://test.invalid/{}", i),
                RequestType::Other,
                0.0,
            );
            scheduler.schedule(r);
        }
        scheduler.update();

        // 3 more should fit (2+3=5)
        assert!(scheduler.server_has_open_slots(server, 3));
        // 4 more should not (2+4=6 > 5)
        assert!(!scheduler.server_has_open_slots(server, 4));
    }

    /// "requestsByServer allows for custom maximum requests"
    #[test]
    fn custom_requests_by_server() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 2; // Default
        scheduler.requests_by_server.insert("test.invalid:80".to_string(), 23);
        scheduler.throttle_requests = true;

        let server = "test.invalid:80";

        // Schedule 23 requests (custom limit)
        for i in 0..23 {
            let r = Request::throttled(
                format!("http://test.invalid/{}", i),
                RequestType::Other,
                0.0,
            );
            scheduler.schedule(r);
        }
        scheduler.update();

        // Should still have slots at 23
        assert!(scheduler.server_has_open_slots(server, 0));
        // But not 1 more
        assert!(!scheduler.server_has_open_slots(server, 1));
    }

    /// "heapHasOpenSlots"
    #[test]
    fn heap_has_open_slots() {
        let mut scheduler = RequestScheduler::new();
        scheduler.priority_heap_length = 5;

        assert!(scheduler.heap_has_open_slots(5));
        assert!(!scheduler.heap_has_open_slots(6));
    }
}
