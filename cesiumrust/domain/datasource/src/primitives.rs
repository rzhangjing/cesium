//! Billboard, Label, and PointPrimitive collections.
//!
//! Maps to CesiumJS:
//! - `Scene/Billboard.js`, `Scene/BillboardCollection.js`
//! - `Scene/Label.js`, `Scene/LabelCollection.js`
//! - `Scene/PointPrimitive.js`, `Scene/PointPrimitiveCollection.js`

use crate::property::Color;

/// Vertical origin for billboard/label positioning.
///
/// Maps to CesiumJS `Scene/VerticalOrigin.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalOrigin {
    /// Origin is at the top of the item.
    Top,
    /// Origin is at the center of the item.
    #[default]
    Center,
    /// Origin is at the bottom of the item.
    Bottom,
    /// Origin is at the baseline of the text (labels only).
    Baseline,
}

/// Horizontal origin for billboard/label positioning.
///
/// Maps to CesiumJS `Scene/HorizontalOrigin.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalOrigin {
    /// Origin is at the left of the item.
    Left,
    /// Origin is at the center of the item.
    #[default]
    Center,
    /// Origin is at the right of the item.
    Right,
}

/// Label style (fill, outline, or both).
///
/// Maps to CesiumJS `Scene/LabelStyle.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelStyle {
    /// Fill only.
    #[default]
    Fill,
    /// Outline only.
    Outline,
    /// Fill and outline.
    FillAndOutline,
}

/// A distance-based scaling condition.
///
/// Maps to CesiumJS `Core/NearFarScalar.js`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearFarScalar {
    /// Near distance.
    pub near: f64,
    /// Value at near distance.
    pub near_value: f64,
    /// Far distance.
    pub far: f64,
    /// Value at far distance.
    pub far_value: f64,
}

impl NearFarScalar {
    /// Creates a new near-far scalar.
    pub fn new(near: f64, near_value: f64, far: f64, far_value: f64) -> Self {
        Self { near, near_value, far, far_value }
    }

    /// Interpolates the value at the given distance.
    pub fn value_at_distance(&self, distance: f64) -> f64 {
        if distance <= self.near {
            self.near_value
        } else if distance >= self.far {
            self.far_value
        } else {
            let t = (distance - self.near) / (self.far - self.near);
            self.near_value + t * (self.far_value - self.near_value)
        }
    }
}

impl Default for NearFarScalar {
    fn default() -> Self {
        Self { near: 0.0, near_value: 0.0, far: 1.0, far_value: 0.0 }
    }
}

/// A distance display condition (near/far clipping).
///
/// Maps to CesiumJS `Core/DistanceDisplayCondition.js`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceDisplayCondition {
    /// Near distance (meters).
    pub near: f64,
    /// Far distance (meters).
    pub far: f64,
}

impl DistanceDisplayCondition {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 2;

    /// Creates a new distance display condition.
    pub fn new(near: f64, far: f64) -> Self {
        Self { near, far }
    }

    /// Returns true if the given distance is within the condition.
    pub fn is_visible(&self, distance: f64) -> bool {
        distance >= self.near && distance <= self.far
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Maps to CesiumJS `DistanceDisplayCondition.pack`
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.near;
        array[starting_index + 1] = self.far;
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Maps to CesiumJS `DistanceDisplayCondition.unpack`
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            near: array[starting_index],
            far: array[starting_index + 1],
        }
    }

    /// Determines if two distance display conditions are equal.
    ///
    /// Maps to CesiumJS `DistanceDisplayCondition.equals`
    pub fn equals(&self, other: &Self) -> bool {
        self.near == other.near && self.far == other.far
    }
}

impl Default for DistanceDisplayCondition {
    fn default() -> Self {
        Self { near: 0.0, far: f64::MAX }
    }
}

