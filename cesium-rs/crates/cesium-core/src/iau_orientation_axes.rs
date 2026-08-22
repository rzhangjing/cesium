//! Ported from `packages/engine/Source/Core/IauOrientationAxes.js`.
//!
//! Axes representing the orientation of a Globe as represented by the data
//! from the IAU/IAG Working Group reports on rotational elements.

use crate::cartesian3::Cartesian3;
use crate::iau2000_orientation;
use crate::iau_orientation_parameters::IauOrientationParameters;
use crate::julian_date::JulianDate;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::quaternion::Quaternion;

/// Type alias for the compute function that produces orientation parameters.
pub type ComputeFunction = Box<dyn Fn(&JulianDate) -> IauOrientationParameters + Send + Sync>;

/// The Axes representing the orientation of a Globe.
pub struct IauOrientationAxes {
    compute_function: ComputeFunction,
}

impl IauOrientationAxes {
    /// Creates a new `IauOrientationAxes` with the given compute function.
    /// If none is provided, defaults to `compute_moon`.
    pub fn new(compute_function: Option<ComputeFunction>) -> Self {
        let compute_function =
            compute_function.unwrap_or_else(|| Box::new(default_compute_moon));
        Self { compute_function }
    }

    /// Computes a rotation from ICRF to a Globe's Fixed axes.
    pub fn evaluate(&self, date: &JulianDate, result: &mut Matrix3) {
        let alpha_delta_w = (self.compute_function)(date);

        let mut prec_mtx = Matrix3::default();
        compute_rotation_matrix(
            alpha_delta_w.right_ascension,
            alpha_delta_w.declination,
            &mut prec_mtx,
        );

        let rot = CesiumMath::zero_to_two_pi(alpha_delta_w.rotation);
        let quat = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, rot);
        let conj = Quaternion::conjugate_new(&quat);
        let rot_mtx = Matrix3::from_quaternion_new(&conj);

        Matrix3::multiply(&rot_mtx, &prec_mtx, result);
    }
}

fn default_compute_moon(date: &JulianDate) -> IauOrientationParameters {
    let mut params = IauOrientationParameters::default();
    iau2000_orientation::compute_moon(date, &mut params);
    params
}

fn compute_rotation_matrix(alpha: f64, delta: f64, result: &mut Matrix3) {
    let x_axis = Cartesian3::new(
        (alpha + CesiumMath::PI_OVER_TWO).cos(),
        (alpha + CesiumMath::PI_OVER_TWO).sin(),
        0.0,
    );

    let cos_dec = delta.cos();
    let z_axis = Cartesian3::new(cos_dec * alpha.cos(), cos_dec * alpha.sin(), delta.sin());

    let y_axis = Cartesian3::cross_new(&z_axis, &x_axis);

    let data = &mut result.elements;
    data[0] = x_axis.x;
    data[1] = y_axis.x;
    data[2] = z_axis.x;
    data[3] = x_axis.y;
    data[4] = y_axis.y;
    data[5] = z_axis.y;
    data[6] = x_axis.z;
    data[7] = y_axis.z;
    data[8] = z_axis.z;
}
