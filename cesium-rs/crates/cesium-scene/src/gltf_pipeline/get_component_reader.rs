//! Ported from `packages/engine/Source/Scene/GltfPipeline/getComponentReader.js`.
//!
//! Returns a reader that reads and converts data from a byte slice (the
//! Rust analogue of `DataView`) into an array of `f64` components
//! (JavaScript numbers). All reads are little-endian, matching the JS
//! `dataView.get*(offset, true)` calls.

use cesium_core::component_datatype::ComponentDatatype;

/// A component reader selected by glTF `componentType` (the Rust analogue
/// of the closure returned by JS `getComponentReader`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentReader {
    component_type: u32,
}

impl ComponentReader {
    /// Reads `number_of_components` components starting at `byte_offset`
    /// into `result`, advancing by `component_type_byte_length` per
    /// component (mirroring the JS reader closure).
    ///
    /// # Panics
    /// Panics when the read runs past the end of `data` (the JS `DataView`
    /// throws `RangeError`) or when the component type is unknown.
    pub fn read(
        self,
        data: &[u8],
        byte_offset: usize,
        number_of_components: usize,
        component_type_byte_length: usize,
        result: &mut [f64],
    ) {
        for i in 0..number_of_components {
            let offset = byte_offset + i * component_type_byte_length;
            let bytes = &data[offset..offset + component_type_byte_length];
            result[i] = match self.component_type {
                value if value == ComponentDatatype::Byte as u32 => {
                    i8::from_le_bytes([bytes[0]]) as f64
                }
                value if value == ComponentDatatype::UnsignedByte as u32 => bytes[0] as f64,
                value if value == ComponentDatatype::Short as u32 => {
                    i16::from_le_bytes([bytes[0], bytes[1]]) as f64
                }
                value if value == ComponentDatatype::UnsignedShort as u32 => {
                    u16::from_le_bytes([bytes[0], bytes[1]]) as f64
                }
                value if value == ComponentDatatype::Int as u32 => {
                    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                }
                value if value == ComponentDatatype::UnsignedInt as u32 => {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                }
                value if value == ComponentDatatype::Float as u32 => {
                    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                }
                value if value == ComponentDatatype::Double as u32 => f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7],
                ]),
                other => panic!("Invalid component type: {other}"),
            };
        }
    }
}

/// Returns a reader to read and convert data from a byte slice into an
/// array, selected by glTF `componentType`.
///
/// # Panics
/// Panics (debug-check style) when `component_type` is not a valid
/// [`ComponentDatatype`] (the JS returns `undefined`, which then throws on
/// call).
pub fn get_component_reader(component_type: u32) -> ComponentReader {
    assert!(
        ComponentDatatype::try_from_u32(component_type).is_some(),
        "componentType is not a valid ComponentDatatype: {component_type}"
    );
    ComponentReader { component_type }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_component_type_little_endian() {
        // BYTE / UNSIGNED_BYTE
        let data = [0xFEu8, 0x7F];
        let mut out = vec![0.0; 2];
        get_component_reader(5120).read(&data, 0, 2, 1, &mut out);
        assert_eq!(out, vec![-2.0, 127.0]);
        get_component_reader(5121).read(&data, 0, 2, 1, &mut out);
        assert_eq!(out, vec![254.0, 127.0]);

        // SHORT / UNSIGNED_SHORT
        let data = [0x01, 0x00, 0xFF, 0x7F, 0xFE, 0xFF, 0xFF, 0xFF];
        get_component_reader(5122).read(&data, 0, 2, 2, &mut out);
        assert_eq!(out, vec![1.0, 32767.0]);
        get_component_reader(5123).read(&data, 4, 2, 2, &mut out);
        assert_eq!(out, vec![65534.0, 65535.0]);

        // INT / UNSIGNED_INT
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00,
        ];
        get_component_reader(5124).read(&data, 0, 2, 4, &mut out);
        assert_eq!(out, vec![-1.0, 1.0]);
        get_component_reader(5125).read(&data, 0, 2, 4, &mut out);
        assert_eq!(out, vec![4294967295.0, 1.0]);

        // FLOAT / DOUBLE (DOUBLE is WebGLConstants.DOUBLE = 0x140A = 5130)
        let data = 1.5f32.to_le_bytes();
        get_component_reader(5126).read(&data, 0, 1, 4, &mut out);
        assert_eq!(out[0], 1.5);
        let data = 2.25f64.to_le_bytes();
        get_component_reader(5130).read(&data, 0, 1, 8, &mut out);
        assert_eq!(out[0], 2.25);
    }

    #[test]
    #[should_panic(expected = "Invalid component type")]
    fn unknown_component_type_panics_on_read() {
        let data = [0u8; 4];
        let mut out = vec![0.0; 1];
        ComponentReader { component_type: 1 }.read(&data, 0, 1, 1, &mut out);
    }

    #[test]
    #[should_panic(expected = "not a valid ComponentDatatype")]
    fn get_component_reader_validates_component_type() {
        get_component_reader(9999);
    }
}
