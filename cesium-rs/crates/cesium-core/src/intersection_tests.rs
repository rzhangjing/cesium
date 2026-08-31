//! Ported from `packages/engine/Source/Core/IntersectionTests.js`.

use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::interval::Interval;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::plane::Plane;
use crate::quadratic_real_polynomial::QuadraticRealPolynomial;
use crate::quartic_real_polynomial::QuarticRealPolynomial;
use crate::ray::Ray;

/// Functions for computing the intersection between geometries such as rays, planes, triangles, and ellipsoids.
pub struct IntersectionTests;

impl IntersectionTests {
    /// Computes the intersection of a ray and a plane.
    pub fn ray_plane(ray: &Ray, plane: &Plane) -> Option<Cartesian3> {
        let origin = &ray.origin;
        let direction = &ray.direction;
        let normal = &plane.normal;
        let denominator = Cartesian3::dot(normal, direction);

        if denominator.abs() < CesiumMath::EPSILON15 {
            return None;
        }

        let t = (-plane.distance - Cartesian3::dot(normal, origin)) / denominator;
        if t < 0.0 {
            return None;
        }

        let mut scaled = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(direction, t, &mut scaled);
        let mut sum = Cartesian3::ZERO;
        Cartesian3::add(origin, &scaled, &mut sum);
        Some(sum)
    }

    /// Computes the intersection of a ray and a triangle as a parametric distance along the ray.
    /// Uses Möller-Trumbore algorithm.
    pub fn ray_triangle_parametric(
        ray: &Ray,
        p0: &Cartesian3,
        p1: &Cartesian3,
        p2: &Cartesian3,
        cull_back_faces: bool,
    ) -> Option<f64> {
        let origin = &ray.origin;
        let direction = &ray.direction;

        let mut edge0 = Cartesian3::ZERO;
        Cartesian3::subtract(p1, p0, &mut edge0);
        let mut edge1 = Cartesian3::ZERO;
        Cartesian3::subtract(p2, p0, &mut edge1);

        let mut p = Cartesian3::ZERO;
        Cartesian3::cross(direction, &edge1, &mut p);
        let det = Cartesian3::dot(&edge0, &p);

        if cull_back_faces {
            if det < CesiumMath::EPSILON6 {
                return None;
            }
            let mut tvec = Cartesian3::ZERO;
            Cartesian3::subtract(origin, p0, &mut tvec);
            let u = Cartesian3::dot(&tvec, &p);
            if u < 0.0 || u > det {
                return None;
            }
            let mut q = Cartesian3::ZERO;
            Cartesian3::cross(&tvec, &edge0, &mut q);
            let v = Cartesian3::dot(direction, &q);
            if v < 0.0 || u + v > det {
                return None;
            }
            let t = Cartesian3::dot(&edge1, &q) / det;
            Some(t)
        } else {
            if det.abs() < CesiumMath::EPSILON6 {
                return None;
            }
            let inv_det = 1.0 / det;

            let mut tvec = Cartesian3::ZERO;
            Cartesian3::subtract(origin, p0, &mut tvec);
            let u = Cartesian3::dot(&tvec, &p) * inv_det;
            if u < 0.0 || u > 1.0 {
                return None;
            }
            let mut q = Cartesian3::ZERO;
            Cartesian3::cross(&tvec, &edge0, &mut q);
            let v = Cartesian3::dot(direction, &q) * inv_det;
            if v < 0.0 || u + v > 1.0 {
                return None;
            }
            let t = Cartesian3::dot(&edge1, &q) * inv_det;
            Some(t)
        }
    }

