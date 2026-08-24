//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: transformToWorldCoordinates).

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;

fn transform_point(matrix: &Matrix4, attribute: Option<&mut GeometryAttribute>) {
    if let Some(attribute) = attribute {
        let values = &mut attribute.values;
        let length = values.len();
        let mut scratch = Cartesian3::ZERO;
        let mut out = Cartesian3::ZERO;
        let mut i = 0usize;
        while i < length {
            Cartesian3::unpack(values, Some(i), &mut scratch);
            Matrix4::multiply_by_point(matrix, &scratch, &mut out);
            Cartesian3::pack(&out, values, Some(i));
            i += 3;
        }
    }
}

fn transform_vector(matrix: &Matrix3, attribute: Option<&mut GeometryAttribute>) {
    if let Some(attribute) = attribute {
        let values = &mut attribute.values;
        let length = values.len();
        let mut scratch = Cartesian3::ZERO;
        let mut out = Cartesian3::ZERO;
        let mut normalized = Cartesian3::ZERO;
        let mut i = 0usize;
        while i < length {
            Cartesian3::unpack(values, Some(i), &mut scratch);
            Matrix3::multiply_by_vector(matrix, &scratch, &mut out);
            Cartesian3::normalize(&out, &mut normalized);
            Cartesian3::pack(&normalized, values, Some(i));
            i += 3;
        }
    }
}

/// Transforms a geometry instance to world coordinates. This changes the
/// instance's `model_matrix` to [`Matrix4::IDENTITY`] and transforms the
/// following attributes if they are present: `position`, `normal`, `tangent`,
/// and `bitangent`.
///
/// Port of `GeometryPipeline.transformToWorldCoordinates(instance)`.
pub fn transform_to_world_coordinates(instance: &mut GeometryInstance) {
    let model_matrix = instance.model_matrix.clone();

    if Matrix4::equals(&model_matrix, &Matrix4::IDENTITY) {
        // Already in world coordinates
        return;
    }

    let geometry = match &mut instance.geometry {
        GeometryInstanceGeometry::Geometry(geometry) => geometry,
        GeometryInstanceGeometry::Placeholder => return,
    };
    let geometry: &mut Geometry = geometry;

    // Transform attributes in known vertex formats
    transform_point(&model_matrix, geometry.attributes.get_mut("position"));
    transform_point(&model_matrix, geometry.attributes.get_mut("prevPosition"));
    transform_point(&model_matrix, geometry.attributes.get_mut("nextPosition"));

    if geometry.attributes.contains_key("normal")
        || geometry.attributes.contains_key("tangent")
        || geometry.attributes.contains_key("bitangent")
    {
        let mut inverse_transpose = Matrix4::default();
        let mut transpose_tmp = Matrix4::default();
        Matrix4::inverse(&model_matrix, &mut inverse_transpose);
        Matrix4::transpose(&inverse_transpose, &mut transpose_tmp);
        let mut normal_matrix = Matrix3::default();
        Matrix4::get_matrix3(&transpose_tmp, &mut normal_matrix);

        transform_vector(&normal_matrix, geometry.attributes.get_mut("normal"));
        transform_vector(&normal_matrix, geometry.attributes.get_mut("tangent"));
        transform_vector(&normal_matrix, geometry.attributes.get_mut("bitangent"));
    }

    let bounding_sphere = geometry.bounding_sphere.clone();
    if let Some(bounding_sphere) = bounding_sphere {
        geometry.bounding_sphere = Some(BoundingSphere::transform(
            &bounding_sphere,
            &model_matrix,
            None,
        ));
    }

    instance.model_matrix = Matrix4::IDENTITY.clone();
}
