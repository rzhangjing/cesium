//! Port of `Core/RequestSchedulerSpec.js` (with the resource fetch
//! throttling wiring tests appended).
//!
//! DEVIATION: JS tests drive the scheduler through promises/deferreds; the
//! Rust port observes the state machine directly (tracked request states,
//! statistics, completion helpers standing in for `deferred.resolve()` /
//! `deferred.reject()`). Browser-only cases (blob uris via
//! `URL.createObjectURL`, unhandled-rejection detection, console
//! `debugShowStatistics` logging) are not mirrored.
//!
//! The scheduler is a process-wide global, so every test here runs under a
//! shared lock with `clearForSpecs` setup/teardown (mirrors the JS
//! `beforeEach`/`afterEach`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cesium_core::request::{PriorityFunction, Request};
use cesium_core::request_scheduler::RequestScheduler;
use cesium_core::request_state::RequestState;
use cesium_core::resource::{
    MockResourceBackend, Resource, ResourceError, ResourceOptions, Response,
};
use cesium_test_utils::expect_to_throw_dev_error;

/// Mirrors the JS `beforeEach`/`afterEach` global reset.
static SPEC_LOCK: Mutex<()> = Mutex::new(());

fn reset_scheduler() {
    RequestScheduler::clear_for_specs();
    RequestScheduler::clear_requests_by_server_for_specs();
    RequestScheduler::set_maximum_requests(50);
    RequestScheduler::set_maximum_requests_per_server(18);
    RequestScheduler::set_throttle_requests(true);
    RequestScheduler::set_priority_heap_length(20);
}

fn serial(f: impl FnOnce()) {
    let _guard = SPEC_LOCK.lock().unwrap();
    reset_scheduler();
    f();
    reset_scheduler();
}

async fn serial_async(f: impl std::future::Future<Output = ()>) {
    // Held across awaits on purpose: the current-thread executor keeps
    // everything on one thread, and the scheduler global must stay isolated
    // from other test functions.
    let _guard = SPEC_LOCK.lock().unwrap();
    reset_scheduler();
    f.await;
    reset_scheduler();
}

fn throttled_request(url: &str, priority: f64) -> Request {
    Request::new(
        Some(url.to_string()),
        Some(priority),
        Some(true),
        None,
        None,
        None,
    )
}

fn immediate_request(url: &str) -> Request {
    Request::new(Some(url.to_string()), None, None, None, None, None)
}

// ── getServerKey ─────────────────────────────────────────────────────

#[test]
fn get_server_key_with_https() {
    // JS: "getServer with https"
    let server = RequestScheduler::get_server_key("https://test.invalid/1");
    assert_eq!(server, "test.invalid:443");
}

#[test]
fn get_server_key_with_http() {
    // JS: "getServer with http"
    let server = RequestScheduler::get_server_key("http://test.invalid/1");
    assert_eq!(server, "test.invalid:80");
}

#[test]
fn request_throws_when_url_is_undefined() {
    // JS: "request throws when request.url is undefined". The
    // requestFunction check has no Rust equivalent (DEVIATION: execution is
    // caller-driven).
    serial(|| {
        let mut request = Request::new(None, None, None, None, None, None);
        expect_to_throw_dev_error(|| {
            let _ = RequestScheduler::request(&mut request);
        });
    });
}

// ── capacity limits ──────────────────────────────────────────────────

