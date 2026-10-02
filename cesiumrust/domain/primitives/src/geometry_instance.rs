//! 几何实例与外观系统。
//!
//! 定义携带变换与属性的 [`GeometryInstance`]，以及描述其渲染方式的
//! [`Appearance`]、材质 [`MaterialType`] 与绘制状态 [`RenderState`]。

use cesium_geospatial::bounding::BoundingSphere;
use glam::{DMat4, DVec3};

/// 一个带变换与属性的几何实例。
///
/// 记录几何类型、模型矩阵与逐实例属性（如颜色），供基本体合批与绘制使用。
#[derive(Debug, Clone)]
pub struct GeometryInstance {
    /// 唯一标识符。
    pub id: String,
    /// 几何类型。
    pub geometry_type: GeometryType,
    /// 模型矩阵（局部到世界的变换）。
    pub model_matrix: DMat4,
    /// 每实例颜色 [r, g, b, a]（0.0-1.0）。
    pub color: [f64; 4],
    /// 实例是否显示。
    pub show: bool,
    /// 计算出的包围球。
    pub bounding_sphere: Option<BoundingSphere>,
}

impl GeometryInstance {
    /// 创建一个新的几何实例。
    pub fn new(id: impl Into<String>, geometry_type: GeometryType) -> Self {
        Self {
            id: id.into(),
            geometry_type,
            model_matrix: DMat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            show: true,
            bounding_sphere: None,
        }
    }

    /// 设置模型矩阵。
    pub fn with_model_matrix(mut self, matrix: DMat4) -> Self {
        self.model_matrix = matrix;
        self
    }

    /// 设置颜色。
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// 设置位置（仅平移）。
    pub fn with_position(mut self, position: DVec3) -> Self {
        self.model_matrix = DMat4::from_translation(position);
        self
    }

    /// 计算世界空间的包围球。
    pub fn compute_bounding_sphere(&mut self) {
        let local_bs = self.geometry_type.bounding_sphere();
        self.bounding_sphere = Some(local_bs.transform(&self.model_matrix));
    }
}

/// 可实例化的几何类型。
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryType {
    /// 盒体几何。
    Box {
        /// 半长（半边长度）。
        half_extents: DVec3,
    },
    /// 球体几何。
    Sphere {
        /// 半径。
        radius: f64,
    },
    /// 柱体几何。
    Cylinder {
        /// 顶部半径。
        top_radius: f64,
        /// 底部半径。
        bottom_radius: f64,
        /// 高度。
        height: f64,
    },
    /// 椭球几何。
    Ellipsoid {
        /// 各轴半径。
        radii: DVec3,
    },
    /// 矩形几何（地理）。
    Rectangle {
        /// 西（弧度）。
        west: f64,
        /// 南（弧度）。
        south: f64,
        /// 东（弧度）。
        east: f64,
        /// 北（弧度）。
        north: f64,
    },
    /// 多边形几何。
    Polygon {
        /// 位置 [lon, lat, height]，单位为弧度/米。
        positions: Vec<[f64; 3]>,
    },
    /// 折线几何。
    Polyline {
        /// 位置 [lon, lat, height]，单位为弧度/米。
        positions: Vec<[f64; 3]>,
        /// 宽度（米）。
        width: f64,
    },
    /// 圆形几何。
    Circle {
        /// 中心 [lon, lat, height]，单位为弧度/米。
        center: [f64; 3],
        /// 半径（米）。
        radius: f64,
    },
    /// 带顶点数据的自定义几何。
    Custom {
        /// 顶点数。
        vertex_count: u32,
        /// 包围球。
        bounding_sphere: BoundingSphere,
    },
}

