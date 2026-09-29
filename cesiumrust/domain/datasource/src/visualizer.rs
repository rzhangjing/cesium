//! Visualizer：管理实体到几何的映射与批处理。
//!
//! 映射到 CesiumJS `DataSources/GeometryVisualizer.js`、`DataSources/Visualizer.js`
//!
//! 该 visualizer 跟踪来自 EntityCollection 的实体，将其图形属性
//! 转换为几何实例，并管理静态/动态批处理。

use std::collections::HashMap;

use cesium_geospatial::Ellipsoid;

use crate::entity_collection::EntityCollection;
use crate::geometry_updater::{update_entity_geometry, EntityGeometry, GeometryInstance};

/// 一个为实体集合管理几何生成的 visualizer。
///
/// 映射到 CesiumJS `DataSources/GeometryVisualizer.js`
#[derive(Debug)]
pub struct GeometryVisualizer {
    /// 每个实体 ID 缓存的几何。
    geometry_cache: HashMap<String, EntityGeometry>,
    /// 上一次更新的时间。
    last_time: f64,
    /// 用于坐标转换的椭球。
    ellipsoid: Ellipsoid,
    /// visualizer 是否需要完全重建。
    dirty: bool,
}

impl GeometryVisualizer {
    /// 创建新的几何 visualizer。
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            geometry_cache: HashMap::new(),
            last_time: 0.0,
            ellipsoid,
            dirty: true,
        }
    }

    /// 创建一个使用 WGS84 椭球的新几何 visualizer。
    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    /// 将 visualizer 标记为脏（需要完全重建）。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 在给定时间处为给定实体集合更新 visualizer。
    ///
    /// 返回被更新的实体数量。
    pub fn update(&mut self, entities: &EntityCollection, time: f64) -> usize {
        let time_changed = (time - self.last_time).abs() > f64::EPSILON;
        self.last_time = time;

        if !self.dirty && !time_changed {
            return 0;
        }

        let mut updated = 0;

        // 移除已不再存在的实体
        let current_ids: Vec<String> = entities.values().map(|e| e.id.clone()).collect();
        self.geometry_cache.retain(|id, _| current_ids.contains(id));

        // 更新或添加实体
        for entity in entities.values() {
            let needs_update = self.dirty
                || time_changed
                || !self.geometry_cache.contains_key(&entity.id);

            if needs_update {
                let geometry = update_entity_geometry(entity, time, &self.ellipsoid);
                self.geometry_cache.insert(entity.id.clone(), geometry);
                updated += 1;
            }
        }

        self.dirty = false;
        updated
    }

    /// 获取特定实体的几何。
    pub fn get_geometry(&self, entity_id: &str) -> Option<&EntityGeometry> {
        self.geometry_cache.get(entity_id)
    }

    /// 返回跨所有实体的全部填充几何实例。
    pub fn all_fill_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| g.fill_instances.iter())
            .collect()
    }

    /// 返回跨所有实体的全部轮廓几何实例。
    pub fn all_outline_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| g.outline_instances.iter())
            .collect()
    }

    /// 返回所有几何实例（填充 + 轮廓）。
    pub fn all_instances(&self) -> Vec<&GeometryInstance> {
        self.geometry_cache
            .values()
            .flat_map(|g| {
                g.fill_instances.iter().chain(g.outline_instances.iter())
            })
            .collect()
    }

    /// 几何实例总数。
    pub fn instance_count(&self) -> usize {
        self.geometry_cache
            .values()
            .map(|g| g.instance_count())
            .sum()
    }

    /// 被跟踪的实体数量。
    pub fn entity_count(&self) -> usize {
        self.geometry_cache.len()
    }

    /// 移除特定实体的几何。
    pub fn remove_entity(&mut self, entity_id: &str) {
        self.geometry_cache.remove(entity_id);
    }

    /// 清除所有缓存的几何。
    pub fn clear(&mut self) {
        self.geometry_cache.clear();
        self.dirty = true;
    }
}

/// 一个静态几何批次，将多个几何实例
/// 合并为单个批次以高效渲染。
///
/// 映射到 CesiumJS `DataSources/StaticGeometryColorBatch.js`
#[derive(Debug, Default)]
pub struct StaticGeometryBatch {
    /// 批量处理的填充实例。
    pub fill_instances: Vec<GeometryInstance>,
    /// 批量处理的轮廓实例。
    pub outline_instances: Vec<GeometryInstance>,
}

impl StaticGeometryBatch {
    /// 创建新的空批次。
    pub fn new() -> Self {
        Self::default()
    }

    /// 向批次添加几何实例。
    pub fn add(&mut self, geometry: &EntityGeometry) {
        self.fill_instances.extend(geometry.fill_instances.iter().cloned());
        self.outline_instances.extend(geometry.outline_instances.iter().cloned());
    }

    /// 批次中实例总数。
    pub fn len(&self) -> usize {
        self.fill_instances.len() + self.outline_instances.len()
    }

    /// 若批次为空则返回 true。
    pub fn is_empty(&self) -> bool {
        self.fill_instances.is_empty() && self.outline_instances.is_empty()
    }