#[test]
fn honors_maximum_requests() {
    serial(|| {
        RequestScheduler::set_maximum_requests(2);

        let mut r1 = throttled_request("http://test.invalid/1", 0.0);
        let mut r2 = throttled_request("http://test.invalid/1", 0.1);
        assert!(RequestScheduler::request(&mut r1).is_some());
        assert!(RequestScheduler::request(&mut r2).is_some());
        RequestScheduler::update();

        // Scheduler is full, r3 will be rejected
        let mut r3 = throttled_request("http://test.invalid/1", 0.2);
        assert!(RequestScheduler::request(&mut r3).is_none());
        RequestScheduler::update();

        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 2);

        // Scheduler now has an empty slot, r4 goes through
        RequestScheduler::complete_request_with_id(r1.id());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 1);

        let mut r4 = throttled_request("http://test.invalid/1", 0.3);
        assert!(RequestScheduler::request(&mut r4).is_some());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 2);

        // Scheduler is full, r5 will be rejected
        let mut r5 = throttled_request("http://test.invalid/1", 0.4);
        assert!(RequestScheduler::request(&mut r5).is_none());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 2);

        // maximumRequests increases, r6 goes through
        RequestScheduler::set_maximum_requests(3);
        let mut r6 = throttled_request("http://test.invalid/1", 0.5);
        assert!(RequestScheduler::request(&mut r6).is_some());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 3);

        for id in [r2.id(), r4.id(), r6.id()] {
            RequestScheduler::complete_request_with_id(id);
        }
        RequestScheduler::update();
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 0);
    });
}

#[test]
fn honors_maximum_requests_per_server() {
    serial(|| {
        RequestScheduler::set_maximum_requests_per_server(2);

        let url = "http://test.invalid/1";
        let server = RequestScheduler::get_server_key(url);

        let mut r1 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        let mut r2 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut r1).is_some());
        assert!(RequestScheduler::request(&mut r2).is_some());
        RequestScheduler::update();

        // Server is full, r3 will be rejected
        let mut r3 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut r3).is_none());
        RequestScheduler::update();

        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 2);

        // Server now has an empty slot, r4 goes through
        RequestScheduler::complete_request_with_id(r1.id());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 1);

        let mut r4 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut r4).is_some());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 2);

        // Server is full, r5 will be rejected
        let mut r5 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut r5).is_none());
        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 2);

        // maximumRequestsPerServer increases, r6 goes through
        RequestScheduler::set_maximum_requests_per_server(3);
        let mut r6 = Request::new(Some(url.to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut r6).is_some());
        RequestScheduler::update();
        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 3);

        for id in [r2.id(), r4.id(), r6.id()] {
            RequestScheduler::complete_request_with_id(id);
        }
        RequestScheduler::update();
        assert_eq!(RequestScheduler::number_of_active_requests_by_server(&server), 0);
    });
}

#[test]
fn honors_priority_heap_length() {
    serial(|| {
        RequestScheduler::set_priority_heap_length(1);

        let mut first = throttled_request("http://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut first).is_some());

        // Heap is full; the new (worse) request is bumped immediately.
        let mut second = throttled_request("http://test.invalid/1", 1.0);
        assert!(RequestScheduler::request(&mut second).is_none());

        RequestScheduler::set_priority_heap_length(3);
        let mut third = throttled_request("http://test.invalid/1", 2.0);
        let mut fourth = throttled_request("http://test.invalid/1", 3.0);
        assert!(RequestScheduler::request(&mut third).is_some());
        assert!(RequestScheduler::request(&mut fourth).is_some());
        let mut fifth = throttled_request("http://test.invalid/1", 4.0);
        assert!(RequestScheduler::request(&mut fifth).is_none());

        // A request is cancelled to accommodate the new heap length (the
        // highest priority one, mirroring JS heap.pop()).
        RequestScheduler::set_priority_heap_length(2);
        assert_eq!(
            RequestScheduler::tracked_request_state(first.id()),
            Some(RequestState::Cancelled)
        );

        for id in [third.id(), fourth.id()] {
            RequestScheduler::update();
            RequestScheduler::complete_request_with_id(id);
        }
    });
}

// ── immediate / data uri paths ───────────────────────────────────────

#[test]
fn data_uri_goes_through_immediately() {
    serial(|| {
        let data_uri = "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3D";
        let mut request = immediate_request(data_uri);
        assert!(RequestScheduler::request(&mut request).is_some());

        assert_eq!(request.state, RequestState::Received);
        assert!(request.server_key.is_none());
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 0);
    });
}