/// A billboard in a billboard collection.
///
/// Maps to CesiumJS `Scene/Billboard.js`
#[derive(Debug, Clone, PartialEq)]
pub struct Billboard {
    /// Whether the billboard is shown.
    pub show: bool,
    /// Position in Cartesian3 [x, y, z].
    pub position: [f64; 3],
    /// Pixel offset [x, y].
    pub pixel_offset: [f64; 2],
    /// Eye offset [x, y, z].
    pub eye_offset: [f64; 3],
    /// Vertical origin.
    pub vertical_origin: VerticalOrigin,
    /// Horizontal origin.
    pub horizontal_origin: HorizontalOrigin,
    /// Scale factor.
    pub scale: f64,
    /// Color tint.
    pub color: Color,
    /// Rotation in radians.
    pub rotation: f64,
    /// Aligned axis [x, y, z].
    pub aligned_axis: [f64; 3],
    /// Width in pixels (None = use image width).
    pub width: Option<f64>,
    /// Height in pixels (None = use image height).
    pub height: Option<f64>,
    /// Whether size is in meters instead of pixels.
    pub size_in_meters: bool,
    /// Image URI or ID.
    pub image: Option<String>,
    /// Scale by distance.
    pub scale_by_distance: Option<NearFarScalar>,
    /// Translucency by distance.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// Distance display condition.
    pub distance_display_condition: Option<DistanceDisplayCondition>,
    /// User-defined ID.
    pub id: Option<String>,
}

impl Default for Billboard {
    fn default() -> Self {
        Self {
            show: true,
            position: [0.0; 3],
            pixel_offset: [0.0; 2],
            eye_offset: [0.0; 3],
            vertical_origin: VerticalOrigin::Center,
            horizontal_origin: HorizontalOrigin::Center,
            scale: 1.0,
            color: Color::WHITE,
            rotation: 0.0,
            aligned_axis: [0.0; 3],
            width: None,
            height: None,
            size_in_meters: false,
            image: None,
            scale_by_distance: None,
            translucency_by_distance: None,
            distance_display_condition: None,
            id: None,
        }
    }
}

/// A collection of billboards.
///
/// Maps to CesiumJS `Scene/BillboardCollection.js`
#[derive(Debug, Default)]
pub struct BillboardCollection {
    billboards: Vec<Billboard>,
    show: bool,
}

impl BillboardCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self { billboards: Vec::new(), show: true }
    }

    /// Adds a billboard to the collection.
    pub fn add(&mut self, billboard: Billboard) -> usize {
        let index = self.billboards.len();
        self.billboards.push(billboard);
        index
    }

    /// Removes a billboard by index.
    pub fn remove(&mut self, index: usize) -> Option<Billboard> {
        if index < self.billboards.len() {
            Some(self.billboards.remove(index))
        } else {
            None
        }
    }

    /// Gets a billboard by index.
    pub fn get(&self, index: usize) -> Option<&Billboard> {
        self.billboards.get(index)
    }

    /// Gets a mutable billboard by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Billboard> {
        self.billboards.get_mut(index)
    }

    /// Number of billboards.
    pub fn len(&self) -> usize {
        self.billboards.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.billboards.is_empty()
    }

    /// Iterates over billboards.
    pub fn iter(&self) -> impl Iterator<Item = &Billboard> {
        self.billboards.iter()
    }

    /// Whether the collection is shown.
    pub fn show(&self) -> bool {
        self.show
    }

    /// Sets whether the collection is shown.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    /// Clears all billboards.
    pub fn clear(&mut self) {
        self.billboards.clear();
    }
}

/// A label in a label collection.
///
/// Maps to CesiumJS `Scene/Label.js`
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// Whether the label is shown.
    pub show: bool,
    /// Position in Cartesian3 [x, y, z].
    pub position: [f64; 3],
    /// Text content.
    pub text: String,
    /// Font (CSS format).
    pub font: String,
    /// Fill color.
    pub fill_color: Color,
    /// Outline color.
    pub outline_color: Color,
    /// Outline width.
    pub outline_width: f64,
    /// Label style.
    pub style: LabelStyle,
    /// Whether to show background.
    pub show_background: bool,
    /// Background color.
    pub background_color: Color,
    /// Background padding [x, y].
    pub background_padding: [f64; 2],
    /// Vertical origin.
    pub vertical_origin: VerticalOrigin,
    /// Horizontal origin.
    pub horizontal_origin: HorizontalOrigin,
    /// Pixel offset [x, y].
    pub pixel_offset: [f64; 2],
    /// Eye offset [x, y, z].
    pub eye_offset: [f64; 3],
    /// Scale factor.
    pub scale: f64,
    /// Scale by distance.
    pub scale_by_distance: Option<NearFarScalar>,
    /// Translucency by distance.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// Distance display condition.
    pub distance_display_condition: Option<DistanceDisplayCondition>,
    /// User-defined ID.
    pub id: Option<String>,
}

