//! Cloud rendering system (CumulusCloud + CloudCollection).
//!
//! Maps to CesiumJS:
//! - `Scene/CumulusCloud.js`
//! - `Scene/CloudCollection.js`
//! - `Scene/CloudType.js`

use glam::DVec3;

/// Cloud type enumeration.
///
/// Maps to CesiumJS `Scene/CloudType.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CloudType {
    /// Cumulus clouds (billboard-based).
    #[default]
    Cumulus,
}

/// A single cumulus cloud billboard in the 3D scene.
///
/// Maps to CesiumJS `Scene/CumulusCloud.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct CumulusCloud {
    /// Whether the cloud is visible.
    pub show: bool,
    /// World position of the cloud.
    pub position: DVec3,
    /// Billboard scale (width, height) in meters.
    pub scale: [f64; 2],
    /// Maximum size of the cloud volume (x, y, z) in meters.
    pub maximum_size: DVec3,
    /// Cross-section slice through the cloud [0, 1], or negative for no slice.
    pub slice: f64,
    /// Brightness multiplier [0, 1].
    pub brightness: f64,
    /// Cloud color as RGBA [0, 1].
    pub color: [f64; 4],
    /// Internal index in the collection.
    index: i32,
}

impl Default for CumulusCloud {
    fn default() -> Self {
        Self {
            show: true,
            position: DVec3::ZERO,
            scale: [20.0, 12.0],
            maximum_size: DVec3::new(20.0, 12.0, 12.0_f64 / 1.5),
            slice: -1.0,
            brightness: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            index: -1,
        }
    }
}

impl CumulusCloud {
    /// Create a new cumulus cloud with position and maximum size.
    pub fn new(position: DVec3, maximum_size: DVec3) -> Self {
        let scale = [maximum_size.x, maximum_size.y];
        Self {
            position,
            scale,
            maximum_size,
            ..Default::default()
        }
    }

    /// Create with full options.
    pub fn with_options(
        position: DVec3,
        scale: [f64; 2],
        maximum_size: DVec3,
        slice: f64,
        brightness: f64,
        color: [f64; 4],
    ) -> Self {
        Self {
            show: true,
            position,
            scale,
            maximum_size,
            slice,
            brightness,
            color,
            index: -1,
        }
    }

    /// Get the cloud's index in the collection.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// Compute the effective billboard dimensions considering slice.
    pub fn effective_dimensions(&self) -> [f64; 2] {
        if self.slice >= 0.0 && self.slice <= 1.0 {
            // Sliced clouds appear smaller
            let factor = 1.0 - (self.slice - 0.5).abs() * 0.5;
            [self.scale[0] * factor, self.scale[1] * factor]
        } else {
            self.scale
        }
    }

    /// Check if the slice value is in the recommended range [0.1, 0.9].
    pub fn is_slice_recommended(&self) -> bool {
        self.slice < 0.0 || (self.slice >= 0.1 && self.slice <= 0.9)
    }
}

/// A renderable collection of clouds in the 3D scene.
///
/// Maps to CesiumJS `Scene/CloudCollection.js`.
#[derive(Debug, Clone)]
pub struct CloudCollection {
    /// Whether to display the clouds.
    pub show: bool,
    /// Desired amount of detail in the noise texture.
    pub noise_detail: f64,
    /// Desired translation of data in noise texture.
    pub noise_offset: DVec3,
    /// For debugging: render billboards with opaque color.
    pub debug_billboards: bool,
    /// For debugging: render clouds as opaque ellipsoids.
    pub debug_ellipsoids: bool,
    /// The clouds in this collection.
    clouds: Vec<CumulusCloud>,
    /// Whether the collection needs a GPU buffer update.
    dirty: bool,
}

impl Default for CloudCollection {
    fn default() -> Self {
        Self {
            show: true,
            noise_detail: 16.0,
            noise_offset: DVec3::ZERO,
            debug_billboards: false,
            debug_ellipsoids: false,
            clouds: Vec::new(),
            dirty: true,
        }
    }
}

