//! Ported from `packages/engine/Source/Core/RequestScheduler.js` (525 lines).
//!
//! The request scheduler is used to track and constrain the number of active
//! requests in order to prioritize incoming requests. The ability to retain
//! control over the number of requests in CesiumJS is important because due
//! to events such as changes in the camera position, a lot of new requests
//! may be generated and a lot of in-flight requests may become redundant.
//! The request scheduler manually constrains the number of requests so that
//! newer requests wait in a shorter queue and don't have to compete for
//! bandwidth with requests that have expired.
//!
//! # Method-level alignment table (JS `RequestScheduler` -> Rust)
//!
//! | CesiumJS                                  | Rust                                      |
//! | ----------------------------------------- | ----------------------------------------- |
//! | `RequestScheduler.maximumRequests`        | [`RequestScheduler::set_maximum_requests`] / [`RequestScheduler::maximum_requests`] |
//! | `RequestScheduler.maximumRequestsPerServer` | [`RequestScheduler::set_maximum_requests_per_server`] / [`RequestScheduler::maximum_requests_per_server`] |
//! | `RequestScheduler.requestsByServer`       | [`RequestScheduler::set_requests_for_server`] |
//! | `RequestScheduler.throttleRequests`       | [`RequestScheduler::set_throttle_requests`] |
//! | `RequestScheduler.statistics`             | [`RequestScheduler::statistics`]          |
//! | `RequestScheduler.priorityHeapLength`     | [`RequestScheduler::set_priority_heap_length`] |
//! | `RequestScheduler.serverHasOpenSlots`     | [`RequestScheduler::server_has_open_slots`] |
//! | `RequestScheduler.heapHasOpenSlots`       | [`RequestScheduler::heap_has_open_slots`] |
//! | `RequestScheduler.getServerKey`           | [`RequestScheduler::get_server_key`]      |
//! | `RequestScheduler.request`                | [`RequestScheduler::request`]             |
//! | `RequestScheduler.update`                 | [`RequestScheduler::update`]              |
//! | `RequestScheduler.clearForSpecs`          | [`RequestScheduler::clear_for_specs`]     |
//! | `RequestScheduler.numberOfActiveRequestsByServer` | [`RequestScheduler::number_of_active_requests_by_server`] |
//!
//! DEVIATION: JS executes `request.requestFunction()` promises; the Rust port
//! tracks the state machine / statistics / throttling decisions only (actual
//! async execution is driven by callers). `update` promotes queued requests
//! to ACTIVE without dispatching them. Caller-owned [`Request`] objects are
//! tracked by [`crate::request::Request::id`] (standing in for JS object
//! identity); state transitions are observable via
//! [`RequestScheduler::tracked_request_state`].
//! DEVIATION: JS is a stateless namespace over module globals; the Rust port
//! keeps the same global state behind a mutex with instance-style accessors,
//! plus a retained unit `RequestScheduler` struct for legacy compatibility.
//! DEVIATION: `requestCompletedEvent` is a JS `Event` (single-threaded);
//! because the scheduler is a global static shared across threads, the Rust
//! port exposes a thread-safe listener registry instead
//! ([`RequestScheduler::add_request_completed_listener`]).
//! DEVIATION: cancelling a request rejects the JS deferred promise with
//! `RuntimeError('Request cancelled: "<url>"')`; the Rust port records the
//! cancellation in the tracked-state table and callers surface it (see
//! `ResourceError::RequestCancelled`).

use std::collections::{BinaryHeap, HashMap};
use std::sync::{Mutex, OnceLock};

use crate::check;
use crate::is_blob_uri::is_blob_uri;
use crate::is_data_uri::is_data_uri;
use crate::request::{PriorityFunction, Request};
use crate::request_state::RequestState;

/// Statistics used by the request scheduler.
///
/// Mirrors the private `statistics` object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchedulerStatistics {
    /// Number of attempted requests.
    pub number_of_attempted_requests: u64,
    /// Number of currently active requests.
    pub number_of_active_requests: u64,
    /// Number of cancelled requests.
    pub number_of_cancelled_requests: u64,
    /// Number of cancelled active requests.
    pub number_of_cancelled_active_requests: u64,
    /// Number of failed requests.
    pub number_of_failed_requests: u64,
    /// Total number of requests ever made active.
    pub number_of_active_requests_ever: u64,
    /// Number of active requests at the previous update.
    pub last_number_of_active_requests: u64,
}