#[test]
fn request_goes_through_immediately_when_throttle_is_false() {
    serial(|| {
        let url = "https://test.invalid/1";
        let mut request = immediate_request(url);
        assert!(RequestScheduler::request(&mut request).is_some());

        let server_key = request.server_key.clone().unwrap();
        assert_eq!(request.state, RequestState::Active);
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 1);
        assert_eq!(
            RequestScheduler::number_of_active_requests_by_server(&server_key),
            1
        );

        RequestScheduler::complete_request_with_id(request.id());
        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Received)
        );
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 0);
        assert_eq!(
            RequestScheduler::number_of_active_requests_by_server(&server_key),
            0
        );
    });
}

// ── throttled lifecycle ──────────────────────────────────────────────

#[test]
fn makes_a_throttled_request() {
    serial(|| {
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        assert_eq!(request.state, RequestState::Unissued);

        assert!(RequestScheduler::request(&mut request).is_some());
        assert_eq!(request.state, RequestState::Issued);

        RequestScheduler::update();
        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Active)
        );

        RequestScheduler::complete_request_with_id(request.id());
        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Received)
        );
    });
}

#[test]
fn cancels_an_issued_request() {
    serial(|| {
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut request).is_some());
        assert_eq!(request.state, RequestState::Issued);

        RequestScheduler::cancel_request(request.id());
        RequestScheduler::update();

        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Cancelled)
        );
        assert_eq!(RequestScheduler::statistics().number_of_cancelled_requests, 1);
        assert_eq!(
            RequestScheduler::statistics().number_of_cancelled_active_requests,
            0
        );
    });
}

#[test]
fn cancels_an_active_request() {
    serial(|| {
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        // DEVIATION: the JS cancelFunction spy has no Rust equivalent.

        assert!(RequestScheduler::request(&mut request).is_some());
        RequestScheduler::update();
        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Active)
        );

        RequestScheduler::cancel_request(request.id());
        RequestScheduler::update();

        let server_key = request.server_key.clone().unwrap();
        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Cancelled)
        );
        assert_eq!(RequestScheduler::statistics().number_of_cancelled_requests, 1);
        assert_eq!(
            RequestScheduler::statistics().number_of_cancelled_active_requests,
            1
        );
        assert_eq!(
            RequestScheduler::number_of_active_requests_by_server(&server_key),
            0
        );
    });
}

#[test]
fn handles_request_failure() {
    serial(|| {
        let mut request = immediate_request("https://test.invalid/1");
        assert!(RequestScheduler::request(&mut request).is_some());
        assert_eq!(request.state, RequestState::Active);
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 1);

        RequestScheduler::fail_request_with_id(request.id(), "Request failed");
        RequestScheduler::update();

        assert_eq!(
            RequestScheduler::tracked_request_state(request.id()),
            Some(RequestState::Failed)
        );
        assert_eq!(RequestScheduler::statistics().number_of_active_requests, 0);
        assert_eq!(RequestScheduler::statistics().number_of_failed_requests, 1);
    });
}

// ── priority ordering ────────────────────────────────────────────────

#[test]
fn prioritizes_requests() {
    serial(|| {
        RequestScheduler::set_maximum_requests(1);

        let priorities = [0.7_f64, 0.2, 0.9, 0.1];
        let mut ids = Vec::new();
        for priority in priorities {
            let mut request = throttled_request("https://test.invalid/1", priority);
            assert!(RequestScheduler::request(&mut request).is_some());
            ids.push((request.id(), priority));
        }

        // With one open slot per update, requests must activate in
        // ascending priority order (lower value = higher priority).
        let mut activation_order = Vec::new();
        for _ in 0..ids.len() {
            RequestScheduler::update();
            let activated: Vec<(u64, f64)> = ids
                .iter()
                .filter(|(id, _)| {
                    RequestScheduler::tracked_request_state(*id)
                        == Some(RequestState::Active)
                })
                .copied()
                .collect();
            assert_eq!(activated.len(), 1);
            activation_order.push(activated[0].1);
            RequestScheduler::complete_request_with_id(activated[0].0);
        }

        let mut expected = priorities.to_vec();
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(activation_order, expected);
    });
}

