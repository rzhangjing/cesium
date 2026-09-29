//! 材质属性：表示 [`Material`] uniform 的属性。
//!
//! 映射到 CesiumJS `DataSources/MaterialProperty.js` 及其具体实现
//! `ColorMaterialProperty`、`ImageMaterialProperty`、
//! `CheckerboardMaterialProperty`、`GridMaterialProperty`、
//! `StripeMaterialProperty`、`PolylineArrowMaterialProperty`、
//! `PolylineDashMaterialProperty`、`PolylineGlowMaterialProperty`、
//! `PolylineOutlineMaterialProperty` 与 `CompositeMaterialProperty`。
//!
//! 每个材质属性都会求值为一个材质类型字符串（例如 `"Color"`、
//! `"Grid"`）加上一组命名 uniform 值。Fabric 材质系统
//! （P1.3）消费这些值来构建实际的 shader 材质。

use crate::property_system::property::{ConstantProperty, DynProperty};
use crate::property_system::value::PropertyValue;
use cesium_time::{JulianDate, TimeInterval, TimeIntervalCollection, TimeIntervalData};
use glam::DVec2;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 以 uniform 名为键的材质 uniform 值。
///
/// 映射到由 CesiumJS `MaterialProperty.prototype.getValue(time, result)`
/// 填充的 `result` 对象。
pub type MaterialUniforms = BTreeMap<String, PropertyValue>;

/// `Color.WHITE`（映射到 CesiumJS `Color.WHITE`）。
pub const COLOR_WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
/// `Color.BLACK`（映射到 CesiumJS `Color.BLACK`）。
pub const COLOR_BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
/// `Color.TRANSPARENT`（映射到 CesiumJS `Color.TRANSPARENT`）。
pub const COLOR_TRANSPARENT: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

/// 所有表示材质 uniform 的属性的接口。
///
/// 映射到 CesiumJS `DataSources/MaterialProperty.js`。
pub trait MaterialProperty: Send + Sync {
    /// 在当前定义下 `get_value` 是否总返回相同结果。映射到 `isConstant`。
    fn is_constant(&self) -> bool;

    /// 获取所提供时间处的材质类型。
    /// 映射到 `MaterialProperty.prototype.getType`。
    fn get_type(&self, time: &JulianDate) -> Option<String>;

    /// 获取所提供时间处该属性的 uniform 值。
    /// 映射到 `MaterialProperty.prototype.getValue(time, result)`。
    fn get_value(&self, time: &JulianDate) -> MaterialUniforms;

    /// 将此属性与另一个属性比较。
    /// 映射到 `MaterialProperty.prototype.equals`。
    fn equals(&self, other: &dyn MaterialProperty) -> bool;

    /// 支持向下转型为具体类型。
    fn as_any(&self) -> &dyn Any;
}

/// 比较两个 trait-object 材质属性是否相等，将 `Arc` 指针相等视为相等。
/// 镜像了用于材质属性时的 `Property.equals(left, right)`。
pub fn arc_material_property_equals(
    left: &Arc<dyn MaterialProperty>,
    right: &Arc<dyn MaterialProperty>,
) -> bool {
    Arc::ptr_eq(left, right) || left.equals(right.as_ref())
}

/// 将原始值包装为常量属性。
fn to_constant(value: PropertyValue) -> Arc<dyn DynProperty> {
    Arc::new(ConstantProperty::new(value))
}

/// 映射到 `Property.getValueOrClonedDefault` / `Property.getValueOrDefault`：
/// 在 `time` 处求值该属性，当属性缺失或产生 undefined 时回退到 `default`。
fn value_or_default(
    property: &Option<Arc<dyn DynProperty>>,
    time: &JulianDate,
    default: PropertyValue,
) -> PropertyValue {
    match property {
        Some(p) => {
            let v = p.get_value(time);
            if v.is_undefined() {
                default
            } else {
                v
            }
        }
        None => default,
    }
}

/// 映射到 `Property.getValueOrUndefined`。
fn value_or_undefined(
    property: &Option<Arc<dyn DynProperty>>,
    time: &JulianDate,
) -> PropertyValue {
    match property {
        Some(p) => p.get_value(time),
        None => PropertyValue::Undefined,
    }
}

/// 映射到用于可选属性的 `Property.isConstant(property)`。
fn option_is_constant(property: &Option<Arc<dyn DynProperty>>) -> bool {
    match property {
        None => true,
        Some(p) => p.is_constant(),
    }
}

/// 映射到用于可选属性的 `Property.equals(left, right)`。
fn option_equals(
    left: &Option<Arc<dyn DynProperty>>,
    right: &Option<Arc<dyn DynProperty>>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(l), Some(r)) => Arc::ptr_eq(l, r) || l.equals(r.as_ref()),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// ColorMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到纯色（solid color）材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/ColorMaterialProperty.js`.