struct SchedulerState {
    statistics: SchedulerStatistics,
    priority_heap_length: usize,
    // BinaryHeap is a max-heap; JS heap keeps the LOWEST priority on top, so
    // invert the ordering.
    request_heap: BinaryHeap<HeapEntry>,
    active_requests: Vec<HeapEntry>,
    number_of_active_requests_by_server: HashMap<String, u64>,
    maximum_requests: usize,
    maximum_requests_per_server: usize,
    requests_by_server: HashMap<String, usize>,
    throttle_requests: bool,
    /// Scheduler-observable state of every request it has seen, keyed by
    /// [`crate::request::Request::id`] (Rust stand-in for the JS shared
    /// request object whose `state` field is mutated in place).
    tracked: HashMap<u64, RequestState>,
}

/// An entry in the priority heap (owned snapshot of a [`Request`]).
struct HeapEntry {
    id: u64,
    priority: f64,
    url: Option<String>,
    throttle_by_server: bool,
    server_key: Option<String>,
    cancelled: bool,
    priority_function: Option<PriorityFunction>,
    /// Whether this entry was started (mirrors JS `request.state === ACTIVE`,
    /// used by [`cancel_request_entry`] to account statistics).
    active: bool,
}

impl Clone for HeapEntry {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            priority: self.priority,
            url: self.url.clone(),
            throttle_by_server: self.throttle_by_server,
            server_key: self.server_key.clone(),
            cancelled: self.cancelled,
            priority_function: self.priority_function.clone(),
            active: self.active,
        }
    }
}

impl Eq for HeapEntry {}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    /// Inverted: lowest JS priority must come out FIRST, so order the
    /// max-heap by reversed priority.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            statistics: SchedulerStatistics::default(),
            priority_heap_length: 20,
            request_heap: BinaryHeap::new(),
            active_requests: Vec::new(),
            number_of_active_requests_by_server: HashMap::new(),
            maximum_requests: 50,
            maximum_requests_per_server: 18,
            requests_by_server: HashMap::new(),
            throttle_requests: true,
            tracked: HashMap::new(),
        }
    }
}

fn state() -> &'static Mutex<SchedulerState> {
    static STATE: OnceLock<Mutex<SchedulerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SchedulerState::default()))
}

// ── requestCompletedEvent (thread-safe listener registry) ────────────

/// Payload passed to `requestCompletedEvent` listeners: `None` mirrors JS
/// `raiseEvent()` on success, `Some(message)` mirrors `raiseEvent(error)`.
type CompletedListener = Box<dyn FnMut(Option<String>) + Send>;

struct CompletedListenerEntry {
    id: u64,
    listener: CompletedListener,
}

struct CompletedEvent {
    listeners: Vec<CompletedListenerEntry>,
    next_id: u64,
}

fn completed_event() -> &'static Mutex<CompletedEvent> {
    static EVENT: OnceLock<Mutex<CompletedEvent>> = OnceLock::new();
    EVENT.get_or_init(|| {
        Mutex::new(CompletedEvent {
            listeners: Vec::new(),
            next_id: 1,
        })
    })
}

fn raise_request_completed_event(error: Option<String>) {
    // DEVIATION: JS `Event.raiseEvent` supports listener reentrancy on the
    // same thread; here the listener list is taken out of the global so
    // listeners may add/remove listeners (or call back into the scheduler)
    // without deadlocking, then restored. Listeners added during a raise
    // are appended after the pre-existing ones.
    let mut entries = {
        let mut event = completed_event().lock().unwrap();
        std::mem::take(&mut event.listeners)
    };
    for entry in entries.iter_mut() {
        (entry.listener)(error.clone());
    }
    completed_event().lock().unwrap().listeners.extend(entries);
}

/// Schedules and prioritizes network requests.
///
/// Mirrors the `RequestScheduler` namespace (all methods are global/static).
pub struct RequestScheduler {
    _private: (),
}

