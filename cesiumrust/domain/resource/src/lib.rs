//! cesium-resource: Resource management and request scheduling.
//!
//! Domain layer — pure Rust, no framework dependency, no network IO.
//!
//! CesiumJS mapping:
//! - `packages/engine/Source/Core/Resource.js` (2281 lines)
//! - `packages/engine/Source/Core/RequestScheduler.js` (525 lines)
//! - `packages/engine/Source/Core/Request.js`
//! - `packages/engine/Source/Core/DefaultProxy.js`
//! - `packages/engine/Source/Core/IonResource.js`
//!
//! # Architecture
//!
//! This crate provides the **pure domain logic** for resource management:
//! URL construction, query parameter handling, proxy policy, data URI decoding,
//! Ion endpoint building, request scheduling with priority/throttling, retry
//! semantics, and statistics aggregation.
//!
//! Actual HTTP IO is NOT performed here — the `Resource::build_fetch_descriptor`
//! family produces [`FetchDescriptor`] values that the adapter layer
//! (`adapters/network`) executes. This separation ensures the domain is fully
//! testable without network access or async runtimes.
//!
//! # Module layout
//!
//! | Module | Responsibility | CesiumJS mapping |
//! |--------|---------------|------------------|
//! | `lib.rs` | Resource, Request, RequestScheduler, FetchDescriptor | Resource.js + RequestScheduler.js |
//! | `proxy.rs` | DefaultProxy, ProxyPolicy + trusted-server gating | DefaultProxy.js |
//! | `data_uri.rs` | data: URI parsing/decoding (base64 + percent) | Resource.js dataUriRegex |
//! | `ion.rs` | Ion asset endpoint URL/header construction | IonResource.js + Ion.js |
//! | `statistics.rs` | RequestStatistics aggregation | RequestScheduler.statistics |
//! | `priority.rs` | PriorityFunction trait + SSED/distance impls | Request.priorityFunction |
//! | `trusted_servers.rs` | TrustedServers registry | TrustedServers.js |

pub mod data_uri;
pub mod ion;
pub mod priority;
pub mod proxy;
pub mod statistics;
pub mod trusted_servers;

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

use crate::priority::{FrameContext, PriorityFunction, PriorityKey};
use crate::proxy::ProxyPolicy;
use crate::statistics::RequestStatistics;

/// The type of request.
/// Maps to CesiumJS `RequestType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RequestType {
    /// Terrain request.
    Terrain,
    /// Imagery request.
    Imagery,
    /// 3D Tiles request.
    Tiles3D,
    /// Other request type.
    #[default]
    Other,
}

/// The state of a request.
/// Maps to CesiumJS `RequestState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RequestState {
    /// Initial state, not yet issued.
    #[default]
    Unissued,
    /// Issued but not yet active.
    Issued,
    /// Actively being processed.
    Active,
    /// Received response, processing.
    Received,
    /// Request failed.
    Failed,
    /// Request was cancelled.
    Cancelled,
}

/// A unique identifier for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// Stores information for making a request.
/// Maps to CesiumJS `Request`
#[derive(Debug, Clone)]
pub struct Request {
    /// Unique identifier.
    pub id: RequestId,
    /// The URL to request.
    pub url: String,
    /// Priority (lower = higher priority).
    pub priority: f64,
    /// Whether to throttle and prioritize the request.
    pub throttle: bool,
    /// Whether to throttle by server.
    pub throttle_by_server: bool,
    /// Type of request.
    pub request_type: RequestType,
    /// Current state.
    pub state: RequestState,
    /// Server key for throttling.
    pub server_key: String,
    /// Optional spatial key consumed by the scheduler's [`PriorityFunction`]
    /// to recompute `priority` each frame (see `update_with_context`).
    ///
    /// Maps to the tile/geometry data captured by a CesiumJS
    /// `request.priorityFunction` closure.
    pub priority_key: Option<PriorityKey>,
}

impl Request {
    /// Creates a new request.
    pub fn new(url: String, request_type: RequestType) -> Self {
        let server_key = extract_server_key(&url);
        Self {
            id: RequestId(0),
            url,
            priority: 0.0,
            throttle: false,
            throttle_by_server: false,
            request_type,
            state: RequestState::Unissued,
            server_key,
            priority_key: None,
        }
    }

    /// Creates a throttled request.
    pub fn throttled(url: String, request_type: RequestType, priority: f64) -> Self {
        let server_key = extract_server_key(&url);
        Self {
            id: RequestId(0),
            url,
            priority,
            throttle: true,
            throttle_by_server: true,
            request_type,
            state: RequestState::Unissued,
            server_key,
            priority_key: None,
        }
    }

    /// Attaches a spatial [`PriorityKey`] so the scheduler's priority function
    /// can recompute this request's priority from frame state.
    pub fn with_priority_key(mut self, key: PriorityKey) -> Self {
        self.priority_key = Some(key);
        self
    }
}

/// Wrapper for priority queue ordering (min-heap by priority).
#[derive(Debug, Clone)]
struct PrioritizedRequest {
    id: RequestId,
    priority: f64,
}

impl PartialEq for PrioritizedRequest {
    fn eq(&self, other: &Self) -> bool {
        // M1 review fix: use `total_cmp` so `eq` agrees with `Ord::cmp` even
        // for NaN priorities. The pre-fix impl used `==` here (false for NaN)
        // while `cmp` used `partial_cmp().unwrap_or(Equal)` (Equal for NaN),
        // violating the `Ord`/`Eq` consistency law `a.cmp(b) == Equal <=> a == b`.
        self.priority.total_cmp(&other.priority) == Ordering::Equal
    }
}

impl Eq for PrioritizedRequest {}

impl PartialOrd for PrioritizedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lower priority value = higher
        // priority). M1 review fix: `total_cmp` gives NaN a deterministic
        // position in the total order (positive NaN sorts above `+inf`), so a
        // NaN-priority request naturally sinks to the bottom of the min-heap
        // and can never corrupt the `BinaryHeap` invariant. The pre-fix
        // `partial_cmp().unwrap_or(Equal)` made NaN compare `Equal` to every
        // value, breaking the heap's total-order contract.
        other.priority.total_cmp(&self.priority)
    }
}

/// Clamps a priority value for safe heap insertion (M1 review fix).
///
/// A NaN priority — reachable in practice when a composite priority sums
/// `f64::MAX`-magnitude components to `inf` and then `inf + (-inf)` — would
/// violate the `BinaryHeap` total-order invariant. Mapping NaN to `f64::MAX`
/// sinks such a request to the *bottom* of the min-heap (lowest priority) so
/// it never disturbs the ordering of well-formed priorities. Finite values
/// (including `±inf`) pass through unchanged; combined with the `total_cmp`
/// `Ord` impl this is defence-in-depth for the NaN path.
fn sanitize_priority(priority: f64) -> f64 {
    if priority.is_nan() {
        f64::MAX
    } else {
        priority
    }
}

/// Manages request throttling and prioritization.
/// Maps to CesiumJS `RequestScheduler`
#[derive(Debug)]
pub struct RequestScheduler {
    /// Maximum number of simultaneous active requests.
    pub maximum_requests: usize,
    /// Maximum number of simultaneous active requests per server.
    pub maximum_requests_per_server: usize,
    /// Per-server overrides for max requests.
    pub requests_by_server: HashMap<String, usize>,
    /// Whether to throttle requests.
    pub throttle_requests: bool,
    /// Maximum size of the priority heap.
    pub priority_heap_length: usize,
    /// Maximum number of deferred requests retained when the priority heap is
    /// saturated. Deferred requests are re-promoted into the heap once slots
    /// free up (maps to CesiumJS re-requesting throttled tiles next frame).
    pub maximum_deferred: usize,

    // Internal state
    active_requests: HashMap<RequestId, Request>,
    pending_heap: BinaryHeap<PrioritizedRequest>,
    active_count_by_server: HashMap<String, usize>,
    next_id: u64,

    /// Aggregate request statistics (attempted/active/succeeded/failed/cancelled
    /// + per-server + per-type). Maps to CesiumJS `RequestScheduler.statistics`.
    statistics: RequestStatistics,

    /// Optional pluggable priority function. When set, `update_with_context`
    /// recomputes each pending request's priority from frame state before
    /// promotion. Maps to CesiumJS `Request.priorityFunction`.
    priority_function: Option<Box<dyn PriorityFunction>>,

    /// Requests rejected from a saturated priority heap, retained for later
    /// promotion. Ordered implicitly; re-inserted by priority when the heap
    /// has open slots again.
    deferred: Vec<Request>,
}

impl RequestScheduler {
    /// Creates a new RequestScheduler with default settings.
    pub fn new() -> Self {
        Self {
            maximum_requests: 50,
            maximum_requests_per_server: 18,
            requests_by_server: HashMap::new(),
            throttle_requests: true,
            priority_heap_length: 20,
            maximum_deferred: 64,
            active_requests: HashMap::new(),
            pending_heap: BinaryHeap::new(),
            active_count_by_server: HashMap::new(),
            next_id: 0,
            statistics: RequestStatistics::new(),
            priority_function: None,
            deferred: Vec::new(),
        }
    }

