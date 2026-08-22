//! Mirrors packages/engine/Specs/Core/appendForwardSlashSpec.js

use cesium_core::append_forward_slash::append_forward_slash;

// describe("Core/appendForwardSlash")

#[test]
fn appends_to_a_url() {
    assert_eq!(
        append_forward_slash("http://cesiumjs.org"),
        "http://cesiumjs.org/"
    );
}

#[test]
fn does_not_append_to_a_url() {
    assert_eq!(
        append_forward_slash("http://cesiumjs.org/"),
        "http://cesiumjs.org/"
    );
}

#[test]
fn appends_to_an_empty_string() {
    assert_eq!(append_forward_slash(""), "/");
}
