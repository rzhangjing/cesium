//! Ported from `packages/engine/Source/Core/Spline.js`.
//!
//! Base trait and shared utilities for all spline types.

use crate::cartesian3::Cartesian3;
use crate::math::CesiumMath;

/// Represents a point that can be interpolated by a spline.
#[derive(Clone, Debug)]
pub enum SplinePoint {
    /// A scalar value.
    Scalar(f64),
    /// A 3D Cartesian value.
    Cartesian3(Cartesian3),
}

impl SplinePoint {
    /// Linearly interpolate between two SplinePoints.
    pub fn lerp(a: &Self, b: &Self, u: f64) -> Self {
        match (a, b) {
            (SplinePoint::Scalar(va), SplinePoint::Scalar(vb)) => {
                SplinePoint::Scalar((1.0 - u) * va + u * vb)
            }
            (SplinePoint::Cartesian3(va), SplinePoint::Cartesian3(vb)) => {
                let mut result = Cartesian3::ZERO;
                Cartesian3::lerp(va, vb, u, &mut result);
                SplinePoint::Cartesian3(result)
            }
            _ => a.clone(),
        }
    }

    /// Clone the value.
    pub fn clone_point(&self) -> Self {
        match self {
            SplinePoint::Scalar(v) => SplinePoint::Scalar(*v),
            SplinePoint::Cartesian3(v) => SplinePoint::Cartesian3(*v),
        }
    }
}

/// Finds an index `i` in `times` such that `time` is in `[times[i], times[i+1])`.
pub fn find_time_interval(times: &[f64], time: f64, start_index: Option<usize>) -> Option<usize> {
    let length = times.len();
    if length < 2 {
        return None;
    }

    if time < times[0] || time > times[length - 1] {
        return None;
    }

    let start_index = start_index.unwrap_or(0);

    // Check current, next, and previous intervals
    if time >= times[start_index] {
        if start_index + 1 < length && time < times[start_index + 1] {
            return Some(start_index);
        } else if start_index + 2 < length && time < times[start_index + 2] {
            return Some(start_index + 1);
        }
    } else if start_index >= 1 && time >= times[start_index - 1] {
        return Some(start_index - 1);
    }

    // Linear search
    if time > times[start_index] {
        for i in start_index..length - 1 {
            if time >= times[i] && time < times[i + 1] {
                return Some(i);
            }
        }
    } else {
        for i in (0..start_index).rev() {
            if time >= times[i] && time < times[i + 1] {
                return Some(i);
            }
        }
    }

    Some(length - 2)
}

/// Wraps the given time to the period covered by the spline.
pub fn wrap_time(times: &[f64], time: f64) -> f64 {
    let time_end = times[times.len() - 1];
    let time_start = times[0];
    let time_stretch = time_end - time_start;

    let mut t = time;
    if t < time_start {
        let divs = ((time_start - t) / time_stretch).floor() + 1.0;
        t += divs * time_stretch;
    }
    if t > time_end {
        let divs = ((t - time_end) / time_stretch).floor() + 1.0;
        t -= divs * time_stretch;
    }
    t
}

/// Clamps the given time to the period covered by the spline.
pub fn clamp_time(times: &[f64], time: f64) -> f64 {
    CesiumMath::clamp(time, times[0], times[times.len() - 1])
}
