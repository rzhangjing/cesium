//! Tests for `cesium_core::Credit`.

use cesium_core::credit::Credit;

#[test]
fn constructor_sets_html_and_show_on_screen() {
    let credit = Credit::new("<b>Test</b>", true);
    assert_eq!(credit.html(), "<b>Test</b>");
    assert!(credit.show_on_screen());
}

#[test]
fn constructor_with_show_on_screen_false() {
    let credit = Credit::new("attribution", false);
    assert_eq!(credit.html(), "attribution");
    assert!(!credit.show_on_screen());
}

#[test]
fn set_show_on_screen_updates_flag() {
    let mut credit = Credit::new("test", false);
    assert!(!credit.show_on_screen());
    credit.set_show_on_screen(true);
    assert!(credit.show_on_screen());
}

#[test]
fn same_html_gets_same_id() {
    let c1 = Credit::new("same content", true);
    let c2 = Credit::new("same content", false);
    assert_eq!(c1.id(), c2.id());
}

#[test]
fn different_html_gets_different_id() {
    let c1 = Credit::new("content A", true);
    let c2 = Credit::new("content B", true);
    assert_ne!(c1.id(), c2.id());
}

#[test]
fn equals_compares_id_and_show_on_screen() {
    let c1 = Credit::new("test", true);
    let c2 = Credit::new("test", true);
    assert!(Credit::equals(&c1, &c2));
}

#[test]
fn equals_returns_false_for_different_show_on_screen() {
    let c1 = Credit::new("test", true);
    let c2 = Credit::new("test", false);
    assert!(!Credit::equals(&c1, &c2));
}

#[test]
fn partial_eq_works() {
    let c1 = Credit::new("hello", true);
    let c2 = Credit::new("hello", true);
    assert_eq!(c1, c2);
}

#[test]
fn clone_credit_creates_equal_copy() {
    let c1 = Credit::new("original", true);
    let c2 = c1.clone_credit();
    assert_eq!(c1, c2);
}
