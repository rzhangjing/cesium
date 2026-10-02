//! 实体专用的 Bevy 组件。
//!
//! 这些组件与五类实体图形一一对应（点、线、面、billboard、模型）。
//! 来自 `cesium_datasource` 的领域类型仍是单一真相源；
//! 这些组件用于面向 GPU 的就绪渲染状态。
//!
//! 设计要点：每个组件都是领域图形向下的“快照”，字段均为
//! GPU 友好的标量/数组（颜色为 `[f32;4]`，坐标为 `DVec3`）；
//! `Default` 提供中性默认值，可视化系统据此构造网格与材质。

// CesiumJS 移植遗留的风格债（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::derivable_impls)]
use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_time::TimeIntervalCollection;

/// 包装地球椭球的资源（供坐标转换读取）。
#[derive(Resource, Deref, DerefMut)]
pub struct GlobeEllipsoid(pub Ellipsoid);

impl Default for GlobeEllipsoid {
    /// 默认使用 WGS84 椭球。
    fn default() -> Self {
        Self(Ellipsoid::WGS84)
    }
}

/// 将领域实体包装为 Bevy 组件的轻量容器。
#[derive(Component, Deref, DerefMut, Clone)]
pub struct EntityWrapper(pub DomainEntity);

impl EntityWrapper {
    /// 由领域实体构造包装器。
    ///
    /// # 参数
    /// - `entity`：被包装的领域实体
    pub fn new(entity: DomainEntity) -> Self {
        Self(entity)
    }
}

/// 实体基础属性组件（标识/名称/可见性/可用性）。
#[derive(Component, Clone)]
pub struct CesiumEntity {
    /// 实体唯一标识。
    pub entity_id: String,
    /// 显示名称。
    pub name: String,
    /// 可选的描述文本。
    pub description: Option<String>,
    /// 是否可见。
    pub show: bool,
    /// 可选的可用时间区间集合。
    pub availability: Option<TimeIntervalCollection<()>>,
}

impl CesiumEntity {
    /// 用标识与名称构造，其余字段取中性默认（可见、无描述/可用性）。
    ///
    /// # 参数
    /// - `entity_id`：实体标识（可为任意 `Into<String>`）
    /// - `name`：显示名称
    pub fn new(entity_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            name: name.into(),
            description: None,
            show: true,
            availability: None,
        }
    }
}

/// 标记需要创建/更新其可视化的实体。
#[derive(Component)]
pub struct NeedsVisualUpdate;

/// 标记其可视化已构建完成的实体。
#[derive(Component)]
pub struct VisualizationBuilt;

/// 标记 billboard 实体（每帧朝向相机）。
#[derive(Component)]
pub struct BillboardTag;

/// 点图形组件（像素大小、填充色、描边）。
#[derive(Component, Clone)]
pub struct PointGraphicsComponent {
    /// 点直径（像素）。
    pub pixel_size: f32,
    /// 填充色（RGBA，0-1）。
    pub color: [f32; 4],
    /// 描边色（RGBA，0-1）。
    pub outline_color: [f32; 4],
    /// 描边宽度（像素）。
    pub outline_width: f32,
}

impl Default for PointGraphicsComponent {
    /// 默认：1 像素、白色不透明填充、黑色描边、无描边宽。
    fn default() -> Self {
        Self {
            pixel_size: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            outline_color: [0.0, 0.0, 0.0, 1.0],
            outline_width: 0.0,
        }
    }
}

/// 线图形组件（顶点序列、宽度、材质色、是否贴地）。
#[derive(Component, Clone)]
pub struct PolylineGraphicsComponent {
    /// 折线顶点（地心坐标）。
    pub positions: Vec<glam::DVec3>,
    /// 线宽（像素）。
    pub width: f32,
    /// 材质颜色（RGBA）。
    pub material_color: [f32; 4],
    /// 是否clamp到地面。
    pub clamp_to_ground: bool,
}

impl Default for PolylineGraphicsComponent {
    /// 默认：空顶点、宽 1、白色不透明、不贴地。
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            material_color: [1.0, 1.0, 1.0, 1.0],
            clamp_to_ground: false,
        }
    }
}

