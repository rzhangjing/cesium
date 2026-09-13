//! cesium-crs: Coordinate Reference Systems and projections.
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
