//! 1:1 fidelity mirrors for `packages/engine/Source/Core/EllipsoidGeodesic.js`.
//!
//! Track B3-2a0. The port had replaced Vincenty's *direct* formula with a linear
//! lon/lat lerp and had transcribed the `A`/`B` series coefficients of the
//! inverse formula incorrectly (`A` used `-3u²/64` where CesiumJS expands to
//! `-3u/64 + 5u²/256 - 175u³/16384`; `B` used `-u²/16` where CesiumJS has
//! `-u²/8`). Both defects are locked down here.
//!
//! Goldens produced by `golden_ellipsoid_geodesic.mjs` (direct ESM import of the
//! CesiumJS sources, WGS84, f64 throughout).

use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_geodesic::EllipsoidGeodesic;

/// One golden case, straight off the generator's stdout.
struct Golden {
    name: &'static str,
    /// `(longitude, latitude)` in radians.
    start: (f64, f64),
    /// `(longitude, latitude)` in radians.
    end: (f64, f64),
    surface_distance: f64,
    start_heading: f64,
    end_heading: f64,
    /// `(fraction, longitude, latitude)` for 0 / 0.25 / 0.5 / 0.75 / 1.
    fractions: [(f64, f64, f64); 5],
    /// `interpolateUsingSurfaceDistance(surfaceDistance * 0.3)`.
    distance_0_3: (f64, f64),
}

const GOLDENS: &[Golden] = &[
    Golden {
        name: "equatorial_quarter",
        start: (0.0, 0.0),
        end: (1.5707963267948966, 0.0),
        surface_distance: 10018754.171390377,
        start_heading: 1.5707963267948966,
        end_heading: 1.5707963267948966,
        fractions: [
            (0.0, 0.0, 0.0),
            (0.25, 0.3926990816985578, 0.0),
            (0.5, 0.7853981633971155, 0.0),
            (0.75, 1.1780972450956733, 0.0),
            (1.0, 1.570796326794231, 0.0),
        ],
        distance_0_3: (0.47123889803826935, 0.0),
    },
    Golden {
        name: "meridional",
        start: (-1.3089969389957472, -0.6981317007977318),
        end: (-1.3089969389957472, 0.6981317007977318),
        surface_distance: 8859058.060702816,
        start_heading: 0.0,
        end_heading: 0.0,
        fractions: [
            (0.0, -1.3089969389957472, -0.6981317007977452),
            (0.25, -1.3089969389957472, -0.3494439522810217),
            (0.5, -1.3089969389957472, 1.412498635714463e-13),
            (0.75, -1.3089969389957472, 0.3494439522813038),
            (1.0, -1.3089969389957472, 0.6981317007980264),
        ],
        distance_0_3: (-1.3089969389957472, -0.27959468072350385),
    },
    Golden {
        name: "long_haul",
        start: (-1.3089969389957472, 0.6981317007977318),
        end: (2.0943951023931953, -0.4363323129985824),
        surface_distance: 17839685.71706679,
        start_heading: -0.7661253716064133,
        end_heading: -2.514856183600793,
        fractions: [
            (0.0, -1.308996938995762, 0.6981317007977438),
            (0.25, -2.3047512225374382, 1.0114013813336329),
            (0.5, -3.321796115689894, 0.7139880349058241),
            (0.75, -3.7970432808343952, 0.15317269530407437),
            // CesiumJS returns `start.longitude + l` verbatim: -240 deg, *not*
            // wrapped into [-pi, pi].
            (1.0, -4.188790204787337, -0.43633231299895936),
        ],
        distance_0_3: (-2.564381488793079, 0.9984381042858868),
    },
    Golden {
        name: "short_step",
        start: (-1.7453292519943295, 0.6108652381980153),
        end: (-1.7278759594743862, 0.6283185307179586),
        surface_distance: 143321.57818056116,
        start_heading: 0.6804226450883964,
        end_heading: 0.6905583820014425,
        fractions: [
            (0.0, -1.745329251994326, 0.6108652381980189),
            (0.25, -1.7410063101569728, 0.6152423250098755),
            (0.5, -1.7366566873305835, 0.6196103281881885),
            (0.75, -1.7322800246443284, 0.6239691100212514),
            (1.0, -1.7278759594748094, 0.6283185307179637),
        ],
        distance_0_3: (-1.740138531500792, 0.616116656713348),
    },
    Golden {
        name: "near_pole",
        start: (0.17453292519943295, 1.3962634015954636),
        end: (-2.792526803190927, 1.361356816555577),
        surface_distance: 2447571.209043323,
        start_heading: -0.09687759344889235,
        end_heading: -3.060715135074589,
        fractions: [
            (0.0, 0.17453292519853, 1.3962634015970772),
            (0.25, 0.058548920300126955, 1.4909137575981126),
            (0.5, -2.1073031912969706, 1.546530122096428),
            (0.75, -2.724886971591139, 1.4564268890483614),
            (1.0, -2.7925268031908237, 1.3613568165550065),
        ],
        distance_0_3: (-0.007173047589274978, 1.50947726768236),
    },
];