    /// Returns a shared reference to the aggregate statistics.
    ///
    /// Maps to CesiumJS `RequestScheduler.statistics` (exposed for diagnostics).
    pub fn statistics(&self) -> &RequestStatistics {
        &self.statistics
    }

    /// Returns a mutable reference to the aggregate statistics.
    pub fn statistics_mut(&mut self) -> &mut RequestStatistics {
        &mut self.statistics
    }

    /// Resets the aggregate statistics to zero.
    ///
    /// Maps to `RequestScheduler.clearForSpecs()` statistics reset.
    pub fn reset_statistics(&mut self) {
        self.statistics.reset();
    }

    /// Installs a pluggable priority function.
    ///
    /// Once set, [`RequestScheduler::update_with_context`] recomputes each
    /// pending request's priority from the frame context before promotion,
    /// mirroring CesiumJS's per-frame `priorityFunction` re-sort.
    pub fn set_priority_function(&mut self, f: Box<dyn PriorityFunction>) {
        self.priority_function = Some(f);
    }

    /// Returns the name of the installed priority function, if any.
    pub fn priority_function_name(&self) -> Option<&str> {
        self.priority_function.as_ref().map(|f| f.name())
    }

    /// Returns the number of currently deferred (heap-rejected) requests.
    pub fn deferred_count(&self) -> usize {
        self.deferred.len()
    }

    /// Returns the number of active requests.
    /// Maps to CesiumJS `RequestScheduler.statistics.numberOfActiveRequests`
    pub fn active_request_count(&self) -> usize {
        self.active_count_by_server.values().sum()
    }

    /// Returns the number of pending requests.
    pub fn pending_request_count(&self) -> usize {
        self.pending_heap.len()
    }

    /// Checks if a server has open slots for more requests.
    /// Maps to `RequestScheduler.serverHasOpenSlots`
    pub fn server_has_open_slots(&self, server_key: &str, desired_requests: usize) -> bool {
        let max_requests = self
            .requests_by_server
            .get(server_key)
            .copied()
            .unwrap_or(self.maximum_requests_per_server);
        let current = self.active_count_by_server.get(server_key).copied().unwrap_or(0);
        current + desired_requests <= max_requests
    }

    /// Checks if the priority heap has open slots.
    /// Maps to `RequestScheduler.heapHasOpenSlots`
    pub fn heap_has_open_slots(&self, desired_requests: usize) -> bool {
        self.pending_heap.len() + desired_requests <= self.priority_heap_length
    }

    /// Schedules a request. Returns the request ID if accepted.
    ///
    /// Maps to `RequestScheduler.request`. When throttling is enabled and the
    /// request cannot be activated immediately, it is placed in the priority
    /// heap. If the heap is saturated, the request is *deferred* (retained for
    /// later promotion) rather than dropped, up to [`Self::maximum_deferred`].
    pub fn schedule(&mut self, mut request: Request) -> Option<RequestId> {
        // Assign ID
        let id = RequestId(self.next_id);
        self.next_id += 1;
        request.id = id;
        self.statistics.on_scheduled();

        // If not throttling, immediately activate
        if !self.throttle_requests || !request.throttle {
            self.activate_request(request);
            return Some(id);
        }

        // Check if we can activate immediately
        if self.can_activate(&request) {
            self.activate_request(request);
            return Some(id);
        }

        // Add to pending heap if there's room
        if self.pending_heap.len() < self.priority_heap_length {
            request.state = RequestState::Issued;
            self.pending_heap.push(PrioritizedRequest {
                id,
                // M1 review fix: guard against a NaN priority corrupting the
                // heap's total order (NaN sinks to the bottom as `f64::MAX`).
                priority: sanitize_priority(request.priority),
            });
            self.active_requests.insert(id, request);
            Some(id)
        } else {
            // Heap saturated — apply the CesiumJS `RequestScheduler.request`
            // priority-rejection rule (`packages/engine/Source/Core/RequestScheduler.js`):
            // the internal `PriorityQueue.insert` returns `false` when the new
            // item's priority is **not better** than the worst resident item,
            // in which case the request is rejected outright. Only when the
            // new item strictly outranks the worst resident do we evict the
            // worst (into the M8.1 deferred queue, if it has room) and admit
            // the newcomer.
            //
            // `pending_heap` is a min-heap by priority value (lower value =
            // higher priority, see `PrioritizedRequest::cmp`), so the *worst*
            // resident is the entry with the **maximum** `priority` field.
            let worst_priority = self
                .pending_heap
                .iter()
                .map(|p| p.priority)
                .fold(f64::NEG_INFINITY, f64::max);

            if request.priority.partial_cmp(&worst_priority) != Some(Ordering::Less) {
                // New request is not strictly better than the worst resident
                // (covers equal-priority, worse-priority, and NaN-incomparable
                // cases). Reject and account for it — this is the
                // `heapHasOpenSlots == false` branch of the CesiumJS spec.
                self.statistics.on_cancelled_pending();
                return None;
            }

            // New request strictly outranks the worst resident: evict the
            // worst, admit the newcomer. `BinaryHeap` has no remove-by-value,
            // so drain + rebuild (heap sizes are bounded by
            // `priority_heap_length`, default 20 — the O(n) rebuild is
            // negligible and keeps the invariant exact).
            let mut entries: Vec<PrioritizedRequest> = self.pending_heap.drain().collect();
            let mut worst_idx = 0;
            for (i, e) in entries.iter().enumerate() {
                if e.priority > entries[worst_idx].priority {
                    worst_idx = i;
                }
            }
            let evicted = entries.remove(worst_idx);
            entries.push(PrioritizedRequest {
                id,
                // M1 review fix: NaN → f64::MAX sink-to-bottom guard.
                priority: sanitize_priority(request.priority),
            });
            for e in entries {
                self.pending_heap.push(e);
            }

            // Route the evicted request into the M8.1 deferred queue (if it
            // has room) so a later `update()` can promote it back once the
            // heap drains; otherwise drop it and account for the cancellation.
            if let Some(mut evicted_req) = self.active_requests.remove(&evicted.id) {
                evicted_req.state = RequestState::Unissued;
                if self.deferred.len() < self.maximum_deferred {
                    self.deferred.push(evicted_req);
                } else {
                    self.statistics.on_cancelled_pending();
                }
            }

            request.state = RequestState::Issued;
            self.active_requests.insert(id, request);
            Some(id)
        }
    }

    /// Cancels a request.
    ///
    /// Updates statistics differently depending on whether the request had been
    /// activated (cancelled-active) or was still pending (cancelled-pending).
    pub fn cancel(&mut self, id: RequestId) -> bool {
        let was_active = self
            .active_requests
            .get(&id)
            .map(|r| r.state == RequestState::Active)
            .unwrap_or(false);
        match self.deactivate_request(id) {
            Some(request) => {
                if was_active {
                    let server_key = request.server_key.clone();
                    let rtype = request.request_type;
                    self.statistics.on_cancelled_active(&server_key, rtype);
                } else {
                    self.statistics.on_cancelled_pending();
                }
                true
            }
            None => false,
        }
    }

    /// Marks a request as completed successfully.
    pub fn complete(&mut self, id: RequestId) -> bool {
        let was_active = self
            .active_requests
            .get(&id)
            .map(|r| r.state == RequestState::Active)
            .unwrap_or(false);
        match self.deactivate_request(id) {
            Some(request) => {
                let server_key = request.server_key.clone();
                let rtype = request.request_type;
                if was_active {
                    self.statistics.on_completed(&server_key, rtype);
                } else {
                    // Completed while still pending: count success without an
                    // active decrement (it was never activated).
                    self.statistics.succeeded += 1;
                    *self.statistics.completed_by_server.entry(server_key).or_insert(0) += 1;
                    *self.statistics.completed_by_type.entry(rtype).or_insert(0) += 1;
                }
                true
            }
            None => false,
        }
    }

    /// Marks a request as failed (retries exhausted or unrecoverable error).
    ///
    /// Mirrors the failure path of CesiumJS `RequestScheduler` where
    /// `statistics.numberOfFailedRequests` is incremented and the request is
    /// released back so its server slot opens.
    pub fn fail(&mut self, id: RequestId) -> bool {
        let was_active = self
            .active_requests
            .get(&id)
            .map(|r| r.state == RequestState::Active)
            .unwrap_or(false);
        match self.deactivate_request(id) {
            Some(request) => {
                let server_key = request.server_key.clone();
                let rtype = request.request_type;
                if was_active {
                    self.statistics.on_failed(&server_key, rtype);
                } else {
                    self.statistics.failed += 1;
                    *self.statistics.failed_by_server.entry(server_key).or_insert(0) += 1;
                    *self.statistics.failed_by_type.entry(rtype).or_insert(0) += 1;
                }
                true
            }
            None => false,
        }
    }

