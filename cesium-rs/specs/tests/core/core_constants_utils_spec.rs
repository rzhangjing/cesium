//! Tests for Core constants, types, and simple utility modules:
//! TimeConstants, VulkanConstants, PixelFormat, Frozen,
//! srgb_to_linear, webgl_constant_to_glsl_type, add_all_to_array,
//! append_forward_slash, combine, clone, defer, destroy_object,
//! get_timestamp, is_bit_set, is_cross_origin_url, is_leap_year,
//! wrap_function, binary_search, parse_response_headers,
//! resize_image_to_next_power_of_two, iso8601, default_proxy,
//! vertical_exaggeration, create_color_ramp, write_text_to_canvas,
//! build_module_url, pin_builder, geometry_instance_attributes,
//! geometry_attributes, packable, interpolation_algorithm,
//! map_projection, geometry_factory, reference_frame.

use cesium_core::assert::cesium_assert;
use cesium_core::global_types::{Destroyable as GlobalDestroyable, GeoJsonPosition, TypedArray};
use cesium_core::add_all_to_array::add_all_to_array;
use cesium_core::append_forward_slash::append_forward_slash;
use cesium_core::binary_search::binary_search;
use cesium_core::clone::clone;
use cesium_core::color_geometry_instance_attribute::ColorGeometryInstanceAttribute;
use cesium_core::combine::combine;
use cesium_core::default_proxy::DefaultProxy;
use cesium_core::defer::Defer;
use cesium_core::destroy_object::{Destroyable, throw_on_destroyed, DESTROYED_MESSAGE};
use cesium_core::frozen;
use cesium_core::get_timestamp::get_timestamp;
use cesium_core::is_bit_set::is_bit_set;
use cesium_core::is_cross_origin_url::is_cross_origin_url;
use cesium_core::is_leap_year::is_leap_year;
use cesium_core::iso8601::Iso8601;
use cesium_core::offset_geometry_instance_attribute::OffsetGeometryInstanceAttribute;
use cesium_core::parse_response_headers::parse_response_headers;
use cesium_core::pixel_format::PixelFormat;
use cesium_core::resize_image_to_next_power_of_two::compute_resize_dimensions;
use cesium_core::show_geometry_instance_attribute::ShowGeometryInstanceAttribute;
use cesium_core::srgb_to_linear::{linear_to_srgb, srgb_to_linear};
use cesium_core::time_constants;
use cesium_core::vertical_exaggeration::VerticalExaggeration;
use cesium_core::vulkan_constants::VulkanConstants;
use cesium_core::web_gl_constant_to_glsl_type::webgl_constant_to_glsl_type;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_core::wrap_function::wrap_function;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geometry_attributes::GeometryAttributes;
use cesium_test_utils::expect_to_throw_dev_error_containing;

// --- TimeConstants ---
#[test]
fn time_constants_values() {
    assert_eq!(time_constants::SECONDS_PER_MILLISECOND, 0.001);
    assert_eq!(time_constants::SECONDS_PER_MINUTE, 60.0);
    assert_eq!(time_constants::MINUTES_PER_HOUR, 60.0);
    assert_eq!(time_constants::HOURS_PER_DAY, 24.0);
    assert_eq!(time_constants::SECONDS_PER_HOUR, 3600.0);
    assert_eq!(time_constants::MINUTES_PER_DAY, 1440.0);
    assert_eq!(time_constants::SECONDS_PER_DAY, 86400.0);
    assert_eq!(time_constants::DAYS_PER_JULIAN_CENTURY, 36525.0);
}

// --- VulkanConstants ---
#[test]
fn vulkan_constants_values() {
    assert_eq!(VulkanConstants::VK_FORMAT_UNDEFINED, 0);
    assert_eq!(VulkanConstants::VK_FORMAT_R8_UNORM, 9);
    assert_eq!(VulkanConstants::VK_FORMAT_R8G8B8A8_UNORM, 37);
    assert_eq!(VulkanConstants::VK_FORMAT_D32_SFLOAT, 126);
}

// --- PixelFormat ---
#[test]
fn pixel_format_components_length() {
    assert_eq!(PixelFormat::Rgb.components_length(), 3);
    assert_eq!(PixelFormat::Rgba.components_length(), 4);
    assert_eq!(PixelFormat::Rg.components_length(), 2);
    assert_eq!(PixelFormat::Red.components_length(), 1);
    assert_eq!(PixelFormat::Alpha.components_length(), 1);
}