    /// Computes the intersection of a ray and a triangle as a Cartesian3 point.
    pub fn ray_triangle(
        ray: &Ray,
        p0: &Cartesian3,
        p1: &Cartesian3,
        p2: &Cartesian3,
        cull_back_faces: bool,
    ) -> Option<Cartesian3> {
        let t = Self::ray_triangle_parametric(ray, p0, p1, p2, cull_back_faces)?;
        if t < 0.0 {
            return None;
        }
        let mut result = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&ray.direction, t, &mut result);
        let mut point = Cartesian3::ZERO;
        Cartesian3::add(&ray.origin, &result, &mut point);
        Some(point)
    }

    /// Computes the intersection of a line segment and a triangle.
    pub fn line_segment_triangle(
        v0: &Cartesian3,
        v1: &Cartesian3,
        p0: &Cartesian3,
        p1: &Cartesian3,
        p2: &Cartesian3,
        cull_back_faces: bool,
    ) -> Option<Cartesian3> {
        let mut direction = Cartesian3::ZERO;
        Cartesian3::subtract(v1, v0, &mut direction);
        let distance = Cartesian3::magnitude(&direction);
        let mut dir_norm = Cartesian3::ZERO;
        Cartesian3::normalize(&direction, &mut dir_norm);

        let ray = Ray::new(Some(v0), Some(&dir_norm));
        let t = Self::ray_triangle_parametric(&ray, p0, p1, p2, cull_back_faces)?;
        if t < 0.0 || t > distance {
            return None;
        }

        let mut result = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&ray.direction, t, &mut result);
        let mut point = Cartesian3::ZERO;
        Cartesian3::add(&ray.origin, &result, &mut point);
        Some(point)
    }

    /// Computes the intersection points of a ray with a sphere.
    pub fn ray_sphere(ray: &Ray, sphere: &BoundingSphere) -> Option<Interval> {
        let origin = &ray.origin;
        let direction = &ray.direction;
        let center = &sphere.center;
        let radius_squared = sphere.radius * sphere.radius;

        let mut diff = Cartesian3::ZERO;
        Cartesian3::subtract(origin, center, &mut diff);

        let a = Cartesian3::dot(direction, direction);
        let b = 2.0 * Cartesian3::dot(direction, &diff);
        let c = Cartesian3::magnitude_squared(&diff) - radius_squared;

        let det = b * b - 4.0 * a * c;
        if det < 0.0 {
            return None;
        }

        let denom = 1.0 / (2.0 * a);
        let disc = det.sqrt();
        let mut root0 = (-b + disc) * denom;
        let mut root1 = (-b - disc) * denom;

        if root0 > root1 {
            std::mem::swap(&mut root0, &mut root1);
        }

        if root1 < 0.0 {
            return None;
        }

        root0 = root0.max(0.0);
        Some(Interval::new(root0, root1))
    }

    /// Computes the intersection points of a line segment with a sphere.
    pub fn line_segment_sphere(
        p0: &Cartesian3,
        p1: &Cartesian3,
        sphere: &BoundingSphere,
    ) -> Option<Interval> {
        let mut direction = Cartesian3::ZERO;
        Cartesian3::subtract(p1, p0, &mut direction);
        let max_t = Cartesian3::magnitude(&direction);
        let mut dir_norm = Cartesian3::ZERO;
        Cartesian3::normalize(&direction, &mut dir_norm);

        let ray = Ray::new(Some(p0), Some(&dir_norm));
        let mut result = Self::ray_sphere(&ray, sphere)?;

        if result.stop < 0.0 || result.start > max_t {
            return None;
        }

        result.start = result.start.max(0.0);
        result.stop = result.stop.min(max_t);
        Some(result)
    }

    /// Computes the intersection points of a ray with an ellipsoid.
    pub fn ray_ellipsoid(ray: &Ray, ellipsoid: &Ellipsoid) -> Option<Interval> {
        let inverse_radii = ellipsoid.one_over_radii();
        let mut q = Cartesian3::ZERO;
        Cartesian3::multiply_components(inverse_radii, &ray.origin, &mut q);
        let mut w = Cartesian3::ZERO;
        Cartesian3::multiply_components(inverse_radii, &ray.direction, &mut w);

        let q2 = Cartesian3::magnitude_squared(&q);
        let qw = Cartesian3::dot(&q, &w);

        if q2 > 1.0 {
            if qw >= 0.0 {
                return None;
            }
            let qw2 = qw * qw;
            let difference = q2 - 1.0;
            let w2 = Cartesian3::magnitude_squared(&w);
            let product = w2 * difference;

            if qw2 < product {
                return None;
            } else if qw2 > product {
                let discriminant = qw2 - product;
                let temp = -qw + discriminant.sqrt();
                let root0 = temp / w2;
                let root1 = difference / temp;
                if root0 < root1 {
                    return Some(Interval::new(root0, root1));
                }
                return Some(Interval::new(root1, root0));
            }
            let root = (difference / w2).sqrt();
            return Some(Interval::new(root, root));
        } else if q2 < 1.0 {
            let difference = q2 - 1.0;
            let w2 = Cartesian3::magnitude_squared(&w);
            let product = w2 * difference;
            let discriminant = qw * qw - product;
            let temp = -qw + discriminant.sqrt();
            return Some(Interval::new(0.0, temp / w2));
        }
        // q2 == 1.0, on ellipsoid
        if qw < 0.0 {
            let w2 = Cartesian3::magnitude_squared(&w);
            return Some(Interval::new(0.0, -qw / w2));
        }
        None
    }

    /// Computes the intersection points of a ray with an axis-aligned bounding box.
    pub fn ray_axis_aligned_bounding_box(
        ray: &Ray,
        box_: &AxisAlignedBoundingBox,
    ) -> Option<Interval> {
        let tx = ray_interval_along_aabb_axis(
            ray.origin.x,
            ray.direction.x,
            box_.minimum.x,
            box_.maximum.x,
        );
        let ty = ray_interval_along_aabb_axis(
            ray.origin.y,
            ray.direction.y,
            box_.minimum.y,
            box_.maximum.y,
        );
        let tz = ray_interval_along_aabb_axis(
            ray.origin.z,
            ray.direction.z,
            box_.minimum.z,
            box_.maximum.z,
        );

        let mut start = if tx.start > ty.start { tx.start } else { ty.start };
        let mut stop = if tx.stop < ty.stop { tx.stop } else { ty.stop };

        if tx.start > ty.stop || ty.start > tx.stop {
            return None;
        }
        if start > tz.stop || tz.start > stop {
            return None;
        }
        if tz.start > start {
            start = tz.start;
        }
        if tz.stop < stop {
            stop = tz.stop;
        }

        Some(Interval::new(start, stop))
    }

    /// Computes the intersection of a line segment and a plane.
    pub fn line_segment_plane(
        end_point0: &Cartesian3,
        end_point1: &Cartesian3,
        plane: &Plane,
    ) -> Option<Cartesian3> {
        let mut difference = Cartesian3::ZERO;
        Cartesian3::subtract(end_point1, end_point0, &mut difference);
        let normal = &plane.normal;
        let n_dot_diff = Cartesian3::dot(normal, &difference);

        if n_dot_diff.abs() < CesiumMath::EPSILON6 {
            return None;
        }

        let n_dot_p0 = Cartesian3::dot(normal, end_point0);
        let t = -(plane.distance + n_dot_p0) / n_dot_diff;

        if t < 0.0 || t > 1.0 {
            return None;
        }

        let mut result = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&difference, t, &mut result);
        let mut point = Cartesian3::ZERO;
        Cartesian3::add(end_point0, &result, &mut point);
        Some(point)
    }

    /// Provides the point along the ray which is nearest to the ellipsoid.
    pub fn grazing_altitude_location(ray: &Ray, ellipsoid: &Ellipsoid) -> Option<Cartesian3> {
        let position = &ray.origin;
        let direction = &ray.direction;

        if !Cartesian3::equals(Some(position), Some(&Cartesian3::ZERO)) {
            let mut normal = Cartesian3::ZERO;
            ellipsoid.geodetic_surface_normal(position, &mut normal);
            if Cartesian3::dot(direction, &normal) >= 0.0 {
                // The location provided is the closest point in altitude.
                return Some(*position);
            }
        }

        let intersects = Self::ray_ellipsoid(ray, ellipsoid).is_some();

        // Compute the scaled direction vector.
        let mut f = Cartesian3::ZERO;
        ellipsoid.transform_position_to_scaled_space(direction, &mut f);

        // Constructs a basis from the unit scaled direction vector. Construct
        // its rotation and transpose.
        let first_axis = Cartesian3::normalize_new(&f);
        let mut reference = Cartesian3::ZERO;
        Cartesian3::most_orthogonal_axis(&f, &mut reference);
        let second_axis =
            Cartesian3::normalize_new(&Cartesian3::cross_new(&reference, &first_axis));
        let third_axis =
            Cartesian3::normalize_new(&Cartesian3::cross_new(&first_axis, &second_axis));

        // JS stores these column-major: column 0 = firstAxis, column 1 =
        // secondAxis, column 2 = thirdAxis.
        let b = Matrix3::new(
            first_axis.x,
            second_axis.x,
            third_axis.x,
            first_axis.y,
            second_axis.y,
            third_axis.y,
            first_axis.z,
            second_axis.z,
            third_axis.z,
        );
        let b_t = Matrix3::transpose_new(&b);

        // Get the scaling matrix and its inverse.
        let d_i = Matrix3::from_scale_new(ellipsoid.radii());
        let d = Matrix3::from_scale_new(ellipsoid.one_over_radii());

        let c = Matrix3::new(
            0.0,
            direction.z,
            -direction.y,
            -direction.z,
            0.0,
            direction.x,
            direction.y,
            -direction.x,
            0.0,
        );

        let temp = Matrix3::multiply_new(&Matrix3::multiply_new(&b_t, &d), &c);
        let a = Matrix3::multiply_new(&Matrix3::multiply_new(&temp, &d_i), &b);
        let b_vec = Matrix3::multiply_by_vector_new(&temp, position);

        // Solve for the solutions to the expression in standard form.
        let solutions = Self::quadratic_vector_expression(
            &a,
            &Cartesian3::negate_new(&b_vec),
            0.0,
            0.0,
            1.0,
        );

        let length = solutions.len();
        if length > 0 {
            let mut closest = Cartesian3::ZERO;
            let mut maximum_value = f64::NEG_INFINITY;

            for solution in &solutions {
                let s = Matrix3::multiply_by_vector_new(
                    &d_i,
                    &Matrix3::multiply_by_vector_new(&b, solution),
                );
                let mut difference = Cartesian3::ZERO;
                Cartesian3::subtract(&s, position, &mut difference);
                let v = Cartesian3::normalize_new(&difference);
                let dot_product = Cartesian3::dot(&v, direction);

                if dot_product > maximum_value {
                    maximum_value = dot_product;
                    closest = s;
                }
            }

            let mut surface_point = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(&closest, &mut surface_point);
            maximum_value = CesiumMath::clamp(maximum_value, 0.0, 1.0);
            let mut difference = Cartesian3::ZERO;
            Cartesian3::subtract(&closest, position, &mut difference);
            let mut altitude = Cartesian3::magnitude(&difference)
                * (1.0 - maximum_value * maximum_value).sqrt();
            altitude = if intersects { -altitude } else { altitude };
            surface_point.height = altitude;
            let mut result = Cartesian3::ZERO;
            ellipsoid.cartographic_to_cartesian(&surface_point, &mut result);
            return Some(result);
        }

        None
    }

    /// Port of the private `addWithCancellationCheck` helper.
    fn add_with_cancellation_check(
        left: f64,
        right: f64,
        tolerance: Option<f64>,
    ) -> f64 {
        let difference = left + right;
        if let Some(tolerance) = tolerance {
            if CesiumMath::sign(left) != CesiumMath::sign(right)
                && (difference / left.abs().max(right.abs())).abs() < tolerance
            {
                return 0.0;
            }
        }
        difference
    }

    /// Port of `IntersectionTests.quadraticVectorExpression` (`@private`).
    pub fn quadratic_vector_expression(
        a: &Matrix3,
        b: &Cartesian3,
        c: f64,
        x: f64,
        w: f64,
    ) -> Vec<Cartesian3> {
        let x_squared = x * x;
        let w_squared = w * w;

        let l2 = (a.elements[4] - a.elements[8]) * w_squared;
        let l1 = w
            * (x
                * Self::add_with_cancellation_check(
                    a.elements[3],
                    a.elements[1],
                    Some(CesiumMath::EPSILON15),
                )
                + b.y);
        let l0 =
            a.elements[0] * x_squared + a.elements[8] * w_squared + x * b.x + c;

        let r1 = w_squared
            * Self::add_with_cancellation_check(
                a.elements[7],
                a.elements[5],
                Some(CesiumMath::EPSILON15),
            );
        // JS calls `addWithCancellationCheck` here without a tolerance
        // (undefined), which never zeroes the sum; mirror that exactly.
        let r0 = w
            * (x * Self::add_with_cancellation_check(a.elements[6], a.elements[2], None)
                + b.z);

        let mut solutions = Vec::new();
        if r0 == 0.0 && r1 == 0.0 {
            let cosines = QuadraticRealPolynomial::compute_real_roots(l2, l1, l0);
            if cosines.is_empty() {
                return solutions;
            }

            let cosine0 = cosines[0];
            let sine0 = (1.0 - cosine0 * cosine0).max(0.0).sqrt();
            solutions.push(Cartesian3::new(x, w * cosine0, w * -sine0));
            solutions.push(Cartesian3::new(x, w * cosine0, w * sine0));

            if cosines.len() == 2 {
                let cosine1 = cosines[1];
                let sine1 = (1.0 - cosine1 * cosine1).max(0.0).sqrt();
                solutions.push(Cartesian3::new(x, w * cosine1, w * -sine1));
                solutions.push(Cartesian3::new(x, w * cosine1, w * sine1));
            }

            return solutions;
        }

        let r0_squared = r0 * r0;
        let r1_squared = r1 * r1;
        let l2_squared = l2 * l2;
        let r0r1 = r0 * r1;

        let c4 = l2_squared + r1_squared;
        let c3 = 2.0 * (l1 * l2 + r0r1);
        let c2 = 2.0 * l0 * l2 + l1 * l1 - r1_squared + r0_squared;
        let c1 = 2.0 * (l0 * l1 - r0r1);
        let c0 = l0 * l0 - r0_squared;

        if c4 == 0.0 && c3 == 0.0 && c2 == 0.0 && c1 == 0.0 {
            return solutions;
        }

        let cosines = QuarticRealPolynomial::compute_real_roots(c4, c3, c2, c1, c0);
        let length = cosines.len();
        if length == 0 {
            return solutions;
        }

        let mut i = 0;
        while i < length {
            let cosine = cosines[i];
            let cosine_squared = cosine * cosine;
            let sine_squared = (1.0 - cosine_squared).max(0.0);
            let sine = sine_squared.sqrt();

            let left;
            if CesiumMath::sign(l2) == CesiumMath::sign(l0) {
                left = Self::add_with_cancellation_check(
                    l2 * cosine_squared + l0,
                    l1 * cosine,
                    Some(CesiumMath::EPSILON12),
                );
            } else if CesiumMath::sign(l0) == CesiumMath::sign(l1 * cosine) {
                left = Self::add_with_cancellation_check(
                    l2 * cosine_squared,
                    l1 * cosine + l0,
                    Some(CesiumMath::EPSILON12),
                );
            } else {
                left = Self::add_with_cancellation_check(
                    l2 * cosine_squared + l1 * cosine,
                    l0,
                    Some(CesiumMath::EPSILON12),
                );
            }

            let right = Self::add_with_cancellation_check(
                r1 * cosine,
                r0,
                Some(CesiumMath::EPSILON15),
            );
            let product = left * right;

            if product < 0.0 {
                solutions.push(Cartesian3::new(x, w * cosine, w * sine));
            } else if product > 0.0 {
                solutions.push(Cartesian3::new(x, w * cosine, w * -sine));
            } else if sine != 0.0 {
                solutions.push(Cartesian3::new(x, w * cosine, w * -sine));
                solutions.push(Cartesian3::new(x, w * cosine, w * sine));
                // JS: `++i` inside the loop body skips the next root.
                i += 1;
            } else {
                solutions.push(Cartesian3::new(x, w * cosine, w * sine));
            }
            i += 1;
        }

        solutions
    }
}

fn ray_interval_along_aabb_axis(
    origin: f64,
    direction: f64,
    min: f64,
    max: f64,
) -> Interval {
    let mut start = (min - origin) / direction;
    let mut stop = (max - origin) / direction;
    if stop < start {
        std::mem::swap(&mut start, &mut stop);
    }
    Interval::new(start, stop)
}
