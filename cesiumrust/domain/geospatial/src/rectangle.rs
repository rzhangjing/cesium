//! Rectangle —— 由西、南、东、北定义的一个二维区域。
//! 映射到 CesiumJS `Core/Rectangle.js`
//!
//! 对原版 CesiumJS `Rectangle` 的忠实移植，包含
//! `intersection`/`union`/`center`/
//! `contains`/`subsection` 中对反经线（IDL）穿越的处理逻辑，以及
//! `fromCartographicArray`/`fromCartesianArray` 的"最小外接矩形"逻辑。

// 遗留的 CesiumJS 移植风格技术债（deferred.md #18）；在 M13 lint-cleanup
// 或本文件在其里程碑被重写时重新审视
#![allow(clippy::neg_cmp_op_on_partial_ord)]
use crate::bounding::BoundingSphere;
use crate::cartographic::Cartographic;
use crate::ellipsoid::{self, Ellipsoid};
use crate::math_utils::{self, EPSILON14, PI_OVER_TWO, TWO_PI};
use crate::transforms;
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 由经度/纬度边界（以弧度计）定义的二维区域。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    /// 最西侧经度，弧度制 [-PI, PI]。
    pub west: f64,
    /// 最南侧纬度，弧度制 [-PI/2, PI/2]。
    pub south: f64,
    /// 最东侧经度，弧度制 [-PI, PI]。
    pub east: f64,
    /// 最北侧纬度，弧度制 [-PI/2, PI/2]。
    pub north: f64,
}

impl Rectangle {
    /// 可能存在的最大矩形。映射到 `Rectangle.MAX_VALUE`。
    pub const MAX_VALUE: Self = Self {
        west: -PI,
        south: -PI_OVER_TWO,
        east: PI,
        north: PI_OVER_TWO,
    };

    /// 空的（全零）矩形，等价于 CesiumJS 的 `new Rectangle()`。
    pub const EMPTY: Self = Self {
        west: 0.0,
        south: 0.0,
        east: 0.0,
        north: 0.0,
    };

    /// 将该对象打包进数组时所使用的元素个数。
    /// 映射到 `Rectangle.packedLength`。
    pub const PACKED_LENGTH: usize = 4;