    /// Updates priorities and activates pending requests.
    /// Should be called once per frame.
    ///
    /// Order of operations (mirrors CesiumJS `RequestScheduler.update`):
    /// 1. Snapshot the previous active count for delta diagnostics.
    /// 2. Promote deferred requests into the priority heap if slots opened.
    /// 3. Activate pending requests while global/per-server slots are available.
    pub fn update(&mut self) {
        self.statistics.snapshot_last_active();
        self.promote_deferred();

        // Activate pending requests. Mirrors CesiumJS `RequestScheduler.update`
        // (packages/engine/Source/Core/RequestScheduler.js L320-340): the loop
        // TERMINATES only when the GLOBAL slot budget is exhausted. A heap-top
        // whose OWN server is saturated is SKIPPED — popped and parked back
        // into the deferred queue for a later frame — and the scan CONTINUES,
        // so pending requests bound for OTHER servers with open slots are still
        // activated. The pre-fix code `break`-ed on any `can_activate == false`,
        // conflating "global budget full" with "this request's server full": a
        // single saturated server sitting at the heap top then starved every
        // other server's backlog for the whole frame (H1 review fix).
        while let Some(prioritized) = self.pending_heap.peek() {
            // Global budget exhausted → nothing more can activate this frame.
            if !self.global_has_slots() {
                break;
            }
            let id = prioritized.id;
            let Some(request) = self.active_requests.get(&id) else {
                // Stale heap entry (request already cancelled/completed) → drop.
                self.pending_heap.pop();
                continue;
            };
            if self.server_has_slots(request) {
                let request = self.active_requests.get_mut(&id).unwrap();
                request.state = RequestState::Active;
                let server_key = request.server_key.clone();
                let rtype = request.request_type;
                self.pending_heap.pop();
                *self.active_count_by_server.entry(server_key.clone()).or_insert(0) += 1;
                self.statistics.on_activated(&server_key, rtype);
            } else {
                // This request's server is saturated. Skip it (pop off the
                // heap) and park it in the deferred queue so `promote_deferred`
                // re-admits it on a later frame once its server drains, then
                // keep scanning for other-server requests. Popping — rather
                // than leaving it at the heap top — is what lets the loop
                // advance past a saturated server instead of re-peeking it
                // forever (which is how the pre-fix `break` starved others).
                self.pending_heap.pop();
                if let Some(mut req) = self.active_requests.remove(&id) {
                    req.state = RequestState::Unissued;
                    if self.deferred.len() < self.maximum_deferred {
                        self.deferred.push(req);
                    } else {
                        self.statistics.on_cancelled_pending();
                    }
                }
            }
        }
    }

    /// Recomputes pending request priorities from frame state, then updates.
    ///
    /// When a [`PriorityFunction`] is installed, each pending (Issued) request
    /// carrying a [`PriorityKey`] has its priority recomputed against the
    /// supplied [`FrameContext`]; the priority heap is then rebuilt and
    /// [`Self::update`] runs. This mirrors CesiumJS re-evaluating
    /// `request.priorityFunction()` every frame before promotion.
    ///
    /// If no priority function is installed, this is equivalent to [`Self::update`].
    pub fn update_with_context(&mut self, context: &FrameContext) {
        if let Some(pf) = self.priority_function.as_ref() {
            let mut new_entries: Vec<PrioritizedRequest> = Vec::new();
            // `pf` borrows `self.priority_function`; the loop borrows
            // `self.active_requests` — disjoint fields.
            for (id, request) in self.active_requests.iter_mut() {
                if request.state == RequestState::Issued {
                    if let Some(key) = &request.priority_key {
                        // M1 review fix: guard the recomputed priority so a NaN
                        // returned by the priority function can't corrupt the
                        // rebuilt heap's total order (NaN → f64::MAX sink).
                        request.priority = sanitize_priority(pf.compute_priority(key, context));
                    }
                    new_entries.push(PrioritizedRequest {
                        id: *id,
                        priority: sanitize_priority(request.priority),
                    });
                }
            }
            self.pending_heap = BinaryHeap::from(new_entries);
        }
        self.update();
    }

    /// Gets a request by ID (active or pending; deferred requests are not yet
    /// tracked here — see [`Self::deferred_requests`]).
    pub fn get_request(&self, id: RequestId) -> Option<&Request> {
        self.active_requests.get(&id)
    }

    /// Returns the currently deferred (heap-rejected, awaiting promotion)
    /// requests.
    pub fn deferred_requests(&self) -> &[Request] {
        &self.deferred
    }

    // Internal helpers

    /// Global-slot predicate: `true` while the scheduler has room for *any*
    /// further active request, regardless of server. Split out from the old
    /// combined `can_activate` so [`Self::update`] can distinguish "global
    /// budget exhausted" (terminate the loop) from "this request's server is
    /// saturated" (skip just that request) — the H1 review fix.
    fn global_has_slots(&self) -> bool {
        let active_count = self.active_count_by_server.values().sum::<usize>();
        active_count < self.maximum_requests
    }

    /// Per-server predicate: `true` if this request may take a slot on its own
    /// server (or isn't server-throttled at all).
    fn server_has_slots(&self, request: &Request) -> bool {
        !request.throttle_by_server || self.server_has_open_slots(&request.server_key, 1)
    }

    /// Combined immediate-activation predicate (global AND per-server). Used by
    /// [`Self::schedule`] for its fast-path check; [`Self::update`] consults
    /// the two predicates separately so one saturated server can't stall the
    /// whole activation loop.
    fn can_activate(&self, request: &Request) -> bool {
        self.global_has_slots() && self.server_has_slots(request)
    }

    /// Promotes deferred requests into the priority heap while slots are open.
    ///
    /// Deferred requests are promoted in ascending priority-value order (best
    /// first) so that the most important deferred requests re-enter the heap
    /// when capacity frees up.
    fn promote_deferred(&mut self) {
        if self.deferred.is_empty() {
            return;
        }
        // Best (lowest priority value) first. M1 review fix: `total_cmp` keeps
        // the sort total (NaN gets a deterministic position) instead of the
        // previous `partial_cmp().unwrap_or(Equal)` which left NaN unordered.
        self.deferred
            .sort_by(|a, b| a.priority.total_cmp(&b.priority));

        let mut remaining: Vec<Request> = Vec::new();
        for mut request in self.deferred.drain(..) {
            if self.pending_heap.len() < self.priority_heap_length {
                let id = request.id;
                // M1 review fix: NaN → f64::MAX sink-to-bottom guard.
                let priority = sanitize_priority(request.priority);
                request.state = RequestState::Issued;
                self.active_requests.insert(id, request);
                self.pending_heap.push(PrioritizedRequest { id, priority });
            } else {
                remaining.push(request);
            }
        }
        self.deferred = remaining;
    }

    fn activate_request(&mut self, mut request: Request) {
        request.state = RequestState::Active;
        let server_key = request.server_key.clone();
        let rtype = request.request_type;
        *self.active_count_by_server.entry(server_key.clone()).or_insert(0) += 1;
        self.statistics.on_activated(&server_key, rtype);
        self.active_requests.insert(request.id, request);
    }

    /// Removes a request from tracking, releasing its server slot if it was
    /// active. Returns the removed request so callers can update statistics
    /// according to the outcome (complete/fail/cancel).
    fn deactivate_request(&mut self, id: RequestId) -> Option<Request> {
        if let Some(request) = self.active_requests.remove(&id) {
            if request.state == RequestState::Active {
                if let Some(count) = self.active_count_by_server.get_mut(&request.server_key) {
                    *count = count.saturating_sub(1);
                }
            }
            Some(request)
        } else {
            None
        }
    }
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the server key (host:port) from a URL.
///
/// Maps to CesiumJS `RequestScheduler.getServerKey`.
/// Adds default ports: http→80, https→443.
pub fn get_server_key(url: &str) -> String {
    if let Some(start) = url.find("://") {
        let scheme = url[..start].to_lowercase();
        let after_scheme = &url[start + 3..];
        let end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let authority = &after_scheme[..end];
        // Strip credentials (user:pass@)
        let authority = if let Some(at) = authority.find('@') {
            &authority[at + 1..]
        } else {
            authority
        };
        // Add default port if missing
        if authority.contains(':') {
            authority.to_lowercase()
        } else {
            match scheme.as_str() {
                "http" => format!("{}:80", authority.to_lowercase()),
                "https" => format!("{}:443", authority.to_lowercase()),
                _ => authority.to_lowercase(),
            }
        }
    } else {
        url.to_string()
    }
}

/// Internal alias for backward compatibility.
fn extract_server_key(url: &str) -> String {
    get_server_key(url)
}

/// A resource URL template with query parameters.
/// Maps to CesiumJS `Resource`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// The base URL (without query string).
    pub url: String,
    /// Query parameters.
    pub query_parameters: HashMap<String, String>,
    /// Template values for URL substitution ({key} → value).
    pub template_values: HashMap<String, String>,
    /// HTTP headers.
    pub headers: HashMap<String, String>,
}

/// Options for creating a derived resource.
/// Maps to CesiumJS `Resource.getDerivedResource` options
#[derive(Debug, Clone, Default)]
pub struct DeriveResourceOptions {
    /// Relative or absolute URL to resolve against the parent.
    pub url: Option<String>,
    /// Additional query parameters.
    pub query_parameters: Vec<(String, String)>,
    /// Additional template values.
    pub template_values: Vec<(String, String)>,
    /// Additional headers.
    pub headers: Vec<(String, String)>,
}