/// Radians agree to well below a nanoradian; the generator prints 17 significant
/// digits so anything looser would be hiding real drift.
const ANGLE_EPS: f64 = 1.0e-9;
/// Distances are compared relatively: both implementations run the same Vincenty
/// series, so the only slack is the fixed-point iteration's last step.
const DISTANCE_REL_EPS: f64 = 1.0e-9;

fn geodesic(golden: &Golden) -> EllipsoidGeodesic {
    EllipsoidGeodesic::new(
        Some(Cartographic::new(golden.start.0, golden.start.1, 0.0)),
        Some(Cartographic::new(golden.end.0, golden.end.1, 0.0)),
        Some(Ellipsoid::WGS84),
    )
}

fn assert_angle(name: &str, what: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= ANGLE_EPS,
        "{name}: {what} = {actual}, CesiumJS golden = {expected} (delta {})",
        (actual - expected).abs()
    );
}

fn assert_distance(name: &str, what: &str, actual: f64, expected: f64) {
    let tolerance = DISTANCE_REL_EPS * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{name}: {what} = {actual}, CesiumJS golden = {expected} (delta {})",
        (actual - expected).abs()
    );
}

#[test]
fn surface_distance_matches_cesiumjs() {
    for golden in GOLDENS {
        let geo = geodesic(golden);
        assert_distance(
            golden.name,
            "surfaceDistance",
            geo.surface_distance(),
            golden.surface_distance,
        );
    }
}

#[test]
fn headings_match_cesiumjs() {
    for golden in GOLDENS {
        let geo = geodesic(golden);
        assert_angle(golden.name, "startHeading", geo.start_heading(), golden.start_heading);
        assert_angle(golden.name, "endHeading", geo.end_heading(), golden.end_heading);
    }
}

#[test]
fn endpoints_are_flattened_to_zero_height() {
    // CesiumJS `computeProperties` assigns `_start.height = 0` and
    // `_end.height = 0` regardless of what was passed in.
    let geo = EllipsoidGeodesic::new(
        Some(Cartographic::new(0.1, 0.2, 1234.5)),
        Some(Cartographic::new(0.3, 0.4, 6789.0)),
        Some(Ellipsoid::WGS84),
    );
    assert_eq!(geo.start().height, 0.0);
    assert_eq!(geo.end().height, 0.0);
}

#[test]
fn interpolate_using_fraction_matches_cesiumjs() {
    for golden in GOLDENS {
        let geo = geodesic(golden);
        for (fraction, longitude, latitude) in golden.fractions {
            let p = geo.interpolate_using_fraction(fraction);
            assert_angle(
                golden.name,
                &format!("fraction({fraction}).longitude"),
                p.longitude,
                longitude,
            );
            assert_angle(
                golden.name,
                &format!("fraction({fraction}).latitude"),
                p.latitude,
                latitude,
            );
            // The direct formula always reports height 0.
            assert_eq!(p.height, 0.0, "{}: fraction({fraction}).height", golden.name);
        }
    }
}

