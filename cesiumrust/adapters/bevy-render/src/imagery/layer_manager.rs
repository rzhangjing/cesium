use bevy::prelude::*;
use cesium_imagery::ImageryLayer;

#[derive(Resource, Default)]
pub struct ImageryLayerManager {
    pub layers: Vec<ImageryLayerDescriptor>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ImageryLayerDescriptor {
    pub id: u64,
    pub url_template: String,
    pub opacity: f32,
    pub visible: bool,
    pub min_level: u32,
    pub max_level: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub show: bool,
}

impl ImageryLayerManager {
    pub fn add_layer(
        &mut self,
        url_template: &str,
        opacity: f32,
        min_level: u32,
        max_level: u32,
    ) -> u64 {
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

    pub fn remove_layer(&mut self, id: u64) {
        self.layers.retain(|l| l.id != id);
    }

    pub fn get_layer(&self, id: u64) -> Option<&ImageryLayerDescriptor> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn visible_layers(&self) -> impl Iterator<Item = &ImageryLayerDescriptor> {
        self.layers.iter().filter(|l| l.show && l.visible)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

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
    fn test_add_and_get_layer() {
        let mut mgr = ImageryLayerManager::default();
        let id = mgr.add_layer("https://tiles/{z}/{x}/{y}.png", 1.0, 0, 18);
        assert_eq!(id, 1);
        assert_eq!(mgr.layer_count(), 1);
        assert!(mgr.get_layer(id).is_some());
    }

    #[test]
    fn test_remove_layer() {
        let mut mgr = ImageryLayerManager::default();
        let id = mgr.add_layer("https://a.tiles/{z}/{x}/{y}.png", 0.5, 0, 12);
        mgr.add_layer("https://b.tiles/{z}/{x}/{y}.png", 0.8, 0, 12);
        assert_eq!(mgr.layer_count(), 2);
        mgr.remove_layer(id);
        assert_eq!(mgr.layer_count(), 1);
    }

    #[test]
    fn test_visible_layers() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://visible/{z}/{x}/{y}.png", 1.0, 0, 12);
        let id2 = mgr.add_layer("https://hidden/{z}/{x}/{y}.png", 0.5, 0, 12);
        mgr.layers.last_mut().unwrap().show = false;
        assert_eq!(mgr.visible_layers().count(), 1);
    }

    #[test]
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