#[test]
fn updates_priority() {
    serial(|| {
        let length = 4;
        RequestScheduler::set_priority_heap_length(length);

        let invert_priority = Arc::new(AtomicBool::new(false));
        for i in 0..length {
            let priority = i as f64 / (length - 1) as f64;
            let mut request = throttled_request("https://test.invalid/1", priority);
            let flag = invert_priority.clone();
            let pf: PriorityFunction = Arc::new(Mutex::new(move || {
                if flag.load(Ordering::Relaxed) {
                    1.0 - priority
                } else {
                    priority
                }
            }));
            request.set_priority_function(pf);
            assert!(RequestScheduler::request(&mut request).is_some());
        }

        // Update priorities while not letting any requests go through
        // (JS: maximumRequests = 0 AFTER the requests were queued).
        RequestScheduler::set_maximum_requests(0);

        RequestScheduler::update();
        let order = RequestScheduler::request_heap_pop_order_for_specs();
        assert_eq!(order.len(), length);
        for window in order.windows(2) {
            assert!(window[0] <= window[1], "expected ascending pop order");
        }

        invert_priority.store(true, Ordering::Relaxed);
        RequestScheduler::update();
        let inverted_order = RequestScheduler::request_heap_pop_order_for_specs();
        let mut expected: Vec<f64> = (0..length)
            .map(|i| 1.0 - i as f64 / (length - 1) as f64)
            .collect();
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(inverted_order.len(), length);
        for (actual, expected) in inverted_order.iter().zip(expected.iter()) {
            assert!((actual - expected).abs() < 1e-12);
        }
    });
}

#[test]
fn handles_low_priority_requests() {
    serial(|| {
        let length = RequestScheduler::priority_heap_length();
        let mut ids = Vec::new();
        for _ in 0..length {
            let mut request = throttled_request("https://test.invalid/1", 0.5);
            assert!(RequestScheduler::request(&mut request).is_some());
            ids.push(request.id());
        }

        // Heap is full so low priority request is not even issued
        let mut low = throttled_request("https://test.invalid/1", 1.0);
        assert!(RequestScheduler::request(&mut low).is_none());
        assert_eq!(RequestScheduler::statistics().number_of_cancelled_requests, 0);

        // Heap is full so high priority request bumps off a lower priority one
        let mut high = throttled_request("https://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut high).is_some());
        assert_eq!(RequestScheduler::statistics().number_of_cancelled_requests, 1);

        RequestScheduler::update();
        assert_eq!(
            RequestScheduler::tracked_request_state(high.id()),
            Some(RequestState::Active)
        );
        for id in ids {
            if RequestScheduler::tracked_request_state(id) == Some(RequestState::Active) {
                RequestScheduler::complete_request_with_id(id);
            }
        }
        RequestScheduler::complete_request_with_id(high.id());
    });
}

#[test]
fn unthrottled_requests_starve_throttled_requests() {
    serial(|| {
        let mut throttled = throttled_request("http://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut throttled).is_some());

        let mut unthrottled_ids = Vec::new();
        for _ in 0..RequestScheduler::maximum_requests() {
            let mut request = immediate_request("http://test.invalid/1");
            assert!(RequestScheduler::request(&mut request).is_some());
            unthrottled_ids.push(request.id());
        }
        RequestScheduler::update();

        assert_eq!(
            RequestScheduler::tracked_request_state(throttled.id()),
            Some(RequestState::Issued)
        );

        // Resolve one of the unthrottled requests
        RequestScheduler::complete_request_with_id(unthrottled_ids[0]);
        RequestScheduler::update();
        assert_eq!(
            RequestScheduler::tracked_request_state(throttled.id()),
            Some(RequestState::Active)
        );

        for id in unthrottled_ids.iter().skip(1) {
            RequestScheduler::complete_request_with_id(*id);
        }
        RequestScheduler::complete_request_with_id(throttled.id());
    });
}

#[test]
fn request_throttled_by_server_is_cancelled() {
    serial(|| {
        let url = "http://test.invalid/1";
        let mut active_ids = Vec::new();
        for _ in 0..RequestScheduler::maximum_requests_per_server() - 1 {
            let mut request = immediate_request(url);
            assert!(RequestScheduler::request(&mut request).is_some());
            active_ids.push(request.id());
        }

        let mut throttled =
            Request::new(Some(url.to_string()), None, Some(true), Some(true), None, None);
        assert!(RequestScheduler::request(&mut throttled).is_some());

        let mut one_more = immediate_request(url);
        assert!(RequestScheduler::request(&mut one_more).is_some());
        active_ids.push(one_more.id());

        RequestScheduler::update();
        assert_eq!(
            RequestScheduler::tracked_request_state(throttled.id()),
            Some(RequestState::Cancelled)
        );

        for id in active_ids {
            RequestScheduler::complete_request_with_id(id);
        }
    });
}

// ── throttleRequests switch ──────────────────────────────────────────

#[test]
fn does_not_throttle_requests_when_throttle_requests_is_false() {
    serial(|| {
        RequestScheduler::set_maximum_requests(0);

        RequestScheduler::set_throttle_requests(true);
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut request).is_none());

        RequestScheduler::set_throttle_requests(false);
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut request).is_some());
        RequestScheduler::complete_request_with_id(request.id());

        RequestScheduler::set_throttle_requests(true);
    });
}

