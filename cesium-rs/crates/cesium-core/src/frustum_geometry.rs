//! Ported from `packages/engine/Source/Core/FrustumGeometry.js`.
//!
//! Describes a frustum at the given origin and orientation, and computes its
//! geometry (six planes: near, far, -x, -y, +x, +y).

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;
use crate::orthographic_frustum::OrthographicFrustum;
use crate::perspective_frustum::PerspectiveFrustum;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::vertex_format::VertexFormat;

const PERSPECTIVE: u32 = 0;
const ORTHOGRAPHIC: u32 = 1;

/// The frustum type used by [`FrustumGeometry`] / [`crate::frustum_outline_geometry::FrustumOutlineGeometry`].
#[derive(Debug, Clone)]
pub enum FrustumKind {
    Perspective(PerspectiveFrustum),
    Orthographic(OrthographicFrustum),
}

impl FrustumKind {
    fn packed_length(&self) -> usize {
        match self {
            FrustumKind::Perspective(_) => PerspectiveFrustum::PACKED_LENGTH,
            FrustumKind::Orthographic(_) => OrthographicFrustum::PACKED_LENGTH,
        }
    }

    /// Packs the frustum into `array` starting at `starting_index`,
    /// returning the new starting index after the frustum.
    fn pack_into(&self, array: &mut [f64], starting_index: usize) -> usize {
        match self {
            FrustumKind::Perspective(p) => {
                PerspectiveFrustum::pack(p, array, starting_index);
                starting_index + PerspectiveFrustum::PACKED_LENGTH
            }
            FrustumKind::Orthographic(o) => {
                OrthographicFrustum::pack(o, array, starting_index);
                starting_index + OrthographicFrustum::PACKED_LENGTH
            }
        }
    }

    /// The projection matrix of the frustum (mirrors JS `projectionMatrix` getter).
    fn projection_matrix(&mut self) -> Matrix4 {
        match self {
            FrustumKind::Perspective(p) => p.projection_matrix(),
            FrustumKind::Orthographic(o) => o.projection_matrix(),
        }
    }

    /// Off-center frustum bounds `(left, right, top, bottom)` (mirrors JS `offCenterFrustum`).
    fn off_center_bounds(&mut self) -> (f64, f64, f64, f64) {
        match self {
            FrustumKind::Perspective(p) => p.off_center_bounds(),
            FrustumKind::Orthographic(o) => o.off_center_bounds(),
        }
    }

    fn near_far(&self) -> (f64, f64) {
        match self {
            FrustumKind::Perspective(p) => (p.near, p.far),
            FrustumKind::Orthographic(o) => (o.near, o.far),
        }
    }
}

/// Describes a frustum at the given origin and orientation.
///
/// DEVIATION: JS `packedLength` is an instance property computed in the
/// constructor; Rust exposes it as `packed_length(&self)`.
#[derive(Debug, Clone)]
pub struct FrustumGeometry {
    frustum_type: u32,
    frustum: FrustumKind,
    origin: Cartesian3,
    orientation: Quaternion,
    draw_near_plane: bool,
    vertex_format: VertexFormat,
}

impl FrustumGeometry {
    /// Creates a FrustumGeometry from a perspective frustum described by
    /// `near`, `far`, `fov` and `aspect_ratio`.
    ///
    /// Retained for spec compatibility; the JS constructor takes an options object
    /// with an explicit frustum instance (see [`FrustumGeometry::from_frustum`]).
    /// `orientation` is a quaternion stored as a `Cartesian4` (x, y, z, w).
    pub fn new(
        origin: Cartesian3,
        orientation: Cartesian4,
        near: f64,
        far: f64,
        fov: f64,
        aspect_ratio: f64,
    ) -> Self {
        let mut frustum = PerspectiveFrustum::new();
        frustum.fov = Some(fov);
        frustum.aspect_ratio = Some(aspect_ratio);
        frustum.near = near;
        frustum.far = far;

        Self {
            frustum_type: PERSPECTIVE,
            frustum: FrustumKind::Perspective(frustum),
            origin,
            orientation: Quaternion::new(orientation.x, orientation.y, orientation.z, orientation.w),
            draw_near_plane: true,
            vertex_format: VertexFormat::default_format(),
        }
    }