impl Resource {
    /// Creates a new resource with the given URL.
    /// Parses query parameters from the URL if present.
    /// Maps to CesiumJS `new Resource({ url })`
    pub fn new(url: impl Into<String>) -> Self {
        let raw = url.into();
        let (base, query_params) = Self::parse_url(&raw);
        Self {
            url: base,
            query_parameters: query_params,
            template_values: HashMap::new(),
            headers: HashMap::new(),
        }
    }

    /// Creates a resource without parsing query parameters from the URL.
    /// Maps to CesiumJS `new Resource({ url, parseUrl: false })`
    pub fn new_unparsed(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            query_parameters: HashMap::new(),
            template_values: HashMap::new(),
            headers: HashMap::new(),
        }
    }

    /// Adds a query parameter.
    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_parameters.insert(key.into(), value.into());
        self
    }

    /// Adds a template value.
    pub fn with_template_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.template_values.insert(key.into(), value.into());
        self
    }

    /// Adds a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Appends a forward slash to the URL if it doesn't already end with one.
    /// Maps to CesiumJS `Resource.appendForwardSlash`
    pub fn append_forward_slash(&mut self) {
        if !self.url.ends_with('/') {
            self.url.push('/');
        }
    }

    /// Gets the URL component, optionally including query parameters.
    /// Maps to CesiumJS `Resource.getUrlComponent(includeQuery, includeProxy)`
    pub fn get_url_component(&self, include_query: bool) -> String {
        if !include_query || self.query_parameters.is_empty() {
            return self.url.clone();
        }
        format!("{}?{}", self.url, self.build_query_string())
    }

    /// Builds the full URL with query parameters and template substitution.
    /// Maps to CesiumJS `Resource.url` getter / `toString()`
    pub fn build_url(&self) -> String {
        let base = self.apply_template_values(&self.url);
        if self.query_parameters.is_empty() {
            return base;
        }
        if base.contains('?') {
            format!("{}&{}", base, self.build_query_string())
        } else {
            format!("{}?{}", base, self.build_query_string())
        }
    }

    /// Gets the server key for this resource.
    pub fn server_key(&self) -> String {
        extract_server_key(&self.url)
    }

    /// Sets query parameters, optionally preserving existing values as defaults.
    /// Maps to CesiumJS `Resource.setQueryParameters(params, useAsDefault)`
    pub fn set_query_parameters(&mut self, params: Vec<(String, String)>, use_as_default: bool) {
        if use_as_default {
            // Only add keys that don't already exist
            for (k, v) in params {
                self.query_parameters.entry(k).or_insert(v);
            }
        } else {
            // Overwrite all
            self.query_parameters = params.into_iter().collect();
        }
    }

    /// Creates a derived resource by resolving a relative URL against this resource.
    /// Maps to CesiumJS `Resource.getDerivedResource`
    pub fn get_derived_resource(&self, options: &DeriveResourceOptions) -> Self {
        let mut derived_url = self.url.clone();

        if let Some(ref rel_url) = options.url {
            derived_url = Self::resolve_url(&derived_url, rel_url);
        }

        // Merge query parameters
        let mut query = self.query_parameters.clone();
        for (k, v) in &options.query_parameters {
            query.insert(k.clone(), v.clone());
        }

        // Parse query from derived URL
        let (base, url_params) = Self::parse_url(&derived_url);
        for (k, v) in url_params {
            query.insert(k, v);
        }

        // Merge template values
        let mut templates = self.template_values.clone();
        for (k, v) in &options.template_values {
            templates.insert(k.clone(), v.clone());
        }

        // Merge headers
        let mut headers = self.headers.clone();
        for (k, v) in &options.headers {
            headers.insert(k.clone(), v.clone());
        }

        // Apply template values to the URL
        let final_url = Self::apply_template_values_static(&base, &templates);

        Self {
            url: final_url,
            query_parameters: query,
            template_values: templates,
            headers,
        }
    }

    /// Creates a derived resource with a relative path appended (legacy API).
    pub fn derive(&self, relative_path: &str) -> Self {
        let base = if self.url.ends_with('/') {
            &self.url
        } else if let Some(pos) = self.url.rfind('/') {
            &self.url[..=pos]
        } else {
            &self.url
        };

        Self {
            url: format!("{}{}", base, relative_path),
            query_parameters: self.query_parameters.clone(),
            template_values: self.template_values.clone(),
            headers: self.headers.clone(),
        }
    }

    // ─── Internal helpers ──────────────────────────────────────────────────────

    /// Parses a URL into base (without query) and query parameters.
    fn parse_url(raw: &str) -> (String, HashMap<String, String>) {
        if let Some(qpos) = raw.find('?') {
            let base = raw[..qpos].to_string();
            let query_str = &raw[qpos + 1..];
            let params = Self::parse_query_string(query_str);
            (base, params)
        } else {
            (raw.to_string(), HashMap::new())
        }
    }

    /// Parses a query string into key-value pairs.
    fn parse_query_string(qs: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for pair in qs.split('&') {
            if pair.is_empty() {
                continue;
            }
            if let Some(eq) = pair.find('=') {
                let key = pair[..eq].to_string();
                let value = pair[eq + 1..].to_string();
                map.insert(key, value);
            } else {
                map.insert(pair.to_string(), String::new());
            }
        }
        map
    }

    /// Builds a query string from parameters (sorted for determinism).
    fn build_query_string(&self) -> String {
        let mut pairs: Vec<_> = self.query_parameters.iter().collect();
        // deferred.md #14: 采纳 clippy 建议解引用克隆内层 String（原 k.clone() 对 &&String 双重引用克隆）。
        pairs.sort_by_key(|(k, _)| (*k).clone());
        pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Applies template values to this resource's URL.
    fn apply_template_values(&self, url: &str) -> String {
        Self::apply_template_values_static(url, &self.template_values)
    }

    /// Replaces {key} placeholders in a URL with template values.
    fn apply_template_values_static(url: &str, templates: &HashMap<String, String>) -> String {
        if templates.is_empty() {
            return url.to_string();
        }
        let mut result = url.to_string();
        for (key, value) in templates {
            let placeholder = format!("{{{}}}", key);
            // URL-encode the value (encode special chars)
            let encoded = Self::encode_uri_component(value);
            result = result.replace(&placeholder, &encoded);
        }
        result
    }

    /// Encodes a URI component (percent-encoding for special characters).
    fn encode_uri_component(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
                | b'*' | b'\'' | b'(' | b')' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        result
    }

    /// Resolves a relative URL against a base URL.
    /// Maps to CesiumJS URI resolution logic.
    fn resolve_url(base: &str, relative: &str) -> String {
        // If relative is absolute (has scheme), use it directly
        if relative.contains("://") {
            return relative.to_string();
        }

        // Get the directory part of the base URL
        let directory = if base.ends_with('/') {
            base.to_string()
        } else if let Some(pos) = base.rfind('/') {
            base[..=pos].to_string()
        } else {
            base.to_string()
        };

        format!("{}{}", directory, relative)
    }

    // ─── URI classification ──────────────────────────────────────────────

    /// Returns `true` if this resource's URL is a `data:` URI.
    ///
    /// Data URIs carry their payload inline and never hit the network, so the
    /// fetch-descriptor pipeline short-circuits them (no proxy, no scheduler).
    /// Maps to CesiumJS `Resource` handling of `dataUriRegex`.
    pub fn is_data_uri(&self) -> bool {
        crate::data_uri::is_data_uri(&self.url)
    }

    /// Returns `true` if this resource's URL is a `blob:` URI.
    ///
    /// Blob URIs reference in-memory browser objects; like data URIs they are
    /// never proxied. Maps to CesiumJS blob handling in `Resource`.
    pub fn is_blob_uri(&self) -> bool {
        self.url.starts_with("blob:")
    }

    /// Returns the base URI (`scheme://authority/`) of this resource.
    ///
    /// Maps to CesiumJS `Resource.getBaseUri`.
    pub fn get_base_uri(&self) -> String {
        if let Some(start) = self.url.find("://") {
            let after = &self.url[start + 3..];
            let end = after.find('/').unwrap_or(after.len());
            format!("{}/", &self.url[..start + 3 + end])
        } else {
            self.url.clone()
        }
    }

    /// Appends query values, overwriting any existing keys.
    ///
    /// Maps to CesiumJS `Resource.appendQueryParameters`.
    pub fn append_query_values(&mut self, params: &[(String, String)]) {
        for (k, v) in params {
            self.query_parameters.insert(k.clone(), v.clone());
        }
    }

    /// Removes the given query parameter keys.
    ///
    /// Maps to CesiumJS `Resource.removeQueryParameters`.
    pub fn remove_query_values(&mut self, keys: &[&str]) {
        for k in keys {
            self.query_parameters.remove(*k);
        }
    }

    /// Clones this resource with a different base URL, preserving query
    /// parameters, template values, and headers.
    ///
    /// Maps to CesiumJS `Resource.getDerivedResource({ url })` when only the
    /// URL changes.
    pub fn clone_with_url(&self, url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            query_parameters: self.query_parameters.clone(),
            template_values: self.template_values.clone(),
            headers: self.headers.clone(),
        }
    }

    // ─── Fetch descriptor construction (pure; no IO) ──────────────────────

    /// Builds a [`FetchDescriptor`] describing *what* to request without
    /// performing any IO.
    ///
    /// This is the domain-side counterpart of CesiumJS `Resource.fetch*`: the
    /// adapter layer (`adapters/network`) consumes the descriptor and executes
    /// the actual HTTP request. Because construction is pure, the entire URL /
    /// header / proxy / retry pipeline is unit-testable without a network.
    ///
    /// Data URIs short-circuit: `is_data_uri` is set and no proxy is applied.
    pub fn build_fetch_descriptor(
        &self,
        response_type: ResponseType,
        proxy: Option<&ProxyPolicy>,
    ) -> FetchDescriptor {
        let raw_url = self.build_url();
        let is_data = self.is_data_uri() || self.is_blob_uri();
        let final_url = if is_data {
            raw_url
        } else {
            match proxy {
                Some(policy) => policy.apply(&raw_url),
                None => raw_url,
            }
        };

        FetchDescriptor {
            url: final_url,
            method: HttpMethod::Get,
            headers: self.headers.clone(),
            response_type,
            request_type: RequestType::Other,
            retry: RetryPolicy::default(),
            priority: 0.0,
            server_key: if is_data { String::new() } else { self.server_key() },
            body: None,
            is_data_uri: is_data,
        }
    }

    /// Descriptor for fetching binary content (`ArrayBuffer`).
    /// Maps to CesiumJS `Resource.fetchArrayBuffer`.
    pub fn fetch_array_buffer(&self, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        self.build_fetch_descriptor(ResponseType::ArrayBuffer, proxy)
    }

    /// Descriptor for fetching JSON content.
    /// Maps to CesiumJS `Resource.fetchJson`.
    pub fn fetch_json(&self, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        self.build_fetch_descriptor(ResponseType::Json, proxy)
    }

    /// Descriptor for fetching text content.
    /// Maps to CesiumJS `Resource.fetchText`.
    pub fn fetch_text(&self, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        self.build_fetch_descriptor(ResponseType::Text, proxy)
    }

    /// Descriptor for fetching image content.
    /// Maps to CesiumJS `Resource.fetchImage`.
    pub fn fetch_image(&self, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        self.build_fetch_descriptor(ResponseType::Image, proxy)
    }

    /// Descriptor for fetching blob content.
    /// Maps to CesiumJS `Resource.fetchBlob`.
    pub fn fetch_blob(&self, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        self.build_fetch_descriptor(ResponseType::Blob, proxy)
    }

    /// Descriptor for a POST request carrying a body.
    /// Maps to CesiumJS `Resource.post`.
    pub fn post(&self, body: Vec<u8>, proxy: Option<&ProxyPolicy>) -> FetchDescriptor {
        let mut descriptor = self.build_fetch_descriptor(ResponseType::Json, proxy);
        descriptor.method = HttpMethod::Post;
        descriptor.body = Some(body);
        descriptor
    }
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.build_url())
    }
}