#[test]
fn does_not_throttle_by_server_when_throttle_requests_is_false() {
    serial(|| {
        RequestScheduler::set_maximum_requests_per_server(0);

        RequestScheduler::set_throttle_requests(true);
        let mut request =
            Request::new(Some("https://test.invalid/1".to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut request).is_none());

        RequestScheduler::set_throttle_requests(false);
        let mut request =
            Request::new(Some("https://test.invalid/1".to_string()), None, None, Some(true), None, None);
        assert!(RequestScheduler::request(&mut request).is_some());
        RequestScheduler::complete_request_with_id(request.id());

        RequestScheduler::set_throttle_requests(true);
    });
}

// ── requestCompletedEvent ────────────────────────────────────────────

#[test]
fn successful_request_causes_request_completed_event_to_be_raised() {
    serial(|| {
        let mut request = immediate_request("https://test.invalid/1");
        assert!(RequestScheduler::request(&mut request).is_some());

        let raised = Arc::new(AtomicBool::new(false));
        let flag = raised.clone();
        let listener =
            RequestScheduler::add_request_completed_listener(move |error| {
                assert!(error.is_none());
                flag.store(true, Ordering::Relaxed);
            });

        RequestScheduler::complete_request_with_id(request.id());
        assert!(raised.load(Ordering::Relaxed));
        assert!(RequestScheduler::remove_request_completed_listener(listener));
    });
}

#[test]
fn successful_data_request_causes_request_completed_event_to_be_raised() {
    serial(|| {
        let raised = Arc::new(AtomicBool::new(false));
        let flag = raised.clone();
        let listener =
            RequestScheduler::add_request_completed_listener(move |error| {
                assert!(error.is_none());
                flag.store(true, Ordering::Relaxed);
            });

        let mut request =
            immediate_request("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3D");
        assert!(RequestScheduler::request(&mut request).is_some());
        assert!(raised.load(Ordering::Relaxed));

        assert!(RequestScheduler::remove_request_completed_listener(listener));
    });
}

#[test]
fn unsuccessful_requests_raise_request_completed_event_with_error() {
    serial(|| {
        let mut request = immediate_request("https://test.invalid/1");
        assert!(RequestScheduler::request(&mut request).is_some());

        let received_error = Arc::new(Mutex::new(None::<String>));
        let sink = received_error.clone();
        let listener = RequestScheduler::add_request_completed_listener(move |error| {
            *sink.lock().unwrap() = error;
        });

        RequestScheduler::fail_request_with_id(request.id(), "boom");
        assert_eq!(
            received_error.lock().unwrap().as_deref(),
            Some("boom")
        );
        assert!(RequestScheduler::remove_request_completed_listener(listener));
    });
}