    /// JS constructor equivalent: `new FrustumGeometry(options)`.
    pub fn from_frustum(
        frustum: FrustumKind,
        origin: Cartesian3,
        orientation: Quaternion,
        vertex_format: Option<VertexFormat>,
        draw_near_plane: Option<bool>,
    ) -> Self {
        let frustum_type = match &frustum {
            FrustumKind::Perspective(_) => PERSPECTIVE,
            FrustumKind::Orthographic(_) => ORTHOGRAPHIC,
        };
        Self {
            frustum_type,
            frustum,
            origin,
            orientation,
            draw_near_plane: draw_near_plane.unwrap_or(true),
            vertex_format: vertex_format.unwrap_or_else(VertexFormat::default_format),
        }
    }

    /// Accessors.
    pub fn origin(&self) -> &Cartesian3 {
        &self.origin
    }

    pub fn orientation(&self) -> &Quaternion {
        &self.orientation
    }

    pub fn frustum(&self) -> &FrustumKind {
        &self.frustum
    }

    pub fn vertex_format(&self) -> &VertexFormat {
        &self.vertex_format
    }

    pub fn draw_near_plane(&self) -> bool {
        self.draw_near_plane
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        2 + self.frustum.packed_length()
            + Cartesian3::PACKED_LENGTH
            + Quaternion::PACKED_LENGTH
            + VertexFormat::PACKED_LENGTH
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut i = starting_index.unwrap_or(0);

        array[i] = self.frustum_type as f64;
        i += 1;

        i = self.frustum.pack_into(array, i);

        Cartesian3::pack(&self.origin, array, Some(i));
        i += Cartesian3::PACKED_LENGTH;
        Quaternion::pack(&self.orientation, array, i);
        i += Quaternion::PACKED_LENGTH;
        self.vertex_format.pack(array, i);
        i += VertexFormat::PACKED_LENGTH;
        array[i] = if self.draw_near_plane { 1.0 } else { 0.0 };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut i = starting_index.unwrap_or(0);

        let frustum_type = array[i] as u32;
        i += 1;

        let frustum = if frustum_type == PERSPECTIVE {
            let f = PerspectiveFrustum::unpack(array, i, None);
            i += PerspectiveFrustum::PACKED_LENGTH;
            FrustumKind::Perspective(f)
        } else {
            let f = OrthographicFrustum::unpack(array, i, None);
            i += OrthographicFrustum::PACKED_LENGTH;
            FrustumKind::Orthographic(f)
        };

        let origin = Cartesian3::unpack_new(array, Some(i));
        i += Cartesian3::PACKED_LENGTH;
        let orientation = Quaternion::unpack_new(array, i);
        i += Quaternion::PACKED_LENGTH;
        let vertex_format = VertexFormat::unpack(array, i, None);
        i += VertexFormat::PACKED_LENGTH;
        let draw_near_plane = array[i] == 1.0;

        match result {
            Some(r) => {
                r.frustum_type = frustum_type;
                r.frustum = frustum;
                r.origin = origin;
                r.orientation = orientation;
                r.vertex_format = vertex_format;
                r.draw_near_plane = draw_near_plane;
                r.clone()
            }
            None => Self {
                frustum_type,
                frustum,
                origin,
                orientation,
                draw_near_plane,
                vertex_format,
            },
        }
    }