impl GeometryType {
    /// 返回该几何的局部空间包围球。
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Box { half_extents } => {
                BoundingSphere::new(DVec3::ZERO, half_extents.length())
            }
            Self::Sphere { radius } => BoundingSphere::new(DVec3::ZERO, *radius),
            Self::Cylinder {
                top_radius,
                bottom_radius,
                height,
            } => {
                // 包围半径 = sqrt(max(顶/底半径)² + (高/2)²)。
                let max_radius = top_radius.max(*bottom_radius);
                let half_height = height / 2.0;
                BoundingSphere::new(DVec3::ZERO, (max_radius * max_radius + half_height * half_height).sqrt())
            }
            Self::Ellipsoid { radii } => {
                // 取三轴最大半径作为外接球半径。
                BoundingSphere::new(DVec3::ZERO, radii.x.max(radii.y).max(radii.z))
            }
            Self::Rectangle {
                west,
                south,
                east,
                north,
            } => {
                // 近似包围球
                let center_lon = (west + east) / 2.0;
                let center_lat = (south + north) / 2.0;
                let angular_radius = ((east - west) / 2.0).max((north - south) / 2.0);
                // 单位球上的近似半径
                let radius = angular_radius.sin() * 6378137.0;
                BoundingSphere::new(
                    DVec3::new(center_lon.cos() * center_lat.cos(), center_lon.sin() * center_lat.cos(), center_lat.sin()) * 6378137.0,
                    radius,
                )
            }
            Self::Polygon { positions } | Self::Polyline { positions, .. } => {
                // 将各地理坐标转 ECEF 后求形心与最远点距离。
                if positions.is_empty() {
                    return BoundingSphere::new(DVec3::ZERO, 0.0);
                }
                // 计算形心与最大距离
                let mut center = DVec3::ZERO;
                for p in positions {
                    center += DVec3::new(
                        p[0].cos() * p[1].cos(),
                        p[0].sin() * p[1].cos(),
                        p[1].sin(),
                    ) * (6378137.0 + p[2]);
                }
                center /= positions.len() as f64;

                let mut max_dist = 0.0f64;
                for p in positions {
                    let pos = DVec3::new(
                        p[0].cos() * p[1].cos(),
                        p[0].sin() * p[1].cos(),
                        p[1].sin(),
                    ) * (6378137.0 + p[2]);
                    max_dist = max_dist.max((pos - center).length());
                }
                BoundingSphere::new(center, max_dist)
            }
            Self::Circle { center, radius } => {
                let pos = DVec3::new(
                    center[0].cos() * center[1].cos(),
                    center[0].sin() * center[1].cos(),
                    center[1].sin(),
                ) * (6378137.0 + center[2]);
                BoundingSphere::new(pos, *radius)
            }
            Self::Custom { bounding_sphere, .. } => *bounding_sphere,
        }
    }

    /// 返回该几何的顶点数估算值。
    pub fn estimated_vertex_count(&self) -> u32 {
        // 各图元取代表性网格密度估算值，供合批预算参考。
        match self {
            Self::Box { .. } => 24,
            Self::Sphere { .. } => 1024,
            Self::Cylinder { .. } => 128,
            Self::Ellipsoid { .. } => 1024,
            Self::Rectangle { .. } => 4,
            Self::Polygon { positions } => positions.len() as u32,
            Self::Polyline { positions, .. } => positions.len() as u32 * 2,
            Self::Circle { .. } => 64,
            Self::Custom { vertex_count, .. } => *vertex_count,
        }
    }
}

/// Appearance 定义几何如何渲染。
///
/// 聚合透明度、双面、平面明暗、材质与绘制状态，决定实例的着色方式。
#[derive(Debug, Clone)]
pub struct Appearance {
    /// 外观是否为半透明。
    pub translucent: bool,
    /// 是否渲染两个面。
    pub two_sided: bool,
    /// 是否使用平面明暗（flat shading）。
    pub flat: bool,
    /// 材质类型。
    pub material: MaterialType,
    /// 渲染状态。
    pub render_state: RenderState,
}

impl Default for Appearance {
    /// 返回缺省外观：不透明、单面、光滑着色、白色颜色材质。
    fn default() -> Self {
        Self {
            translucent: false,
            two_sided: false,
            flat: false,
            material: MaterialType::Color([1.0, 1.0, 1.0, 1.0]),
            render_state: RenderState::default(),
        }
    }
}

impl Appearance {
    /// 创建一个新的外观。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建一个每实例颜色的外观。
    pub fn per_instance_color() -> Self {
        Self {
            material: MaterialType::PerInstanceColor,
            ..Default::default()
        }
    }

    /// 设置材质。
    pub fn with_material(mut self, material: MaterialType) -> Self {
        self.material = material;
        self
    }

    /// 设置外观是否为半透明。
    pub fn with_translucent(mut self, translucent: bool) -> Self {
        self.translucent = translucent;
        self
    }
}

/// 材质类型。
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialType {
    /// 纯色。
    Color([f64; 4]),
    /// 使用每实例颜色。
    PerInstanceColor,
    /// 图像纹理。
    Image {
        /// 纹理 URL。
        url: String,
        /// X 方向重复。
        repeat_x: f64,
        /// Y 方向重复。
        repeat_y: f64,
    },
    /// 漫反射贴图。
    DiffuseMap {
        /// 纹理 URL。
        url: String,
    },
    /// 法线贴图。
    NormalMap {
        /// 纹理 URL。
        url: String,
    },
    /// 网格图案。
    Grid {
        /// 网格颜色。
        color: [f64; 4],
        /// 单元素数。
        cells: u32,
    },
    /// 条纹图案。
    Stripe {
        /// 偶数颜颜色。
        even_color: [f64; 4],
        /// 奇数颜色。
        odd_color: [f64; 4],
        /// 重复次数。
        repeat: f64,
    },
}