/// 面图形组件（外环/内环、高度/拉伸、材质与描边）。
#[derive(Component, Clone)]
pub struct PolygonGraphicsComponent {
    /// 外环顶点（地心坐标）。
    pub positions: Vec<glam::DVec3>,
    /// 内环（孔洞）列表，每个为一圈顶点。
    pub holes: Vec<Vec<glam::DVec3>>,
    /// 底面高度（米）。
    pub height: f64,
    /// 可选的拉伸顶部高度（米）。
    pub extruded_height: Option<f64>,
    /// 材质颜色（RGBA）。
    pub material_color: [f32; 4],
    /// 是否绘制轮廓。
    pub outline: bool,
    /// 轮廓颜色（RGBA）。
    pub outline_color: [f32; 4],
}

impl Default for PolygonGraphicsComponent {
    /// 默认：无环、高 0、不拉伸、白色不透明、无轮廓。
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            holes: Vec::new(),
            height: 0.0,
            extruded_height: None,
            material_color: [1.0, 1.0, 1.0, 1.0],
            outline: false,
            outline_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Billboard 图形组件（图像、缩放、颜色）。
#[derive(Component, Clone)]
pub struct BillboardGraphicsComponent {
    /// 图像资源 URL（可为空）。
    pub image_url: Option<String>,
    /// 缩放因子。
    pub scale: f32,
    /// 颜色调制（RGBA）。
    pub color: [f32; 4],
}

impl Default for BillboardGraphicsComponent {
    /// 默认：无图像、缩放 1、白色不透明。
    fn default() -> Self {
        Self {
            image_url: None,
            scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// 模型图形组件（资源 URI、缩放、最小像素尺寸）。
#[derive(Component, Clone)]
pub struct ModelGraphicsComponent {
    /// 模型资源 URI。
    pub uri: String,
    /// 统一缩放。
    pub scale: f32,
    /// 最小可见像素尺寸。
    pub minimum_pixel_size: f32,
}

impl Default for ModelGraphicsComponent {
    /// 默认：空 URI、缩放 1、无最小像素限制。
    fn default() -> Self {
        Self {
            uri: String::new(),
            scale: 1.0,
            minimum_pixel_size: 0.0,
        }
    }
}

/// 时动态属性标记组件（记录哪些属性需要逐帧插值）。
#[derive(Component)]
pub struct TimeDynamicProperties {
    /// 位置是否随时间插值。
    pub has_interpolated_position: bool,
    /// 颜色是否随时间插值。
    pub has_interpolated_color: bool,
    /// 姿态是否随时间插值。
    pub has_interpolated_orientation: bool,
    /// 是否存在可用性时间区间。
    pub has_availability: bool,
}

impl Default for TimeDynamicProperties {
    /// 默认：全部不动态。
    fn default() -> Self {
        Self {
            has_interpolated_position: false,
            has_interpolated_color: false,
            has_interpolated_orientation: false,
            has_availability: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证 CesiumEntity 默认字段（可见、无描述/可用性）。
    fn test_cesium_entity_defaults() {
        let entity = CesiumEntity::new("test-01", "Test Entity");
        assert_eq!(entity.entity_id, "test-01");
        assert_eq!(entity.name, "Test Entity");
        assert!(entity.show);
        assert!(entity.description.is_none());
        assert!(entity.availability.is_none());
    }

    #[test]
    /// 验证点图形默认值（尺寸/颜色/描边）。
    fn test_point_graphics_defaults() {
        let pg = PointGraphicsComponent::default();
        assert!((pg.pixel_size - 1.0).abs() < 1e-6);
        assert_eq!(pg.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(pg.outline_color, [0.0, 0.0, 0.0, 1.0]);
        assert!((pg.outline_width - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_graphics_with_holes() {
        let mut pg = PolygonGraphicsComponent::default();
        pg.holes.push(vec![glam::DVec3::new(0.0, 0.0, 0.0)]);
        assert_eq!(pg.holes.len(), 1);
    }
}