impl RequestScheduler {
    /// Creates a new RequestScheduler (legacy compatibility; the scheduler
    /// itself is global).
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// The maximum number of simultaneous active requests. Un-throttled
    /// requests do not observe this limit. Default 50.
    pub fn maximum_requests() -> usize {
        state().lock().unwrap().maximum_requests
    }

    /// Sets [`Self::maximum_requests`].
    pub fn set_maximum_requests(value: usize) {
        state().lock().unwrap().maximum_requests = value;
    }

    /// The maximum number of simultaneous active requests per server.
    /// Default 18.
    pub fn maximum_requests_per_server() -> usize {
        state().lock().unwrap().maximum_requests_per_server
    }

    /// Sets [`Self::maximum_requests_per_server`].
    pub fn set_maximum_requests_per_server(value: usize) {
        state().lock().unwrap().maximum_requests_per_server = value;
    }

    /// A per server key override for throttling instead of
    /// `maximumRequestsPerServer`.
    pub fn set_requests_for_server(server_key: &str, max: usize) {
        state()
            .lock()
            .unwrap()
            .requests_by_server
            .insert(server_key.to_string(), max);
    }

    /// Specifies if the request scheduler should throttle incoming requests.
    /// Default true.
    pub fn throttle_requests() -> bool {
        state().lock().unwrap().throttle_requests
    }

    /// Sets [`Self::throttle_requests`].
    pub fn set_throttle_requests(value: bool) {
        state().lock().unwrap().throttle_requests = value;
    }

    /// Returns the statistics used by the request scheduler.
    pub fn statistics() -> SchedulerStatistics {
        state().lock().unwrap().statistics
    }

    /// The maximum size of the priority heap. This limits the number of
    /// requests that are sorted by priority. Default 20.
    pub fn priority_heap_length() -> usize {
        state().lock().unwrap().priority_heap_length
    }

    /// Sets [`Self::priority_heap_length`]; shrinking cancels the lowest
    /// priority queued requests (JS behavior).
    pub fn set_priority_heap_length(value: usize) {
        let mut state = state().lock().unwrap();
        // If the new length shrinks the heap, need to cancel some of the
        // requests.
        while state.request_heap.len() > value {
            let request = state.request_heap.pop().unwrap();
            cancel_request_entry(&mut state, request);
        }
        state.priority_heap_length = value;
    }

    /// Check if there are open slots for a particular server key. If
    /// `desired_requests` is greater than 1, this checks if the queue has
    /// room for scheduling multiple requests.
    ///
    /// Mirrors `RequestScheduler.serverHasOpenSlots(serverKey, desiredRequests)`.
    pub fn server_has_open_slots(server_key: &str, desired_requests: Option<usize>) -> bool {
        let desired_requests = desired_requests.unwrap_or(1);
        let state = state().lock().unwrap();
        let max_requests = *state
            .requests_by_server
            .get(server_key)
            .unwrap_or(&state.maximum_requests_per_server);
        let active = state
            .number_of_active_requests_by_server
            .get(server_key)
            .copied()
            .unwrap_or(0) as usize;
        active + desired_requests <= max_requests
    }

    /// Check if the priority heap has open slots, regardless of which server
    /// they are from.
    ///
    /// Mirrors `RequestScheduler.heapHasOpenSlots(desiredRequests)`.
    pub fn heap_has_open_slots(desired_requests: usize) -> bool {
        let state = state().lock().unwrap();
        state.request_heap.len() + desired_requests <= state.priority_heap_length
    }

    /// Get the server key from a given url.
    ///
    /// Mirrors `RequestScheduler.getServerKey(url)`.
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `url` is empty.
    pub fn get_server_key(url: &str) -> String {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::string("url", Some(url));
            if url.is_empty() {
                crate::developer_error::throw_developer_error("url is required.");
            }
        }
        //>>includeEnd('debug');

        let parsed = url::Url::parse(url);
        let server_key = match parsed.as_ref().ok().and_then(|u| u.host_str()) {
            Some(host) => {
                let port = parsed
                    .as_ref()
                    .unwrap()
                    .port_or_known_default()
                    .unwrap_or(80);
                format!("{host}:{port}")
            }
            // DEVIATION: urijs resolves scheme-less urls against the page uri;
            // the native port uses the raw string with the default port.
            None => format!("{url}:80"),
        };

