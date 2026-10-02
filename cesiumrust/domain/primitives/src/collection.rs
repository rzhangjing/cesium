//! 基本体集合与合批（batching）。
//!
//! 定义以某种外观绘制几何实例的 [`Primitive`]、管理多基本体的
//! [`PrimitiveCollection`]，以及面向高效绘制的 [`GeometryBatch`] 与合批算法。

use crate::geometry_instance::{Appearance, GeometryInstance};
use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

/// 一个以某种外观渲染几何实例的基本体。
///
/// 聚合若干 [`GeometryInstance`] 并共享同一 [`Appearance`]，缓存合并包围球以供绘制。
#[derive(Debug, Clone)]
pub struct Primitive {
    /// 唯一标识符。
    ///
    /// 供集合按 ID 查找/增删使用，调用方保证在同一集合内唯一。
    pub id: String,
    /// 待渲染的几何实例。
    ///
    /// 同一基本体内所有实例共享下方 [`Appearance`] 进行绘制。
    pub instances: Vec<GeometryInstance>,
    /// 用于渲染的外观。
    pub appearance: Appearance,
    /// 基本体是否显示。
    ///
    /// 为 false 时不参与可见性筛选与包围球聚合。
    pub show: bool,
    /// 是否剔除背面。
    pub cull: bool,
    /// 是否压缩顶点以提升性能。
    ///
    /// 开启后以更低精度存储顶点，换取显存与带宽节省。
    pub compress_vertices: bool,
    /// 计算出的包围球缓存。
    ///
    /// 惰性计算；实例变动时置 None 以作废缓存。
    pub bounding_sphere: Option<BoundingSphere>,
}

impl Primitive {
    /// 创建一个新基本体。
    pub fn new(id: impl Into<String>) -> Self {
        // 默认显示、剔除背面、压缩顶点，外观取缺省。
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
        // 新增实例改变几何范围，需作废已缓存的包围球。
        self.instances.push(instance);
        self.bounding_sphere = None; // 使缓存失效
    }

    /// 设置外观。
    pub fn with_appearance(mut self, appearance: Appearance) -> Self {
        // 链式设置外观并返回自身。
        self.appearance = appearance;
        self
    }

    /// 计算合并后的包围球。
    pub fn compute_bounding_sphere(&mut self) {
        // 无实例则无包围体，直接清空缓存。
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
        // 汇总各实例几何类型的估算顶点数，供预算与合批参考。
        self.instances
            .iter()
            .map(|inst| inst.geometry_type.estimated_vertex_count())
            .sum()
    }
}

/// 基本体的有序集合。
///
/// 以 ID 查找/增删基本体，并聚合其可见者的合并包围球。
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
        // 初始化为空集合且整体可见。
        Self {
            primitives: Vec::new(),
            show: true,
        }
    }

    /// 向集合添加一个基本体。
    pub fn add(&mut self, primitive: Primitive) {
        // 追加到集合尾部，保持插入顺序。
        self.primitives.push(primitive);
    }

    /// 按 ID 移除一个基本体。
    pub fn remove(&mut self, id: &str) -> Option<Primitive> {
        // 按 ID 定位后从 Vec 中移除并返回；未命中则返回 None。
        if let Some(idx) = self.primitives.iter().position(|p| p.id == id) {
            Some(self.primitives.remove(idx))
        } else {
            None
        }
    }

    /// 按 ID 获取一个基本体。
    pub fn get(&self, id: &str) -> Option<&Primitive> {
        // 线性查找首个匹配 ID 的基本体。
        self.primitives.iter().find(|p| p.id == id)
    }

    /// 按 ID 获取一个可变基本体。
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Primitive> {
        // 同上，返回可变借用以供就地修改。
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
        // 暴露按插入顺序的只读迭代器。
        self.primitives.iter()
    }

    /// 返回一个可变遍历基本体的迭代器。
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Primitive> {
        // 暴露可按需修改元素的可写迭代器。
        self.primitives.iter_mut()
    }

    /// 计算合并后的包围球。
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        // 仅统计可见基本体的实例包围球，逐层求并集。
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
        // 需集合并身与个体均可见才纳入绘制。
        self.primitives.iter().filter(|p| p.show && self.show)
    }
}

/// 计算多个包围球的并集。
pub fn compute_bounding_sphere_union(spheres: &[BoundingSphere]) -> BoundingSphere {
    if spheres.is_empty() {
        // 空集回退到原点零半径球。
        return BoundingSphere::new(DVec3::ZERO, 0.0);
    }

    if spheres.len() == 1 {
        // 单元素直接返回自身，避免多余计算。
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
        // 每球对并集的最坏贡献 = 球心到形心距离 + 自身半径。
        let dist = (sphere.center - center).length() + sphere.radius;
        max_radius = max_radius.max(dist);
    }

    BoundingSphere::new(center, max_radius)
}

/// 几何合并的合批配置。
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// 每批最大实例数。
    ///
    /// 超过则切分新批次，避免单次绘制调用过大。
    pub max_instances_per_batch: usize,
    /// 是否合并材质相同的几何。
    ///
    /// 开启后相同 [`Appearance`] 的实例倾向于归入同批。
    pub merge_by_material: bool,
    /// 是否为透明效果按距离排序。
    ///
    /// 透明绘制需从后往前排序以正确混合。
    pub sort_by_distance: bool,
}

impl Default for BatchConfig {
    /// 返回缺省合批配置：每批 1000、按材质合并、不按距离排序。
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
    ///
    /// 按创建顺序递增，用于调试与绘制排序。
    pub id: u32,
    /// 本批次中的实例。
    ///
    /// 数量受 [`BatchConfig::max_instances_per_batch`] 约束。
    pub instances: Vec<GeometryInstance>,
    /// 共享外观。
    ///
    /// 本批所有实例以同一外观绘制。
    pub appearance: Appearance,
    /// 合并后的包围球。
    ///
    /// 惰性计算；实例变动时置 None 作废。
    pub bounding_sphere: Option<BoundingSphere>,
}

impl GeometryBatch {
    /// 创建一个新批次。
    pub fn new(id: u32, appearance: Appearance) -> Self {
        // 建立空批次并预留共享外观。
        Self {
            id,
            instances: Vec::new(),
            appearance,
            bounding_sphere: None,
        }
    }

    /// 向批次添加一个实例。
    pub fn add(&mut self, instance: GeometryInstance) {
        // 追加实例并作废缓存的包围球。
        self.instances.push(instance);
        self.bounding_sphere = None; // 使缓存失效
    }

    /// 若批次已满则返回 true。
    pub fn is_full(&self, config: &BatchConfig) -> bool {
        // 实例数达到每批上限即视为满批。
        self.instances.len() >= config.max_instances_per_batch
    }

    /// 计算批次的包围球。
    pub fn compute_bounding_sphere(&mut self) {
        // 汇总本批实例包围球求并集，空批次记为 None。
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
    // 逐实例填充当前批次，满则收尾并切换新批。
    let mut batches = Vec::new();
    let mut current_batch = GeometryBatch::new(0, appearance.clone());

    for instance in instances {
        // 满批则收尾并开新批次（ID 顺延为已有批数）。
        if current_batch.is_full(config) {
            current_batch.compute_bounding_sphere();
            batches.push(current_batch);
            current_batch = GeometryBatch::new(batches.len() as u32, appearance.clone());
        }
        current_batch.add(instance);
    }

    if !current_batch.instances.is_empty() {
        // 收尾：最后一批若非空则一并纳入。
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
