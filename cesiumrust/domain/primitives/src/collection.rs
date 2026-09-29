//! 基本体集合与合批（batching）。
//!
//! 映射到 CesiumJS：
//! - `Scene/Primitive.js`
//! - `Scene/PrimitiveCollection.js`
//! - 面向性能的几何合批

use crate::geometry_instance::{Appearance, GeometryInstance};
use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

/// 一个以某种外观渲染几何实例的基本体。
///
/// 映射到 CesiumJS `Scene/Primitive.js`
#[derive(Debug, Clone)]
pub struct Primitive {
    /// 唯一标识符。
    pub id: String,
    /// 待渲染的几何实例。
    pub instances: Vec<GeometryInstance>,
    /// 用于渲染的外观。
    pub appearance: Appearance,
    /// 基本体是否显示。
    pub show: bool,
    /// 是否敲除背面。
    pub cull: bool,
    /// 是否压缩顶点以提升性能。
    pub compress_vertices: bool,
    /// 计算出的包围球。
    pub bounding_sphere: Option<BoundingSphere>,
}

impl Primitive {
    /// 创建一个新基本体。
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instances: Vec::new(),
            appearance: Appearance::default(),
            show: true,
            cull: true,
            compress_vertices: true,
            bounding_sphere: None,
        }
    }

    /// 添加一个几何实例。
    pub fn add_instance(&mut self, instance: GeometryInstance) {
        self.instances.push(instance);
        self.bounding_sphere = None; // 使缓存失效
    }

    /// 设置外观。
    pub fn with_appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// 计算合并后的包围球。
    pub fn compute_bounding_sphere(&mut self) {
        if self.instances.is_empty() {
            self.bounding_sphere = None;
            return;
        }

        // 为所有实例计算包围球
        let spheres: Vec<BoundingSphere> = self
            .instances
            .iter()
            .map(|inst| {
                let local_bs = inst.geometry_type.bounding_sphere();
                local_bs.transform(&inst.model_matrix)
            })
            .collect();

        // 计算并集
        self.bounding_sphere = Some(compute_bounding_sphere_union(&spheres));
    }

    /// 返回顶点总数的估算值。
    pub fn total_vertex_count(&self) -> u32 {
        self.instances
            .iter()
            .map(|inst| inst.geometry_type.estimated_vertex_count())
            .sum()
    }
}

/// 基本体的集合。
///
/// 映射到 CesiumJS `Scene/PrimitiveCollection.js`
#[derive(Debug, Default)]
pub struct PrimitiveCollection {
    /// 集合中的基本体。
    primitives: Vec<Primitive>,
    /// 集合是否显示。
    pub show: bool,
}

impl PrimitiveCollection {
    /// 创建一个新基本体集合。
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            show: true,
        }
    }

    /// 向集合添加一个基本体。
    pub fn add(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// 按 ID 移除一个基本体。
    pub fn remove(&mut self, id: &str) -> Option<Primitive> {
        if let Some(idx) = self.primitives.iter().position(|p| p.id == id) {
            Some(self.primitives.remove(idx))
        } else {
            None
        }
    }

    /// 按 ID 获取一个基本体。
    pub fn get(&self, id: &str) -> Option<&Primitive> {
        self.primitives.iter().find(|p| p.id == id)
    }

    /// 按 ID 获取一个可变基本体。
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Primitive> {
        self.primitives.iter_mut().find(|p| p.id == id)
    }

    /// 返回基本体的数量。
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// 若集合为空则返回 true。
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    /// 返回一个遍历基本体的迭代器。
    pub fn iter(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter()
    }

    /// 返回一个可变遍历基本体的迭代器。
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Primitive> {
        self.primitives.iter_mut()
    }

    /// 计算合并后的包围球。
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        let spheres: Vec<BoundingSphere> = self
            .primitives
            .iter()
            .filter(|p| p.show)
            .filter_map(|p| {
                let spheres: Vec<BoundingSphere> = p
                    .instances
                    .iter()
                    .map(|inst| {
                        let local_bs = inst.geometry_type.bounding_sphere();
                        local_bs.transform(&inst.model_matrix)
                    })
                    .collect();
                if spheres.is_empty() {
                    None
                } else {
                    Some(compute_bounding_sphere_union(&spheres))
                }
            })
            .collect();

        if spheres.is_empty() {
            None
        } else {
            Some(compute_bounding_sphere_union(&spheres))
        }
    }

    /// 返回可见的基本体。
    pub fn visible_primitives(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter().filter(|p| p.show && self.show)
    }
}

