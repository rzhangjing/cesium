//! Ported from `packages/engine/Source/Core/GeocoderService.js` (60 lines).
//!
//! The geocoder-service interface plus the shared result types used by all
//! concrete geocoder services (Pelias / Ion / Bing / Google / OpenCage /
//! Cartographic).

use crate::cartesian3::Cartesian3;
use crate::credit::Credit;
use crate::geocode_type::GeocodeType;
use crate::rectangle::Rectangle;

/// An attribution entry of a geocoder result, mirroring the JS
/// `GeocoderService.Result.attributions[]` objects (`html` / `collapsible`).
#[derive(Debug, Clone, PartialEq)]
pub struct GeocoderAttribution {
    /// The attribution HTML.
    pub html: String,
    /// Whether the credit may be collapsed (`collapsible`).
    pub collapsible: Option<bool>,
}

/// The bounding box / point destination of a geocoder result, mirroring the
/// JS `Rectangle | Cartesian3` `destination`.
#[derive(Debug, Clone)]
pub enum GeocodeDestination {
    /// A bounding box (`Rectangle`).
    Rectangle(Rectangle),
    /// A single point (`Cartesian3`).
    Cartesian3(Cartesian3),
}

impl PartialEq for GeocodeDestination {
    /// Field-wise equality (`Rectangle` has no derived `PartialEq`).
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cartesian3(a), Self::Cartesian3(b)) => a == b,
            (Self::Rectangle(a), Self::Rectangle(b)) => {
                a.west == b.west
                    && a.south == b.south
                    && a.east == b.east
                    && a.north == b.north
            }
            _ => false,
        }
    }
}

/// A single geocoder result, mirroring `GeocoderService.Result`.
#[derive(Debug, Clone)]
pub struct GeocoderResult {
    /// The display name for a location (`displayName`).
    pub display_name: String,
    /// The bounding box or point for a location (`destination`).
    pub destination: GeocodeDestination,
    /// The result attributions, if present (`attributions`).
    pub attributions: Option<Vec<GeocoderAttribution>>,
    /// The singular `attribution` key used by the JS Google geocoder result
    /// mapping (unused by `getCreditsFromResult`, mirrored for fidelity).
    pub attribution: Option<GeocoderAttribution>,
}

/// Provides geocoding through an external service.
///
/// Port of the `GeocoderService` interface type. The JS constructor throws
/// an instantiation error (interface-only); the Rust analogue is a trait.
///
/// DEVIATION: JS `geocode` is promise-based; the port is synchronous
/// (services resolve results directly), matching the workspace convention
/// for headless backends.
pub trait GeocoderService {
    /// Gets the credit to display after a geocode is performed. Typically
    /// this is used to credit the geocoder service (`credit` property).
    fn credit(&self) -> Option<Credit>;

    /// Performs the geocode query (`geocode(query, type)`).
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult>;
}

/// Parses credits from the geocoder result attributions, if present.
///
/// Port of `GeocoderService.getCreditsFromResult`.
pub fn get_credits_from_result(geocoder_result: &GeocoderResult) -> Option<Vec<Credit>> {
    geocoder_result
        .attributions
        .as_ref()
        .map(|attributions| attributions.iter().map(Credit::get_ion_credit).collect())
}