#[test]
fn pixel_format_is_color_format() {
    assert!(PixelFormat::Red.is_color_format());
    assert!(PixelFormat::Rgb.is_color_format());
    assert!(PixelFormat::Rgba.is_color_format());
    assert!(!PixelFormat::DepthComponent.is_color_format());
}

#[test]
fn pixel_format_is_depth_format() {
    assert!(PixelFormat::DepthComponent.is_depth_format());
    assert!(PixelFormat::DepthStencil.is_depth_format());
    assert!(!PixelFormat::Rgb.is_depth_format());
}

// --- srgb_to_linear ---
#[test]
fn srgb_to_linear_roundtrip() {
    let val = 0.5;
    let linear = srgb_to_linear(val);
    let back = linear_to_srgb(linear);
    assert!((back - val).abs() < 1e-10);
}

#[test]
fn srgb_to_linear_boundary() {
    assert!((srgb_to_linear(0.0) - 0.0).abs() < 1e-10);
    assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-10);
}

// --- webgl_constant_to_glsl_type ---
#[test]
fn webgl_constant_to_glsl_type_known() {
    assert_eq!(webgl_constant_to_glsl_type(WebGLConstants::FLOAT), Some("float"));
    assert_eq!(webgl_constant_to_glsl_type(WebGLConstants::FLOAT_VEC3), Some("vec3"));
    assert_eq!(webgl_constant_to_glsl_type(WebGLConstants::FLOAT_MAT4), Some("mat4"));
    assert_eq!(webgl_constant_to_glsl_type(WebGLConstants::SAMPLER_2D), Some("sampler2D"));
}

#[test]
fn webgl_constant_to_glsl_type_unknown() {
    assert_eq!(webgl_constant_to_glsl_type(99999), None);
}

// --- Frozen ---
#[test]
fn frozen_empty_object() {
    let obj = frozen::empty_object();
    assert!(obj.is_object());
    assert_eq!(obj.as_object().unwrap().len(), 0);
}

#[test]
fn frozen_empty_array() {
    let arr = frozen::empty_array();
    assert!(arr.is_empty());
}