    /// 清除批次。
    pub fn clear(&mut self) {
        self.fill_instances.clear();
        self.outline_instances.clear();
    }
}

/// 一个动态几何更新器，对于具有时间动态属性的
/// 实体，每帧重新生成几何。
///
/// 映射到 CesiumJS `DataSources/DynamicGeometryUpdater.js`
#[derive(Debug)]
pub struct DynamicGeometryUpdater {
    /// 具有动态（随时间变化）几何的实体 ID。
    dynamic_entities: Vec<String>,
    /// 用于坐标转换的椭球。
    ellipsoid: Ellipsoid,
}

impl DynamicGeometryUpdater {
    /// 创建新的动态几何更新器。
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            dynamic_entities: Vec::new(),
            ellipsoid,
        }
    }

    /// 将一个实体注册为动态。
    pub fn add_entity(&mut self, entity_id: &str) {
        if !self.dynamic_entities.contains(&entity_id.to_string()) {
            self.dynamic_entities.push(entity_id.to_string());
        }
    }

    /// 将一个实体从动态跟踪中移除。
    pub fn remove_entity(&mut self, entity_id: &str) {
        self.dynamic_entities.retain(|id| id != entity_id);
    }

    /// 在给定时间处为所有被跟踪的实体更新动态几何。
    pub fn update(&self, entities: &EntityCollection, time: f64) -> Vec<(String, EntityGeometry)> {
        self.dynamic_entities
            .iter()
            .filter_map(|id| {
                entities.get(id).map(|entity| {
                    let geometry = update_entity_geometry(entity, time, &self.ellipsoid);
                    (id.clone(), geometry)
                })
            })
            .collect()
    }

    /// 被跟踪的动态实体数量。
    pub fn entity_count(&self) -> usize {
        self.dynamic_entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::*;
    use crate::property::Property;

    fn make_collection() -> EntityCollection {
        let mut collection = EntityCollection::new();
        collection.add(
            Entity::new("box-1")
                .with_position(0.0, 0.0, 0.0)
                .with_box(BoxGraphics {
                    dimensions: Property::Constant([100.0, 100.0, 100.0]),
                    ..Default::default()
                }),
        );
        collection.add(
            Entity::new("cyl-1")
                .with_position(0.1, 0.1, 0.0)
                .with_cylinder(CylinderGraphics {
                    length: Property::Constant(200.0),
                    top_radius: Property::Constant(50.0),
                    bottom_radius: Property::Constant(50.0),
                    ..Default::default()
                }),
        );
        collection
    }

    #[test]
    fn test_visualizer_update() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        let updated = viz.update(&collection, 0.0);
        assert_eq!(updated, 2);
        assert_eq!(viz.entity_count(), 2);
        assert!(viz.instance_count() >= 2);
    }

    #[test]
    fn test_visualizer_no_change() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let updated = viz.update(&collection, 0.0);
        assert_eq!(updated, 0); // 无变化
    }

    #[test]
    fn test_visualizer_time_change() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let updated = viz.update(&collection, 1.0);
        assert_eq!(updated, 2); // 时间变化，全部更新
    }

    #[test]
    fn test_visualizer_entity_removal() {
        let mut viz = GeometryVisualizer::wgs84();
        let mut collection = make_collection();

        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 2);

        collection.remove("box-1");
        viz.mark_dirty();
        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 1);
    }

    #[test]
    fn test_visualizer_get_geometry() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let geo = viz.get_geometry("box-1").unwrap();
        assert_eq!(geo.fill_instances.len(), 1);
    }

    #[test]
    fn test_visualizer_all_instances() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        let fills = viz.all_fill_instances();
        assert_eq!(fills.len(), 2);
    }

    #[test]
    fn test_static_batch() {
        let mut batch = StaticGeometryBatch::new();
        assert!(batch.is_empty());

        let geo = EntityGeometry {
            fill_instances: vec![],
            outline_instances: vec![],
        };
        batch.add(&geo);
        assert!(batch.is_empty());

        // 添加真实几何
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();
        viz.update(&collection, 0.0);

        for instance in viz.all_fill_instances() {
            batch.fill_instances.push(instance.clone());
        }
        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_dynamic_updater() {
        let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
        let collection = make_collection();

        dynamic.add_entity("box-1");
        assert_eq!(dynamic.entity_count(), 1);

        let results = dynamic.update(&collection, 0.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "box-1");
        assert_eq!(results[0].1.fill_instances.len(), 1);
    }

    #[test]
    fn test_dynamic_updater_remove() {
        let mut dynamic = DynamicGeometryUpdater::new(Ellipsoid::WGS84);
        dynamic.add_entity("box-1");
        dynamic.add_entity("cyl-1");
        assert_eq!(dynamic.entity_count(), 2);

        dynamic.remove_entity("box-1");
        assert_eq!(dynamic.entity_count(), 1);
    }

    #[test]
    fn test_visualizer_clear() {
        let mut viz = GeometryVisualizer::wgs84();
        let collection = make_collection();

        viz.update(&collection, 0.0);
        assert_eq!(viz.entity_count(), 2);

        viz.clear();
        assert_eq!(viz.entity_count(), 0);
        assert_eq!(viz.instance_count(), 0);
    }
}
