//! Imagery layer collection.
//! Maps to CesiumJS `Scene/ImageryLayerCollection.js`

use crate::imagery_layer::ImageryLayer;

/// A collection of imagery layers with ordering.
///
/// Layers are rendered in order from bottom (index 0) to top.
/// Maps to CesiumJS `ImageryLayerCollection`
#[derive(Debug, Clone, Default)]
pub struct ImageryLayerCollection {
    /// The layers in bottom-to-top order.
    layers: Vec<ImageryLayer>,

    /// Counter for generating unique layer IDs.
    next_id: u64,
}

impl ImageryLayerCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds a layer to the top of the collection.
    ///
    /// # Returns
    /// The ID assigned to the layer
    pub fn add(&mut self, mut layer: ImageryLayer) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        layer.id = id;
        self.layers.push(layer);
        id
    }

    /// Adds a layer at a specific index.
    ///
    /// # Returns
    /// The ID assigned to the layer
    pub fn add_at(&mut self, mut layer: ImageryLayer, index: usize) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        layer.id = id;
        let index = index.min(self.layers.len());
        self.layers.insert(index, layer);
        id
    }

    /// Removes a layer by ID.
    ///
    /// # Returns
    /// The removed layer, if found
    pub fn remove(&mut self, id: u64) -> Option<ImageryLayer> {
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Removes a layer at a specific index.
    ///
    /// # Returns
    /// The removed layer, if the index was valid
    pub fn remove_at(&mut self, index: usize) -> Option<ImageryLayer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Gets a layer by ID.
    pub fn get(&self, id: u64) -> Option<&ImageryLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Gets a mutable layer by ID.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut ImageryLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Gets a layer by index.
    pub fn get_at(&self, index: usize) -> Option<&ImageryLayer> {
        self.layers.get(index)
    }

    /// Gets a mutable layer by index.
    pub fn get_at_mut(&mut self, index: usize) -> Option<&mut ImageryLayer> {
        self.layers.get_mut(index)
    }

    /// Returns the number of layers.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Returns an iterator over the layers.
    pub fn iter(&self) -> impl Iterator<Item = &ImageryLayer> {
        self.layers.iter()
    }

    /// Returns a mutable iterator over the layers.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ImageryLayer> {
        self.layers.iter_mut()
    }

    /// Moves a layer up in the collection (towards the top).
    pub fn raise(&mut self, id: u64) {
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            if index < self.layers.len() - 1 {
                self.layers.swap(index, index + 1);
            }
        }
    }

    /// Moves a layer down in the collection (towards the bottom).
    pub fn lower(&mut self, id: u64) {
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            if index > 0 {
                self.layers.swap(index, index - 1);
            }
        }
    }

    /// Moves a layer to the top of the collection.
    pub fn raise_to_top(&mut self, id: u64) {
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            let layer = self.layers.remove(index);
            self.layers.push(layer);
        }
    }

    /// Moves a layer to the bottom of the collection.
    pub fn lower_to_bottom(&mut self, id: u64) {
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            let layer = self.layers.remove(index);
            self.layers.insert(0, layer);
        }
    }

    /// Returns the index of a layer by ID.
    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    /// Returns only the visible layers.
    pub fn visible_layers(&self) -> impl Iterator<Item = &ImageryLayer> {
        self.layers.iter().filter(|l| l.show)
    }

    /// Computes the blended alpha for a pixel given all visible layers.
    ///
    /// This implements standard alpha compositing from bottom to top.
    ///
    /// # Arguments
    /// * `layer_alphas` - Alpha values for each layer (in collection order)
    ///
    /// # Returns
    /// The final blended alpha value
    pub fn compute_blended_alpha(&self, layer_alphas: &[f64]) -> f64 {
        let mut result = 0.0;
        let mut remaining = 1.0;

        for (layer, &alpha) in self.layers.iter().zip(layer_alphas.iter()) {
            if !layer.show {
                continue;
            }
            let contribution = alpha * remaining;
            result += contribution;
            remaining *= 1.0 - alpha;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::rectangle::Rectangle;

    fn create_test_layer() -> ImageryLayer {
        ImageryLayer::new(0, Rectangle::MAX_VALUE)
    }

    #[test]
    fn test_add_and_get() {
        let mut collection = ImageryLayerCollection::new();
        let id = collection.add(create_test_layer());

        assert_eq!(collection.len(), 1);
        assert!(collection.get(id).is_some());
    }

    #[test]
    fn test_remove() {
        let mut collection = ImageryLayerCollection::new();
        let id = collection.add(create_test_layer());

        assert!(collection.remove(id).is_some());
        assert_eq!(collection.len(), 0);
    }

    #[test]
    fn test_ordering() {
        let mut collection = ImageryLayerCollection::new();
        let id1 = collection.add(create_test_layer());
        let id2 = collection.add(create_test_layer());
        let id3 = collection.add(create_test_layer());

        assert_eq!(collection.index_of(id1), Some(0));
        assert_eq!(collection.index_of(id2), Some(1));
        assert_eq!(collection.index_of(id3), Some(2));

        collection.raise(id1);
        assert_eq!(collection.index_of(id1), Some(1));
        assert_eq!(collection.index_of(id2), Some(0));

        collection.raise_to_top(id1);
        assert_eq!(collection.index_of(id1), Some(2));

        collection.lower_to_bottom(id1);
        assert_eq!(collection.index_of(id1), Some(0));
    }

    #[test]
    fn test_visible_layers() {
        let mut collection = ImageryLayerCollection::new();
        collection.add(create_test_layer().with_show(true));
        collection.add(create_test_layer().with_show(false));
        collection.add(create_test_layer().with_show(true));

        assert_eq!(collection.visible_layers().count(), 2);
    }

    #[test]
    fn test_blended_alpha() {
        let mut collection = ImageryLayerCollection::new();
        collection.add(create_test_layer());
        collection.add(create_test_layer());

        // Two layers with 0.5 alpha each
        // Result = 0.5 + 0.5 * 0.5 = 0.75
        let alphas = vec![0.5, 0.5];
        let blended = collection.compute_blended_alpha(&alphas);
        assert!((blended - 0.75).abs() < 1e-10);
    }
}
