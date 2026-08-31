//! Ported from `packages/engine/Source/Core/RectangleGeometryLibrary.js`.
//!
//! Shared computation for `RectangleGeometry` and `RectangleOutlineGeometry`:
//! grid options (with optional rotation / texture-coordinate rotation) and
//! per-vertex position/ST computation.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::geographic_projection::GeographicProjection;
use crate::math::CesiumMath;
use crate::matrix2::Matrix2;
use crate::rectangle::Rectangle;

/// The options object produced by
/// [`RectangleGeometryLibrary::compute_options`] (JS anonymous object).
#[derive(Debug, Clone)]
pub struct ComputedOptions {
    pub gran_y_cos: f64,
    pub gran_y_sin: f64,
    pub gran_x_cos: f64,
    pub gran_x_sin: f64,
    pub nw_corner: Cartographic,
    pub bounding_rectangle: Rectangle,
    pub width: usize,
    pub height: usize,
    pub north_cap: bool,
    pub south_cap: bool,
    /// Set by `RectangleGeometry` after `compute_options` (JS monkey-patches
    /// the returned object): `1.0 / rectangle.width`.
    pub lon_scalar: f64,
    /// `1.0 / rectangle.height` (see `lon_scalar`).
    pub lat_scalar: f64,
    /// ST-rotation fields — only populated when `st_rotation != 0`.
    pub st_gran_y_cos: f64,
    pub st_gran_y_sin: f64,
    pub st_gran_x_cos: f64,
    pub st_gran_x_sin: f64,
    pub st_nw_corner: Option<Cartographic>,
    pub st_west: f64,
    pub st_south: f64,
}

/// Port of `RectangleGeometryLibrary.computePosition`.
pub fn compute_position(
    computed_options: &ComputedOptions,
    ellipsoid: &Ellipsoid,
    compute_st: bool,
    row: f64,
    col: f64,
    position: &mut Cartesian3,
    st: &mut Cartesian2,
) {
    let radii_squared = ellipsoid.radii_squared();
    let nw_corner = &computed_options.nw_corner;
    let rectangle = &computed_options.bounding_rectangle;

    let mut st_latitude = nw_corner.latitude - computed_options.gran_y_cos * row
        + col * computed_options.gran_x_sin;
    let cos_latitude = st_latitude.cos();
    let n_z = st_latitude.sin();
    let k_z = radii_squared.z * n_z;

    let mut st_longitude = nw_corner.longitude
        + row * computed_options.gran_y_sin
        + col * computed_options.gran_x_cos;
    let n_x = cos_latitude * st_longitude.cos();
    let n_y = cos_latitude * st_longitude.sin();

    let k_x = radii_squared.x * n_x;
    let k_y = radii_squared.y * n_y;

    let gamma = (k_x * n_x + k_y * n_y + k_z * n_z).sqrt();

    position.x = k_x / gamma;
    position.y = k_y / gamma;
    position.z = k_z / gamma;

    if compute_st {
        if let Some(st_nw_corner) = &computed_options.st_nw_corner {
            st_latitude = st_nw_corner.latitude - computed_options.st_gran_y_cos * row
                + col * computed_options.st_gran_x_sin;
            st_longitude = st_nw_corner.longitude
                + row * computed_options.st_gran_y_sin
                + col * computed_options.st_gran_x_cos;

            st.x = (st_longitude - computed_options.st_west) * computed_options.lon_scalar;
            st.y = (st_latitude - computed_options.st_south) * computed_options.lat_scalar;
        } else {
            st.x = (st_longitude - rectangle.west) * computed_options.lon_scalar;
            st.y = (st_latitude - rectangle.south) * computed_options.lat_scalar;
        }
    }
}

/// JS `getRotationOptions` result (anonymous object).
struct RotationOptions {
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    gran_y_cos: f64,
    gran_y_sin: f64,
    gran_x_cos: f64,
    gran_x_sin: f64,
    nw_corner: Cartographic,
}