#[derive(Clone)]
pub struct ColorMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
}

impl ColorMaterialProperty {
    /// 创建新的颜色材质属性。`color` 可以是一个
    /// `PropertyValue::Color`；其他属性种类可通过
    /// [`set_color_property`](Self::set_color_property) 赋值。
    /// 映射到 `new ColorMaterialProperty(color)`。
    pub fn new(color: Option<PropertyValue>) -> Self {
        Self {
            color: color.map(to_constant),
        }
    }

    /// 从常量 RGBA 颜色创建颜色材质属性。
    pub fn from_color(color: [f64; 4]) -> Self {
        Self::new(Some(PropertyValue::Color(color)))
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }
}

impl MaterialProperty for ColorMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Color".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<ColorMaterialProperty>() {
            Some(o) => option_equals(&self.color, &o.color),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ImageMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到图像材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/ImageMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct ImageMaterialProperty {
    image: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
    color: Option<Arc<dyn DynProperty>>,
    transparent: Option<Arc<dyn DynProperty>>,
}

impl ImageMaterialProperty {
    /// 创建新的图像材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 图像属性（URL/canvas 等）。映射到 `image`。
    pub fn image_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.image.as_ref()
    }

    /// 设置图像属性。映射到 `image` setter。
    pub fn set_image_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.image = property;
    }

    /// 将图像设为常量值（通常为 URL 字符串）。
    pub fn set_image(&mut self, image: Option<PropertyValue>) {
        self.image = image.map(to_constant);
    }

    /// repeat（重复）属性。映射到 `repeat`。
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// 设置 repeat 属性。映射到 `repeat` setter。
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// 将 repeat 设为常量值。
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// transparent 属性。映射到 `transparent`。
    pub fn transparent_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.transparent.as_ref()
    }

    /// 设置 transparent 属性。映射到 `transparent` setter。
    pub fn set_transparent_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.transparent = property;
    }

    /// 将 transparent 标志设为常量值。
    pub fn set_transparent(&mut self, transparent: Option<PropertyValue>) {
        self.transparent = transparent.map(to_constant);
    }
}

impl MaterialProperty for ImageMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.image) && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Image".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "image".to_string(),
            value_or_undefined(&self.image, time),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(
                &self.repeat,
                time,
                PropertyValue::Cartesian2(DVec2::new(1.0, 1.0)),
            ),
        );
        let mut color = value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE));
        let transparent = value_or_default(
            &self.transparent,
            time,
            PropertyValue::Boolean(false),
        );
        if matches!(transparent, PropertyValue::Boolean(true)) {
            if let PropertyValue::Color(ref mut c) = color {
                c[3] = c[3].min(0.99);
            }
        }
        uniforms.insert("color".to_string(), color);
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<ImageMaterialProperty>() {
            Some(o) => {
                option_equals(&self.image, &o.image)
                    && option_equals(&self.repeat, &o.repeat)
                    && option_equals(&self.color, &o.color)
                    && option_equals(&self.transparent, &o.transparent)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CheckerboardMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到棋盘格材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/CheckerboardMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct CheckerboardMaterialProperty {
    even_color: Option<Arc<dyn DynProperty>>,
    odd_color: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
}

impl CheckerboardMaterialProperty {
    /// 创建新的棋盘格材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 偶数颜色属性。映射到 `evenColor`。
    pub fn even_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.even_color.as_ref()
    }

    /// 设置偶数颜色属性。映射到 `evenColor` setter。
    pub fn set_even_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.even_color = property;
    }

    /// 将偶数颜色设为常量值。
    pub fn set_even_color(&mut self, color: Option<PropertyValue>) {
        self.even_color = color.map(to_constant);
    }

    /// 奇数颜色属性。映射到 `oddColor`。
    pub fn odd_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.odd_color.as_ref()
    }

    /// 设置奇数颜色属性。映射到 `oddColor` setter。
    pub fn set_odd_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.odd_color = property;
    }

    /// 将奇数颜色设为常量值。
    pub fn set_odd_color(&mut self, color: Option<PropertyValue>) {
        self.odd_color = color.map(to_constant);
    }

    /// repeat（重复）属性。映射到 `repeat`。
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// 设置 repeat 属性。映射到 `repeat` setter。
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// 将 repeat 设为常量值。
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }
}

