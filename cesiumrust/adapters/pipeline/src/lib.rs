//! cesium-pipeline — Generic tile pipeline adapter (M1.2).
//!
//! Bevy-free, pure `std` implementation of the M1.1 `TilePipeline` contract
//! from `cesium-ports-driven`. Faithfully replicates the budget/eviction/
//! staleness/retry semantics of `dynamic_globe.rs` (the golden-path reference)
//! in a reusable, testable crate.
//!
//! ## Architecture
//!
//! ```text
//!  submit(key, prio)          poll_ready(budget)
//!       │                          ▲
//!       ▼                          │
//!  ┌─────────┐   job_tx    ┌──────────────┐   result_tx   ┌──────────┐
//!  │ Dedup   │────────────►│ Worker Pool  │──────────────►│ Drain    │
//!  │ + Wanted│             │ (N threads)  │               │ Queue    │
//!  └─────────┘             └──────────────┘               └──────────┘
//!       ▲                          │
//!       │                          ▼
//!  refresh_wanted()         NetworkBackend (ureq / reqwest)
//! ```
//!
//! ## Module map
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | `runtime` | `GenericPipeline<K,P>` — the `TilePipeline` impl |
//! | `pool` | Blocking worker pool (16 threads, keep-alive) |
//! | `budget` | `DefaultBudget` — verbatim dynamic_globe constants |
//! | `gpu_cache` | FIFO eviction with base-layer lock + live deferral |
//! | `hidden_lru` | LRU tracking for hidden (warm fallback) entities |
//! | `dedup` | In-flight deduplication set |
//! | `base_layer` | Base-layer zoom exemption logic |
//! | `retry` | `DefaultRetry` — dual-timescale retry policy |
//! | `staleness` | `DefaultStaleness` — three-state result classification |
//! | `net` | `NetworkBackend` trait + ureq/reqwest impls |
//! | `resource_backend` | M8 `ResourceBackend` — bulk asset streaming via the reused cache hierarchy |

pub mod base_layer;
pub mod budget;
pub mod dedup;
pub mod gpu_cache;
pub mod hidden_lru;
pub mod net;
pub mod pool;
pub mod resource_backend;
pub mod retry;
pub mod runtime;
pub mod staleness;

// Re-export the core pipeline type + default policy impls for convenience.
pub use budget::DefaultBudget;
pub use resource_backend::PipelineResourceBackend;
pub use retry::DefaultRetry;
pub use runtime::GenericPipeline;
pub use staleness::DefaultStaleness;