/// JS `getRotationOptions` helper.
fn get_rotation_options(
    nw_corner: &Cartographic,
    rotation: f64,
    granularity_x: f64,
    granularity_y: f64,
    center_cartesian: &Cartesian3,
    width: usize,
    height: usize,
) -> RotationOptions {
    let cos_rotation = rotation.cos();
    let gran_y_cos = granularity_y * cos_rotation;
    let gran_x_cos = granularity_x * cos_rotation;

    let sin_rotation = rotation.sin();
    let gran_y_sin = granularity_y * sin_rotation;
    let gran_x_sin = granularity_x * sin_rotation;

    // DEVIATION: JS reuses a module-level `GeographicProjection` and resets
    // its ellipsoid to `Ellipsoid.default`; this port creates one locally.
    let proj = GeographicProjection::new(None);
    let mut nw_cartesian = Cartesian3::ZERO;
    proj.project_into(nw_corner, &mut nw_cartesian);

    // DEVIATION: JS does these in place (`subtract(nw, center, nw)`);
    // Rust needs distinct temporaries.
    let mut rel = Cartesian3::ZERO;
    Cartesian3::subtract(&nw_cartesian, center_cartesian, &mut rel);
    nw_cartesian = rel;
    let rotation_matrix = Matrix2::from_rotation_new(rotation);
    // DEVIATION: JS applies the 2×2 rotation to the x/y components of the
    // Cartesian3 in place (z untouched); the projected z is the height (0)
    // and never influences the following `unproject` result.
    let rel_2d = Cartesian2::new(nw_cartesian.x, nw_cartesian.y);
    let mut rotated_2d = Cartesian2::default();
    Matrix2::multiply_by_vector(&rotation_matrix, &rel_2d, &mut rotated_2d);
    nw_cartesian.x = rotated_2d.x;
    nw_cartesian.y = rotated_2d.y;
    let mut abs = Cartesian3::ZERO;
    Cartesian3::add(&nw_cartesian, center_cartesian, &mut abs);
    nw_cartesian = abs;
    let mut nw_corner = Cartographic::default();
    proj.unproject_into(&nw_cartesian, &mut nw_corner);

    let width = (width - 1) as f64;
    let height = (height - 1) as f64;

    let latitude = nw_corner.latitude;
    let latitude0 = latitude + width * gran_x_sin;
    let latitude1 = latitude - gran_y_cos * height;
    let latitude2 = latitude - gran_y_cos * height + width * gran_x_sin;

    let north = latitude.max(latitude0).max(latitude1).max(latitude2);
    let south = latitude.min(latitude0).min(latitude1).min(latitude2);

    let longitude = nw_corner.longitude;
    let longitude0 = longitude + width * gran_x_cos;
    let longitude1 = longitude + height * gran_y_sin;
    let longitude2 = longitude + height * gran_y_sin + width * gran_x_cos;

    let east = longitude.max(longitude0).max(longitude1).max(longitude2);
    let west = longitude.min(longitude0).min(longitude1).min(longitude2);

    RotationOptions {
        north,
        south,
        east,
        west,
        gran_y_cos,
        gran_y_sin,
        gran_x_cos,
        gran_x_sin,
        nw_corner,
    }
}