// ─── Response / method / fetch descriptor types ─────────────────────────

/// The expected response body type.
///
/// Maps to CesiumJS `Resource.ResponseType` (`ARRAY_BUFFER`, `BLOB`,
/// `DOCUMENT`, `JSON`, `TEXT`, `IMAGE`, `IMAGE_BITMAP`). The adapter layer
/// uses this to decide how to decode the HTTP response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseType {
    /// Raw bytes.
    ArrayBuffer,
    /// Binary large object (images, etc.).
    Blob,
    /// Parsed document (XML/HTML).
    Document,
    /// JSON value.
    Json,
    /// UTF-8 text.
    Text,
    /// Decoded image.
    Image,
    /// Decoded image bitmap (GPU-uploadable).
    ImageBitmap,
}

/// HTTP method for a fetch.
///
/// Maps to the methods CesiumJS `Resource` supports (`fetch*` use GET,
/// `post`/`put`/`patch`/`delete` mutate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    /// Returns the canonical uppercase method token.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// A fully-resolved description of a network request, produced by the pure
/// domain layer and executed by the adapter layer.
///
/// This is the boundary type that keeps IO out of the domain: `Resource`
/// builds it, `adapters/network` consumes it.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchDescriptor {
    /// Final URL (proxy already applied, query/template substituted).
    pub url: String,
    /// HTTP method.
    pub method: HttpMethod,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Expected response body type.
    pub response_type: ResponseType,
    /// Logical request type (for scheduler statistics/throttling).
    pub request_type: RequestType,
    /// Retry policy applied on failure.
    pub retry: RetryPolicy,
    /// Initial priority hint (lower = higher priority).
    pub priority: f64,
    /// Server key (`host:port`) for per-server throttling. Empty for data URIs.
    pub server_key: String,
    /// Optional request body (POST/PUT/PATCH).
    pub body: Option<Vec<u8>>,
    /// Whether the URL is an inline `data:`/`blob:` URI (no network needed).
    pub is_data_uri: bool,
}

impl FetchDescriptor {
    /// Overrides the logical request type.
    pub fn with_request_type(mut self, request_type: RequestType) -> Self {
        self.request_type = request_type;
        self
    }

    /// Overrides the retry policy.
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Overrides the initial priority hint.
    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = priority;
        self
    }

    /// Adds (or replaces) a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Converts this descriptor into a schedulable [`Request`].
    pub fn to_request(&self) -> Request {
        let mut request = Request::throttled(
            self.url.clone(),
            self.request_type,
            self.priority,
        );
        request.server_key = if self.server_key.is_empty() {
            extract_server_key(&self.url)
        } else {
            self.server_key.clone()
        };
        request
    }
}

// ─── Retry semantics (pure) ─────────────────────────────────────────────

/// Classification of a request failure, used to decide retryability.
///
/// Maps to the CesiumJS `retryCallback(resource, error)` decision, where the
/// error's HTTP status determines whether a retry is worthwhile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestErrorClass {
    /// Recoverable (5xx, network timeout) — retry with backoff.
    Transient,
    /// Rate-limited (HTTP 429) — retry only if `retry_on_throttled`.
    Throttled,
    /// Client error (4xx except 429) — never retry.
    Permanent,
}

/// Backoff shape applied between retry attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Constant delay between attempts.
    Fixed,
    /// Delay grows linearly with the attempt number.
    Linear,
    /// Delay doubles each attempt (capped at `max_delay_millis`).
    Exponential,
}

/// The scheduler/caller's decision after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry immediately (no delay).
    RetryNow,
    /// Retry after the given delay in milliseconds.
    RetryAfterMillis(u64),
    /// Stop retrying; surface the failure.
    GiveUp,
}

/// Pure retry policy.
///
/// Maps to CesiumJS `Resource.retryAttempts` + `Resource.retryCallback`. The
/// default mirrors CesiumJS's `retryAttempts = 1` with no delay, but hosts can
/// configure exponential backoff for transient/throttled failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts after the initial request.
    pub max_attempts: u32,
    /// Base delay in milliseconds (used by the backoff strategy).
    pub base_delay_millis: u64,
    /// Upper bound on the computed delay.
    pub max_delay_millis: u64,
    /// Backoff shape.
    pub backoff: BackoffStrategy,
    /// Whether to retry throttled (429) responses.
    pub retry_on_throttled: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            // CesiumJS `Resource.retryAttempts` defaults to 1.
            max_attempts: 1,
            base_delay_millis: 0,
            max_delay_millis: 30_000,
            backoff: BackoffStrategy::Fixed,
            retry_on_throttled: true,
        }
    }
}

impl RetryPolicy {
    /// A policy that never retries.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            ..Default::default()
        }
    }

    /// Exponential backoff with a sane default base delay.
    pub fn exponential(max_attempts: u32, base_delay_millis: u64) -> Self {
        Self {
            max_attempts,
            base_delay_millis,
            backoff: BackoffStrategy::Exponential,
            ..Default::default()
        }
    }

    /// Computes the delay before the given (0-based) retry attempt.
    pub fn delay_for(&self, attempt: u32) -> u64 {
        if self.base_delay_millis == 0 {
            return 0;
        }
        let raw = match self.backoff {
            BackoffStrategy::Fixed => self.base_delay_millis,
            BackoffStrategy::Linear => {
                self.base_delay_millis.saturating_mul((attempt.max(1)) as u64)
            }
            BackoffStrategy::Exponential => {
                self.base_delay_millis.saturating_mul(1u64 << attempt.min(20))
            }
        };
        raw.min(self.max_delay_millis)
    }

    /// Decides whether to retry after a failed attempt.
    ///
    /// `attempt` is the number of retries already performed (0 = first failure).
    /// This is the pure core of CesiumJS's `retryCallback` decision.
    pub fn decide(&self, attempt: u32, error: RequestErrorClass) -> RetryDecision {
        match error {
            RequestErrorClass::Permanent => RetryDecision::GiveUp,
            RequestErrorClass::Throttled if !self.retry_on_throttled => RetryDecision::GiveUp,
            _ => {
                if attempt >= self.max_attempts {
                    RetryDecision::GiveUp
                } else {
                    let delay = self.delay_for(attempt);
                    if delay == 0 {
                        RetryDecision::RetryNow
                    } else {
                        RetryDecision::RetryAfterMillis(delay)
                    }
                }
            }
        }
    }
}

