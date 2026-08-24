//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: combineInstances).

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::developer_error::throw_developer_error;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::matrix4::Matrix4;
use crate::primitive_type::PrimitiveType;

/// Which geometry slot of a [`GeometryInstance`] to combine.
#[derive(Clone, Copy)]
enum PropertyName {
    Geometry,
    WestHemisphereGeometry,
    EastHemisphereGeometry,
}

fn instance_geometry<'a>(
    instance: &'a GeometryInstance,
    property_name: PropertyName,
) -> Option<&'a Geometry> {
    match property_name {
        PropertyName::Geometry => instance.geometry.as_geometry(),
        PropertyName::WestHemisphereGeometry => instance.west_hemisphere_geometry.as_ref(),
        PropertyName::EastHemisphereGeometry => instance.east_hemisphere_geometry.as_ref(),
    }
}

fn find_attributes_in_all_geometries(
    instances: &[&GeometryInstance],
    property_name: PropertyName,
) -> HashMap<String, GeometryAttribute> {
    let mut attributes_in_all_geometries: HashMap<String, GeometryAttribute> = HashMap::new();

    let attributes0 = &instance_geometry(instances[0], property_name).unwrap().attributes;

    let names: Vec<String> = attributes0.keys().cloned().collect();
    for name in names {
        let attribute = &attributes0[&name];
        let mut number_of_components = attribute.values.len();
        let mut in_all_geometries = true;

        // Does this same attribute exist in all geometries?
        for instance in instances.iter().skip(1) {
            let other_attribute =
                instance_geometry(instance, property_name).unwrap().attributes.get(&name);

            match other_attribute {
                Some(other)
                    if attribute.component_datatype == other.component_datatype
                        && attribute.components_per_attribute == other.components_per_attribute
                        && attribute.normalize == other.normalize =>
                {
                    number_of_components += other.values.len();
                }
                _ => {
                    in_all_geometries = false;
                    break;
                }
            }
        }

        if in_all_geometries {
            attributes_in_all_geometries.insert(
                name.clone(),
                GeometryAttribute::new(
                    attribute.component_datatype,
                    attribute.components_per_attribute,
                    attribute.normalize,
                    vec![0.0f64; number_of_components],
                ),
            );
        }
    }

    attributes_in_all_geometries
}

