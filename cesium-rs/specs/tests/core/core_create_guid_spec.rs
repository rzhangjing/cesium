//! Mirrors packages/engine/Specs/Core/createGuidSpec.js

use cesium_core::create_guid::create_guid;

// describe("Core/createGuid")

/// Port of the spec's GUID regex:
/// /^(\{){0,1}[0-9a-fA-F]{8}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{4}\-[0-9a-fA-F]{12}(\}){0,1}$/
fn is_guid(guid: &str) -> bool {
    let bytes = guid.as_bytes();
    let mut i = 0;
    if bytes.first() == Some(&b'{') {
        i += 1;
    }
    let group_lengths = [8, 4, 4, 4, 12];
    for (group, &len) in group_lengths.iter().enumerate() {
        if group > 0 {
            if bytes.get(i) != Some(&b'-') {
                return false;
            }
            i += 1;
        }
        if i + len > bytes.len() {
            return false;
        }
        if !bytes[i..i + len]
            .iter()
            .all(|&c| c.is_ascii_hexdigit())
        {
            return false;
        }
        i += len;
    }
    if bytes.get(i) == Some(&b'}') {
        i += 1;
    }
    i == bytes.len()
}

#[test]
fn creates_guids() {
    // Create three GUIDs
    let guid1 = create_guid();
    let guid2 = create_guid();
    let guid3 = create_guid();

    // Make sure they are all unique
    assert_ne!(guid1, guid2);
    assert_ne!(guid1, guid3);
    assert_ne!(guid2, guid3);

    // Make sure they are all properly formatted
    assert!(is_guid(&guid1));
    assert_eq!(guid1.len(), 36);

    assert!(is_guid(&guid2));
    assert_eq!(guid2.len(), 36);

    assert!(is_guid(&guid3));
    assert_eq!(guid3.len(), 36);
}