    /// Computes the 8 corner positions of the near and far planes.
    ///
    /// Mirrors JS `FrustumGeometry._computeNearFarPlanes`. Fills `positions`
    /// (near plane corners at `[0..12)`, far plane corners at `[12..24)`) and
    /// returns the `(x, y, z)` view directions, where `x` is negated and all
    /// three are normalized — exactly as the JS scratch variables are left.
    pub(crate) fn compute_near_far_planes(
        origin: &Cartesian3,
        orientation: &Quaternion,
        frustum_type: u32,
        frustum: &mut FrustumKind,
        positions: &mut [f64],
    ) -> (Cartesian3, Cartesian3, Cartesian3) {
        let rotation_matrix = Matrix3::from_quaternion_new(orientation);

        let mut x = Matrix3::get_column_new(&rotation_matrix, 0);
        let mut y = Matrix3::get_column_new(&rotation_matrix, 1);
        let mut z = Matrix3::get_column_new(&rotation_matrix, 2);

        let mut tmp = Cartesian3::default();
        Cartesian3::normalize(&x, &mut tmp);
        x = tmp;
        Cartesian3::normalize(&y, &mut tmp);
        y = tmp;
        Cartesian3::normalize(&z, &mut tmp);
        z = tmp;

        Cartesian3::negate(&x, &mut tmp);
        x = tmp;

        let view = Matrix4::compute_view_new(origin, &z, &y, &x);

        let mut inverse_view_projection: Option<Matrix4> = None;
        let mut inverse_view: Option<Matrix4> = None;
        if frustum_type == PERSPECTIVE {
            let projection = frustum.projection_matrix();
            let view_projection = Matrix4::multiply_new(&projection, &view);
            inverse_view_projection = Matrix4::inverse_new(&view_projection);
        } else {
            let mut iv = Matrix4::default();
            Matrix4::inverse_transformation(&view, &mut iv);
            inverse_view = Some(iv);
        }

        let (near, far) = frustum.near_far();
        let frustum_splits = if inverse_view_projection.is_some() {
            [near, far, 0.0]
        } else {
            [0.0, near, far]
        };

        // (left, right, top, bottom) of the off-center frustum (orthographic path).
        let mut off_center: Option<(f64, f64, f64, f64)> = None;

        let ndc_corners = [
            Cartesian4::new(-1.0, -1.0, 1.0, 1.0),
            Cartesian4::new(1.0, -1.0, 1.0, 1.0),
            Cartesian4::new(1.0, 1.0, 1.0, 1.0),
            Cartesian4::new(-1.0, 1.0, 1.0, 1.0),
        ];

        for i in 0..2usize {
            for j in 0..4usize {
                let mut corner = ndc_corners[j].clone();

                if inverse_view_projection.is_none() {
                    if off_center.is_none() {
                        off_center = Some(frustum.off_center_bounds());
                    }
                    let (left, right, top, bottom) = off_center.unwrap();

                    let split_near = frustum_splits[i];
                    let split_far = frustum_splits[i + 1];

                    corner.x = (corner.x * (right - left) + left + right) * 0.5;
                    corner.y = (corner.y * (top - bottom) + bottom + top) * 0.5;
                    corner.z = (corner.z * (split_near - split_far) - split_near - split_far) * 0.5;
                    corner.w = 1.0;

                    let mut corner_out = Cartesian4::default();
                    Matrix4::multiply_by_vector(inverse_view.as_ref().unwrap(), &corner, &mut corner_out);
                    corner = corner_out;
                } else {
                    let mut corner_out = Cartesian4::default();
                    Matrix4::multiply_by_vector(
                        inverse_view_projection.as_ref().unwrap(),
                        &corner,
                        &mut corner_out,
                    );
                    corner = corner_out;

                    // Reverse perspective divide
                    let w = 1.0 / corner.w;
                    let mut corner3 = Cartesian3::new(corner.x * w, corner.y * w, corner.z * w);

                    let mut c3 = Cartesian3::default();
                    Cartesian3::subtract(&corner3, origin, &mut c3);
                    Cartesian3::normalize(&c3, &mut corner3);

                    let fac = Cartesian3::dot(&z, &corner3);
                    Cartesian3::multiply_by_scalar(&corner3, frustum_splits[i] / fac, &mut c3);
                    Cartesian3::add(&c3, origin, &mut corner3);

                    corner.x = corner3.x;
                    corner.y = corner3.y;
                    corner.z = corner3.z;
                }

                positions[12 * i + j * 3] = corner.x;
                positions[12 * i + j * 3 + 1] = corner.y;
                positions[12 * i + j * 3 + 2] = corner.z;
            }
        }

        (x, y, z)
    }