        let mut state = state().lock().unwrap();
        state
            .number_of_active_requests_by_server
            .entry(server_key.clone())
            .or_insert(0);

        server_key
    }

    /// Issue a request. If `request.throttle` is false, the request is sent
    /// immediately. Otherwise the request will be queued and sorted by
    /// priority before being sent.
    ///
    /// Mirrors `RequestScheduler.request(request)`: returns `Some` when the
    /// request was accepted (started or queued) and `None` when it does not
    /// have high enough priority / there is no capacity (JS `undefined`).
    /// The request's `state` and `server_key` are updated in place.
    ///
    /// DEVIATION: JS debug-checks `request.requestFunction` too; the Rust
    /// port has no request function (execution is caller-driven).
    pub fn request(request: &mut Request) -> Option<()> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::string("request.url", request.url.as_deref());
        }
        //>>includeEnd('debug');

        let url = request.url.clone().unwrap_or_default();
        if is_data_uri(Some(&url)) || is_blob_uri(Some(&url)) {
            // Skip the scheduler for data/blob uris.
            raise_request_completed_event(None);
            request.state = RequestState::Received;
            return Some(());
        }

        let mut state = state().lock().unwrap();
        state.statistics.number_of_attempted_requests += 1;

        if request.server_key.is_none() {
            request.server_key = Some(get_server_key_inner(&mut state, &url));
        }
        let server_key = request.server_key.clone().unwrap_or_default();

        if state.throttle_requests
            && request.throttle_by_server
            && !server_has_open_slots_inner(&state, &server_key, 1)
        {
            // Server is saturated. Try again later.
            return None;
        }

        if !state.throttle_requests || !request.throttle {
            start_request_inner(&mut state, request, &server_key);
            return Some(());
        }

        if state.active_requests.len() >= state.maximum_requests {
            // Active requests are saturated. Try again later.
            return None;
        }

        // updatePriority(request)
        if let Some(priority_function) = request.priority_function().cloned() {
            request.priority = priority_function.lock().unwrap()();
        }

        // Insert into the priority heap and see if a request was bumped off.
        // If this request is the lowest priority it will be returned.
        let entry = HeapEntry {
            id: request.id(),
            priority: request.priority,
            url: request.url.clone(),
            throttle_by_server: request.throttle_by_server,
            server_key: request.server_key.clone(),
            cancelled: false,
            priority_function: request.priority_function().cloned(),
            active: false,
        };

        let removed = if state.request_heap.len() >= state.priority_heap_length {
            // Mirror Heap.insert with maximumLength: only keep the best N.
            let worst = state
                .request_heap
                .peek()
                .map(|w| w.priority)
                .unwrap_or(f64::INFINITY);
            if entry.priority > worst {
                // New request is worse than the current worst: it is bumped.
                Some(entry)
            } else {
                let removed = state.request_heap.pop();
                state.request_heap.push(entry);
                removed
            }
        } else {
            state.request_heap.push(entry);
            None
        };

        if let Some(removed_request) = removed {
            if removed_request.id == request.id() {
                // Request does not have high enough priority to be issued
                return None;
            }
            // A previously issued request has been bumped off the priority
            // heap, so cancel it
            cancel_request_entry(&mut state, removed_request);
        }

        state.tracked.insert(request.id(), RequestState::Issued);

        // issueRequest(request)
        if request.state == RequestState::Unissued {
            request.state = RequestState::Issued;
        }
        Some(())
    }

    /// Sort requests by priority and start requests.
    ///
    /// Mirrors `RequestScheduler.update()`. DEVIATION: promotion to ACTIVE
    /// only updates the state machine; no async dispatch happens.
    pub fn update() {
        let mut state = state().lock().unwrap();

        // Loop over all active requests. Cancelled requests are removed
        // from the array to make room for new requests (JS also removes
        // non-ACTIVE states; the Rust state machine resolves entries via
        // `complete_request`/`fail_request` instead).
        let mut i = 0;
        while i < state.active_requests.len() {
            if state.active_requests[i].cancelled {
                let entry = state.active_requests.remove(i);
                cancel_request_entry(&mut state, entry);
            } else {
                i += 1;
            }
        }

        // Update priority of issued requests and resort the heap.
        let mut issued: Vec<HeapEntry> = state.request_heap.drain().collect();
        for entry in issued.iter_mut() {
            if let Some(priority_function) = &entry.priority_function {
                entry.priority = priority_function.lock().unwrap()();
            }
        }
        state.request_heap.extend(issued);

        // Get the number of open slots and fill with the highest priority
        // requests.
        let open_slots = state
            .maximum_requests
            .saturating_sub(state.active_requests.len());
        let mut filled_slots = 0;
        while filled_slots < open_slots && !state.request_heap.is_empty() {
            let mut request = state.request_heap.pop().unwrap();
            if request.cancelled {
                cancel_request_entry(&mut state, request);
                continue;
            }

            let server_key = request.server_key.clone().unwrap_or_default();
            if request.throttle_by_server
                && !server_has_open_slots_inner(&state, &server_key, 1)
            {
                // Open slots are available, but the request is throttled by
                // its server. Cancel and try again later.
                cancel_request_entry(&mut state, request);
                continue;
            }

            // startRequest: mark active and account statistics.
            request.active = true;
            state.tracked.insert(request.id, RequestState::Active);
            state.statistics.number_of_active_requests += 1;
            state.statistics.number_of_active_requests_ever += 1;
            *state
                .number_of_active_requests_by_server
                .entry(server_key)
                .or_insert(0) += 1;
            state.active_requests.push(request);
            filled_slots += 1;
        }

        update_statistics(&mut state);
    }

    /// Marks a request as cancelled by its [`crate::request::Request::id`].
    ///
    /// Rust stand-in for JS `request.cancel()` (the scheduler holds owned
    /// snapshots, so the flag is applied to the tracked copy); the
    /// cancellation takes effect on the next [`RequestScheduler::update`],
    /// mirroring JS.
    pub fn cancel_request(id: u64) {
        let mut state = state().lock().unwrap();
        // BinaryHeap has no mutable iteration; drain, flag, and rebuild.
        let mut entries: Vec<HeapEntry> = state.request_heap.drain().collect();
        for entry in entries.iter_mut() {
            if entry.id == id {
                entry.cancelled = true;
            }
        }
        state.request_heap.extend(entries);
        for entry in state.active_requests.iter_mut() {
            if entry.id == id {
                entry.cancelled = true;
            }
        }
    }

    /// The scheduler-observable state of a request, by
    /// [`crate::request::Request::id`]. `None` if the scheduler never saw
    /// the request (or it was cleared).
    ///
    /// DEVIATION: Rust stand-in for reading the shared JS `request.state`.
    pub fn tracked_request_state(id: u64) -> Option<RequestState> {
        state().lock().unwrap().tracked.get(&id).copied()
    }

    /// Marks a tracked request as received (successful completion) by its
    /// [`crate::request::Request::id`], freeing its slot and raising
    /// `requestCompletedEvent`.
    ///
    /// DEVIATION: Rust-side helper standing in for the JS
    /// `getRequestReceivedFunction` closure resolution.
    pub fn complete_request_with_id(id: u64) {
        let mut state = state().lock().unwrap();
        if let Some(pos) = state.active_requests.iter().position(|e| e.id == id) {
            let entry = state.active_requests.remove(pos);
            state.statistics.number_of_active_requests =
                state.statistics.number_of_active_requests.saturating_sub(1);
            if let Some(server_key) = &entry.server_key {
                if let Some(count) = state
                    .number_of_active_requests_by_server
                    .get_mut(server_key)
                {
                    *count = count.saturating_sub(1);
                }
            }
            state.tracked.insert(id, RequestState::Received);
            drop(state);
            raise_request_completed_event(None);
        }
    }

    /// Marks a tracked request as failed by its
    /// [`crate::request::Request::id`], raising `requestCompletedEvent`
    /// with the error.
    ///
    /// DEVIATION: Rust-side helper standing in for the JS
    /// `getRequestFailedFunction` closure resolution.
    pub fn fail_request_with_id(id: u64, error: &str) {
        let mut state = state().lock().unwrap();
        if let Some(pos) = state.active_requests.iter().position(|e| e.id == id) {
            let entry = state.active_requests.remove(pos);
            state.statistics.number_of_failed_requests += 1;
            state.statistics.number_of_active_requests =
                state.statistics.number_of_active_requests.saturating_sub(1);
            if let Some(server_key) = &entry.server_key {
                if let Some(count) = state
                    .number_of_active_requests_by_server
                    .get_mut(server_key)
                {
                    *count = count.saturating_sub(1);
                }
            }
            state.tracked.insert(id, RequestState::Failed);
            drop(state);
            raise_request_completed_event(Some(error.to_string()));
        }
    }

    /// Marks a tracked active request as received (successful completion),
    /// freeing its slot.
    ///
    /// DEVIATION: Rust-side helper standing in for the JS
    /// `getRequestReceivedFunction` closure resolution.
    pub fn complete_request(url: &str) {
        let id = state()
            .lock()
            .unwrap()
            .active_requests
            .iter()
            .find(|e| e.url.as_deref() == Some(url))
            .map(|e| e.id);
        if let Some(id) = id {
            Self::complete_request_with_id(id);
        }
    }

    /// Marks a tracked active request as failed.
    ///
    /// DEVIATION: Rust-side helper standing in for the JS
    /// `getRequestFailedFunction` closure resolution.
    pub fn fail_request(url: &str) {
        let id = state()
            .lock()
            .unwrap()
            .active_requests
            .iter()
            .find(|e| e.url.as_deref() == Some(url))
            .map(|e| e.id);
        if let Some(id) = id {
            Self::fail_request_with_id(id, "");
        }
    }

    // ── requestCompletedEvent ────────────────────────────────────────

    /// Registers a listener raised whenever a request completes (with
    /// `Some(error)` when it failed, `None` on success).
    ///
    /// Mirrors `RequestScheduler.requestCompletedEvent.addEventListener`.
    /// Returns the listener id used by
    /// [`RequestScheduler::remove_request_completed_listener`].
    pub fn add_request_completed_listener(
        listener: impl FnMut(Option<String>) + Send + 'static,
    ) -> u64 {
        let mut event = completed_event().lock().unwrap();
        let id = event.next_id;
        event.next_id += 1;
        event.listeners.push(CompletedListenerEntry {
            id,
            listener: Box::new(listener),
        });
        id
    }

    /// Removes a previously registered `requestCompletedEvent` listener.
    ///
    /// Mirrors invoking the `removeCallback` returned by `addEventListener`.
    pub fn remove_request_completed_listener(id: u64) -> bool {
        let mut event = completed_event().lock().unwrap();
        if let Some(pos) = event.listeners.iter().position(|e| e.id == id) {
            event.listeners.remove(pos);
            true
        } else {
            false
        }
    }

    /// Number of `requestCompletedEvent` listeners.
    ///
    /// Mirrors `requestCompletedEvent.numberOfListeners`.
    pub fn number_of_request_completed_listeners() -> usize {
        completed_event().lock().unwrap().listeners.len()
    }

    /// For testing only. Clears any requests that may not have completed
    /// from previous tests.
    ///
    /// Mirrors `RequestScheduler.clearForSpecs()`.
    pub fn clear_for_specs() {
        let mut state = state().lock().unwrap();
        while let Some(request) = state.request_heap.pop() {
            cancel_request_entry(&mut state, request);
        }
        let active = std::mem::take(&mut state.active_requests);
        for request in active {
            cancel_request_entry(&mut state, request);
        }
        state.number_of_active_requests_by_server.clear();
        state.tracked.clear();

        // Clear stats
        state.statistics = SchedulerStatistics::default();
    }

    /// For testing only. Clears the per-server throttling overrides.
    ///
    /// Mirrors `RequestScheduler.requestsByServer = {}` in the spec
    /// `beforeEach`.
    pub fn clear_requests_by_server_for_specs() {
        state().lock().unwrap().requests_by_server.clear();
    }

    /// For testing only: number of active requests for a server key.
    ///
    /// Mirrors `RequestScheduler.numberOfActiveRequestsByServer(serverKey)`.
    pub fn number_of_active_requests_by_server(server_key: &str) -> u64 {
        state()
            .lock()
            .unwrap()
            .number_of_active_requests_by_server
            .get(server_key)
            .copied()
            .unwrap_or(0)
    }

    /// For testing only: number of requests currently in the priority heap.
    pub fn request_heap_length() -> usize {
        state().lock().unwrap().request_heap.len()
    }

    /// For testing only: priorities of the queued heap requests in the
    /// order they would be popped (best first), without mutating the heap.
    ///
    /// Rust stand-in for popping `RequestScheduler.requestHeap` in specs.
    pub fn request_heap_pop_order_for_specs() -> Vec<f64> {
        let mut heap = state().lock().unwrap().request_heap.clone();
        let mut priorities = Vec::with_capacity(heap.len());
        while let Some(entry) = heap.pop() {
            priorities.push(entry.priority);
        }
        priorities
    }

    /// For testing only: number of active requests.
    pub fn active_requests_length() -> usize {
        state().lock().unwrap().active_requests.len()
    }
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new()
    }
}

