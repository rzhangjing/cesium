//! Ported from `packages/engine/Source/Core/TridiagonalSystemSolver.js`.
//!
//! Solves a tridiagonal system of linear equations using the Thomas algorithm.

use crate::cartesian3::Cartesian3;

/// Solves a tridiagonal system of linear equations.
///
/// - `lower`: lower diagonal (length n-1)
/// - `diagonal`: main diagonal (length n)
/// - `upper`: upper diagonal (length n-1)
/// - `right`: right-hand side Cartesian3 values (length n)
///
/// Returns the solution as a Vec<Cartesian3> of length n.
pub fn solve(
    lower: &[f64],
    diagonal: &[f64],
    upper: &[f64],
    right: &[Cartesian3],
) -> Vec<Cartesian3> {
    let n = diagonal.len();
    assert_eq!(n, right.len(), "diagonal and right must have the same lengths");
    assert_eq!(lower.len(), upper.len(), "lower and upper must have the same lengths");
    assert_eq!(lower.len(), n - 1, "lower and upper must be one less than diagonal");

    let mut c = vec![0.0; upper.len()];
    let mut d = vec![Cartesian3::default(); n];
    let mut x = vec![Cartesian3::default(); n];

    c[0] = upper[0] / diagonal[0];
    d[0] = Cartesian3::multiply_by_scalar_new(&right[0], 1.0 / diagonal[0]);

    for i in 1..c.len() {
        let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
        c[i] = upper[i] * scalar;
        let tmp = Cartesian3::multiply_by_scalar_new(&d[i - 1], lower[i - 1]);
        d[i] = Cartesian3::subtract_new(&right[i], &tmp);
        d[i] = Cartesian3::multiply_by_scalar_new(&d[i], scalar);
    }

    let i = c.len();
    let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
    let tmp = Cartesian3::multiply_by_scalar_new(&d[i - 1], lower[i - 1]);
    d[i] = Cartesian3::subtract_new(&right[i], &tmp);
    d[i] = Cartesian3::multiply_by_scalar_new(&d[i], scalar);

    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        let tmp = Cartesian3::multiply_by_scalar_new(&x[i + 1], c[i]);
        x[i] = Cartesian3::subtract_new(&d[i], &tmp);
    }

    x
}