impl MaterialProperty for CheckerboardMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.even_color)
            && option_is_constant(&self.odd_color)
            && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Checkerboard".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "lightColor".to_string(),
            value_or_default(&self.even_color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "darkColor".to_string(),
            value_or_default(&self.odd_color, time, PropertyValue::Color(COLOR_BLACK)),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(
                &self.repeat,
                time,
                PropertyValue::Cartesian2(DVec2::new(2.0, 2.0)),
            ),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<CheckerboardMaterialProperty>() {
            Some(o) => {
                option_equals(&self.even_color, &o.even_color)
                    && option_equals(&self.odd_color, &o.odd_color)
                    && option_equals(&self.repeat, &o.repeat)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GridMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到网格材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/GridMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct GridMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    cell_alpha: Option<Arc<dyn DynProperty>>,
    line_count: Option<Arc<dyn DynProperty>>,
    line_thickness: Option<Arc<dyn DynProperty>>,
    line_offset: Option<Arc<dyn DynProperty>>,
}

impl GridMaterialProperty {
    /// 创建新的网格材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// cell alpha（单元格透明度）属性。映射到 `cellAlpha`。
    pub fn cell_alpha_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.cell_alpha.as_ref()
    }

    /// 设置 cell alpha 属性。映射到 `cellAlpha` setter。
    pub fn set_cell_alpha_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.cell_alpha = property;
    }

    /// 将 cell alpha 设为常量值。
    pub fn set_cell_alpha(&mut self, cell_alpha: Option<PropertyValue>) {
        self.cell_alpha = cell_alpha.map(to_constant);
    }

    /// line count（线数）属性。映射到 `lineCount`。
    pub fn line_count_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_count.as_ref()
    }

    /// 设置 line count 属性。映射到 `lineCount` setter。
    pub fn set_line_count_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_count = property;
    }

    /// 将 line count 设为常量值。
    pub fn set_line_count(&mut self, line_count: Option<PropertyValue>) {
        self.line_count = line_count.map(to_constant);
    }

    /// line thickness（线宽）属性。映射到 `lineThickness`。
    pub fn line_thickness_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_thickness.as_ref()
    }

    /// 设置 line thickness 属性。映射到 `lineThickness` setter。
    pub fn set_line_thickness_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_thickness = property;
    }

    /// 将 line thickness 设为常量值。
    pub fn set_line_thickness(&mut self, line_thickness: Option<PropertyValue>) {
        self.line_thickness = line_thickness.map(to_constant);
    }

    /// line offset（线偏移）属性。映射到 `lineOffset`。
    pub fn line_offset_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_offset.as_ref()
    }

    /// 设置 line offset 属性。映射到 `lineOffset` setter。
    pub fn set_line_offset_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_offset = property;
    }

    /// 将 line offset 设为常量值。
    pub fn set_line_offset(&mut self, line_offset: Option<PropertyValue>) {
        self.line_offset = line_offset.map(to_constant);
    }
}

impl MaterialProperty for GridMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.cell_alpha)
            && option_is_constant(&self.line_count)
            && option_is_constant(&self.line_thickness)
            && option_is_constant(&self.line_offset)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Grid".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "cellAlpha".to_string(),
            value_or_default(&self.cell_alpha, time, PropertyValue::Number(0.1)),
        );
        uniforms.insert(
            "lineCount".to_string(),
            value_or_default(
                &self.line_count,
                time,
                PropertyValue::Cartesian2(DVec2::new(8.0, 8.0)),
            ),
        );
        uniforms.insert(
            "lineThickness".to_string(),
            value_or_default(
                &self.line_thickness,
                time,
                PropertyValue::Cartesian2(DVec2::new(1.0, 1.0)),
            ),
        );
        uniforms.insert(
            "lineOffset".to_string(),
            value_or_default(
                &self.line_offset,
                time,
                PropertyValue::Cartesian2(DVec2::new(0.0, 0.0)),
            ),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<GridMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.cell_alpha, &o.cell_alpha)
                    && option_equals(&self.line_count, &o.line_count)
                    && option_equals(&self.line_thickness, &o.line_thickness)
                    && option_equals(&self.line_offset, &o.line_offset)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// StripeMaterialProperty
// ---------------------------------------------------------------------------

/// `StripeMaterialProperty` 中条纹的方向。
///
/// 映射到 CesiumJS `DataSources/StripeOrientation.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StripeOrientation {
    /// 水平方向（`StripeOrientation.HORIZONTAL` = 0）。
    #[default]
    Horizontal,
    /// 垂直方向（`StripeOrientation.VERTICAL` = 1）。
    Vertical,
}

impl StripeOrientation {
    /// 转换为 CesiumJS 使用的数值表示。
    pub fn to_number(self) -> f64 {
        match self {
            StripeOrientation::Horizontal => 0.0,
            StripeOrientation::Vertical => 1.0,
        }
    }