impl Default for Label {
    fn default() -> Self {
        Self {
            show: true,
            position: [0.0; 3],
            text: String::new(),
            font: "30px sans-serif".to_string(),
            fill_color: Color::WHITE,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            style: LabelStyle::Fill,
            show_background: false,
            background_color: Color::new(0.165, 0.165, 0.165, 0.8),
            background_padding: [7.0, 5.0],
            vertical_origin: VerticalOrigin::Baseline,
            horizontal_origin: HorizontalOrigin::Left,
            pixel_offset: [0.0; 2],
            eye_offset: [0.0; 3],
            scale: 1.0,
            scale_by_distance: None,
            translucency_by_distance: None,
            distance_display_condition: None,
            id: None,
        }
    }
}

/// A collection of labels.
///
/// Maps to CesiumJS `Scene/LabelCollection.js`
#[derive(Debug, Default)]
pub struct LabelCollection {
    labels: Vec<Label>,
    show: bool,
}

impl LabelCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self { labels: Vec::new(), show: true }
    }

    /// Adds a label to the collection.
    pub fn add(&mut self, label: Label) -> usize {
        let index = self.labels.len();
        self.labels.push(label);
        index
    }

    /// Removes a label by index.
    pub fn remove(&mut self, index: usize) -> Option<Label> {
        if index < self.labels.len() {
            Some(self.labels.remove(index))
        } else {
            None
        }
    }

    /// Gets a label by index.
    pub fn get(&self, index: usize) -> Option<&Label> {
        self.labels.get(index)
    }

    /// Gets a mutable label by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Label> {
        self.labels.get_mut(index)
    }

    /// Number of labels.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Iterates over labels.
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.labels.iter()
    }

    /// Whether the collection is shown.
    pub fn show(&self) -> bool {
        self.show
    }

    /// Sets whether the collection is shown.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    /// Clears all labels.
    pub fn clear(&mut self) {
        self.labels.clear();
    }
}

/// A point primitive in a point primitive collection.
///
/// Maps to CesiumJS `Scene/PointPrimitive.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PointPrimitive {
    /// Whether the point is shown.
    pub show: bool,
    /// Position in Cartesian3 [x, y, z].
    pub position: [f64; 3],
    /// Point color.
    pub color: Color,
    /// Outline color.
    pub outline_color: Color,
    /// Outline width in pixels.
    pub outline_width: f64,
    /// Pixel size.
    pub pixel_size: f64,
    /// Scale by distance.
    pub scale_by_distance: Option<NearFarScalar>,
    /// Translucency by distance.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// Distance display condition.
    pub distance_display_condition: Option<DistanceDisplayCondition>,
    /// User-defined ID.
    pub id: Option<String>,
}

impl Default for PointPrimitive {
    fn default() -> Self {
        Self {
            show: true,
            position: [0.0; 3],
            color: Color::WHITE,
            outline_color: Color::TRANSPARENT,
            outline_width: 0.0,
            pixel_size: 10.0,
            scale_by_distance: None,
            translucency_by_distance: None,
            distance_display_condition: None,
            id: None,
        }
    }
}

/// A collection of point primitives.
///
/// Maps to CesiumJS `Scene/PointPrimitiveCollection.js`
#[derive(Debug, Default)]
pub struct PointPrimitiveCollection {
    points: Vec<PointPrimitive>,
    show: bool,
}