impl CloudCollection {
    /// Create a new empty cloud collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with noise parameters.
    pub fn with_noise(noise_detail: f64, noise_offset: DVec3) -> Self {
        Self {
            noise_detail,
            noise_offset,
            ..Default::default()
        }
    }

    /// Add a cloud to the collection. Returns the cloud's index.
    pub fn add(&mut self, mut cloud: CumulusCloud) -> usize {
        let index = self.clouds.len();
        cloud.index = index as i32;
        self.clouds.push(cloud);
        self.dirty = true;
        index
    }

    /// Remove a cloud by index.
    pub fn remove(&mut self, index: usize) -> Option<CumulusCloud> {
        if index < self.clouds.len() {
            let cloud = self.clouds.remove(index);
            // Reindex remaining clouds
            for (i, c) in self.clouds.iter_mut().enumerate().skip(index) {
                c.index = i as i32;
            }
            self.dirty = true;
            Some(cloud)
        } else {
            None
        }
    }

    /// Remove all clouds.
    pub fn remove_all(&mut self) {
        self.clouds.clear();
        self.dirty = true;
    }

    /// Get a cloud by index.
    pub fn get(&self, index: usize) -> Option<&CumulusCloud> {
        self.clouds.get(index)
    }

    /// Get a mutable cloud by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut CumulusCloud> {
        if index < self.clouds.len() {
            self.dirty = true;
            self.clouds.get_mut(index)
        } else {
            None
        }
    }

    /// Get the number of clouds.
    pub fn len(&self) -> usize {
        self.clouds.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty()
    }

    /// Get all clouds.
    pub fn clouds(&self) -> &[CumulusCloud] {
        &self.clouds
    }

    /// Get visible clouds only.
    pub fn visible_clouds(&self) -> impl Iterator<Item = &CumulusCloud> {
        self.clouds.iter().filter(|c| c.show)
    }

    /// Check if the collection needs a GPU update.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the collection as clean (after GPU update).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Compute the total bounding sphere of all visible clouds.
    pub fn compute_bounding_sphere(&self) -> Option<(DVec3, f64)> {
        let visible: Vec<_> = self.visible_clouds().collect();
        if visible.is_empty() {
            return None;
        }

        // Simple centroid + max distance approach
        let mut center = DVec3::ZERO;
        for cloud in &visible {
            center += cloud.position;
        }
        center /= visible.len() as f64;

        let mut max_dist = 0.0_f64;
        for cloud in &visible {
            let dist = (cloud.position - center).length()
                + cloud.maximum_size.length() * 0.5;
            max_dist = max_dist.max(dist);
        }

        Some((center, max_dist))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulus_cloud_default() {
        let cloud = CumulusCloud::default();
        assert!(cloud.show);
        assert_eq!(cloud.position, DVec3::ZERO);
        assert_eq!(cloud.scale, [20.0, 12.0]);
        assert_eq!(cloud.slice, -1.0);
        assert_eq!(cloud.brightness, 1.0);
        assert_eq!(cloud.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_cumulus_cloud_new() {
        let pos = DVec3::new(100.0, 200.0, 300.0);
        let size = DVec3::new(30.0, 20.0, 15.0);
        let cloud = CumulusCloud::new(pos, size);
        assert_eq!(cloud.position, pos);
        assert_eq!(cloud.maximum_size, size);
        assert_eq!(cloud.scale, [30.0, 20.0]);
    }

    #[test]
    fn test_cumulus_cloud_with_options() {
        let cloud = CumulusCloud::with_options(
            DVec3::new(1.0, 2.0, 3.0),
            [25.0, 15.0],
            DVec3::new(25.0, 15.0, 10.0),
            0.5,
            0.8,
            [0.9, 0.9, 0.9, 1.0],
        );
        assert_eq!(cloud.scale, [25.0, 15.0]);
        assert_eq!(cloud.slice, 0.5);
        assert_eq!(cloud.brightness, 0.8);
    }

    #[test]
    fn test_cumulus_cloud_effective_dimensions() {
        let mut cloud = CumulusCloud::default();
        // No slice (negative) => full scale
        assert_eq!(cloud.effective_dimensions(), [20.0, 12.0]);

        // Slice at 0.5 => factor = 1.0
        cloud.slice = 0.5;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 20.0).abs() < 1e-10);

        // Slice at 0.0 => factor = 0.75
        cloud.slice = 0.0;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_cumulus_cloud_slice_recommended() {
        let mut cloud = CumulusCloud::default();
        assert!(cloud.is_slice_recommended()); // -1.0 is ok

        cloud.slice = 0.5;
        assert!(cloud.is_slice_recommended());

        cloud.slice = 0.05;
        assert!(!cloud.is_slice_recommended());

        cloud.slice = 0.95;
        assert!(!cloud.is_slice_recommended());
    }

    #[test]
    fn test_cloud_collection_default() {
        let collection = CloudCollection::new();
        assert!(collection.show);
        assert_eq!(collection.noise_detail, 16.0);
        assert_eq!(collection.noise_offset, DVec3::ZERO);
        assert!(collection.is_empty());
    }

    #[test]
    fn test_cloud_collection_add_remove() {
        let mut collection = CloudCollection::new();
        let idx = collection.add(CumulusCloud::new(DVec3::new(1.0, 2.0, 3.0), DVec3::new(20.0, 12.0, 8.0)));
        assert_eq!(idx, 0);
        assert_eq!(collection.len(), 1);

        let idx2 = collection.add(CumulusCloud::new(DVec3::new(4.0, 5.0, 6.0), DVec3::new(15.0, 9.0, 9.0)));
        assert_eq!(idx2, 1);
        assert_eq!(collection.len(), 2);

        let removed = collection.remove(0);
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
        // Remaining cloud should be reindexed
        assert_eq!(collection.get(0).unwrap().index(), 0);
    }

    #[test]
    fn test_cloud_collection_remove_all() {
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::default());
        collection.add(CumulusCloud::default());
        collection.add(CumulusCloud::default());
        assert_eq!(collection.len(), 3);

        collection.remove_all();
        assert!(collection.is_empty());
    }

    #[test]
    fn test_cloud_collection_visible_clouds() {
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::default());
        let mut hidden = CumulusCloud::default();
        hidden.show = false;
        collection.add(hidden);
        collection.add(CumulusCloud::default());

        assert_eq!(collection.len(), 3);
        assert_eq!(collection.visible_clouds().count(), 2);
    }

    #[test]
    fn test_cloud_collection_dirty() {
        let mut collection = CloudCollection::new();
        assert!(collection.is_dirty());

        collection.mark_clean();
        assert!(!collection.is_dirty());

        collection.add(CumulusCloud::default());
        assert!(collection.is_dirty());
    }

    #[test]
    fn test_cloud_collection_bounding_sphere() {
        let mut collection = CloudCollection::new();
        assert!(collection.compute_bounding_sphere().is_none());

        collection.add(CumulusCloud::new(DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 10.0, 10.0)));
        collection.add(CumulusCloud::new(DVec3::new(100.0, 0.0, 0.0), DVec3::new(10.0, 10.0, 10.0)));

        let (center, radius) = collection.compute_bounding_sphere().unwrap();
        assert!((center.x - 50.0).abs() < 1e-10);
        assert!(radius > 50.0);
    }

    #[test]
    fn test_cloud_collection_with_noise() {
        let collection = CloudCollection::with_noise(32.0, DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(collection.noise_detail, 32.0);
        assert_eq!(collection.noise_offset, DVec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_cloud_type() {
        assert_eq!(CloudType::default(), CloudType::Cumulus);
    }
}