    /// Computes the geometric representation of a frustum, including its
    /// vertices, indices, and a bounding sphere.
    pub fn create_geometry(frustum_geometry: &Self) -> Option<Geometry> {
        let frustum_type = frustum_geometry.frustum_type;
        let draw_near_plane = frustum_geometry.draw_near_plane;
        let vertex_format = &frustum_geometry.vertex_format;

        let number_of_planes = if draw_near_plane { 6 } else { 5 };
        let mut positions = vec![0.0f64; 3 * 4 * 6];
        let frustum = &mut frustum_geometry.frustum.clone();
        let (x, y, z) = FrustumGeometry::compute_near_far_planes(
            &frustum_geometry.origin,
            &frustum_geometry.orientation,
            frustum_type,
            frustum,
            &mut positions,
        );

        // -x plane
        let mut offset = 3 * 4 * 2;
        positions[offset] = positions[3 * 4];
        positions[offset + 1] = positions[3 * 4 + 1];
        positions[offset + 2] = positions[3 * 4 + 2];
        positions[offset + 3] = positions[0];
        positions[offset + 4] = positions[1];
        positions[offset + 5] = positions[2];
        positions[offset + 6] = positions[3 * 3];
        positions[offset + 7] = positions[3 * 3 + 1];
        positions[offset + 8] = positions[3 * 3 + 2];
        positions[offset + 9] = positions[3 * 7];
        positions[offset + 10] = positions[3 * 7 + 1];
        positions[offset + 11] = positions[3 * 7 + 2];

        // -y plane
        offset += 3 * 4;
        positions[offset] = positions[3 * 5];
        positions[offset + 1] = positions[3 * 5 + 1];
        positions[offset + 2] = positions[3 * 5 + 2];
        positions[offset + 3] = positions[3];
        positions[offset + 4] = positions[3 + 1];
        positions[offset + 5] = positions[3 + 2];
        positions[offset + 6] = positions[0];
        positions[offset + 7] = positions[1];
        positions[offset + 8] = positions[2];
        positions[offset + 9] = positions[3 * 4];
        positions[offset + 10] = positions[3 * 4 + 1];
        positions[offset + 11] = positions[3 * 4 + 2];

        // +x plane
        offset += 3 * 4;
        positions[offset] = positions[3];
        positions[offset + 1] = positions[3 + 1];
        positions[offset + 2] = positions[3 + 2];
        positions[offset + 3] = positions[3 * 5];
        positions[offset + 4] = positions[3 * 5 + 1];
        positions[offset + 5] = positions[3 * 5 + 2];
        positions[offset + 6] = positions[3 * 6];
        positions[offset + 7] = positions[3 * 6 + 1];
        positions[offset + 8] = positions[3 * 6 + 2];
        positions[offset + 9] = positions[3 * 2];
        positions[offset + 10] = positions[3 * 2 + 1];
        positions[offset + 11] = positions[3 * 2 + 2];

        // +y plane
        offset += 3 * 4;
        positions[offset] = positions[3 * 2];
        positions[offset + 1] = positions[3 * 2 + 1];
        positions[offset + 2] = positions[3 * 2 + 2];
        positions[offset + 3] = positions[3 * 6];
        positions[offset + 4] = positions[3 * 6 + 1];
        positions[offset + 5] = positions[3 * 6 + 2];
        positions[offset + 6] = positions[3 * 7];
        positions[offset + 7] = positions[3 * 7 + 1];
        positions[offset + 8] = positions[3 * 7 + 2];
        positions[offset + 9] = positions[3 * 3];
        positions[offset + 10] = positions[3 * 3 + 1];
        positions[offset + 11] = positions[3 * 3 + 2];

        if !draw_near_plane {
            positions = positions[3 * 4..].to_vec();
        }

        let mut attributes = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
        );

