use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;
use cesium_core::tridiagonal_system_solver;

#[test]
fn solve_three_unknowns() {
    let lower = vec![1.0, 1.0];
    let diagonal = vec![-2.175, -2.15, -2.125];
    let upper = vec![1.0, 1.0];
    let right = vec![
        Cartesian3::new(-1.625, -1.625, -1.625),
        Cartesian3::new(0.5, 0.5, 0.5),
        Cartesian3::new(1.625, 1.625, 1.625),
    ];

    let expected = vec![
        Cartesian3::new(0.552, 0.552, 0.552),
        Cartesian3::new(-0.4244, -0.4244, -0.4244),
        Cartesian3::new(-0.9644, -0.9644, -0.9644),
    ];

    let actual = tridiagonal_system_solver::solve(&lower, &diagonal, &upper, &right);

    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert!(
            a.equals_epsilon_method(e, None, Some(CesiumMath::EPSILON4)),
            "actual ({}, {}, {}) != expected ({}, {}, {})",
            a.x, a.y, a.z, e.x, e.y, e.z
        );
    }
}

#[test]
fn solve_nine_unknowns() {
    let lower = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let diagonal = vec![
        -2.0304, -2.0288, -2.0272, -2.0256, -2.024, -2.0224, -2.0208, -2.0192, -2.0176,
    ];
    let upper = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let right = vec![
        Cartesian3::new(-1.952, -1.952, -1.952),
        Cartesian3::new(0.056, 0.056, 0.056),
        Cartesian3::new(0.064, 0.064, 0.064),
        Cartesian3::new(0.072, 0.072, 0.072),
        Cartesian3::new(0.08, 0.08, 0.08),
        Cartesian3::new(0.088, 0.088, 0.088),
        Cartesian3::new(0.096, 0.096, 0.096),
        Cartesian3::new(0.104, 0.104, 0.104),
        Cartesian3::new(1.112, 1.112, 1.112),
    ];

    let expected = vec![
        Cartesian3::new(1.3513, 1.3513, 1.3513),
        Cartesian3::new(0.7918, 0.7918, 0.7918),
        Cartesian3::new(0.311, 0.311, 0.311),
        Cartesian3::new(-0.0974, -0.0974, -0.0974),
        Cartesian3::new(-0.4362, -0.4362, -0.4362),
        Cartesian3::new(-0.7055, -0.7055, -0.7055),
        Cartesian3::new(-0.9025, -0.9025, -0.9025),
        Cartesian3::new(-1.0224, -1.0224, -1.0224),
        Cartesian3::new(-1.0579, -1.0579, -1.0579),
    ];

    let actual = tridiagonal_system_solver::solve(&lower, &diagonal, &upper, &right);

    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert!(
            a.equals_epsilon_method(e, None, Some(CesiumMath::EPSILON4)),
            "actual ({}, {}, {}) != expected ({}, {}, {})",
            a.x, a.y, a.z, e.x, e.y, e.z
        );
    }
}
