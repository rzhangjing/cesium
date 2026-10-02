// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
//! 影像图层管理：维护一组图层描述符并映射为领域层 [`ImageryLayer`]。
//!
//! [`ImageryLayerManager`] 作为资源保存所有图层的可见性/不透明度/层级范围，
//! 提供增删改查；[`ImageryLayerDescriptor`] 为适配层描述，`to_domain_layer`
//! 负责向领域层转换。
#![allow(unused_variables)]
use bevy::prelude::*;
use cesium_imagery::ImageryLayer;

/// 影像图层管理器资源（持有全部图层描述与总开关）。
#[derive(Resource, Default)]
pub struct ImageryLayerManager {
    /// 图层描述符列表。
    pub layers: Vec<ImageryLayerDescriptor>,
    /// 总开关：关闭时不参与混合。
    pub enabled: bool,
}

/// 单个影像图层的适配层描述。
#[derive(Debug, Clone)]
pub struct ImageryLayerDescriptor {
    /// 图层唯一标识（自增）。
    pub id: u64,
    /// 瓦片 URL 模板（含 {z}/{x}/{y} 占位）。
    pub url_template: String,
    /// 不透明度 [0,1]。
    pub opacity: f32,
    /// 是否可见（渲染参与）。
    pub visible: bool,
    /// 最小层级。
    pub min_level: u32,
    /// 最大层级。
    pub max_level: u32,
    /// 瓦片宽（像素）。
    pub tile_width: u32,
    /// 瓦片高（像素）。
    pub tile_height: u32,
    /// 是否显示（用户开关）。
    pub show: bool,
}

impl ImageryLayerManager {
    /// 追加一个新图层，返回分配的 id。
    ///
    /// # 参数
    /// - `url_template`：瓦片 URL 模板
    /// - `opacity`：不透明度
    /// - `min_level`/`max_level`：层级范围
    pub fn add_layer(
        &mut self,
        url_template: &str,
        opacity: f32,
        min_level: u32,
        max_level: u32,
    ) -> u64 {
        // id 从 1 起自增；新图层默认可见/显示、256×256。
        let id = self.layers.len() as u64 + 1;
        self.layers.push(ImageryLayerDescriptor {
            id,
            url_template: url_template.to_string(),
            opacity,
            visible: true,
            min_level,
            max_level,
            tile_width: 256,
            tile_height: 256,
            show: true,
        });
        id
    }

    /// 按 id 移除图层（不存在时无副作用）。
    ///
    /// # 参数
    /// - `id`：待移除的图层 id
    pub fn remove_layer(&mut self, id: u64) {
        self.layers.retain(|l| l.id != id);
    }

    /// 按 id 查找图层描述（不存在返回 `None`）。
    ///
    /// # 参数
    /// - `id`：目标图层 id
    pub fn get_layer(&self, id: u64) -> Option<&ImageryLayerDescriptor> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// 迭代当前可见且显示的图层。
    pub fn visible_layers(&self) -> impl Iterator<Item = &ImageryLayerDescriptor> {
        self.layers.iter().filter(|l| l.show && l.visible)
    }

    /// 图层总数。
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// 把适配层描述转为领域层 [`ImageryLayer`]（全球矩形、携 alpha/层级/瓦片尺寸）。
    ///
    /// # 参数
    /// - `desc`：图层描述
    pub fn to_domain_layer(&self, desc: &ImageryLayerDescriptor) -> ImageryLayer {
        ImageryLayer::new(
            desc.id,
            cesium_geospatial::rectangle::Rectangle::MAX_VALUE,
        )
        .with_alpha(desc.opacity as f64)
        .with_show(desc.show)
        .with_level_range(desc.min_level, desc.max_level)
        .with_tile_size(desc.tile_width, desc.tile_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证 add_layer 后 id 自增且可查到。
    fn test_add_and_get_layer() {
        let mut mgr = ImageryLayerManager::default();
        let id = mgr.add_layer("https://tiles/{z}/{x}/{y}.png", 1.0, 0, 18);
        assert_eq!(id, 1);
        assert_eq!(mgr.layer_count(), 1);
        assert!(mgr.get_layer(id).is_some());
    }

    #[test]
    /// 验证 remove_layer 只移除目标图层。
    fn test_remove_layer() {
        let mut mgr = ImageryLayerManager::default();
        let id = mgr.add_layer("https://a.tiles/{z}/{x}/{y}.png", 0.5, 0, 12);
        mgr.add_layer("https://b.tiles/{z}/{x}/{y}.png", 0.8, 0, 12);
        assert_eq!(mgr.layer_count(), 2);
        mgr.remove_layer(id);
        assert_eq!(mgr.layer_count(), 1);
    }

    #[test]
    /// 验证只有 show&&visible 的图层计入可见集。
    fn test_visible_layers() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://visible/{z}/{x}/{y}.png", 1.0, 0, 12);
        let id2 = mgr.add_layer("https://hidden/{z}/{x}/{y}.png", 0.5, 0, 12);
        mgr.layers.last_mut().unwrap().show = false;
        assert_eq!(mgr.visible_layers().count(), 1);
    }

    #[test]
    /// 验证描述符向领域层的 alpha 映射。
    fn test_to_domain_layer() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://tiles/{z}/{x}/{y}.png", 0.75, 2, 15);
        let desc = mgr.get_layer(1).unwrap();
        let layer = mgr.to_domain_layer(desc);
        assert_eq!(layer.alpha, 0.75);
        assert_eq!(layer.minimum_level, 2);
        assert_eq!(layer.maximum_level, 15);
    }
}
