//! `data:` URI scheme parsing and decoding.
//!
//! Maps to CesiumJS `Resource.js` data URI handling:
//! - The `dataUriRegex` (`/^data:(.*?)(;base64)?,(.*)$/`) extraction.
//! - `decodeDataUri(match, responseType)` text/arraybuffer branches.
//! - `Resource.prototype.isDataUri` property.
//!
//! Supports both base64-encoded and percent-encoded (plain text) payloads.
//! This module is **pure domain logic** — no IO, no framework dependency.

/// Result of parsing a `data:` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataUriParts {
    /// The media type / MIME type (e.g. `"text/plain"`, `"image/png"`).
    /// Empty string if not specified (RFC 2397 default: `text/plain;charset=US-ASCII`).
    pub media_type: String,
    /// Whether the payload is base64-encoded (`;base64` parameter present).
    pub is_base64: bool,
    /// The raw payload string (before decoding).
    pub raw_payload: String,
}

/// Errors that can occur when parsing or decoding a data URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataUriError {
    /// The URI does not start with `data:`.
    NotaDataUri,
    /// The URI is missing the comma separator.
    MissingComma,
    /// Base64 decoding failed (invalid characters or length).
    InvalidBase64(String),
    /// Percent-decoding failed (malformed escape sequence).
    InvalidPercentEncoding(String),
}

impl std::fmt::Display for DataUriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotaDataUri => write!(f, "URI does not start with 'data:'"),
            Self::MissingComma => write!(f, "data URI missing ',' separator"),
            Self::InvalidBase64(msg) => write!(f, "invalid base64: {msg}"),
            Self::InvalidPercentEncoding(msg) => write!(f, "invalid percent-encoding: {msg}"),
        }
    }
}

impl std::error::Error for DataUriError {}

/// Returns `true` if the given URL string is a `data:` URI.
///
/// Maps to CesiumJS `isDataUri(url)` / `Resource.prototype.isDataUri`.
pub fn is_data_uri(url: &str) -> bool {
    url.trim_start().starts_with("data:")
}

/// Parses a `data:` URI into its constituent parts without decoding the
/// payload.
///
/// Mirrors the CesiumJS regex match: `/^data:(.*?)(;base64)?,(.*)$/`
///
/// # Examples
/// ```
/// use cesium_resource::data_uri::parse_data_uri;
/// let parts = parse_data_uri("data:text/plain;base64,SGVsbG8=").unwrap();
/// assert_eq!(parts.media_type, "text/plain");
/// assert!(parts.is_base64);
/// assert_eq!(parts.raw_payload, "SGVsbG8=");
/// ```
pub fn parse_data_uri(uri: &str) -> Result<DataUriParts, DataUriError> {
    let uri = uri.trim_start();
    if !uri.starts_with("data:") {
        return Err(DataUriError::NotaDataUri);
    }

    let after_scheme = &uri[5..]; // skip "data:"

    // Find the comma that separates metadata from payload.
    let comma_pos = after_scheme
        .find(',')
        .ok_or(DataUriError::MissingComma)?;

    let metadata = &after_scheme[..comma_pos];
    let payload = &after_scheme[comma_pos + 1..];

    // Check for ;base64 suffix in metadata.
    let (media_type, is_base64) = match metadata.strip_suffix(";base64") {
        Some(stripped) => (stripped, true),
        None => (metadata, false),
    };

    Ok(DataUriParts {
        media_type: media_type.to_string(),
        is_base64,
        raw_payload: payload.to_string(),
    })
}

/// Decodes a `data:` URI and returns the payload as raw bytes.
///
/// Handles both base64 and percent-encoded payloads. For percent-encoded
/// payloads the decoded bytes are the UTF-8 representation of the
/// percent-decoded string.
///
/// Maps to CesiumJS `decodeDataUriArrayBuffer` (the arraybuffer/blob branch).
pub fn decode_data_uri_bytes(uri: &str) -> Result<Vec<u8>, DataUriError> {
    let parts = parse_data_uri(uri)?;
    if parts.is_base64 {
        base64_decode(&parts.raw_payload)
    } else {
        Ok(percent_decode(&parts.raw_payload).into_bytes())
    }
}

/// Decodes a `data:` URI and returns the payload as a UTF-8 string.
///
/// For base64 payloads, the decoded bytes are interpreted as UTF-8 (lossy).
/// For percent-encoded payloads, the result is the percent-decoded string.
///
/// Maps to CesiumJS `decodeDataUriText` (the text branch).
pub fn decode_data_uri_text(uri: &str) -> Result<String, DataUriError> {
    let parts = parse_data_uri(uri)?;
    if parts.is_base64 {
        let bytes = base64_decode(&parts.raw_payload)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        Ok(percent_decode(&parts.raw_payload))
    }
}