// --- add_all_to_array ---
#[test]
fn add_all_to_array_basic() {
    let mut target = vec![1, 2, 3];
    let source = vec![4, 5, 6];
    add_all_to_array(&mut target, Some(&source));
    assert_eq!(target, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn add_all_to_array_none_source() {
    let mut target = vec![1, 2];
    add_all_to_array::<i32>(&mut target, None);
    assert_eq!(target, vec![1, 2]);
}

#[test]
fn add_all_to_array_empty_source() {
    let mut target = vec![1, 2];
    let empty: Vec<i32> = vec![];
    add_all_to_array(&mut target, Some(&empty));
    assert_eq!(target, vec![1, 2]);
}

// --- append_forward_slash ---
#[test]
fn append_forward_slash_adds_slash() {
    assert_eq!(append_forward_slash("http://example.com"), "http://example.com/");
}

#[test]
fn append_forward_slash_no_double_slash() {
    assert_eq!(append_forward_slash("http://example.com/"), "http://example.com/");
}

#[test]
fn append_forward_slash_empty() {
    assert_eq!(append_forward_slash(""), "/");
}

// --- combine ---
#[test]
fn combine_shallow() {
    let o1 = serde_json::json!({"a": 1, "b": 2});
    let o2 = serde_json::json!({"b": 3, "c": 4});
    let result = combine(Some(&o1), Some(&o2), None);
    assert_eq!(result["a"], 1);
    assert_eq!(result["b"], 2); // o1 wins
    assert_eq!(result["c"], 4);
}

#[test]
fn combine_none_inputs() {
    let o1 = serde_json::json!({"a": 1});
    let result = combine(Some(&o1), None, None);
    assert_eq!(result["a"], 1);
}

// --- clone ---
#[test]
fn clone_basic() {
    let v = vec![1, 2, 3];
    let cloned = clone(&v, false);
    assert_eq!(cloned, vec![1, 2, 3]);
}

// --- Defer ---
#[test]
fn defer_resolve() {
    let d = Defer::<i32>::new();
    d.resolve.send(42).unwrap();
    let val = d.promise.recv().unwrap();
    assert_eq!(val, 42);
}

// --- destroy_object ---
#[test]
#[should_panic(expected = "This object was destroyed")]
fn throw_on_destroyed_panics() {
    throw_on_destroyed(None);
}

#[test]
fn destroyed_message_constant() {
    assert!(DESTROYED_MESSAGE.contains("destroyed"));
}

// --- get_timestamp ---
#[test]
fn get_timestamp_returns_positive() {
    let ts = get_timestamp();
    assert!(ts >= 0.0);
}

#[test]
fn get_timestamp_monotonic() {
    let t1 = get_timestamp();
    let t2 = get_timestamp();
    assert!(t2 >= t1);
}

// --- is_bit_set ---
#[test]
fn is_bit_set_true() {
    assert!(is_bit_set(0b1010, 0b0010));
}

#[test]
fn is_bit_set_false() {
    assert!(!is_bit_set(0b1010, 0b0001));
}

// --- is_cross_origin_url ---
#[test]
fn is_cross_origin_same_origin() {
    assert!(!is_cross_origin_url("http://example.com/a", "http://example.com/b"));
}

#[test]
fn is_cross_origin_different_origin() {
    assert!(is_cross_origin_url("http://other.com/a", "http://example.com/b"));
}

// --- is_leap_year ---
#[test]
fn is_leap_year_cases() {
    assert!(is_leap_year(2000.0));
    assert!(!is_leap_year(1900.0));
    assert!(is_leap_year(2024.0));
    assert!(!is_leap_year(2023.0));
}

// --- wrap_function ---
#[test]
fn wrap_function_calls_both() {
    use std::cell::RefCell;
    let log = RefCell::new(Vec::new());
    let new_fn = |_: &()| { log.borrow_mut().push("new"); };
    let old_fn = |_: &()| { log.borrow_mut().push("old"); };
    let wrapped = wrap_function(new_fn, old_fn);
    wrapped(&());
    assert_eq!(*log.borrow(), vec!["new", "old"]);
}

// --- binary_search ---
#[test]
fn binary_search_found() {
    let nums = [0.0f64, 2.0, 4.0, 6.0, 8.0];
    let idx = binary_search(&nums, &6.0, |a: &f64, b: &f64| a - b);
    assert_eq!(idx, 3);
}

#[test]
fn binary_search_not_found() {
    let nums = [0.0f64, 2.0, 4.0, 6.0, 8.0];
    let idx = binary_search(&nums, &5.0, |a: &f64, b: &f64| a - b);
    assert!(idx < 0); // not found, returns complement
}

// --- parse_response_headers ---
#[test]
fn parse_response_headers_basic() {
    let headers = parse_response_headers("Content-Type: text/html\r\nX-Custom: value");
    assert_eq!(headers.get("Content-Type").unwrap(), "text/html");
    assert_eq!(headers.get("X-Custom").unwrap(), "value");
}

#[test]
fn parse_response_headers_empty() {
    let headers = parse_response_headers("");
    assert!(headers.is_empty());
}

// --- compute_resize_dimensions ---
#[test]
fn resize_dimensions_power_of_two() {
    let (w, h) = compute_resize_dimensions(100, 200);
    assert_eq!(w, 128);
    assert_eq!(h, 256);
}

#[test]
fn resize_dimensions_already_power_of_two() {
    let (w, h) = compute_resize_dimensions(256, 512);
    assert_eq!(w, 256);
    assert_eq!(h, 512);
}

// --- Iso8601 ---
#[test]
fn iso8601_minimum_value() {
    let min = Iso8601::minimum_value();
    let _ = min; // just verify it doesn't panic
}

#[test]
fn iso8601_maximum_value() {
    let max = Iso8601::maximum_value();
    let _ = max;
}

#[test]
fn iso8601_maximum_interval() {
    let interval = Iso8601::maximum_interval();
    assert!(interval.is_start_included);
    assert!(interval.is_stop_included);
}

// --- DefaultProxy ---
#[test]
fn default_proxy_get_url() {
    let proxy = DefaultProxy::new("http://proxy.example.com");
    let url = proxy.get_url("http://target.com/resource");
    assert_eq!(url, "http://proxy.example.com?http://target.com/resource");
}

#[test]
fn default_proxy_get_url_with_query() {
    let proxy = DefaultProxy::new("http://proxy.example.com?token=abc");
    let url = proxy.get_url("http://target.com");
    assert_eq!(url, "http://proxy.example.com?token=abchttp://target.com");
}

// --- VerticalExaggeration ---
#[test]
fn vertical_exaggeration_get_height() {
    let h = VerticalExaggeration::get_height(100.0, 2.0, 50.0);
    assert_eq!(h, 150.0); // (100-50)*2 + 50 = 150
}

#[test]
fn vertical_exaggeration_get_height_identity() {
    let h = VerticalExaggeration::get_height(100.0, 1.0, 0.0);
    assert_eq!(h, 100.0);
}

#[test]
fn vertical_exaggeration_throws_with_non_finite_scale() {
    // Phase 2 diff regression (D6, case ve.getHeight.h4): mirror of the JS
    // debug guard `scale must be a finite number.`.
    expect_to_throw_dev_error_containing(
        || {
            let _ = VerticalExaggeration::get_height(100.0, f64::NAN, 0.0);
        },
        "scale must be a finite number.",
    );
}

#[test]
fn vertical_exaggeration_throws_with_non_finite_relative_height() {
    // Phase 2 diff regression (D6, case ve.getHeight.h5): mirror of the JS
    // debug guard `relativeHeight must be a finite number.`.
    expect_to_throw_dev_error_containing(
        || {
            let _ = VerticalExaggeration::get_height(100.0, 1.0, f64::NEG_INFINITY);
        },
        "relativeHeight must be a finite number.",
    );
}

// --- ShowGeometryInstanceAttribute ---
#[test]
fn show_attribute_default_true() {
    let attr = ShowGeometryInstanceAttribute::new(None);
    assert_eq!(attr.value, vec![1.0]);
}

#[test]
fn show_attribute_false() {
    let attr = ShowGeometryInstanceAttribute::new(Some(false));
    assert_eq!(attr.value, vec![0.0]);
}

#[test]
fn show_attribute_to_value() {
    assert_eq!(ShowGeometryInstanceAttribute::to_value(true), vec![1.0]);
    assert_eq!(ShowGeometryInstanceAttribute::to_value(false), vec![0.0]);
}

// --- OffsetGeometryInstanceAttribute ---
#[test]
fn offset_attribute_from_cartesian3() {
    let offset = Cartesian3::new(1.0, 2.0, 3.0);
    let attr = OffsetGeometryInstanceAttribute::from_cartesian3(&offset);
    assert_eq!(attr.value, vec![1.0, 2.0, 3.0]);
}

// --- ColorGeometryInstanceAttribute ---
#[test]
fn color_attribute_default_white() {
    let attr = ColorGeometryInstanceAttribute::new(None, None, None, None);
    assert_eq!(attr.value, vec![1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn color_attribute_equals() {
    let a = ColorGeometryInstanceAttribute::new(Some(1.0), Some(0.0), Some(0.0), Some(1.0));
    let b = ColorGeometryInstanceAttribute::new(Some(1.0), Some(0.0), Some(0.0), Some(1.0));
    assert!(ColorGeometryInstanceAttribute::equals(&a, &b));
}

// --- GeometryAttributes ---
#[test]
fn geometry_attributes_default() {
    let ga = GeometryAttributes::default();
    assert!(ga.position.is_none());
    assert!(ga.normal.is_none());
}

// --- cesium_assert ---
#[test]
fn cesium_assert_true_no_panic() {
    cesium_assert(true, "should not panic");
}

#[test]
#[should_panic(expected = "condition failed")]
fn cesium_assert_false_panics() {
    cesium_assert(false, "condition failed");
}

// --- global_types ---
#[test]
fn typed_array_is_vec_f64() {
    let arr: TypedArray = vec![1.0, 2.0, 3.0];
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], 1.0);
}

#[test]
fn geo_json_position_is_vec_f64() {
    let pos: GeoJsonPosition = vec![-73.9857, 40.7484]; // lon, lat
    assert_eq!(pos.len(), 2);
}

struct TestDestroyable {
    destroyed: bool,
}

impl GlobalDestroyable for TestDestroyable {
    fn destroy(&mut self) {
        self.destroyed = true;
    }
}

#[test]
fn destroyable_trait_impl() {
    let mut obj = TestDestroyable { destroyed: false };
    assert!(!obj.destroyed);
    obj.destroy();
    assert!(obj.destroyed);
}