    /// 转换为 `PropertyValue::Number`。
    pub fn to_value(self) -> PropertyValue {
        PropertyValue::Number(self.to_number())
    }

    /// 从属性值解析。除数值 `1.0` 以外的任何值都
    /// 产生 `Horizontal`（与 CesiumJS 的 `=== StripeOrientation.HORIZONTAL`
    /// 比较语义一致，其中默认值生效）。
    pub fn from_value(value: &PropertyValue) -> Self {
        match value {
            PropertyValue::Number(n) if *n == 1.0 => StripeOrientation::Vertical,
            _ => StripeOrientation::Horizontal,
        }
    }
}

/// 一种映射到条纹材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/StripeMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct StripeMaterialProperty {
    orientation: Option<Arc<dyn DynProperty>>,
    even_color: Option<Arc<dyn DynProperty>>,
    odd_color: Option<Arc<dyn DynProperty>>,
    offset: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
}

impl StripeMaterialProperty {
    /// 创建新的条纹材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// orientation 属性。映射到 `orientation`。
    pub fn orientation_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.orientation.as_ref()
    }

    /// 设置 orientation 属性。映射到 `orientation` setter。
    pub fn set_orientation_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.orientation = property;
    }

    /// 将 orientation 设为常量值。
    pub fn set_orientation(&mut self, orientation: StripeOrientation) {
        self.orientation = Some(to_constant(orientation.to_value()));
    }

    /// 偶数颜色属性。映射到 `evenColor`。
    pub fn even_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.even_color.as_ref()
    }

    /// 设置偶数颜色属性。映射到 `evenColor` setter。
    pub fn set_even_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.even_color = property;
    }

    /// 将偶数颜色设为常量值。
    pub fn set_even_color(&mut self, color: Option<PropertyValue>) {
        self.even_color = color.map(to_constant);
    }

    /// 奇数颜色属性。映射到 `oddColor`。
    pub fn odd_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.odd_color.as_ref()
    }

    /// 设置奇数颜色属性。映射到 `oddColor` setter。
    pub fn set_odd_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.odd_color = property;
    }

    /// 将奇数颜色设为常量值。
    pub fn set_odd_color(&mut self, color: Option<PropertyValue>) {
        self.odd_color = color.map(to_constant);
    }

    /// offset 属性。映射到 `offset`。
    pub fn offset_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.offset.as_ref()
    }

    /// 设置 offset 属性。映射到 `offset` setter。
    pub fn set_offset_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.offset = property;
    }

    /// 将 offset 设为常量值。
    pub fn set_offset(&mut self, offset: Option<PropertyValue>) {
        self.offset = offset.map(to_constant);
    }

    /// repeat（重复）属性。映射到 `repeat`。
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// 设置 repeat 属性。映射到 `repeat` setter。
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// 将 repeat 设为常量值。
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }
}