/// 计算多个包围球的并集。
pub fn compute_bounding_sphere_union(spheres: &[BoundingSphere]) -> BoundingSphere {
    if spheres.is_empty() {
        return BoundingSphere::new(DVec3::ZERO, 0.0);
    }

    if spheres.len() == 1 {
        return spheres[0];
    }

    // 计算形心
    let mut center = DVec3::ZERO;
    for sphere in spheres {
        center += sphere.center;
    }
    center /= spheres.len() as f64;

    // 计算到形心的最大距离
    let mut max_radius = 0.0f64;
    for sphere in spheres {
        let dist = (sphere.center - center).length() + sphere.radius;
        max_radius = max_radius.max(dist);
    }

    BoundingSphere::new(center, max_radius)
}

/// 几何合并的合批配置。
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// 每批最大实例数。
    pub max_instances_per_batch: usize,
    /// 是否合并材质相同的几何。
    pub merge_by_material: bool,
    /// 是否为透明效果按距离排序。
    pub sort_by_distance: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_instances_per_batch: 1000,
            merge_by_material: true,
            sort_by_distance: false,
        }
    }
}

/// 一个用于高效渲染的几何实例批次。
#[derive(Debug, Clone)]
pub struct GeometryBatch {
    /// 批次 ID。
    pub id: u32,
    /// 本批次中的实例。
    pub instances: Vec<GeometryInstance>,
    /// 共享外观。
    pub appearance: Appearance,
    /// 合并后的包围球。
    pub bounding_sphere: Option<BoundingSphere>,
}

impl GeometryBatch {
    /// 创建一个新批次。
    pub fn new(id: u32, appearance: Appearance) -> Self {
        Self {
            id,
            instances: Vec::new(),
            appearance,
            bounding_sphere: None,
        }
    }

    /// 向批次添加一个实例。
    pub fn add(&mut self, instance: GeometryInstance) {
        self.instances.push(instance);
        self.bounding_sphere = None; // 使缓存失效
    }

    /// 若批次已满则返回 true。
    pub fn is_full(&self, config: &BatchConfig) -> bool {
        self.instances.len() >= config.max_instances_per_batch
    }

    /// 计算批次的包围球。
    pub fn compute_bounding_sphere(&mut self) {
        let spheres: Vec<BoundingSphere> = self
            .instances
            .iter()
            .map(|inst| {
                let local_bs = inst.geometry_type.bounding_sphere();
                local_bs.transform(&inst.model_matrix)
            })
            .collect();

        self.bounding_sphere = if spheres.is_empty() {
            None
        } else {
            Some(compute_bounding_sphere_union(&spheres))
        };
    }
}