/// 渲染状态配置。
#[derive(Debug, Clone)]
pub struct RenderState {
    /// 是否启用深度测试。
    pub depth_test: bool,
    /// 是否启用深度写入。
    pub depth_write: bool,
    /// 是否启用混合。
    pub blending: bool,
    /// 敲除模式。
    pub cull_mode: CullMode,
}

impl Default for RenderState {
    /// 返回缺省绘制状态：启用深度测试与写入、关闭混合、背面剔除。
    fn default() -> Self {
        Self {
            depth_test: true,
            depth_write: true,
            blending: false,
            cull_mode: CullMode::Back,
        }
    }
}

/// 面敲除模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CullMode {
    /// 不敲除。
    None,
    /// 敲除正面。
    Front,
    /// 敲除背面。
    #[default]
    Back,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_instance_creation() {
        let instance = GeometryInstance::new("test", GeometryType::Sphere { radius: 100.0 });
        assert_eq!(instance.id, "test");
        assert!(instance.show);
        assert_eq!(instance.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_geometry_instance_builder() {
        let instance = GeometryInstance::new("test", GeometryType::Box { half_extents: DVec3::ONE })
            .with_color([1.0, 0.0, 0.0, 1.0])
            .with_position(DVec3::new(100.0, 200.0, 300.0));

        assert_eq!(instance.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(instance.model_matrix.w_axis.truncate(), DVec3::new(100.0, 200.0, 300.0));
    }

    #[test]
    fn test_box_bounding_sphere() {
        let geometry = GeometryType::Box {
            half_extents: DVec3::new(10.0, 20.0, 30.0),
        };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.center, DVec3::ZERO);
        assert!((bs.radius - DVec3::new(10.0, 20.0, 30.0).length()).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_bounding_sphere() {
        let geometry = GeometryType::Sphere { radius: 50.0 };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.center, DVec3::ZERO);
        assert_eq!(bs.radius, 50.0);
    }

    #[test]
    fn test_cylinder_bounding_sphere() {
        let geometry = GeometryType::Cylinder {
            top_radius: 10.0,
            bottom_radius: 20.0,
            height: 30.0,
        };
        let bs = geometry.bounding_sphere();
        // max_radius = 20, half_height = 15
        // radius = sqrt(20^2 + 15^2) = sqrt(625) = 25
        assert!((bs.radius - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_ellipsoid_bounding_sphere() {
        let geometry = GeometryType::Ellipsoid {
            radii: DVec3::new(100.0, 200.0, 150.0),
        };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.radius, 200.0); // radii 的最大值
    }

    #[test]
    fn test_polygon_bounding_sphere() {
        let geometry = GeometryType::Polygon {
            positions: vec![
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [0.1, 0.1, 0.0],
                [0.0, 0.1, 0.0],
            ],
        };
        let bs = geometry.bounding_sphere();
        assert!(bs.radius > 0.0);
    }

    #[test]
    fn test_empty_polygon_bounding_sphere() {
        let geometry = GeometryType::Polygon { positions: vec![] };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.radius, 0.0);
    }

    #[test]
    fn test_vertex_count_estimates() {
        assert_eq!(GeometryType::Box { half_extents: DVec3::ONE }.estimated_vertex_count(), 24);
        assert_eq!(GeometryType::Sphere { radius: 1.0 }.estimated_vertex_count(), 1024);
        assert_eq!(
            GeometryType::Polyline { positions: vec![[0.0; 3]; 5], width: 1.0 }.estimated_vertex_count(),
            10
        );
    }

    #[test]
    fn test_appearance_default() {
        let appearance = Appearance::default();
        assert!(!appearance.translucent);
        assert!(!appearance.two_sided);
        assert!(!appearance.flat);
    }

    #[test]
    fn test_per_instance_color_appearance() {
        let appearance = Appearance::per_instance_color();
        assert_eq!(appearance.material, MaterialType::PerInstanceColor);
    }

    #[test]
    fn test_material_types() {
        let color = MaterialType::Color([1.0, 0.0, 0.0, 1.0]);
        assert!(matches!(color, MaterialType::Color(_)));

        let grid = MaterialType::Grid {
            color: [0.0, 1.0, 0.0, 1.0],
            cells: 10,
        };
        assert!(matches!(grid, MaterialType::Grid { .. }));
    }

    #[test]
    fn test_render_state_default() {
        let state = RenderState::default();
        assert!(state.depth_test);
        assert!(state.depth_write);
        assert!(!state.blending);
        assert_eq!(state.cull_mode, CullMode::Back);
    }

    #[test]
    fn test_cull_mode_default() {
        assert_eq!(CullMode::default(), CullMode::Back);
    }
}
