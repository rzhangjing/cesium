//! Ported from `packages/engine/Source/Core/BoxGeometry.js`.
//!
//! A cube centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// Describes a cube centered at the origin.
#[derive(Debug, Clone)]
pub struct BoxGeometry {
    minimum: Cartesian3,
    maximum: Cartesian3,
    vertex_format: VertexFormat,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl BoxGeometry {
    /// Creates a new `BoxGeometry` from min/max corners.
    pub fn new(
        minimum: &Cartesian3,
        maximum: &Cartesian3,
        vertex_format: Option<VertexFormat>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        Self {
            minimum: *minimum,
            maximum: *maximum,
            vertex_format: vertex_format.unwrap_or(VertexFormat::default()),
            offset_attribute,
        }
    }

    /// Creates a cube from its dimensions (width, depth, height).
    pub fn from_dimensions(
        dimensions: &Cartesian3,
        vertex_format: Option<VertexFormat>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let half = Cartesian3::multiply_by_scalar_new(dimensions, 0.5);
        let neg_half = Cartesian3::negate_new(&half);
        Self::new(&neg_half, &half, vertex_format, offset_attribute)
    }

    /// Creates a cube that encloses an axis-aligned bounding box.
    ///
    /// Port of `BoxGeometry.fromAxisAlignedBoundingBox`.
    pub fn from_axis_aligned_bounding_box(bounding_box: &AxisAlignedBoundingBox) -> Self {
        Self::new(&bounding_box.minimum, &bounding_box.maximum, None, None)
    }

    /// The number of `f64` elements needed to pack/unpack a `BoxGeometry`.
    pub const PACKED_LENGTH: usize =
        2 * Cartesian3::PACKED_LENGTH + VertexFormat::PACKED_LENGTH + 1;

    /// Packs the geometry into `array` starting at `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let si = starting_index.unwrap_or(0);
        Cartesian3::pack(&self.minimum, array, Some(si));
        Cartesian3::pack(&self.maximum, array, Some(si + Cartesian3::PACKED_LENGTH));
        self.vertex_format
            .pack(array, si + 2 * Cartesian3::PACKED_LENGTH);
        array[si + 2 * Cartesian3::PACKED_LENGTH + VertexFormat::PACKED_LENGTH] =
            self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks a `BoxGeometry` from `array`.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let si = starting_index.unwrap_or(0);
        let min = Cartesian3::unpack_new(array, Some(si));
        let max = Cartesian3::unpack_new(array, Some(si + Cartesian3::PACKED_LENGTH));
        let vf = VertexFormat::unpack(array, si + 2 * Cartesian3::PACKED_LENGTH, None);
        let offset_raw = array[si + 2 * Cartesian3::PACKED_LENGTH + VertexFormat::PACKED_LENGTH];
        let offset = if offset_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_raw as u32)
        };

        if let Some(r) = result {
            r.minimum = min;
            r.maximum = max;
            r.vertex_format = vf;
            r.offset_attribute = offset;
            r.clone()
        } else {
            Self {
                minimum: min,
                maximum: max,
                vertex_format: vf,
                offset_attribute: offset,
            }
        }
    }

    /// Computes the geometric representation of a box.
    pub fn create_geometry(&self) -> Option<Geometry> {
        let min = &self.minimum;
        let max = &self.maximum;
        let vf = &self.vertex_format;

        if Cartesian3::equals(Some(min), Some(max)) {
            return None;
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        let indices: Vec<u16>;

        if vf.position
            && (vf.st || vf.normal || vf.tangent || vf.bitangent)
        {
            // Full vertex: 6 faces × 4 vertices × 3 components = 72
            build_full_face_attributes(min, max, vf, &mut attributes);
            indices = build_full_indices();
        } else {
            // Position-only: 8 unique corners
            let positions = build_position_only(min, max);
            attributes.insert(
                "position".to_string(),
                GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
            );
            indices = build_position_only_indices();
        }

        // Bounding sphere
        let diff = Cartesian3::subtract_new(max, min);
        let radius = Cartesian3::magnitude(&diff) * 0.5;

        // Offset attribute
        if let Some(offset) = self.offset_attribute {
            let num_verts = attributes["position"].values.len() / 3;
            let offset_value = if offset == GeometryOffsetAttribute::None {
                0.0
            } else {
                1.0
            };
            let apply_offset = vec![offset_value; num_verts];
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }

        Some(Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Triangles),
            Some(BoundingSphere::new(Cartesian3::ZERO, radius)),
        ))
    }

    /// Returns the geometry for a unit box `[-0.5, 0.5]³`.
    pub fn get_unit_box() -> Geometry {
        let box_geom = Self::from_dimensions(
            &Cartesian3::new(1.0, 1.0, 1.0),
            Some(VertexFormat::position_only()),
            None,
        );
        box_geom.create_geometry().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Position-only path (8 corners, 12 triangles)
// ---------------------------------------------------------------------------

fn build_position_only(min: &Cartesian3, max: &Cartesian3) -> Vec<f64> {
    vec![
        // 0: (−,−,−)
        min.x, min.y, min.z,
        // 1: (+,−,−)
        max.x, min.y, min.z,
        // 2: (+,+,−)
        max.x, max.y, min.z,
        // 3: (−,+,−)
        min.x, max.y, min.z,
        // 4: (−,−,+)
        min.x, min.y, max.z,
        // 5: (+,−,+)
        max.x, min.y, max.z,
        // 6: (+,+,+)
        max.x, max.y, max.z,
        // 7: (−,+,+)
        min.x, max.y, max.z,
    ]
}

fn build_position_only_indices() -> Vec<u16> {
    vec![
        // +z face
        4, 5, 6, 4, 6, 7,
        // −z face
        1, 0, 3, 1, 3, 2,
        // +x face
        1, 6, 5, 1, 2, 6,
        // +y face
        2, 3, 7, 2, 7, 6,
        // −x face
        3, 0, 4, 3, 4, 7,
        // −y face
        0, 1, 5, 0, 5, 4,
    ]
}

// ---------------------------------------------------------------------------
// Full vertex path (6 faces × 4 verts, with normal/st/tangent/bitangent)
// ---------------------------------------------------------------------------

fn build_full_face_attributes(
    min: &Cartesian3,
    max: &Cartesian3,
    vf: &VertexFormat,
    attributes: &mut HashMap<String, GeometryAttribute>,
) {
    // 6 faces × 4 vertices = 24 positions
    let positions: Vec<f64> = vec![
        // +z face
        min.x, min.y, max.z,  max.x, min.y, max.z,  max.x, max.y, max.z,  min.x, max.y, max.z,
        // −z face
        min.x, min.y, min.z,  max.x, min.y, min.z,  max.x, max.y, min.z,  min.x, max.y, min.z,
        // +x face
        max.x, min.y, min.z,  max.x, max.y, min.z,  max.x, max.y, max.z,  max.x, min.y, max.z,
        // −x face
        min.x, min.y, min.z,  min.x, max.y, min.z,  min.x, max.y, max.z,  min.x, min.y, max.z,
        // +y face
        min.x, max.y, min.z,  max.x, max.y, min.z,  max.x, max.y, max.z,  min.x, max.y, max.z,
        // −y face
        min.x, min.y, min.z,  max.x, min.y, min.z,  max.x, min.y, max.z,  min.x, min.y, max.z,
    ];

    if vf.position {
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
        );
    }

    if vf.normal {
        let normals: Vec<f64> = vec![
            // +z
            0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
            // −z
            0.,0.,-1., 0.,0.,-1., 0.,0.,-1., 0.,0.,-1.,
            // +x
            1.,0.,0., 1.,0.,0., 1.,0.,0., 1.,0.,0.,
            // −x
            -1.,0.,0., -1.,0.,0., -1.,0.,0., -1.,0.,0.,
            // +y
            0.,1.,0., 0.,1.,0., 0.,1.,0., 0.,1.,0.,
            // −y
            0.,-1.,0., 0.,-1.,0., 0.,-1.,0., 0.,-1.,0.,
        ];
        attributes.insert(
            "normal".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
        );
    }

    if vf.st {
        let tex: Vec<f64> = vec![
            // +z
            0.,0., 1.,0., 1.,1., 0.,1.,
            // −z
            1.,0., 0.,0., 0.,1., 1.,1.,
            // +x
            0.,0., 1.,0., 1.,1., 0.,1.,
            // −x
            1.,0., 0.,0., 0.,1., 1.,1.,
            // +y
            1.,0., 0.,0., 0.,1., 1.,1.,
            // −y
            0.,0., 1.,0., 1.,1., 0.,1.,
        ];
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, tex),
        );
    }

    if vf.tangent {
        let tangents: Vec<f64> = vec![
            // +z
            1.,0.,0., 1.,0.,0., 1.,0.,0., 1.,0.,0.,
            // −z
            -1.,0.,0., -1.,0.,0., -1.,0.,0., -1.,0.,0.,
            // +x
            0.,1.,0., 0.,1.,0., 0.,1.,0., 0.,1.,0.,
            // −x
            0.,-1.,0., 0.,-1.,0., 0.,-1.,0., 0.,-1.,0.,
            // +y
            -1.,0.,0., -1.,0.,0., -1.,0.,0., -1.,0.,0.,
            // −y
            1.,0.,0., 1.,0.,0., 1.,0.,0., 1.,0.,0.,
        ];
        attributes.insert(
            "tangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
        );
    }

    if vf.bitangent {
        let bitangents: Vec<f64> = vec![
            // +z
            0.,1.,0., 0.,1.,0., 0.,1.,0., 0.,1.,0.,
            // −z
            0.,1.,0., 0.,1.,0., 0.,1.,0., 0.,1.,0.,
            // +x
            0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
            // −x
            0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
            // +y
            0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
            // −y
            0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
        ];
        attributes.insert(
            "bitangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
        );
    }
}

fn build_full_indices() -> Vec<u16> {
    // 6 faces × 2 triangles × 3 indices = 36
    let mut idx = Vec::with_capacity(36);
    for face in 0u16..6 {
        let base = face * 4;
        // CCW winding for +z, +x, −y; CW for −z, −x, +y
        match face {
            0 | 2 | 5 => {
                // CCW: 0,1,2, 0,2,3
                idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            _ => {
                // CW: 2,1,0, 3,2,0
                idx.extend_from_slice(&[
                    base + 2, base + 1, base,
                    base + 3, base + 2, base,
                ]);
            }
        }
    }
    idx
}