/// Classifies an HTTP status code into a [`RequestErrorClass`].
///
/// Maps to the status-based retry heuristics in CesiumJS request handling
/// (429 = throttled, 5xx = transient, other 4xx = permanent).
pub fn classify_status(status: u16) -> RequestErrorClass {
    match status {
        429 => RequestErrorClass::Throttled,
        400..=499 => RequestErrorClass::Permanent,
        _ => RequestErrorClass::Transient,
    }
}

/// Context handed to a [`RetryCallback`] after a failed attempt.
#[derive(Debug, Clone)]
pub struct RetryContext {
    /// The URL that failed.
    pub url: String,
    /// Number of retries already performed (0 = first failure).
    pub attempt: u32,
    /// HTTP status code, if the failure came from a response.
    pub status: Option<u16>,
    /// Pre-classified error class.
    pub error_class: RequestErrorClass,
    /// The policy in effect.
    pub policy: RetryPolicy,
}

impl RetryContext {
    /// Builds a context from a status code, classifying it automatically.
    pub fn from_status(url: impl Into<String>, attempt: u32, status: u16, policy: RetryPolicy) -> Self {
        Self {
            url: url.into(),
            attempt,
            status: Some(status),
            error_class: classify_status(status),
            policy,
        }
    }

    /// The default decision: delegate to the policy's `decide`.
    pub fn default_decision(&self) -> RetryDecision {
        self.policy.decide(self.attempt, self.error_class)
    }
}

/// A pluggable retry callback.
///
/// Maps to CesiumJS `Resource.retryCallback(resource, error)`. The domain
/// provides the pure decision; the adapter invokes the callback between
/// attempts. Kept as a boxed closure so hosts can inject custom logic
/// (e.g. honor `Retry-After` headers).
pub type RetryCallback = Box<dyn Fn(&RetryContext) -> RetryDecision + Send + Sync>;