/// Extracts the media type from a data URI, or `None` if not a data URI.
///
/// Returns the default `"text/plain"` when the media type field is empty
/// (RFC 2397 specifies `text/plain;charset=US-ASCII` as the default).
pub fn media_type_of(uri: &str) -> Option<String> {
    parse_data_uri(uri).ok().map(|parts| {
        if parts.media_type.is_empty() {
            "text/plain".to_string()
        } else {
            parts.media_type
        }
    })
}

// ── Percent-decoding ────────────────────────────────────────────────────────

/// Decodes percent-encoded characters (`%XX`) in a string.
///
/// Maps to JavaScript `decodeURIComponent`. Unlike JS, invalid sequences are
/// passed through verbatim rather than throwing (robustness for malformed
/// data URIs encountered in the wild).
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(byte) = hex_pair_to_byte(bytes[i + 1], bytes[i + 2]) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&result).into_owned()
}

/// Encodes a string with percent-encoding for URI components.
///
/// Maps to JavaScript `encodeURIComponent`. Unreserved characters
/// (`A-Z a-z 0-9 - _ . ! ~ * ' ( )`) are passed through; everything else
/// is percent-encoded.
pub fn percent_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
            | b'*' | b'\'' | b'(' | b')' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// Converts a hex character pair to a byte value.
fn hex_pair_to_byte(hi: u8, lo: u8) -> Option<u8> {
    let h = hex_val(hi)?;
    let l = hex_val(lo)?;
    Some((h << 4) | l)
}

/// Converts a single hex ASCII character to its 4-bit value.
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

// ── Base64 decoding ─────────────────────────────────────────────────────────

/// Minimal base64 decoder (mirrors `atob` semantics).
///
/// Handles standard base64 alphabet (`A-Z a-z 0-9 + /`) with `=` padding.
/// Whitespace characters are silently skipped (mirrors browser leniency).
///
/// Returns [`DataUriError::InvalidBase64`] on malformed input.
pub fn base64_decode(input: &str) -> Result<Vec<u8>, DataUriError> {
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();

    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            return Err(DataUriError::InvalidBase64(
                "incomplete base64 quantum".to_string(),
            ));
        }

        // Count padding
        let padding = chunk.iter().filter(|&&b| b == b'=').count();

        let mut buf: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            let v = if b == b'=' {
                0
            } else {
                base64_char_value(b).ok_or_else(|| {
                    DataUriError::InvalidBase64(format!(
                        "invalid character '{}' at position {}",
                        b as char, i
                    ))
                })?
            };
            buf |= (v as u32) << (18 - 6 * i);
        }

        output.push((buf >> 16) as u8);
        if padding < 2 {
            output.push((buf >> 8) as u8);
        }
        if padding < 1 {
            output.push(buf as u8);
        }
    }

    Ok(output)
}

/// Encodes bytes to base64 string (standard alphabet, with padding).
///
/// Provided for round-trip testing and for constructing data URIs
/// programmatically (e.g. in test fixtures or offline asset generators).
pub fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Converts a base64 character to its 6-bit value.
fn base64_char_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

// ── Data URI construction helper ────────────────────────────────────────────

/// Constructs a `data:` URI from a media type and raw bytes (base64-encoded).
///
/// Useful for embedding small assets inline (mirrors the pattern used in
/// CesiumJS `createResourceFromDataUri` helpers and test fixtures).
pub fn build_data_uri_base64(media_type: &str, data: &[u8]) -> String {
    format!("data:{};base64,{}", media_type, base64_encode(data))
}