#[test]
fn cancelled_requests_do_not_cause_request_completed_event_to_be_raised() {
    serial(|| {
        let mut request = throttled_request("https://test.invalid/1", 0.0);
        assert!(RequestScheduler::request(&mut request).is_some());

        let raised = Arc::new(AtomicBool::new(false));
        let flag = raised.clone();
        let listener =
            RequestScheduler::add_request_completed_listener(move |_| {
                flag.store(true, Ordering::Relaxed);
            });

        RequestScheduler::cancel_request(request.id());
        RequestScheduler::update();
        assert!(!raised.load(Ordering::Relaxed));

        assert!(RequestScheduler::remove_request_completed_listener(listener));
        assert_eq!(RequestScheduler::number_of_request_completed_listeners(), 0);
    });
}

// ── requestsByServer / serverHasOpenSlots ────────────────────────────

#[test]
fn requests_by_server_allows_custom_maximum_requests() {
    serial(|| {
        RequestScheduler::set_requests_for_server("test.invalid:80", 23);

        let mut ids = Vec::new();
        for _ in 0..23 {
            let mut request = Request::new(
                Some("http://test.invalid/1".to_string()),
                None,
                Some(true),
                Some(true),
                None,
                None,
            );
            assert!(RequestScheduler::request(&mut request).is_some());
            RequestScheduler::update();
            ids.push(request.id());
        }

        let mut one_more = Request::new(
            Some("http://test.invalid/1".to_string()),
            None,
            Some(true),
            Some(true),
            None,
            None,
        );
        assert!(RequestScheduler::request(&mut one_more).is_none());

        for id in ids {
            RequestScheduler::complete_request_with_id(id);
        }
    });
}

#[test]
fn server_has_open_slots_works_for_single_requests() {
    serial(|| {
        RequestScheduler::set_maximum_requests_per_server(5);

        let mut ids = Vec::new();
        for _ in 0..2 {
            let mut request = immediate_request("https://test.invalid:80/1");
            assert!(RequestScheduler::request(&mut request).is_some());
            ids.push(request.id());
        }
        assert!(RequestScheduler::server_has_open_slots("test.invalid:80", None));

        for _ in 0..3 {
            let mut request = immediate_request("https://test.invalid:80/1");
            assert!(RequestScheduler::request(&mut request).is_some());
            ids.push(request.id());
        }
        assert!(!RequestScheduler::server_has_open_slots("test.invalid:80", None));

        for id in ids {
            RequestScheduler::complete_request_with_id(id);
        }
    });
}

#[test]
fn server_has_open_slots_works_for_multiple_requests() {
    serial(|| {
        RequestScheduler::set_maximum_requests_per_server(5);

        let mut ids = Vec::new();
        for _ in 0..2 {
            let mut request = immediate_request("https://test.invalid:80/1");
            assert!(RequestScheduler::request(&mut request).is_some());
            ids.push(request.id());
        }
        assert!(RequestScheduler::server_has_open_slots("test.invalid:80", Some(3)));
        assert!(!RequestScheduler::server_has_open_slots("test.invalid:80", Some(4)));

        for id in ids {
            RequestScheduler::complete_request_with_id(id);
        }
    });
}

// ── Resource fetch throttling wiring (CZ-07) ─────────────────────────

