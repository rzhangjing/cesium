//! Port of `Core/RectangleCollisionCheckerSpec.js`.

use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_collision_checker::RectangleCollisionChecker;

#[test]
fn checks_for_collisions_with_contained_rectangles() {
    let test_rect1 = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    let test_rect2 = Rectangle::new(1.1, 1.1, 2.1, 2.1);
    let test_rect3 = Rectangle::new(1.1, 1.1, 1.2, 1.2);

    let mut checker = RectangleCollisionChecker::new();
    checker.insert("test1".to_string(), test_rect1);
    assert!(!checker.collides(&test_rect2));

    checker.insert("test3".to_string(), test_rect3);
    assert!(checker.collides(&test_rect2));
}

#[test]
fn removes_rectangles() {
    let test_rect1 = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    let test_rect2 = Rectangle::new(1.1, 1.1, 2.1, 2.1);
    let test_rect3 = Rectangle::new(1.1, 1.1, 1.2, 1.2);

    let mut checker = RectangleCollisionChecker::new();
    checker.insert("test1".to_string(), test_rect1);
    checker.insert("test3".to_string(), test_rect3);
    assert!(checker.collides(&test_rect2));

    checker.remove("test3");
    assert!(!checker.collides(&test_rect2));
}
