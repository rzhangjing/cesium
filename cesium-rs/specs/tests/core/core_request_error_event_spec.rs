//! Port of `Core/RequestErrorEventSpec.js`.

use cesium_core::request_error_event::RequestErrorEvent;

#[test]
fn parses_response_headers_from_string() {
    let event = RequestErrorEvent::new(
        Some(404),
        Some("foo".to_string()),
        Some("This-is-a-test: first\r\nAnother: second value!".to_string()),
    );
    let headers = event.response_headers.as_ref().unwrap();
    assert_eq!(headers.get("This-is-a-test").unwrap(), "first");
    assert_eq!(headers.get("Another").unwrap(), "second value!");
}

#[test]
fn no_headers_when_none_provided() {
    let event = RequestErrorEvent::new(Some(500), Some("error".to_string()), None);
    assert!(event.response_headers.is_none());
}

#[test]
fn display_includes_status_code() {
    let event = RequestErrorEvent::new(Some(404), None, None);
    let msg = format!("{}", event);
    assert!(msg.contains("404"));
}