/// Port of `RectangleGeometryLibrary.computeOptions`.
///
/// DEVIATION: JS takes three scratch objects
/// (`boundingRectangleScratch`, `nwCornerResult`, `stNwCornerResult`) which
/// are absorbed into the returned [`ComputedOptions`] here.
pub fn compute_options(
    rectangle: &Rectangle,
    granularity: f64,
    mut rotation: f64,
    st_rotation: f64,
) -> ComputedOptions {
    let east = rectangle.east;
    let west = rectangle.west;
    let north = rectangle.north;
    let south = rectangle.south;

    let north_cap = north == CesiumMath::PI_OVER_TWO;
    let south_cap = south == -CesiumMath::PI_OVER_TWO;

    let dx = if west > east {
        CesiumMath::TWO_PI - west + east
    } else {
        east - west
    };
    let dy = north - south;

    let width = (dx / granularity).ceil() as usize + 1;
    let height = (dy / granularity).ceil() as usize + 1;
    let granularity_x = dx / (width - 1) as f64;
    let granularity_y = dy / (height - 1) as f64;

    let nw_corner = Rectangle::northwest(rectangle);
    let mut center = Rectangle::center(rectangle);
    let mut center_cartesian = Cartesian3::ZERO;
    if rotation != 0.0 || st_rotation != 0.0 {
        if center.longitude < nw_corner.longitude {
            center.longitude += CesiumMath::TWO_PI;
        }
        let proj = GeographicProjection::new(None);
        proj.project_into(&center, &mut center_cartesian);
    }

    let gran_y_cos = granularity_y;
    let gran_x_cos = granularity_x;
    let gran_y_sin = 0.0;
    let gran_x_sin = 0.0;

    let bounding_rectangle = *rectangle;

    let mut computed_options = ComputedOptions {
        gran_y_cos,
        gran_y_sin,
        gran_x_cos,
        gran_x_sin,
        nw_corner,
        bounding_rectangle,
        width,
        height,
        north_cap,
        south_cap,
        lon_scalar: 0.0,
        lat_scalar: 0.0,
        st_gran_y_cos: 0.0,
        st_gran_y_sin: 0.0,
        st_gran_x_cos: 0.0,
        st_gran_x_sin: 0.0,
        st_nw_corner: None,
        st_west: 0.0,
        st_south: 0.0,
    };

    if rotation != 0.0 {
        let rotation_options = get_rotation_options(
            &nw_corner,
            rotation,
            granularity_x,
            granularity_y,
            &center_cartesian,
            width,
            height,
        );
        let north = rotation_options.north;
        let south = rotation_options.south;
        let east = rotation_options.east;
        let west = rotation_options.west;

        if cfg!(debug_assertions) {
            if north < -CesiumMath::PI_OVER_TWO
                || north > CesiumMath::PI_OVER_TWO
                || south < -CesiumMath::PI_OVER_TWO
                || south > CesiumMath::PI_OVER_TWO
            {
                throw_developer_error(
                    "Rotated rectangle is invalid.  It crosses over either the north or south pole.",
                );
            }
        }

        computed_options.gran_y_cos = rotation_options.gran_y_cos;
        computed_options.gran_y_sin = rotation_options.gran_y_sin;
        computed_options.gran_x_cos = rotation_options.gran_x_cos;
        computed_options.gran_x_sin = rotation_options.gran_x_sin;
        // JS aliasing: `rotationOptions.nwCorner` IS the same scratch object
        // as `computedOptions.nwCorner` (`nwCornerResult`), so the in-place
        // project/rotate/unproject inside `getRotationOptions` already
        // rotated it. The value port must write the rotated corner back.
        computed_options.nw_corner = rotation_options.nw_corner;

        computed_options.bounding_rectangle.north = north;
        computed_options.bounding_rectangle.south = south;
        computed_options.bounding_rectangle.east = east;
        computed_options.bounding_rectangle.west = west;
    }

    if st_rotation != 0.0 {
        rotation -= st_rotation;
        let st_nw_corner = Rectangle::northwest(&computed_options.bounding_rectangle);

        let st_rotation_options = get_rotation_options(
            &st_nw_corner,
            rotation,
            granularity_x,
            granularity_y,
            &center_cartesian,
            width,
            height,
        );

        computed_options.st_gran_y_cos = st_rotation_options.gran_y_cos;
        computed_options.st_gran_x_cos = st_rotation_options.gran_x_cos;
        computed_options.st_gran_y_sin = st_rotation_options.gran_y_sin;
        computed_options.st_gran_x_sin = st_rotation_options.gran_x_sin;
        // Same JS aliasing as above: `stNwCornerResult` is rotated in place
        // inside `getRotationOptions`; write the rotated corner back.
        computed_options.st_nw_corner = Some(st_rotation_options.nw_corner);
        computed_options.st_west = st_rotation_options.west;
        computed_options.st_south = st_rotation_options.south;
    }

    computed_options
}