    /// 由弧度创建一个新 Rectangle。
    /// 映射到 CesiumJS `Rectangle` 构造函数。
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// 给定以度为单位的边界经纬度创建矩形。
    /// 映射到 `Rectangle.fromDegrees`
    pub fn from_degrees(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west: math_utils::to_radians(west),
            south: math_utils::to_radians(south),
            east: math_utils::to_radians(east),
            north: math_utils::to_radians(north),
        }
    }

    /// 给定以弧度为单位的边界经纬度创建矩形。
    /// 映射到 `Rectangle.fromRadians`
    pub fn from_radians(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self::new(west, south, east, north)
    }

    /// 将给定的实例存入给定的数组。
    /// 映射到 `Rectangle.pack`
    pub fn pack_into(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.west;
        array[starting_index + 1] = self.south;
        array[starting_index + 2] = self.east;
        array[starting_index + 3] = self.north;
    }

    /// 将该矩形打包进一个新的 `[f64; 4]`（`[west, south, east, north]`）。
    pub fn pack(&self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }

    /// 从打包数组中取回一个实例。
    /// 映射到 `Rectangle.unpack`
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            west: array[starting_index],
            south: array[starting_index + 1],
            east: array[starting_index + 2],
            north: array[starting_index + 3],
        }
    }

    /// 以弧度计算矩形的宽度。
    /// 映射到 `Rectangle.computeWidth`
    pub fn width(&self) -> f64 {
        let mut east = self.east;
        let west = self.west;
        if east < west {
            east += TWO_PI;
        }
        east - west
    }

    /// 以弧度计算矩形的高度。
    /// 映射到 `Rectangle.computeHeight`
    pub fn height(&self) -> f64 {
        self.north - self.south
    }

    /// 创建能包围给定数组中所有位置的最小可能 Rectangle。
    /// 映射到 `Rectangle.fromCartographicArray`
    pub fn from_cartographic_array(cartographics: &[Cartographic]) -> Self {
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for position in cartographics {
            west = west.min(position.longitude);
            east = east.max(position.longitude);
            south = south.min(position.latitude);
            north = north.max(position.latitude);

            let lon_adjusted = if position.longitude >= 0.0 {
                position.longitude
            } else {
                position.longitude + TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;

            if east > PI {
                east -= TWO_PI;
            }
            if west > PI {
                west -= TWO_PI;
            }
        }

        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// 创建能包围给定的笛卡尔位置数组中所有位置的最小可能 Rectangle。
    /// 映射到 `Rectangle.fromCartesianArray`
    pub fn from_cartesian_array(cartesians: &[DVec3], ellipsoid: &Ellipsoid) -> Self {
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for cartesian in cartesians {
            let position = ellipsoid
                .cartesian_to_cartographic(*cartesian)
                .expect("cartesian must not be at the center of the ellipsoid");
            west = west.min(position.longitude);
            east = east.max(position.longitude);
            south = south.min(position.latitude);
            north = north.max(position.latitude);

            let lon_adjusted = if position.longitude >= 0.0 {
                position.longitude
            } else {
                position.longitude + TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;

            if east > PI {
                east -= TWO_PI;
            }
            if west > PI {
                west -= TWO_PI;
            }
        }

        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// 由包围球创建一个矩形，忽略高度。
    /// 映射到 `Rectangle.fromBoundingSphere`
    pub fn from_bounding_sphere(bounding_sphere: &BoundingSphere, ellipsoid: &Ellipsoid) -> Self {
        let center = bounding_sphere.center;
        let radius = bounding_sphere.radius;

        if center == DVec3::ZERO {
            return Self::MAX_VALUE;
        }

        let from_enu = transforms::east_north_up_to_fixed_frame(center, ellipsoid);
        // Matrix4.multiplyByPointAsVector：仅应用线性（旋转）部分。
        let east = ellipsoid::normalize_cartesian3(from_enu.transform_vector3(DVec3::X));
        let north = ellipsoid::normalize_cartesian3(from_enu.transform_vector3(DVec3::Y));

        let north = north * radius;
        let east = east * radius;
        let south = -north;
        let west = -east;

        let positions = [
            center + north,
            center + west,
            center + south,
            center + east,
            center,
        ];
        Self::from_cartesian_array(&positions, ellipsoid)
    }

    /// 检查矩形的各属性，若它们不在有效范围内则返回错误。
    /// 映射到 `Rectangle._validate`（CesiumJS 抛出 `DeveloperError`；Rust
    /// 返回 `Err`，从而使该检查无需 panic 即可测试）。
    pub fn validate(&self) -> Result<(), String> {
        let north = self.north;
        if !(north >= -PI_OVER_TWO) || !(north <= PI_OVER_TWO) {
            return Err("north must be in the interval [-Pi/2, Pi/2].".to_string());
        }

        let south = self.south;
        if !(south >= -PI_OVER_TWO) || !(south <= PI_OVER_TWO) {
            return Err("south must be in the interval [-Pi/2, Pi/2].".to_string());
        }

        let west = self.west;
        if !(west >= -PI) || !(west <= PI) {
            return Err("west must be in the interval [-Pi, Pi].".to_string());
        }

        let east = self.east;
        if !(east >= -PI) || !(east <= PI) {
            return Err("east must be in the interval [-Pi, Pi].".to_string());
        }

        Ok(())
    }

    /// 计算矩形的西南角。
    /// 映射到 `Rectangle.southwest`
    pub fn southwest(&self) -> Cartographic {
        Cartographic::from_radians(self.west, self.south, 0.0)
    }

    /// 计算矩形的西北角。
    /// 映射到 `Rectangle.northwest`
    pub fn northwest(&self) -> Cartographic {
        Cartographic::from_radians(self.west, self.north, 0.0)
    }

    /// 计算矩形的东北角。
    /// 映射到 `Rectangle.northeast`
    pub fn northeast(&self) -> Cartographic {
        Cartographic::from_radians(self.east, self.north, 0.0)
    }

    /// 计算矩形的东南角。
    /// 映射到 `Rectangle.southeast`
    pub fn southeast(&self) -> Cartographic {
        Cartographic::from_radians(self.east, self.south, 0.0)
    }

    /// 计算矩形的中心。
    /// 映射到 `Rectangle.center`
    pub fn center(&self) -> Cartographic {
        let mut east = self.east;
        let west = self.west;

        if east < west {
            east += TWO_PI;
        }

        let longitude = math_utils::negative_pi_to_pi((west + east) * 0.5);
        let latitude = (self.south + self.north) * 0.5;

        Cartographic::from_radians(longitude, latitude, 0.0)
    }

    /// 计算两个矩形的交集，考虑经度在反经线处的环绕。
    /// 映射到 `Rectangle.intersection`
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let mut rectangle_east = self.east;
        let mut rectangle_west = self.west;

        let mut other_rectangle_east = other.east;
        let mut other_rectangle_west = other.west;

        if rectangle_east < rectangle_west && other_rectangle_east > 0.0 {
            rectangle_east += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_east > 0.0 {
            other_rectangle_east += TWO_PI;
        }

        if rectangle_east < rectangle_west && other_rectangle_west < 0.0 {
            other_rectangle_west += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_west < 0.0 {
            rectangle_west += TWO_PI;
        }

        let west = math_utils::negative_pi_to_pi(rectangle_west.max(other_rectangle_west));
        let east = math_utils::negative_pi_to_pi(rectangle_east.min(other_rectangle_east));

        if (self.west < self.east || other.west < other.east) && east <= west {
            return None;
        }

        let south = self.south.max(other.south);
        let north = self.north.min(other.north);

        if south >= north {
            return None;
        }

        Some(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// 计算两个矩形的简单交集，忽略反经线
    /// （可用于投影坐标）。
    /// 映射到 `Rectangle.simpleIntersection`
    pub fn simple_intersection(&self, other: &Self) -> Option<Self> {
        let west = self.west.max(other.west);
        let south = self.south.max(other.south);
        let east = self.east.min(other.east);
        let north = self.north.min(other.north);

        if south >= north || west >= east {
            return None;
        }

        Some(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// 计算作为两个矩形并集的矩形，考虑经度在反经线处的环绕。
    /// 映射到 `Rectangle.union`
    pub fn union(&self, other: &Self) -> Self {
        let mut rectangle_east = self.east;
        let mut rectangle_west = self.west;

        let mut other_rectangle_east = other.east;
        let mut other_rectangle_west = other.west;

        if rectangle_east < rectangle_west && other_rectangle_east > 0.0 {
            rectangle_east += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_east > 0.0 {
            other_rectangle_east += TWO_PI;
        }

        if rectangle_east < rectangle_west && other_rectangle_west < 0.0 {
            other_rectangle_west += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_west < 0.0 {
            rectangle_west += TWO_PI;
        }

        let west = math_utils::negative_pi_to_pi(rectangle_west.min(other_rectangle_west));
        let east = math_utils::negative_pi_to_pi(rectangle_east.max(other_rectangle_east));

        Self {
            west,
            south: self.south.min(other.south),
            east,
            north: self.north.max(other.north),
        }
    }

    /// 通过不断放大本矩形直到其包含给定的测绘坐标，从而计算出一个矩形。
    /// 映射到 `Rectangle.expand`（CesiumJS 会扩展以包围一个点；
    /// 该点的高度被忽略）。
    pub fn expand(&self, cartographic: &Cartographic) -> Self {
        Self {
            west: self.west.min(cartographic.longitude),
            south: self.south.min(cartographic.latitude),
            east: self.east.max(cartographic.longitude),
            north: self.north.max(cartographic.latitude),
        }
    }

    /// 若测绘位置（经度/纬度，弧度制）位于矩形上或其内部则返回 true，
    /// 否则返回 false。
    /// 映射到 `Rectangle.contains`
    pub fn contains(&self, longitude: f64, latitude: f64) -> bool {
        let mut longitude = longitude;

        let west = self.west;
        let mut east = self.east;

        if east < west {
            east += TWO_PI;
            if longitude < 0.0 {
                longitude += TWO_PI;
            }
        }
        (longitude > west
            || math_utils::equals_epsilon(longitude, west, EPSILON14, EPSILON14))
            && (longitude < east
                || math_utils::equals_epsilon(longitude, east, EPSILON14, EPSILON14))
            && latitude >= self.south
            && latitude <= self.north
    }

    /// 对矩形进行采样，使其包含一组适合传给
    /// `BoundingSphere.fromPoints` 的笛卡尔点。采样对于覆盖极点或
    /// 跨越赤道的矩形而言是必要的。
    /// 映射到 `Rectangle.subsample`
    pub fn subsample(&self, ellipsoid: &Ellipsoid, surface_height: f64) -> Vec<DVec3> {
        let mut result = Vec::new();

        let north = self.north;
        let south = self.south;
        let east = self.east;
        let west = self.west;

        let mut lla = Cartographic::from_radians(west, north, surface_height);
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.longitude = east;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.latitude = south;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.longitude = west;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        if north < 0.0 {
            lla.latitude = north;
        } else if south > 0.0 {
            lla.latitude = south;
        } else {
            lla.latitude = 0.0;
        }

        for i in 1..8 {
            lla.longitude = -PI + i as f64 * math_utils::PI_OVER_TWO;
            if self.contains(lla.longitude, lla.latitude) {
                result.push(ellipsoid.cartographic_to_cartesian(&lla));
            }
        }

        if lla.latitude == 0.0 {
            lla.longitude = west;
            result.push(ellipsoid.cartographic_to_cartesian(&lla));
            lla.longitude = east;
            result.push(ellipsoid.cartographic_to_cartesian(&lla));
        }
        result
    }

    /// 由 [0.0, 1.0] 范围内的归一化坐标计算矩形的一个子区域。
    /// 映射到 `Rectangle.subsection`（CesiumJS 对超出范围的 lerp 抛出
    /// `DeveloperError`；Rust 返回 `Err`）。
    pub fn subsection(
        &self,
        west_lerp: f64,
        south_lerp: f64,
        east_lerp: f64,
        north_lerp: f64,
    ) -> Result<Self, String> {
        if !(west_lerp >= 0.0) || !(west_lerp <= 1.0) {
            return Err("westLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(south_lerp >= 0.0) || !(south_lerp <= 1.0) {
            return Err("southLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(east_lerp >= 0.0) || !(east_lerp <= 1.0) {
            return Err("eastLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(north_lerp >= 0.0) || !(north_lerp <= 1.0) {
            return Err("northLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(west_lerp <= east_lerp) {
            return Err("westLerp must be less than or equal to eastLerp.".to_string());
        }
        if !(south_lerp <= north_lerp) {
            return Err("southLerp must be less than or equal to northLerp.".to_string());
        }

        // 本函数不使用 lerp，因为当起始值和结束值相同但 t 变化时，
        // lerp 会有浮点精度问题。
        let (mut west, mut east) = if self.west <= self.east {
            let width = self.east - self.west;
            (self.west + west_lerp * width, self.west + east_lerp * width)
        } else {
            let width = TWO_PI + self.east - self.west;
            (
                math_utils::negative_pi_to_pi(self.west + west_lerp * width),
                math_utils::negative_pi_to_pi(self.west + east_lerp * width),
            )
        };
        let height = self.north - self.south;
        let mut south = self.south + south_lerp * height;
        let mut north = self.south + north_lerp * height;

        // 修复 t = 1 时的浮点精度问题
        if west_lerp == 1.0 {
            west = self.east;
        }
        if east_lerp == 1.0 {
            east = self.east;
        }
        if south_lerp == 1.0 {
            south = self.north;
        }
        if north_lerp == 1.0 {
            north = self.north;
        }

        Ok(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// 将该矩形细分为一个小矩形网格。
    /// （Rust 侧的扩展；没有直接对应的 CesiumJS `Rectangle` 方法。）
    pub fn subdivide(&self, x_segments: u32, y_segments: u32) -> Vec<Self> {
        let mut result = Vec::with_capacity((x_segments * y_segments) as usize);
        let width = self.width();
        let height = self.height();
        let dx = width / x_segments as f64;
        let dy = height / y_segments as f64;

        for j in 0..y_segments {
            for i in 0..x_segments {
                let west = self.west + dx * i as f64;
                let south = self.south + dy * j as f64;
                result.push(Self {
                    west,
                    south,
                    east: west + dx,
                    north: south + dy,
                });
            }
        }
        result
    }

    /// 判断本矩形是否在 epsilon 容差内等于另一个矩形。
    /// 映射到 `Rectangle.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.west - other.west).abs() <= epsilon
            && (self.south - other.south).abs() <= epsilon
            && (self.east - other.east).abs() <= epsilon
            && (self.north - other.north).abs() <= epsilon
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_degrees() {
        let r = Rectangle::from_degrees(-180.0, -90.0, 180.0, 90.0);
        assert!(r.equals_epsilon(&Rectangle::MAX_VALUE, 1e-10));
    }

    #[test]
    fn test_width_height() {
        let r = Rectangle::from_degrees(-90.0, -45.0, 90.0, 45.0);
        assert!((r.width() - PI).abs() < 1e-10);
        assert!((r.height() - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_contains() {
        let r = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        assert!(r.contains(0.0, 0.0));
        assert!(!r.contains(math_utils::to_radians(20.0), 0.0));
    }

    #[test]
    fn test_intersection() {
        let a = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let b = Rectangle::from_degrees(0.0, 0.0, 20.0, 20.0);
        let inter = a.intersection(&b).unwrap();
        assert!((inter.west - 0.0).abs() < 1e-10);
        assert!((inter.south - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_union() {
        let a = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let b = Rectangle::from_degrees(0.0, 0.0, 20.0, 20.0);
        let u = a.union(&b);
        assert!((u.west - math_utils::to_radians(-10.0)).abs() < 1e-10);
        assert!((u.east - math_utils::to_radians(20.0)).abs() < 1e-10);
    }

    #[test]
    fn test_center() {
        let r = Rectangle::from_degrees(-90.0, -45.0, 90.0, 45.0);
        let c = r.center();
        assert!(c.longitude.abs() < 1e-10);
        assert!(c.latitude.abs() < 1e-10);
    }
}
