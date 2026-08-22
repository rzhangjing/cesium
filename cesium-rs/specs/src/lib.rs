//! Mirror test container for the CesiumJS Jasmine spec suite.
//!
//! Every Rust test file under `tests/` mirrors a Jasmine spec under the
//! CesiumJS repository (`Specs/` and the per-module `Specs/` folders of
//! `packages/engine`), one `#[test]` per original `it(...)`. The ported
//! specs are tracked in `docs/MAPPING.md`.
//!
//! This library part only provides shared helpers (test-data resolution);
//! the actual tests live in `tests/` and use the `cesium-*` crates via
//! dev-dependencies.

pub mod data;

pub use data::{data_path, specs_data_root};

use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());

/// Global lock for specs that mutate process-wide state (e.g. the
/// `oneTimeWarning` sink). The CesiumJS suite runs sequentially; cargo runs
/// tests in parallel, so ported specs that touch shared globals must hold
/// this guard for their whole body.
#[must_use]
pub fn spec_serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