impl MaterialProperty for StripeMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.orientation)
            && option_is_constant(&self.even_color)
            && option_is_constant(&self.odd_color)
            && option_is_constant(&self.offset)
            && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Stripe".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        let orientation_value = value_or_default(
            &self.orientation,
            time,
            StripeOrientation::Horizontal.to_value(),
        );
        let horizontal =
            StripeOrientation::from_value(&orientation_value) == StripeOrientation::Horizontal;
        uniforms.insert("horizontal".to_string(), PropertyValue::Boolean(horizontal));
        uniforms.insert(
            "evenColor".to_string(),
            value_or_default(&self.even_color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "oddColor".to_string(),
            value_or_default(&self.odd_color, time, PropertyValue::Color(COLOR_BLACK)),
        );
        uniforms.insert(
            "offset".to_string(),
            value_or_default(&self.offset, time, PropertyValue::Number(0.0)),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(&self.repeat, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<StripeMaterialProperty>() {
            Some(o) => {
                option_equals(&self.orientation, &o.orientation)
                    && option_equals(&self.even_color, &o.even_color)
                    && option_equals(&self.odd_color, &o.odd_color)
                    && option_equals(&self.offset, &o.offset)
                    && option_equals(&self.repeat, &o.repeat)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineArrowMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到 PolylineArrow 材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/PolylineArrowMaterialProperty.js`.
#[derive(Clone)]
pub struct PolylineArrowMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
}

impl PolylineArrowMaterialProperty {
    /// 创建新的折线箭头材质属性。
    /// 映射到 `new PolylineArrowMaterialProperty(color)`。
    pub fn new(color: Option<PropertyValue>) -> Self {
        Self {
            color: color.map(to_constant),
        }
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }
}

impl MaterialProperty for PolylineArrowMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineArrow".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineArrowMaterialProperty>() {
            Some(o) => option_equals(&self.color, &o.color),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineDashMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到折线虚线材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/PolylineDashMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineDashMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    gap_color: Option<Arc<dyn DynProperty>>,
    dash_length: Option<Arc<dyn DynProperty>>,
    dash_pattern: Option<Arc<dyn DynProperty>>,
}

impl PolylineDashMaterialProperty {
    /// 创建新的折线虚线材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// gap（间隙）颜色属性。映射到 `gapColor`。
    pub fn gap_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.gap_color.as_ref()
    }

    /// 设置 gap 颜色属性。映射到 `gapColor` setter。
    pub fn set_gap_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.gap_color = property;
    }

    /// 将 gap 颜色设为常量值。
    pub fn set_gap_color(&mut self, color: Option<PropertyValue>) {
        self.gap_color = color.map(to_constant);
    }

    /// dash 长度属性。映射到 `dashLength`。
    pub fn dash_length_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.dash_length.as_ref()
    }

    /// 设置 dash 长度属性。映射到 `dashLength` setter。
    pub fn set_dash_length_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.dash_length = property;
    }

    /// 将 dash 长度设为常量值。
    pub fn set_dash_length(&mut self, dash_length: Option<PropertyValue>) {
        self.dash_length = dash_length.map(to_constant);
    }

    /// dash 图案属性。映射到 `dashPattern`。
    pub fn dash_pattern_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.dash_pattern.as_ref()
    }

    /// 设置 dash 图案属性。映射到 `dashPattern` setter。
    pub fn set_dash_pattern_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.dash_pattern = property;
    }

    /// 将 dash 图案设为常量值。
    pub fn set_dash_pattern(&mut self, dash_pattern: Option<PropertyValue>) {
        self.dash_pattern = dash_pattern.map(to_constant);
    }
}

impl MaterialProperty for PolylineDashMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.gap_color)
            && option_is_constant(&self.dash_length)
            && option_is_constant(&self.dash_pattern)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineDash".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "gapColor".to_string(),
            value_or_default(
                &self.gap_color,
                time,
                PropertyValue::Color(COLOR_TRANSPARENT),
            ),
        );
        uniforms.insert(
            "dashLength".to_string(),
            value_or_default(&self.dash_length, time, PropertyValue::Number(16.0)),
        );
        uniforms.insert(
            "dashPattern".to_string(),
            value_or_default(&self.dash_pattern, time, PropertyValue::Number(255.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineDashMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.gap_color, &o.gap_color)
                    && option_equals(&self.dash_length, &o.dash_length)
                    && option_equals(&self.dash_pattern, &o.dash_pattern)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineGlowMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到折线光晕材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/PolylineGlowMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineGlowMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    glow_power: Option<Arc<dyn DynProperty>>,
    taper_power: Option<Arc<dyn DynProperty>>,
}

impl PolylineGlowMaterialProperty {
    /// 创建新的折线光晕材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// glow power（光晕强度）属性。映射到 `glowPower`。
    pub fn glow_power_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.glow_power.as_ref()
    }

    /// 设置 glow power 属性。映射到 `glowPower` setter。
    pub fn set_glow_power_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.glow_power = property;
    }

    /// 将 glow power 设为常量值。
    pub fn set_glow_power(&mut self, glow_power: Option<PropertyValue>) {
        self.glow_power = glow_power.map(to_constant);
    }

    /// taper power（收缩强度）属性。映射到 `taperPower`。
    pub fn taper_power_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.taper_power.as_ref()
    }

    /// 设置 taper power 属性。映射到 `taperPower` setter。
    pub fn set_taper_power_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.taper_power = property;
    }

    /// 将 taper power 设为常量值。
    pub fn set_taper_power(&mut self, taper_power: Option<PropertyValue>) {
        self.taper_power = taper_power.map(to_constant);
    }
}

impl MaterialProperty for PolylineGlowMaterialProperty {
    fn is_constant(&self) -> bool {
        // 注意：CesiumJS 在此处检查 `Property.isConstant(this._glow)`，该引用
        // 指向一个不存在的字段，因而总是通过；预期的语义（以及 `equals` 中
        // 比较的字段）是 color、glowPower 与 taperPower，我们实现的正是后者。
        option_is_constant(&self.color)
            && option_is_constant(&self.glow_power)
            && option_is_constant(&self.taper_power)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineGlow".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "glowPower".to_string(),
            value_or_default(&self.glow_power, time, PropertyValue::Number(0.25)),
        );
        uniforms.insert(
            "taperPower".to_string(),
            value_or_default(&self.taper_power, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineGlowMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.glow_power, &o.glow_power)
                    && option_equals(&self.taper_power, &o.taper_power)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineOutlineMaterialProperty
// ---------------------------------------------------------------------------

/// 一种映射到折线轮廓材质 uniform 的材质属性。
///
/// 映射到 CesiumJS `DataSources/PolylineOutlineMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineOutlineMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    outline_color: Option<Arc<dyn DynProperty>>,
    outline_width: Option<Arc<dyn DynProperty>>,
}

impl PolylineOutlineMaterialProperty {
    /// 创建新的折线轮廓材质属性，所有值均取默认。
    pub fn new() -> Self {
        Self::default()
    }

    /// 颜色属性。映射到 `color`。
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// 设置颜色属性。映射到 `color` setter。
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// 将颜色设为常量值。
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// 轮廓颜色属性。映射到 `outlineColor`。
    pub fn outline_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.outline_color.as_ref()
    }

    /// 设置轮廓颜色属性。映射到 `outlineColor` setter。
    pub fn set_outline_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.outline_color = property;
    }

    /// 将轮廓颜色设为常量值。
    pub fn set_outline_color(&mut self, color: Option<PropertyValue>) {
        self.outline_color = color.map(to_constant);
    }

    /// 轮廓宽度属性。映射到 `outlineWidth`。
    pub fn outline_width_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.outline_width.as_ref()
    }

    /// 设置轮廓宽度属性。映射到 `outlineWidth` setter。
    pub fn set_outline_width_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.outline_width = property;
    }

    /// 将轮廓宽度设为常量值。
    pub fn set_outline_width(&mut self, width: Option<PropertyValue>) {
        self.outline_width = width.map(to_constant);
    }
}

