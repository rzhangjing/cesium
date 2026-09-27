//! cesium-crs: Coordinate Reference Systems and projections.
//!
//! STATUS (P2 code-health audit, 2026-09-27): implemented and covered by the
//! `cesium-specs` suite, but **not wired into any production runtime path** — no
//! adapter or application crate depends on it. Retained as a CesiumJS
//! feature-parity domain model reserved for future adapter bridging; do NOT read
//! it as a shipped capability. See docs/ARCHITECTURE.md "Test-only domain crates".
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! Implements:
//! - Map projections (Web Mercator, UTM, Polar Stereographic, Equirectangular)
//! - Datum definitions and transformations (WGS84, CGCS2000, ITRF, NAD83)
//! - Helmert/Molodensky coordinate transformations

pub mod datum;
pub mod projections;

pub use datum::{
    Datum, DatumConverter, HelmertTransform, MolodenskyTransform,
    get_helmert_transform, transform_ecef,
};
pub use projections::{
    Equirectangular, GeographicCoordinate, PolarStereographic, ProjectedCoordinate,
    Utm, UtmZone, WebMercator,
};