impl PointPrimitiveCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self { points: Vec::new(), show: true }
    }

    /// Adds a point to the collection.
    pub fn add(&mut self, point: PointPrimitive) -> usize {
        let index = self.points.len();
        self.points.push(point);
        index
    }

    /// Removes a point by index.
    pub fn remove(&mut self, index: usize) -> Option<PointPrimitive> {
        if index < self.points.len() {
            Some(self.points.remove(index))
        } else {
            None
        }
    }

    /// Gets a point by index.
    pub fn get(&self, index: usize) -> Option<&PointPrimitive> {
        self.points.get(index)
    }

    /// Gets a mutable point by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PointPrimitive> {
        self.points.get_mut(index)
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Iterates over points.
    pub fn iter(&self) -> impl Iterator<Item = &PointPrimitive> {
        self.points.iter()
    }

    /// Whether the collection is shown.
    pub fn show(&self) -> bool {
        self.show
    }

    /// Sets whether the collection is shown.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    /// Clears all points.
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billboard_collection() {
        let mut collection = BillboardCollection::new();
        assert!(collection.is_empty());

        let bb = Billboard {
            position: [1.0, 2.0, 3.0],
            scale: 2.0,
            color: Color::RED,
            image: Some("marker.png".to_string()),
            ..Default::default()
        };
        let idx = collection.add(bb);
        assert_eq!(idx, 0);
        assert_eq!(collection.len(), 1);

        let retrieved = collection.get(0).unwrap();
        assert_eq!(retrieved.position, [1.0, 2.0, 3.0]);
        assert_eq!(retrieved.scale, 2.0);
        assert_eq!(retrieved.image, Some("marker.png".to_string()));
    }

    #[test]
    fn test_billboard_remove() {
        let mut collection = BillboardCollection::new();
        collection.add(Billboard::default());
        collection.add(Billboard { scale: 3.0, ..Default::default() });
        assert_eq!(collection.len(), 2);

        let removed = collection.remove(0).unwrap();
        assert_eq!(removed.scale, 1.0);
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.get(0).unwrap().scale, 3.0);
    }

    #[test]
    fn test_label_collection() {
        let mut collection = LabelCollection::new();
        let label = Label {
            text: "Hello World".to_string(),
            position: [100.0, 200.0, 300.0],
            fill_color: Color::YELLOW,
            font: "16px monospace".to_string(),
            ..Default::default()
        };
        collection.add(label);
        assert_eq!(collection.len(), 1);

        let l = collection.get(0).unwrap();
        assert_eq!(l.text, "Hello World");
        assert_eq!(l.font, "16px monospace");
    }

    #[test]
    fn test_point_primitive_collection() {
        let mut collection = PointPrimitiveCollection::new();
        let point = PointPrimitive {
            position: [10.0, 20.0, 30.0],
            color: Color::GREEN,
            pixel_size: 15.0,
            outline_width: 2.0,
            outline_color: Color::BLACK,
            ..Default::default()
        };
        collection.add(point);
        assert_eq!(collection.len(), 1);

        let p = collection.get(0).unwrap();
        assert_eq!(p.pixel_size, 15.0);
        assert_eq!(p.color, Color::GREEN);
    }

    #[test]
    fn test_near_far_scalar() {
        let nfs = NearFarScalar::new(100.0, 1.0, 1000.0, 0.5);
        assert!((nfs.value_at_distance(50.0) - 1.0).abs() < 1e-10);
        assert!((nfs.value_at_distance(100.0) - 1.0).abs() < 1e-10);
        assert!((nfs.value_at_distance(550.0) - 0.75).abs() < 1e-10);
        assert!((nfs.value_at_distance(1000.0) - 0.5).abs() < 1e-10);
        assert!((nfs.value_at_distance(2000.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_distance_display_condition() {
        let ddc = DistanceDisplayCondition::new(100.0, 10000.0);
        assert!(!ddc.is_visible(50.0));
        assert!(ddc.is_visible(100.0));
        assert!(ddc.is_visible(5000.0));
        assert!(ddc.is_visible(10000.0));
        assert!(!ddc.is_visible(20000.0));
    }

    #[test]
    fn test_collection_show() {
        let mut collection = BillboardCollection::new();
        assert!(collection.show());
        collection.set_show(false);
        assert!(!collection.show());
    }

    #[test]
    fn test_label_style_default() {
        assert_eq!(LabelStyle::default(), LabelStyle::Fill);
        assert_eq!(VerticalOrigin::default(), VerticalOrigin::Center);
        assert_eq!(HorizontalOrigin::default(), HorizontalOrigin::Center);
    }

    #[test]
    fn test_point_collection_clear() {
        let mut collection = PointPrimitiveCollection::new();
        collection.add(PointPrimitive::default());
        collection.add(PointPrimitive::default());
        assert_eq!(collection.len(), 2);
        collection.clear();
        assert!(collection.is_empty());
    }
}