fn get_server_key_inner(state: &mut SchedulerState, url: &str) -> String {
    let parsed = url::Url::parse(url);
    let server_key = match parsed.as_ref().ok().and_then(|u| u.host_str()) {
        Some(host) => {
            let port = parsed
                .as_ref()
                .unwrap()
                .port_or_known_default()
                .unwrap_or(80);
            format!("{host}:{port}")
        }
        None => format!("{url}:80"),
    };
    state
        .number_of_active_requests_by_server
        .entry(server_key.clone())
        .or_insert(0);
    server_key
}

fn server_has_open_slots_inner(state: &SchedulerState, server_key: &str, desired: usize) -> bool {
    let max_requests = *state
        .requests_by_server
        .get(server_key)
        .unwrap_or(&state.maximum_requests_per_server);
    let active = state
        .number_of_active_requests_by_server
        .get(server_key)
        .copied()
        .unwrap_or(0) as usize;
    active + desired <= max_requests
}

fn start_request_inner(state: &mut SchedulerState, request: &mut Request, server_key: &str) {
    // issueRequest(request)
    if request.state == RequestState::Unissued {
        request.state = RequestState::Issued;
    }
    request.state = RequestState::Active;
    state.tracked.insert(request.id(), RequestState::Active);
    state.statistics.number_of_active_requests += 1;
    state.statistics.number_of_active_requests_ever += 1;
    *state
        .number_of_active_requests_by_server
        .entry(server_key.to_string())
        .or_insert(0) += 1;
    state.active_requests.push(HeapEntry {
        id: request.id(),
        priority: request.priority,
        url: request.url.clone(),
        throttle_by_server: request.throttle_by_server,
        server_key: request.server_key.clone(),
        cancelled: false,
        priority_function: request.priority_function().cloned(),
        active: true,
    });
    // DEVIATION: JS calls request.requestFunction() here.
}

fn cancel_request_entry(state: &mut SchedulerState, request: HeapEntry) {
    // JS cancelRequest checks `request.state === RequestState.ACTIVE`; the
    // Rust entry carries that flag directly.
    let active = request.active;
    state.statistics.number_of_cancelled_requests += 1;
    state.tracked.insert(request.id, RequestState::Cancelled);

    if active {
        state.statistics.number_of_active_requests =
            state.statistics.number_of_active_requests.saturating_sub(1);
        if let Some(server_key) = &request.server_key {
            if let Some(count) = state.number_of_active_requests_by_server.get_mut(server_key) {
                *count = count.saturating_sub(1);
            }
        }
        state.statistics.number_of_cancelled_active_requests += 1;
    }
    // DEVIATION: JS rejects the deferred promise with
    // RuntimeError(`Request cancelled: "${request.url}"`) and calls
    // `request.cancelFunction()`; callers observe the Cancelled tracked
    // state instead.
}

fn update_statistics(state: &mut SchedulerState) {
    // DEVIATION: debugShowStatistics console logging is not ported.
    state.statistics.last_number_of_active_requests = state.statistics.number_of_active_requests;
}