fn combine_geometries(
    instances: &[&GeometryInstance],
    property_name: PropertyName,
) -> Geometry {
    let length = instances.len();

    let m = &instances[0].model_matrix;
    let have_indices = instance_geometry(instances[0], property_name)
        .unwrap()
        .indices
        .is_some();
    let primitive_type = instance_geometry(instances[0], property_name)
        .unwrap()
        .primitive_type;

    if cfg!(debug_assertions) {
        for instance in instances.iter().skip(1) {
            if !Matrix4::equals(&instance.model_matrix, m) {
                throw_developer_error("All instances must have the same modelMatrix.");
            }
            let geometry = instance_geometry(instance, property_name).unwrap();
            if geometry.indices.is_some() != have_indices {
                throw_developer_error(
                    "All instance geometries must have an indices or not have one.",
                );
            }
            if geometry.primitive_type != primitive_type {
                throw_developer_error("All instance geometries must have the same primitiveType.");
            }
        }
    }

    // Find subset of attributes in all geometries
    let mut attributes = find_attributes_in_all_geometries(instances, property_name);

    // Combine attributes from each geometry into a single typed array
    let names: Vec<String> = attributes.keys().cloned().collect();
    for name in &names {
        let values_len = attributes[name].values.len();
        let values = &mut attributes.get_mut(name).unwrap().values[..values_len];
        let mut k = 0usize;
        for instance in instances {
            let source_values = &instance_geometry(instance, property_name)
                .unwrap()
                .attributes[name]
                .values;
            for &value in source_values {
                values[k] = value;
                k += 1;
            }
        }
    }

    // Combine index lists
    let indices: Option<IndexStorage> = if have_indices {
        let mut number_of_indices = 0usize;
        for instance in instances {
            number_of_indices += instance_geometry(instance, property_name)
                .unwrap()
                .indices
                .as_ref()
                .unwrap()
                .len();
        }

        let number_of_vertices = Geometry::new(
            attributes.clone(),
            None,
            Some(PrimitiveType::Points),
            None,
        )
        .compute_number_of_vertices()
        .unwrap_or(0);
        let mut dest_indices =
            IndexDatatype::create_typed_array(number_of_vertices, number_of_indices);

        let mut dest_offset = 0usize;
        let mut offset = 0usize;

        for instance in instances {
            let geometry = instance_geometry(instance, property_name).unwrap();
            let source_indices = geometry.indices.as_ref().unwrap();
            let source_indices_len = source_indices.len();

            for k in 0..source_indices_len {
                let value = match source_indices {
                    IndexStorage::U16(v) => v[k] as u32,
                    IndexStorage::U32(v) => v[k],
                };
                write_index(&mut dest_indices, dest_offset, offset as u32 + value);
                dest_offset += 1;
            }

            offset += geometry.compute_number_of_vertices().unwrap_or(0);
        }

        Some(dest_indices)
    } else {
        None
    };

    // Create bounding sphere that includes all instances
    let mut center: Option<Cartesian3> = Some(Cartesian3::ZERO);
    let mut radius = 0.0f64;

    for instance in instances {
        let bs = &instance_geometry(instance, property_name).unwrap().bounding_sphere;
        match (bs, &mut center) {
            (Some(bs), Some(center)) => {
                let mut sum = Cartesian3::ZERO;
                Cartesian3::add(&bs.center, center, &mut sum);
                *center = sum;
            }
            _ => {
                // If any geometries have an undefined bounding sphere, then so
                // does the combined geometry.
                center = None;
                break;
            }
        }
    }

    let bounding_sphere = if let Some(mut center) = center {
        // DEVIATION: JS does `divideByScalar(center, length, center)` in place;
        // Rust needs a distinct temporary to avoid simultaneous borrows.
        let mut center_tmp = Cartesian3::ZERO;
        Cartesian3::divide_by_scalar(&center, length as f64, &mut center_tmp);
        center = center_tmp;

        for instance in instances {
            let bs = instance_geometry(instance, property_name)
                .unwrap()
                .bounding_sphere
                .as_ref()
                .unwrap();
            let mut temp = Cartesian3::ZERO;
            Cartesian3::subtract(&bs.center, &center, &mut temp);
            let temp_radius = Cartesian3::magnitude(&temp) + bs.radius;

            if temp_radius > radius {
                radius = temp_radius;
            }
        }
        Some(BoundingSphere::new(center, radius))
    } else {
        None
    };

    Geometry::new(attributes, indices, Some(primitive_type), bounding_sphere)
}

/// Combines geometry from several [`GeometryInstance`] objects into one
/// geometry. This concatenates the attributes, concatenates and adjusts the
/// indices, and creates a bounding sphere encompassing all instances.
///
/// If the instances do not have the same attributes, a subset of attributes
/// common to all instances is used, and the others are ignored.
///
/// Port of `GeometryPipeline.combineInstances(instances)`.
///
/// # Panics (debug)
/// Panics if `instances` is empty.
pub fn combine_instances(instances: &[GeometryInstance]) -> Vec<Geometry> {
    if cfg!(debug_assertions) {
        if instances.is_empty() {
            throw_developer_error(
                "instances is required and must have length greater than zero.",
            );
        }
    }

    let mut instance_geometry: Vec<&GeometryInstance> = Vec::new();
    let mut instance_split_geometry: Vec<&GeometryInstance> = Vec::new();
    for instance in instances {
        if instance.geometry.as_geometry().is_some() {
            instance_geometry.push(instance);
        } else if instance.west_hemisphere_geometry.is_some()
            && instance.east_hemisphere_geometry.is_some()
        {
            instance_split_geometry.push(instance);
        }
    }

    let mut geometries: Vec<Geometry> = Vec::new();
    if !instance_geometry.is_empty() {
        geometries.push(combine_geometries(&instance_geometry, PropertyName::Geometry));
    }

    if !instance_split_geometry.is_empty() {
        geometries.push(combine_geometries(
            &instance_split_geometry,
            PropertyName::WestHemisphereGeometry,
        ));
        geometries.push(combine_geometries(
            &instance_split_geometry,
            PropertyName::EastHemisphereGeometry,
        ));
    }

    geometries
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