#[tokio::test]
async fn fetch_goes_through_the_scheduler_and_completes() {
    serial_async(async {
        let mut backend = MockResourceBackend::new();
        backend.register_response("https://test.invalid/data", b"hello".to_vec());

        let raised = Arc::new(AtomicBool::new(false));
        let flag = raised.clone();
        let listener =
            RequestScheduler::add_request_completed_listener(move |error| {
                assert!(error.is_none());
                flag.store(true, Ordering::Relaxed);
            });

        let mut resource = Resource::new("https://test.invalid/data".to_string());
        let response = resource.fetch(&backend, None).await.unwrap();
        assert_eq!(response, Response::Text("hello".to_string()));

        // Slot released, event raised, resource reusable.
        assert_eq!(RequestScheduler::active_requests_length(), 0);
        assert!(raised.load(Ordering::Relaxed));

        let response = resource.fetch(&backend, None).await.unwrap();
        assert_eq!(response, Response::Text("hello".to_string()));

        RequestScheduler::remove_request_completed_listener(listener);
    })
    .await;
}

#[tokio::test]
async fn fetch_with_throttled_request_is_promoted_before_sending() {
    serial_async(async {
        let mut backend = MockResourceBackend::new();
        backend.register_response("https://test.invalid/throttled", b"ok".to_vec());

        let mut scheduler_request =
            throttled_request("https://test.invalid/throttled", 0.0);
        scheduler_request.throttle = true;
        let mut resource = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/throttled".to_string()),
            scheduler_request: Some(scheduler_request),
            ..Default::default()
        });

        let response = resource.fetch(&backend, None).await.unwrap();
        assert_eq!(response, Response::Text("ok".to_string()));
        assert_eq!(RequestScheduler::active_requests_length(), 0);
    })
    .await;
}

#[tokio::test]
async fn fetch_returns_throttled_when_the_scheduler_rejects_the_request() {
    serial_async(async {
        // DEVIATION: JS fetch returns undefined here.
        RequestScheduler::set_maximum_requests(0);

        let backend = MockResourceBackend::new();
        let mut scheduler_request = throttled_request("https://test.invalid/rejected", 0.0);
        scheduler_request.throttle = true;
        let mut resource = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/rejected".to_string()),
            scheduler_request: Some(scheduler_request),
            ..Default::default()
        });

        let error = resource.fetch(&backend, None).await.unwrap_err();
        assert!(matches!(error, ResourceError::RequestThrottled));

        // The resource remains reusable after being rejected.
        RequestScheduler::set_maximum_requests(50);
        let mut backend = MockResourceBackend::new();
        backend.register_response("https://test.invalid/rejected", b"ok".to_vec());
        let response = resource.fetch(&backend, None).await.unwrap();
        assert_eq!(response, Response::Text("ok".to_string()));
    })
    .await;
}

#[tokio::test]
async fn fetch_is_cancelled_while_waiting_in_the_queue() {
    serial_async(async {
        // Two throttled same-server requests compete for a single
        // per-server slot; the lower priority one stays in the heap and is
        // cancelled by the scheduler during promotion (JS update():
        // `cancelRequest` when the server has no open slots).
        RequestScheduler::set_maximum_requests_per_server(1);

        let mut backend = MockResourceBackend::new();
        backend.register_response("https://test.invalid/a", b"ok".to_vec());
        backend.register_response("https://test.invalid/b", b"n/a".to_vec());

        let mut resource1 = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/a".to_string()),
            scheduler_request: Some(Request::new(
                Some("https://test.invalid/a".to_string()),
                Some(0.0),
                Some(true),
                Some(true),
                None,
                None,
            )),
            ..Default::default()
        });
        let mut resource2 = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/b".to_string()),
            scheduler_request: Some(Request::new(
                Some("https://test.invalid/b".to_string()),
                Some(1.0),
                Some(true),
                Some(true),
                None,
                None,
            )),
            ..Default::default()
        });

        let (result1, result2) = tokio::join!(
            resource1.fetch(&backend, None),
            resource2.fetch(&backend, None),
        );

        // Higher priority wins the slot...
        assert_eq!(result1.unwrap(), Response::Text("ok".to_string()));
        // ...the queued loser is cancelled by the scheduler.
        let error = result2.unwrap_err();
        match error {
            ResourceError::RequestCancelled(message) => {
                assert!(message.contains("Request cancelled"));
            }
            other => panic!("expected RequestCancelled, got {other:?}"),
        }
        assert_eq!(RequestScheduler::active_requests_length(), 0);
    })
    .await;
}