impl MaterialProperty for PolylineOutlineMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.outline_color)
            && option_is_constant(&self.outline_width)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineOutline".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "outlineColor".to_string(),
            value_or_default(
                &self.outline_color,
                time,
                PropertyValue::Color(COLOR_BLACK),
            ),
        );
        uniforms.insert(
            "outlineWidth".to_string(),
            value_or_default(&self.outline_width, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineOutlineMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.outline_color, &o.outline_color)
                    && option_equals(&self.outline_width, &o.outline_width)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CompositeMaterialProperty
// ---------------------------------------------------------------------------

fn material_same_data(
    left: &Arc<dyn MaterialProperty>,
    right: &Arc<dyn MaterialProperty>,
) -> bool {
    arc_material_property_equals(left, right)
}

/// 一个既是 `MaterialProperty` 的 `CompositeProperty`。
///
/// 每个区间的数据本身就是一个材质属性；求值时委托给
/// 内部属性。
///
/// 映射到 CesiumJS `DataSources/CompositeMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct CompositeMaterialProperty {
    intervals: TimeIntervalCollection<Arc<dyn MaterialProperty>>,
}

impl CompositeMaterialProperty {
    /// 创建一个空的组合材质属性。
    pub fn new() -> Self {
        Self::default()
    }

    /// 底层的区间集合。映射到 `intervals`。
    pub fn intervals(&self) -> &TimeIntervalCollection<Arc<dyn MaterialProperty>> {
        &self.intervals
    }

    /// 添加一个数据为另一个材质属性的区间。
    pub fn add_interval(
        &mut self,
        interval: TimeInterval,
        data: Option<Arc<dyn MaterialProperty>>,
    ) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &material_same_data);
    }
}