        if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent || vertex_format.st {
            let mut normals: Option<Vec<f64>> = if vertex_format.normal {
                Some(vec![0.0f64; 3 * 4 * number_of_planes])
            } else {
                None
            };
            let mut tangents: Option<Vec<f64>> = if vertex_format.tangent {
                Some(vec![0.0f64; 3 * 4 * number_of_planes])
            } else {
                None
            };
            let mut bitangents: Option<Vec<f64>> = if vertex_format.bitangent {
                Some(vec![0.0f64; 3 * 4 * number_of_planes])
            } else {
                None
            };
            let mut st: Option<Vec<f64>> = if vertex_format.st {
                Some(vec![0.0f64; 2 * 4 * number_of_planes])
            } else {
                None
            };

            let negative_x = Cartesian3::negate_new(&x);
            let negative_y = Cartesian3::negate_new(&y);
            let negative_z = Cartesian3::negate_new(&z);

            let mut attr_offset = 0usize;
            if draw_near_plane {
                get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &negative_z, &x, &y); // near
                attr_offset += 3 * 4;
            }
            get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &z, &negative_x, &y); // far
            attr_offset += 3 * 4;
            get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &negative_x, &negative_z, &y); // -x
            attr_offset += 3 * 4;
            get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &negative_y, &negative_z, &negative_x); // -y
            attr_offset += 3 * 4;
            get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &x, &z, &y); // +x
            attr_offset += 3 * 4;
            get_attributes(attr_offset, &mut normals, &mut tangents, &mut bitangents, &mut st, &y, &z, &negative_x); // +y

            if let Some(n) = normals {
                attributes.insert(
                    "normal".to_string(),
                    GeometryAttribute::new(ComponentDatatype::Float, 3, false, n),
                );
            }
            if let Some(t) = tangents {
                attributes.insert(
                    "tangent".to_string(),
                    GeometryAttribute::new(ComponentDatatype::Float, 3, false, t),
                );
            }
            if let Some(b) = bitangents {
                attributes.insert(
                    "bitangent".to_string(),
                    GeometryAttribute::new(ComponentDatatype::Float, 3, false, b),
                );
            }
            if let Some(s) = st {
                attributes.insert(
                    "st".to_string(),
                    GeometryAttribute::new(ComponentDatatype::Float, 2, false, s),
                );
            }
        }

        let mut indices = IndexDatatype::create_typed_array(4 * number_of_planes, 6 * number_of_planes);
        for i in 0..number_of_planes {
            let index_offset = i * 6;
            let index = i * 4;

            indices.push(index as u32);
            indices.push((index + 1) as u32);
            indices.push((index + 2) as u32);
            indices.push(index as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 3) as u32);
            let _ = index_offset;
        }

        let bounding_sphere = BoundingSphere::from_vertices(&positions, None, None, None);

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Triangles),
            Some(bounding_sphere),
            GeometryType::None,
            None,
            None,
        ))
    }
}

/// Mirrors JS `getAttributes`: writes the per-plane normal/tangent/bitangent
/// values for 4 vertices and the fixed st quad.
fn get_attributes(
    offset: usize,
    normals: &mut Option<Vec<f64>>,
    tangents: &mut Option<Vec<f64>>,
    bitangents: &mut Option<Vec<f64>>,
    st: &mut Option<Vec<f64>>,
    normal: &Cartesian3,
    tangent: &Cartesian3,
    bitangent: &Cartesian3,
) {
    let st_offset = (offset / 3) * 2;

    let mut o = offset;
    for _ in 0..4 {
        if let Some(n) = normals {
            n[o] = normal.x;
            n[o + 1] = normal.y;
            n[o + 2] = normal.z;
        }
        if let Some(t) = tangents {
            t[o] = tangent.x;
            t[o + 1] = tangent.y;
            t[o + 2] = tangent.z;
        }
        if let Some(b) = bitangents {
            b[o] = bitangent.x;
            b[o + 1] = bitangent.y;
            b[o + 2] = bitangent.z;
        }
        o += 3;
    }

    if let Some(s) = st {
        s[st_offset] = 0.0;
        s[st_offset + 1] = 0.0;
        s[st_offset + 2] = 1.0;
        s[st_offset + 3] = 0.0;
        s[st_offset + 4] = 1.0;
        s[st_offset + 5] = 1.0;
        s[st_offset + 6] = 0.0;
        s[st_offset + 7] = 1.0;
    }
}