#[tokio::test]
async fn fetch_waits_for_an_open_slot_and_then_completes() {
    serial_async(async {
        // Two active slots: one taken by a blocker, the second is filled
        // after the fetch is queued, so the fetch must wait in the heap
        // until a blocker completes.
        RequestScheduler::set_maximum_requests(2);
        let mut blocker1 = immediate_request("https://blocker.invalid/y");
        assert!(RequestScheduler::request(&mut blocker1).is_some());
        let blocker1_id = blocker1.id();

        let mut backend = MockResourceBackend::new();
        backend.register_response("https://test.invalid/waits", b"done".to_vec());

        let mut scheduler_request = throttled_request("https://test.invalid/waits", 0.0);
        scheduler_request.throttle = true;
        let mut resource = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/waits".to_string()),
            scheduler_request: Some(scheduler_request),
            ..Default::default()
        });
        let request_id = resource.scheduler_request().id();

        let mut blocker2 = immediate_request("https://blocker.invalid/z");
        let (result, _) = tokio::join!(
            resource.fetch(&backend, None),
            async {
                // Fill the second slot after the fetch entered the heap, so
                // promotion is impossible for a few scheduler turns.
                assert!(RequestScheduler::request(&mut blocker2).is_some());
                for _ in 0..10 {
                    tokio::task::yield_now().await;
                    assert_eq!(
                        RequestScheduler::tracked_request_state(request_id),
                        Some(RequestState::Issued),
                        "the fetch must stay queued while both slots are full"
                    );
                }
                // Free a slot; the next update() promotes the fetch.
                RequestScheduler::complete_request_with_id(blocker1_id);
            }
        );
        assert_eq!(result.unwrap(), Response::Text("done".to_string()));
        RequestScheduler::complete_request_with_id(blocker2.id());
        assert_eq!(RequestScheduler::active_requests_length(), 0);
    })
    .await;
}

#[tokio::test]
async fn fetch_of_a_data_uri_bypasses_the_scheduler_exactly_once() {
    serial_async(async {
        let backend = MockResourceBackend::new();
        let mut resource =
            Resource::new("data:text/plain;charset=utf-8,hello".to_string());

        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sink = count.clone();
        let listener = RequestScheduler::add_request_completed_listener(move |_| {
            sink.fetch_add(1, Ordering::Relaxed);
        });

        let response = resource.fetch(&backend, None).await.unwrap();
        assert_eq!(response, Response::Text("hello".to_string()));
        // Raised once, at schedule time (JS: requestCompletedEvent in the
        // data uri branch); completion does not raise again.
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(RequestScheduler::active_requests_length(), 0);

        RequestScheduler::remove_request_completed_listener(listener);
    })
    .await;
}

#[tokio::test]
async fn fetch_failure_releases_the_slot_and_reschedules_on_retry() {
    serial_async(async {
        let backend = MockResourceBackend::new(); // no responses -> always fails
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = attempts.clone();
        let mut resource = Resource::with_options(ResourceOptions {
            url: Some("https://test.invalid/missing".to_string()),
            retry_attempts: Some(1),
            retry_callback: Some(Box::new(move |_error| {
                counter.fetch_add(1, Ordering::Relaxed);
                true
            })),
            ..Default::default()
        });

        let attempted_before =
            RequestScheduler::statistics().number_of_attempted_requests;
        let error = resource.fetch(&backend, None).await.unwrap_err();
        assert!(matches!(error, ResourceError::RequestFailed(_)));

        // Both attempts went through the scheduler and released their slots.
        assert_eq!(attempts.load(Ordering::Relaxed), 1);
        assert_eq!(
            RequestScheduler::statistics().number_of_attempted_requests - attempted_before,
            2
        );
        assert_eq!(RequestScheduler::active_requests_length(), 0);
    })
    .await;
}