impl MaterialProperty for CompositeMaterialProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_type(&self, time: &JulianDate) -> Option<String> {
        self.intervals
            .find_data_for_interval_containing_date(time)?
            .get_type(time)
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        match self.intervals.find_data_for_interval_containing_date(time) {
            Some(inner) => inner.get_value(time),
            None => MaterialUniforms::new(),
        }
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<CompositeMaterialProperty>() {
            Some(o) => self.intervals.equals(&o.intervals, &material_same_data),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_system::property::SampledProperty;
    use crate::property_system::value::PackableType;

    fn t(seconds: f64) -> JulianDate {
        JulianDate::new(2451545.0, seconds)
    }

    fn uniform<'a>(uniforms: &'a MaterialUniforms, name: &str) -> &'a PropertyValue {
        uniforms.get(name).unwrap_or_else(|| panic!("missing uniform {name}"))
    }

    #[test]
    fn test_color_material_defaults() {
        let prop = ColorMaterialProperty::new(None);
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Color".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_color_material_custom_color() {
        let red = [1.0, 0.0, 0.0, 1.0];
        let prop = ColorMaterialProperty::from_color(red);
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "color"), &PropertyValue::Color(red));
    }

    #[test]
    fn test_color_material_dynamic() {
        let mut sampled = SampledProperty::new(PackableType::Color);
        sampled.add_sample(t(0.0), &PropertyValue::Color([1.0, 0.0, 0.0, 1.0]), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Color([0.0, 0.0, 1.0, 1.0]), &[]);

        let mut prop = ColorMaterialProperty::new(None);
        prop.set_color_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());

        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([0.5, 0.0, 0.5, 1.0])
        );
    }

    #[test]
    fn test_color_material_equals() {
        let a = ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]);
        let b = ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]);
        let c = ColorMaterialProperty::from_color([0.0, 1.0, 0.0, 1.0]);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_image_material_defaults() {
        let prop = ImageMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Image".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "image"), &PropertyValue::Undefined);
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(1.0, 1.0))
        );
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_image_material_transparent_caps_alpha() {
        let mut prop = ImageMaterialProperty::new();
        prop.set_image(Some(PropertyValue::Text("test.png".to_string())));
        prop.set_transparent(Some(PropertyValue::Boolean(true)));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "image"),
            &PropertyValue::Text("test.png".to_string())
        );
        // 当 transparent 时，默认的 WHITE alpha 1.0 被限幅到 0.99。
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([1.0, 1.0, 1.0, 0.99])
        );
    }

    #[test]
    fn test_image_material_not_transparent_keeps_alpha() {
        let mut prop = ImageMaterialProperty::new();
        prop.set_color(Some(PropertyValue::Color([1.0, 1.0, 1.0, 0.5])));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([1.0, 1.0, 1.0, 0.5])
        );
    }

    #[test]
    fn test_checkerboard_defaults() {
        let prop = CheckerboardMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Checkerboard".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "lightColor"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "darkColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(2.0, 2.0))
        );
    }

    #[test]
    fn test_checkerboard_custom() {
        let mut prop = CheckerboardMaterialProperty::new();
        prop.set_even_color(Some(PropertyValue::Color([1.0, 0.0, 0.0, 1.0])));
        prop.set_odd_color(Some(PropertyValue::Color([0.0, 1.0, 0.0, 1.0])));
        prop.set_repeat(Some(PropertyValue::Cartesian2(DVec2::new(4.0, 4.0))));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "lightColor"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(
            uniform(&uniforms, "darkColor"),
            &PropertyValue::Color([0.0, 1.0, 0.0, 1.0])
        );
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(4.0, 4.0))
        );
    }

    #[test]
    fn test_grid_defaults() {
        let prop = GridMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Grid".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.1));
        assert_eq!(
            uniform(&uniforms, "lineCount"),
            &PropertyValue::Cartesian2(DVec2::new(8.0, 8.0))
        );
        assert_eq!(
            uniform(&uniforms, "lineThickness"),
            &PropertyValue::Cartesian2(DVec2::new(1.0, 1.0))
        );
        assert_eq!(
            uniform(&uniforms, "lineOffset"),
            &PropertyValue::Cartesian2(DVec2::new(0.0, 0.0))
        );
    }

    #[test]
    fn test_grid_custom_and_constancy() {
        let mut prop = GridMaterialProperty::new();
        prop.set_cell_alpha(Some(PropertyValue::Number(0.5)));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.5));
        assert!(prop.is_constant());

        // 动态的子属性会使整个材质变为非常量。
        let mut sampled = SampledProperty::new(PackableType::Number);
        sampled.add_sample(t(0.0), &PropertyValue::Number(0.0), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Number(1.0), &[]);
        prop.set_cell_alpha_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());
        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.5));
    }

    #[test]
    fn test_stripe_defaults() {
        let prop = StripeMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Stripe".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "horizontal"), &PropertyValue::Boolean(true));
        assert_eq!(
            uniform(&uniforms, "evenColor"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "oddColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(uniform(&uniforms, "offset"), &PropertyValue::Number(0.0));
        assert_eq!(uniform(&uniforms, "repeat"), &PropertyValue::Number(1.0));
    }

    #[test]
    fn test_stripe_vertical() {
        let mut prop = StripeMaterialProperty::new();
        prop.set_orientation(StripeOrientation::Vertical);
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "horizontal"),
            &PropertyValue::Boolean(false)
        );
    }

    #[test]
    fn test_stripe_orientation_value_roundtrip() {
        assert_eq!(
            StripeOrientation::from_value(&StripeOrientation::Horizontal.to_value()),
            StripeOrientation::Horizontal
        );
        assert_eq!(
            StripeOrientation::from_value(&StripeOrientation::Vertical.to_value()),
            StripeOrientation::Vertical
        );
        assert_eq!(StripeOrientation::Horizontal.to_number(), 0.0);
        assert_eq!(StripeOrientation::Vertical.to_number(), 1.0);
    }

    #[test]
    fn test_polyline_arrow() {
        let prop = PolylineArrowMaterialProperty::new(None);
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineArrow".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_polyline_dash_defaults() {
        let prop = PolylineDashMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineDash".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "gapColor"),
            &PropertyValue::Color(COLOR_TRANSPARENT)
        );
        assert_eq!(uniform(&uniforms, "dashLength"), &PropertyValue::Number(16.0));
        assert_eq!(
            uniform(&uniforms, "dashPattern"),
            &PropertyValue::Number(255.0)
        );
    }

    #[test]
    fn test_polyline_dash_custom() {
        let mut prop = PolylineDashMaterialProperty::new();
        prop.set_dash_length(Some(PropertyValue::Number(32.0)));
        prop.set_gap_color(Some(PropertyValue::Color([1.0, 0.0, 0.0, 0.5])));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "dashLength"), &PropertyValue::Number(32.0));
        assert_eq!(
            uniform(&uniforms, "gapColor"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 0.5])
        );
    }

    #[test]
    fn test_polyline_glow_defaults() {
        let prop = PolylineGlowMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineGlow".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(uniform(&uniforms, "glowPower"), &PropertyValue::Number(0.25));
        assert_eq!(uniform(&uniforms, "taperPower"), &PropertyValue::Number(1.0));
    }

    #[test]
    fn test_polyline_glow_dynamic_not_constant() {
        // 修正了 CesiumJS 的 `isConstant` bug（它检查了不存在的 `_glow`）：
        // 动态的 glowPower 必须使属性变为非常量。
        let mut prop = PolylineGlowMaterialProperty::new();
        let mut sampled = SampledProperty::new(PackableType::Number);
        sampled.add_sample(t(0.0), &PropertyValue::Number(0.1), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Number(0.9), &[]);
        prop.set_glow_power_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());
        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(uniform(&uniforms, "glowPower"), &PropertyValue::Number(0.5));
    }

    #[test]
    fn test_polyline_outline_defaults() {
        let prop = PolylineOutlineMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineOutline".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "outlineColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(
            uniform(&uniforms, "outlineWidth"),
            &PropertyValue::Number(1.0)
        );
    }

    #[test]
    fn test_composite_material() {
        let mut prop = CompositeMaterialProperty::new();
        assert!(prop.is_constant()); // 空 → 常量

        let color_mat = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        let grid_mat = Arc::new(GridMaterialProperty::new()) as Arc<dyn MaterialProperty>;

        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, false), Some(color_mat));
        prop.add_interval(TimeInterval::new(t(10.0), t(20.0), true, true), Some(grid_mat));
        assert!(!prop.is_constant());

        assert_eq!(prop.get_type(&t(5.0)), Some("Color".to_string()));
        assert_eq!(
            uniform(&prop.get_value(&t(5.0)), "color"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 1.0])
        );

        assert_eq!(prop.get_type(&t(15.0)), Some("Grid".to_string()));
        assert!(prop.get_value(&t(15.0)).contains_key("cellAlpha"));

        // 在所有区间之外：无类型，uniform 为空。
        assert_eq!(prop.get_type(&t(30.0)), None);
        assert!(prop.get_value(&t(30.0)).is_empty());
    }

    #[test]
    fn test_composite_material_equals() {
        let mut a = CompositeMaterialProperty::new();
        let mut b = CompositeMaterialProperty::new();
        let mat_a = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        let mat_b = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        a.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_a));
        b.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_b));
        assert!(a.equals(&b));

        let mut c = CompositeMaterialProperty::new();
        let mat_c = Arc::new(ColorMaterialProperty::from_color([0.0, 0.0, 1.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        c.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_c));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_material_type_names() {
        assert_eq!(
            ColorMaterialProperty::new(None).get_type(&t(0.0)).unwrap(),
            "Color"
        );
        assert_eq!(
            ImageMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Image"
        );
        assert_eq!(
            CheckerboardMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Checkerboard"
        );
        assert_eq!(GridMaterialProperty::new().get_type(&t(0.0)).unwrap(), "Grid");
        assert_eq!(
            StripeMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Stripe"
        );
        assert_eq!(
            PolylineArrowMaterialProperty::new(None).get_type(&t(0.0)).unwrap(),
            "PolylineArrow"
        );
        assert_eq!(
            PolylineDashMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineDash"
        );
        assert_eq!(
            PolylineGlowMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineGlow"
        );
        assert_eq!(
            PolylineOutlineMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineOutline"
        );
    }
}