/// Constructs a `data:` URI from a media type and plain text (percent-encoded).
pub fn build_data_uri_text(media_type: &str, text: &str) -> String {
    format!("data:{},{}", media_type, percent_encode(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_data_uri ─────────────────────────────────────────────────────

    #[test]
    fn recognizes_data_uris() {
        assert!(is_data_uri("data:text/plain,hello"));
        assert!(is_data_uri("data:;base64,AAA="));
        assert!(is_data_uri("data:,minimal"));
        assert!(!is_data_uri("https://example.com"));
        assert!(!is_data_uri("blob:https://example.com/uuid"));
        assert!(!is_data_uri(""));
    }

    // ── parse_data_uri ──────────────────────────────────────────────────

    #[test]
    fn parses_plain_text_data_uri() {
        let parts = parse_data_uri("data:text/plain,Hello%20World").unwrap();
        assert_eq!(parts.media_type, "text/plain");
        assert!(!parts.is_base64);
        assert_eq!(parts.raw_payload, "Hello%20World");
    }

    #[test]
    fn parses_base64_data_uri() {
        let parts = parse_data_uri("data:image/png;base64,iVBOR...").unwrap();
        assert_eq!(parts.media_type, "image/png");
        assert!(parts.is_base64);
        assert_eq!(parts.raw_payload, "iVBOR...");
    }

    #[test]
    fn parses_empty_media_type() {
        let parts = parse_data_uri("data:,hello").unwrap();
        assert_eq!(parts.media_type, "");
        assert!(!parts.is_base64);
        assert_eq!(parts.raw_payload, "hello");
    }

    #[test]
    fn rejects_non_data_uri() {
        assert_eq!(
            parse_data_uri("https://example.com"),
            Err(DataUriError::NotaDataUri)
        );
    }

    #[test]
    fn rejects_missing_comma() {
        assert_eq!(
            parse_data_uri("data:text/plain"),
            Err(DataUriError::MissingComma)
        );
    }

    // ── decode ──────────────────────────────────────────────────────────

    #[test]
    fn decodes_percent_encoded_text() {
        let text = decode_data_uri_text("data:text/plain,A%20brief%20note").unwrap();
        assert_eq!(text, "A brief note");
    }

    #[test]
    fn decodes_base64_text() {
        // base64("Hello") = "SGVsbG8="
        let text = decode_data_uri_text("data:text/plain;base64,SGVsbG8=").unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn decodes_base64_bytes() {
        // base64("abc") = "YWJj"
        let bytes = decode_data_uri_bytes("data:application/octet-stream;base64,YWJj").unwrap();
        assert_eq!(bytes, vec![97, 98, 99]);
    }

    #[test]
    fn decodes_percent_encoded_bytes() {
        let bytes = decode_data_uri_bytes("data:text/plain,Hi%21").unwrap();
        assert_eq!(bytes, b"Hi!".to_vec());
    }

    // ── media_type_of ───────────────────────────────────────────────────

    #[test]
    fn media_type_extraction() {
        assert_eq!(
            media_type_of("data:application/json;base64,e30="),
            Some("application/json".to_string())
        );
        assert_eq!(
            media_type_of("data:,bare"),
            Some("text/plain".to_string()) // RFC 2397 default
        );
        assert_eq!(media_type_of("https://example.com"), None);
    }

    // ── percent encoding/decoding ───────────────────────────────────────

    #[test]
    fn percent_encode_special_chars() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a/b?c=d&e"), "a%2Fb%3Fc%3Dd%26e");
        assert_eq!(percent_encode("unreserved-_.!~*'()"), "unreserved-_.!~*'()");
    }

    #[test]
    fn percent_decode_roundtrip() {
        let original = "hello world & goodbye/special=chars";
        let encoded = percent_encode(original);
        let decoded = percent_decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn percent_decode_passes_through_invalid() {
        // Invalid percent sequences pass through verbatim
        assert_eq!(percent_decode("%ZZ%"), "%ZZ%");
        assert_eq!(percent_decode("100%"), "100%");
    }

    // ── base64 ──────────────────────────────────────────────────────────

    #[test]
    fn base64_roundtrip() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    #[test]
    fn base64_handles_padding() {
        // 1 byte → 2 padding chars
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_decode("YQ==").unwrap(), b"a".to_vec());
        // 2 bytes → 1 padding char
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_decode("YWI=").unwrap(), b"ab".to_vec());
        // 3 bytes → no padding
        assert_eq!(base64_encode(b"abc"), "YWJj");
        assert_eq!(base64_decode("YWJj").unwrap(), b"abc".to_vec());
    }

    #[test]
    fn base64_rejects_invalid_char() {
        assert!(base64_decode("!!!!").is_err());
    }

    #[test]
    fn base64_empty_input() {
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
        assert_eq!(base64_encode(b""), "");
    }

    // ── build helpers ───────────────────────────────────────────────────

    #[test]
    fn build_base64_data_uri() {
        let uri = build_data_uri_base64("image/png", b"abc");
        assert_eq!(uri, "data:image/png;base64,YWJj");
        // Round-trip
        let bytes = decode_data_uri_bytes(&uri).unwrap();
        assert_eq!(bytes, b"abc".to_vec());
    }

    #[test]
    fn build_text_data_uri() {
        let uri = build_data_uri_text("text/plain", "hello world");
        assert_eq!(uri, "data:text/plain,hello%20world");
        let text = decode_data_uri_text(&uri).unwrap();
        assert_eq!(text, "hello world");
    }

    // ── Query parameter multi-value support ─────────────────────────────

    #[test]
    fn data_uri_with_query_like_payload() {
        // Data URIs can contain '?' and '&' in the payload — these are NOT
        // query parameters, they're part of the data.
        let parts = parse_data_uri("data:application/x-www-form-urlencoded,a=1&b=2").unwrap();
        assert_eq!(parts.raw_payload, "a=1&b=2");
        assert_eq!(parts.media_type, "application/x-www-form-urlencoded");
    }
}