/// Returns the default retry callback, which simply applies the policy.
///
/// This is the pure-domain equivalent of CesiumJS's built-in retry behaviour
/// when no custom `retryCallback` is supplied.
pub fn default_retry_callback() -> RetryCallback {
    Box::new(|ctx: &RetryContext| ctx.default_decision())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_scheduler_basic() {
        let mut scheduler = RequestScheduler::new();
        let request = Request::new(
            "https://example.com/tile.b3dm".to_string(),
            RequestType::Tiles3D,
        );

        let id = scheduler.schedule(request).unwrap();
        assert_eq!(scheduler.active_request_count(), 1);

        scheduler.complete(id);
        assert_eq!(scheduler.active_request_count(), 0);
    }

    #[test]
    fn test_server_throttling() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests_per_server = 2;

        // Schedule 2 requests to the same server
        for i in 0..2 {
            let request = Request::throttled(
                format!("https://example.com/tile{}.b3dm", i),
                RequestType::Tiles3D,
                i as f64,
            );
            scheduler.schedule(request).unwrap();
        }
        scheduler.update();

        // Third request should be pending (not active)
        let request = Request::throttled(
            "https://example.com/tile2.b3dm".to_string(),
            RequestType::Tiles3D,
            2.0,
        );
        scheduler.schedule(request).unwrap();

        // Only 2 should be active (server key includes default port)
        assert!(scheduler.server_has_open_slots("example.com:443", 0));
        assert!(!scheduler.server_has_open_slots("example.com:443", 1));
    }

    #[test]
    fn test_priority_ordering() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 1;
        scheduler.throttle_requests = true;

        // First request activates immediately
        let r1 = Request::throttled(
            "https://a.com/1".to_string(),
            RequestType::Other,
            10.0,
        );
        scheduler.schedule(r1).unwrap();

        // These should be pending
        let r2 = Request::throttled(
            "https://b.com/2".to_string(),
            RequestType::Other,
            5.0, // Higher priority (lower value)
        );
        let r3 = Request::throttled(
            "https://c.com/3".to_string(),
            RequestType::Other,
            1.0, // Highest priority
        );
        scheduler.schedule(r2).unwrap();
        scheduler.schedule(r3).unwrap();

        assert_eq!(scheduler.pending_request_count(), 2);
    }

    /// H1 review fix: a saturated server sitting at the top of the priority
    /// heap must NOT stall activation of pending requests bound for OTHER
    /// servers with open slots (the pre-fix `break`-on-any-full starved them).
    #[test]
    fn update_activates_other_server_when_heap_top_server_is_saturated() {
        let mut sched = RequestScheduler::new();
        sched.throttle_requests = true;
        sched.maximum_requests = 50; // ample global budget
        sched.maximum_requests_per_server = 1; // one active per server
        sched.priority_heap_length = 20;

        // Saturate BOTH servers with one active request each.
        let a1 = sched
            .schedule(Request::throttled(
                "https://servera.example/a1".into(),
                RequestType::Terrain,
                0.0,
            ))
            .unwrap();
        let b0 = sched
            .schedule(Request::throttled(
                "https://serverb.example/b0".into(),
                RequestType::Terrain,
                0.0,
            ))
            .unwrap();
        assert_eq!(sched.get_request(a1).unwrap().state, RequestState::Active);
        assert_eq!(sched.get_request(b0).unwrap().state, RequestState::Active);

        // These two land in the pending heap (both servers currently full).
        // The serverA request has the BETTER priority (0.0 < 1.0) so it sits at
        // the heap top — exactly the position that starved others pre-fix.
        let a2 = sched
            .schedule(Request::throttled(
                "https://servera.example/a2".into(),
                RequestType::Terrain,
                0.0,
            ))
            .unwrap();
        let b1 = sched
            .schedule(Request::throttled(
                "https://serverb.example/b1".into(),
                RequestType::Terrain,
                1.0,
            ))
            .unwrap();
        assert_eq!(sched.pending_request_count(), 2);

        // Free serverB (complete b0); serverA stays saturated (a1 still active).
        assert!(sched.complete(b0));

        sched.update();

        // serverB's pending request (b1) MUST activate even though the heap top
        // (a2) belongs to the still-saturated serverA.
        assert_eq!(
            sched.get_request(b1).map(|r| r.state),
            Some(RequestState::Active),
            "serverB pending request must not be starved by a saturated serverA at the heap top"
        );
        // serverA's a2 is skipped this frame and parked in the deferred queue.
        assert!(
            sched.get_request(a2).is_none(),
            "a2 leaves the active map (deferred) rather than activating on a full server"
        );
        assert_eq!(sched.deferred_count(), 1, "a2 parked in deferred for a later frame");
        assert_eq!(sched.deferred_requests()[0].id, a2);
    }

    /// M1 review fix: `Ord::cmp` and `PartialEq::eq` must agree even for NaN
    /// priorities (`a.cmp(b) == Equal <=> a == b`).
    #[test]
    fn prioritized_request_ord_eq_consistent_for_nan() {
        let nan = f64::NAN;
        let a = PrioritizedRequest { id: RequestId(1), priority: nan };
        let b = PrioritizedRequest { id: RequestId(2), priority: nan };
        assert_eq!(
            a.cmp(&b) == Ordering::Equal,
            a == b,
            "Ord/Eq consistency law must hold for NaN"
        );
        // NaN vs a finite value: strictly ordered, never Equal (pre-fix cmp
        // returned Equal for the NaN comparison, corrupting the heap order).
        let c = PrioritizedRequest { id: RequestId(3), priority: 0.0 };
        assert_ne!(a.cmp(&c), Ordering::Equal);
        assert!(a != c);
        // sanitize_priority sinks NaN to the bottom value.
        assert_eq!(sanitize_priority(nan), f64::MAX);
        assert_eq!(sanitize_priority(2.5), 2.5);
    }

    /// M1 review fix: a NaN-priority request sinks to the BOTTOM of the
    /// schedule heap (activated last) without corrupting the ordering of
    /// well-formed priorities. NaN is reachable via `inf + (-inf)` from a
    /// composite priority summing `f64::MAX`-magnitude components.
    #[test]
    fn nan_priority_request_activates_last_without_corrupting_heap() {
        let mut sched = RequestScheduler::new();
        sched.throttle_requests = true;
        sched.maximum_requests = 1; // drain one-at-a-time → activation order == heap order
        sched.maximum_requests_per_server = 50;
        sched.priority_heap_length = 20;

        let nan = f64::INFINITY + f64::NEG_INFINITY;
        assert!(nan.is_nan());

        let mk = |prio: f64| {
            let mut r =
                Request::throttled("https://s.example/t".into(), RequestType::Terrain, prio);
            r.throttle_by_server = false; // isolate global-budget ordering
            r
        };

        // First (5.0) activates (budget 1); the rest pile into the heap.
        let id_first = sched.schedule(mk(5.0)).unwrap();
        let id_nan = sched.schedule(mk(nan)).unwrap();
        let id_low = sched.schedule(mk(1.0)).unwrap();
        let id_mid = sched.schedule(mk(3.0)).unwrap();
        assert_eq!(sched.get_request(id_first).unwrap().state, RequestState::Active);
        assert_eq!(sched.pending_request_count(), 3, "nan/1.0/3.0 all pending");

        // Drain one-at-a-time, recording activation order.
        let mut order = Vec::new();
        let mut current = id_first;
        loop {
            sched.complete(current);
            sched.update();
            let next = [id_low, id_mid, id_nan].into_iter().find(|id| {
                sched
                    .get_request(*id)
                    .map(|r| r.state == RequestState::Active)
                    .unwrap_or(false)
            });
            match next {
                Some(id) => {
                    order.push(id);
                    current = id;
                }
                None => break,
            }
        }
        // Best-first heap order: 1.0, 3.0, then NaN sinks to the bottom (last).
        assert_eq!(
            order,
            vec![id_low, id_mid, id_nan],
            "NaN priority must activate LAST (sunk to heap bottom), well-formed priorities keep their order"
        );
    }

    #[test]
    fn test_resource_build_url() {
        let resource = Resource::new("https://example.com/api")
            .with_query("key", "value")
            .with_query("format", "json");

        let url = resource.build_url();
        assert!(url.starts_with("https://example.com/api?"));
        assert!(url.contains("key=value"));
        assert!(url.contains("format=json"));
    }

    #[test]
    fn test_resource_derive() {
        let base = Resource::new("https://example.com/tileset.json");
        let derived = base.derive("tiles/tile.b3dm");
        assert_eq!(derived.url, "https://example.com/tiles/tile.b3dm");
    }

    #[test]
    fn test_extract_server_key() {
        assert_eq!(
            extract_server_key("https://example.com:443/path"),
            "example.com:443"
        );
        assert_eq!(
            extract_server_key("http://localhost:8080/api"),
            "localhost:8080"
        );
    }

    // ─── 门③: query multi-value ──────────────────────────────────────────

    #[test]
    fn test_append_and_remove_query_values() {
        let mut resource = Resource::new("https://example.com/api");
        resource.append_query_values(&[
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ]);
        assert_eq!(resource.query_parameters.len(), 2);
        // Overwrite existing key.
        resource.append_query_values(&[("a".to_string(), "9".to_string())]);
        assert_eq!(resource.query_parameters.get("a").unwrap(), "9");
        resource.remove_query_values(&["a"]);
        assert!(!resource.query_parameters.contains_key("a"));
        assert!(resource.query_parameters.contains_key("b"));
    }

    #[test]
    fn test_query_string_is_sorted_and_multi_key() {
        let resource = Resource::new("https://example.com/tile")
            .with_query("z", "3")
            .with_query("x", "1")
            .with_query("y", "2");
        let url = resource.build_url();
        // Sorted for determinism: x, y, z.
        assert!(url.find("x=1").unwrap() < url.find("y=2").unwrap());
        assert!(url.find("y=2").unwrap() < url.find("z=3").unwrap());
    }

    #[test]
    fn test_set_query_parameters_default_preserves_existing() {
        let mut resource = Resource::new("https://example.com/api").with_query("k", "original");
        resource.set_query_parameters(
            vec![
                ("k".to_string(), "new".to_string()),
                ("j".to_string(), "added".to_string()),
            ],
            true,
        );
        // use_as_default=true: existing key preserved, new key added.
        assert_eq!(resource.query_parameters.get("k").unwrap(), "original");
        assert_eq!(resource.query_parameters.get("j").unwrap(), "added");
    }

    // ─── URI classification / base uri ───────────────────────────────────

    #[test]
    fn test_is_data_and_blob_uri() {
        assert!(Resource::new("data:text/plain;base64,SGk=").is_data_uri());
        assert!(!Resource::new("https://example.com/x").is_data_uri());
        assert!(Resource::new("blob:https://example.com/uuid").is_blob_uri());
    }

    #[test]
    fn test_get_base_uri() {
        let resource = Resource::new("https://example.com:8080/a/b/c.json");
        assert_eq!(resource.get_base_uri(), "https://example.com:8080/");
    }

    #[test]
    fn test_clone_with_url_preserves_state() {
        let resource = Resource::new("https://example.com/a")
            .with_query("k", "v")
            .with_header("X-Token", "abc");
        let cloned = resource.clone_with_url("https://other.com/b");
        assert_eq!(cloned.url, "https://other.com/b");
        assert_eq!(cloned.query_parameters.get("k").unwrap(), "v");
        assert_eq!(cloned.headers.get("X-Token").unwrap(), "abc");
    }

    #[test]
    fn test_resource_display() {
        let resource = Resource::new("https://example.com/api").with_query("k", "v");
        assert_eq!(format!("{}", resource), "https://example.com/api?k=v");
    }

    // ─── FetchDescriptor construction ────────────────────────────────────

    #[test]
    fn test_fetch_descriptor_get() {
        let resource = Resource::new("https://example.com/tile.b3dm");
        let descriptor = resource.fetch_array_buffer(None);
        assert_eq!(descriptor.method, HttpMethod::Get);
        assert_eq!(descriptor.response_type, ResponseType::ArrayBuffer);
        assert_eq!(descriptor.url, "https://example.com/tile.b3dm");
        assert_eq!(descriptor.server_key, "example.com:443");
        assert!(!descriptor.is_data_uri);
    }

    #[test]
    fn test_fetch_descriptor_response_type_variants() {
        let resource = Resource::new("https://example.com/x");
        assert_eq!(resource.fetch_json(None).response_type, ResponseType::Json);
        assert_eq!(resource.fetch_text(None).response_type, ResponseType::Text);
        assert_eq!(resource.fetch_image(None).response_type, ResponseType::Image);
        assert_eq!(resource.fetch_blob(None).response_type, ResponseType::Blob);
    }

    #[test]
    fn test_fetch_descriptor_data_uri_short_circuits_proxy() {
        let resource = Resource::new("data:application/octet-stream;base64,QUJD");
        let proxy = crate::proxy::ProxyPolicy::with_proxy(crate::proxy::DefaultProxy::new(
            "https://proxy.example.com/".to_string(),
        ));
        let descriptor = resource.fetch_array_buffer(Some(&proxy));
        // Data URI: no proxy applied, empty server key, flagged inline.
        assert!(descriptor.is_data_uri);
        assert!(descriptor.url.starts_with("data:"));
        assert!(descriptor.server_key.is_empty());
    }

    #[test]
    fn test_fetch_descriptor_applies_proxy_for_untrusted() {
        let resource = Resource::new("https://untrusted.com/tile.png");
        let proxy = crate::proxy::ProxyPolicy::with_proxy(crate::proxy::DefaultProxy::new(
            "https://proxy.example.com/".to_string(),
        ));
        let descriptor = resource.fetch_image(Some(&proxy));
        assert!(descriptor.url.contains("proxy.example.com"));
        assert!(descriptor.url.contains("untrusted.com"));
    }

    #[test]
    fn test_post_descriptor_carries_body() {
        let resource = Resource::new("https://example.com/service");
        let descriptor = resource.post(vec![1, 2, 3], None);
        assert_eq!(descriptor.method, HttpMethod::Post);
        assert_eq!(descriptor.body, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_descriptor_to_request_server_key() {
        let resource = Resource::new("https://example.com/tile.b3dm");
        let descriptor = resource
            .fetch_array_buffer(None)
            .with_request_type(RequestType::Tiles3D)
            .with_priority(5.0);
        let request = descriptor.to_request();
        assert_eq!(request.request_type, RequestType::Tiles3D);
        assert_eq!(request.priority, 5.0);
        assert_eq!(request.server_key, "example.com:443");
    }

    // ─── Retry semantics ─────────────────────────────────────────────────

    #[test]
    fn test_retry_policy_default_single_attempt() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 1);
        // First transient failure retries immediately (no base delay).
        assert_eq!(
            policy.decide(0, RequestErrorClass::Transient),
            RetryDecision::RetryNow
        );
        // Second attempt exceeds max_attempts=1.
        assert_eq!(
            policy.decide(1, RequestErrorClass::Transient),
            RetryDecision::GiveUp
        );
    }

    #[test]
    fn test_retry_policy_permanent_never_retries() {
        let policy = RetryPolicy::exponential(5, 100);
        assert_eq!(
            policy.decide(0, RequestErrorClass::Permanent),
            RetryDecision::GiveUp
        );
    }

    #[test]
    fn test_retry_policy_exponential_backoff_delays() {
        let policy = RetryPolicy::exponential(5, 100);
        assert_eq!(
            policy.decide(0, RequestErrorClass::Transient),
            RetryDecision::RetryAfterMillis(100)
        );
        assert_eq!(
            policy.decide(1, RequestErrorClass::Transient),
            RetryDecision::RetryAfterMillis(200)
        );
        assert_eq!(
            policy.decide(2, RequestErrorClass::Transient),
            RetryDecision::RetryAfterMillis(400)
        );
    }

    #[test]
    fn test_retry_policy_delay_capped_at_max() {
        let mut policy = RetryPolicy::exponential(30, 1000);
        policy.max_delay_millis = 5000;
        assert_eq!(policy.delay_for(20), 5000);
    }

    #[test]
    fn test_retry_policy_throttled_honours_flag() {
        let mut policy = RetryPolicy::exponential(3, 100);
        assert_eq!(
            policy.decide(0, RequestErrorClass::Throttled),
            RetryDecision::RetryAfterMillis(100)
        );
        policy.retry_on_throttled = false;
        assert_eq!(
            policy.decide(0, RequestErrorClass::Throttled),
            RetryDecision::GiveUp
        );
    }

    #[test]
    fn test_classify_status() {
        assert_eq!(classify_status(429), RequestErrorClass::Throttled);
        assert_eq!(classify_status(404), RequestErrorClass::Permanent);
        assert_eq!(classify_status(500), RequestErrorClass::Transient);
        assert_eq!(classify_status(503), RequestErrorClass::Transient);
    }

    #[test]
    fn test_default_retry_callback_applies_policy() {
        let callback = default_retry_callback();
        let ctx = RetryContext::from_status("https://x.com/a", 0, 500, RetryPolicy::default());
        assert_eq!(callback(&ctx), RetryDecision::RetryNow);

        let ctx404 = RetryContext::from_status("https://x.com/a", 0, 404, RetryPolicy::default());
        assert_eq!(callback(&ctx404), RetryDecision::GiveUp);
    }

    // ─── 门③: statistics integration ─────────────────────────────────────

    #[test]
    fn test_scheduler_statistics_success() {
        let mut scheduler = RequestScheduler::new();
        let id = scheduler
            .schedule(Request::new(
                "https://example.com/a.b3dm".to_string(),
                RequestType::Tiles3D,
            ))
            .unwrap();
        assert_eq!(scheduler.statistics().attempted, 1);
        assert_eq!(scheduler.statistics().active, 1);
        scheduler.complete(id);
        assert_eq!(scheduler.statistics().succeeded, 1);
        assert_eq!(scheduler.statistics().active, 0);
        assert_eq!(
            scheduler.statistics().active_for_type(RequestType::Tiles3D),
            0
        );
    }

    #[test]
    fn test_scheduler_statistics_failure() {
        let mut scheduler = RequestScheduler::new();
        let id = scheduler
            .schedule(Request::new(
                "https://example.com/a".to_string(),
                RequestType::Imagery,
            ))
            .unwrap();
        scheduler.fail(id);
        assert_eq!(scheduler.statistics().failed, 1);
        assert_eq!(scheduler.statistics().active, 0);
    }

    #[test]
    fn test_scheduler_statistics_cancel_active_vs_pending() {
        // Active cancellation.
        let mut scheduler = RequestScheduler::new();
        let id = scheduler
            .schedule(Request::new(
                "https://example.com/a".to_string(),
                RequestType::Other,
            ))
            .unwrap();
        scheduler.cancel(id);
        assert_eq!(scheduler.statistics().cancelled_active, 1);

        // Pending cancellation (forced into heap by maximum_requests=0).
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0;
        let id = scheduler
            .schedule(Request::throttled(
                "https://example.com/b".to_string(),
                RequestType::Other,
                1.0,
            ))
            .unwrap();
        scheduler.cancel(id);
        assert_eq!(scheduler.statistics().cancelled_pending, 1);
    }

    #[test]
    fn test_scheduler_statistics_reset() {
        let mut scheduler = RequestScheduler::new();
        scheduler
            .schedule(Request::new(
                "https://example.com/a".to_string(),
                RequestType::Other,
            ))
            .unwrap();
        scheduler.reset_statistics();
        assert_eq!(scheduler.statistics().attempted, 0);
    }

    // ─── Deferred promotion ──────────────────────────────────────────────

    #[test]
    fn test_scheduler_deferred_when_heap_saturated() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0; // nothing can activate
        scheduler.priority_heap_length = 1;

        // First throttled request fills the heap (priority 1.0).
        scheduler
            .schedule(Request::throttled(
                "https://example.com/a".to_string(),
                RequestType::Other,
                1.0,
            ))
            .unwrap();
        assert_eq!(scheduler.pending_request_count(), 1);

        // Second request has **better** priority (0.5 < 1.0), so under the
        // CesiumJS `PriorityQueue.insert` rule it evicts the worst resident
        // (the p=1.0 request) into the M8.1 deferred queue and takes its heap
        // slot. This is the eviction path that keeps deferred-promotion alive
        // while still honouring the spec's rejection of worse-priority
        // newcomers (see `test_scheduler_rejects_worse_priority_when_heap_full`).
        scheduler
            .schedule(Request::throttled(
                "https://example.com/b".to_string(),
                RequestType::Other,
                0.5,
            ))
            .unwrap();
        assert_eq!(scheduler.deferred_count(), 1);
        assert_eq!(scheduler.pending_request_count(), 1);

        // Growing the heap and updating promotes the deferred request back.
        scheduler.priority_heap_length = 2;
        scheduler.update();
        assert_eq!(scheduler.deferred_count(), 0);
        assert_eq!(scheduler.pending_request_count(), 2);
    }

    #[test]
    fn test_scheduler_rejects_when_deferred_saturated() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0;
        scheduler.priority_heap_length = 1;
        scheduler.maximum_deferred = 1;

        // r1 (p=1.0) fills the heap.
        scheduler
            .schedule(Request::throttled(
                "https://example.com/a".to_string(),
                RequestType::Other,
                1.0,
            ))
            .unwrap();
        // r2 (p=0.5) is better → evicts r1 into the deferred queue (now full).
        scheduler
            .schedule(Request::throttled(
                "https://example.com/b".to_string(),
                RequestType::Other,
                0.5,
            ))
            .unwrap();
        assert_eq!(scheduler.deferred_count(), 1);
        // r3 (p=2.0) is worse than the heap resident (p=0.5) → rejected
        // outright by the CesiumJS priority rule, regardless of deferred room.
        assert!(scheduler
            .schedule(Request::throttled(
                "https://example.com/c".to_string(),
                RequestType::Other,
                2.0,
            ))
            .is_none());
    }

    /// CesiumJS spec alignment: a newcomer whose priority is **not strictly
    /// better** than the worst resident of a saturated heap is rejected
    /// outright (never deferred). This is the rule that
    /// `specs/tests/core/request_scheduler_spec.rs::honors_priority_heap_length`
    /// and `::handles_low_priority_requests` assert against.
    #[test]
    fn test_scheduler_rejects_worse_priority_when_heap_full() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0;
        scheduler.priority_heap_length = 1;

        scheduler
            .schedule(Request::throttled(
                "https://example.com/a".to_string(),
                RequestType::Other,
                0.0,
            ))
            .unwrap();
        // Worse priority (1.0 > 0.0) → rejected, not deferred.
        assert!(scheduler
            .schedule(Request::throttled(
                "https://example.com/b".to_string(),
                RequestType::Other,
                1.0,
            ))
            .is_none());
        assert_eq!(scheduler.deferred_count(), 0);
        assert_eq!(scheduler.pending_request_count(), 1);

        // Equal priority (0.0 == 0.0) is also rejected (not *strictly* better).
        assert!(scheduler
            .schedule(Request::throttled(
                "https://example.com/c".to_string(),
                RequestType::Other,
                0.0,
            ))
            .is_none());
    }

    // ─── Priority function integration ───────────────────────────────────

    #[test]
    fn test_scheduler_priority_function_recomputes() {
        use crate::priority::{FrameContext, PriorityKey, SsedPriority};

        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0; // keep everything pending
        scheduler.set_priority_function(Box::new(SsedPriority::new()));
        assert_eq!(scheduler.priority_function_name(), Some("SSED"));

        let key = PriorityKey::new(0, 0, 15)
            .with_center(0.0, 0.0, 6_378_137.0 - 5000.0)
            .with_geometric_error(50.0);
        let id = scheduler
            .schedule(
                Request::throttled(
                    "https://example.com/tile".to_string(),
                    RequestType::Tiles3D,
                    999.0,
                )
                .with_priority_key(key),
            )
            .unwrap();

        let context = FrameContext::new().with_camera(0.0, 0.0, 6_378_137.0);
        scheduler.update_with_context(&context);

        // The stored priority was recomputed by the SSED function (no longer 999).
        let request = scheduler.get_request(id).unwrap();
        assert!((request.priority - 999.0).abs() > f64::EPSILON);
    }

    #[test]
    fn test_scheduler_update_without_priority_function_is_noop_recompute() {
        let mut scheduler = RequestScheduler::new();
        scheduler.maximum_requests = 0;
        let id = scheduler
            .schedule(Request::throttled(
                "https://example.com/a".to_string(),
                RequestType::Other,
                7.0,
            ))
            .unwrap();
        // No priority function installed: priority unchanged.
        scheduler.update_with_context(&crate::priority::FrameContext::new());
        assert_eq!(scheduler.get_request(id).unwrap().priority, 7.0);
    }
}