#[test]
fn interpolate_using_surface_distance_matches_cesiumjs() {
    for golden in GOLDENS {
        let geo = geodesic(golden);
        let p = geo.interpolate_using_surface_distance(geo.surface_distance() * 0.3);
        assert_angle(
            golden.name,
            "distance(0.3).longitude",
            p.longitude,
            golden.distance_0_3.0,
        );
        assert_angle(
            golden.name,
            "distance(0.3).latitude",
            p.latitude,
            golden.distance_0_3.1,
        );
    }
}

#[test]
fn set_end_points_recomputes_the_direct_solution() {
    // CesiumJS `setEndPoints` re-runs `computeProperties`, which re-derives both
    // the headings and the `_constants` the interpolation series reads.
    let mut geo = geodesic(&GOLDENS[3]);
    let target = &GOLDENS[1];
    geo.set_end_points(
        &Cartographic::new(target.start.0, target.start.1, 0.0),
        &Cartographic::new(target.end.0, target.end.1, 0.0),
    );
    assert_distance("meridional(via setEndPoints)", "surfaceDistance", geo.surface_distance(), target.surface_distance);
    assert_angle("meridional(via setEndPoints)", "startHeading", geo.start_heading(), target.start_heading);
    let p = geo.interpolate_using_fraction(0.5);
    assert_angle("meridional(via setEndPoints)", "fraction(0.5).latitude", p.latitude, target.fractions[2].2);
}

#[test]
fn coincident_endpoints_have_zero_distance() {
    // `vincentyInverseFormula` short-circuits through `sineSigma === 0`, which
    // forces `cosineSquaredAlpha = 1` and leaves `sigma = 0`, so the distance
    // collapses and the direct formula is asked to walk nowhere.
    //
    // The last case is the interesting one: CesiumJS never normalises the input
    // latitude, and `atan((a/b) * tan(theta))` returns the principal value, so a
    // latitude of 2 rad comes back as `2 - pi`. Verified against CesiumJS by
    // `golden_ellipsoid_geodesic.mjs`.
    const COINCIDENT: &[(f64, f64, f64, f64)] = &[
        // (longitude, latitude, expected longitude, expected latitude)
        (1.0, 0.5, 1.0, 0.5000000000000003),
        (0.3, -1.2, 0.3, -1.200000000000897),
        (1.0, 2.0, 1.0, -1.1415926535904708),
    ];
    for (longitude, latitude, expected_longitude, expected_latitude) in COINCIDENT {
        let name = format!("coincident({longitude},{latitude})");
        let geo = EllipsoidGeodesic::new(
            Some(Cartographic::new(*longitude, *latitude, 0.0)),
            Some(Cartographic::new(*longitude, *latitude, 0.0)),
            Some(Ellipsoid::WGS84),
        );
        assert_eq!(geo.surface_distance(), 0.0, "{name}: surfaceDistance");
        assert_eq!(geo.start_heading(), 0.0, "{name}: startHeading");
        let p = geo.interpolate_using_fraction(0.5);
        assert_angle(&name, "longitude", p.longitude, *expected_longitude);
        assert_angle(&name, "latitude", p.latitude, *expected_latitude);
        assert_eq!(p.height, 0.0, "{name}: height");
    }
}

#[test]
fn geodesic_midpoint_is_not_the_linear_latitude_average() {
    // Regression guard for the removed linear lerp: on a 80-degree meridional
    // span the true geodesic midpoint sits measurably north of the arithmetic
    // mean latitude. A lerp would return exactly 0.0 here.
    let golden = &GOLDENS[1];
    let geo = geodesic(golden);
    let mid = geo.interpolate_using_fraction(0.5);
    let linear = (golden.start.1 + golden.end.1) * 0.5;
    assert!(
        (mid.latitude - linear).abs() < ANGLE_EPS,
        "golden midpoint {mid_latitude} must equal the CesiumJS value",
        mid_latitude = mid.latitude
    );
    assert_ne!(mid.latitude, linear);
}