/// 为高效渲染对几何实例进行合批。
pub fn batch_instances(
    instances: Vec<GeometryInstance>,
    appearance: Appearance,
    config: &BatchConfig,
) -> Vec<GeometryBatch> {
    let mut batches = Vec::new();
    let mut current_batch = GeometryBatch::new(0, appearance.clone());

    for instance in instances {
        if current_batch.is_full(config) {
            current_batch.compute_bounding_sphere();
            batches.push(current_batch);
            current_batch = GeometryBatch::new(batches.len() as u32, appearance.clone());
        }
        current_batch.add(instance);
    }

    if !current_batch.instances.is_empty() {
        current_batch.compute_bounding_sphere();
        batches.push(current_batch);
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_instance::GeometryType;

    #[test]
    fn test_primitive_creation() {
        let primitive = Primitive::new("test");
        assert_eq!(primitive.id, "test");
        assert!(primitive.show);
        assert!(primitive.instances.is_empty());
    }

    #[test]
    fn test_primitive_add_instance() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        assert_eq!(primitive.instances.len(), 1);
    }

    #[test]
    fn test_primitive_bounding_sphere() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(
            GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 })
                .with_position(DVec3::new(100.0, 0.0, 0.0)),
        );
        primitive.add_instance(
            GeometryInstance::new("inst2", GeometryType::Sphere { radius: 10.0 })
                .with_position(DVec3::new(-100.0, 0.0, 0.0)),
        );

        primitive.compute_bounding_sphere();
        let bs = primitive.bounding_sphere.unwrap();

        // 中心应在原点，半径应覆盖两个球
        assert!(bs.center.length() < 1e-10);
        assert!(bs.radius >= 110.0);
    }

    #[test]
    fn test_primitive_total_vertex_count() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(GeometryInstance::new("box", GeometryType::Box { half_extents: DVec3::ONE }));
        primitive.add_instance(GeometryInstance::new("sphere", GeometryType::Sphere { radius: 1.0 }));

        assert_eq!(primitive.total_vertex_count(), 24 + 1024);
    }

    #[test]
    fn test_primitive_collection() {
        let mut collection = PrimitiveCollection::new();
        assert!(collection.is_empty());

        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());
    }

    #[test]
    fn test_primitive_collection_get() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        assert!(collection.get("p1").is_some());
        assert!(collection.get("p3").is_none());
    }

    #[test]
    fn test_primitive_collection_remove() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        let removed = collection.remove("p1");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_primitive_collection_visible() {
        let mut collection = PrimitiveCollection::new();
        let mut p1 = Primitive::new("p1");
        p1.show = true;
        let mut p2 = Primitive::new("p2");
        p2.show = false;

        collection.add(p1);
        collection.add(p2);

        let visible: Vec<_> = collection.visible_primitives().collect();
        assert_eq!(visible.len(), 1);
    }

    #[test]
    fn test_bounding_sphere_union() {
        let spheres = vec![
            BoundingSphere::new(DVec3::new(0.0, 0.0, 0.0), 10.0),
            BoundingSphere::new(DVec3::new(100.0, 0.0, 0.0), 10.0),
        ];

        let union = compute_bounding_sphere_union(&spheres);
        assert!(union.center.x > 40.0 && union.center.x < 60.0);
        assert!(union.radius >= 60.0);
    }

    #[test]
    fn test_bounding_sphere_union_empty() {
        let union = compute_bounding_sphere_union(&[]);
        assert_eq!(union.radius, 0.0);
    }

    #[test]
    fn test_bounding_sphere_union_single() {
        let spheres = vec![BoundingSphere::new(DVec3::new(10.0, 20.0, 30.0), 50.0)];
        let union = compute_bounding_sphere_union(&spheres);
        assert_eq!(union.center, DVec3::new(10.0, 20.0, 30.0));
        assert_eq!(union.radius, 50.0);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_instances_per_batch, 1000);
        assert!(config.merge_by_material);
        assert!(!config.sort_by_distance);
    }

    #[test]
    fn test_geometry_batch() {
        let mut batch = GeometryBatch::new(0, Appearance::default());
        batch.add(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        batch.add(GeometryInstance::new("inst2", GeometryType::Sphere { radius: 20.0 }));

        assert_eq!(batch.instances.len(), 2);

        batch.compute_bounding_sphere();
        assert!(batch.bounding_sphere.is_some());
    }

    #[test]
    fn test_batch_instances() {
        let instances: Vec<GeometryInstance> = (0..5)
            .map(|i| GeometryInstance::new(format!("inst{}", i), GeometryType::Sphere { radius: 10.0 }))
            .collect();

        let config = BatchConfig {
            max_instances_per_batch: 2,
            ..Default::default()
        };

        let batches = batch_instances(instances, Appearance::default(), &config);

        // 5 个实例 / 每批 2 个 = 3 批（2, 2, 1）
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].instances.len(), 2);
        assert_eq!(batches[1].instances.len(), 2);
        assert_eq!(batches[2].instances.len(), 1);
    }

    #[test]
    fn test_batch_is_full() {
        let config = BatchConfig {
            max_instances_per_batch: 2,
            ..Default::default()
        };

        let mut batch = GeometryBatch::new(0, Appearance::default());
        assert!(!batch.is_full(&config));

        batch.add(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        assert!(!batch.is_full(&config));

        batch.add(GeometryInstance::new("inst2", GeometryType::Sphere { radius: 10.0 }));
        assert!(batch.is_full(&config));
    }
}
