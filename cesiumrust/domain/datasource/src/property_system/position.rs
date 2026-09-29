//! 位置属性：其值为一个世界位置（`Cartesian3`）并带有
//! 相关联参考系的属性。
//!
//! 映射到 CesiumJS `DataSources/PositionProperty.js` 及具体实现
//! `ConstantPositionProperty`、`SampledPositionProperty`、
//! `CompositePositionProperty`、`TimeIntervalCollectionPositionProperty` 与
//! `CallbackPositionProperty`。

use crate::property_system::interpolation::{ExtrapolationType, InterpolationAlgorithmKind};
use crate::property_system::property::{CompositeProperty, DynProperty, SampledProperty};
use crate::property_system::value::{PackableType, PropertyValue, ReferenceFrame};
use cesium_geospatial::transforms::compute_icrf_to_fixed_matrix;
use cesium_time::{JulianDate, TimeInterval, TimeIntervalCollection, TimeIntervalData};
use glam::DVec3;
use std::any::Any;
use std::sync::Arc;

/// 在给定时间处将一个位置从一个参考系转换到另一个。
///
/// 映射到 `PositionProperty.convertToReferenceFrame`。当两参考系相同
/// 时值原样返回。否则为 `time` 计算 ICRF 到 fixed 的旋转
/// 矩阵；inertial→fixed 乘以该矩阵，
/// fixed→inertial 乘以其转置。
pub fn convert_to_reference_frame(
    time: &JulianDate,
    value: DVec3,
    input_frame: ReferenceFrame,
    output_frame: ReferenceFrame,
) -> Option<DVec3> {
    if input_frame == output_frame {
        return Some(value);
    }

    let julian_date_seconds = time.total_days() * 86400.0;
    let icrf_to_fixed = compute_icrf_to_fixed_matrix(julian_date_seconds)?;
    match input_frame {
        ReferenceFrame::Inertial => Some(icrf_to_fixed * value),
        ReferenceFrame::Fixed => Some(icrf_to_fixed.transpose() * value),
    }
}

/// 将可选位置包装为 `PropertyValue`。
fn position_to_value(position: Option<DVec3>) -> PropertyValue {
    match position {
        Some(p) => PropertyValue::Cartesian3(p),
        None => PropertyValue::Undefined,
    }
}

// ---------------------------------------------------------------------------
// ConstantPositionProperty
// ---------------------------------------------------------------------------

/// 一种位置属性，其值相对于其定义所在的参考系
/// 不发生变化。
///
/// 映射到 CesiumJS `DataSources/ConstantPositionProperty.js`。
#[derive(Debug, Clone, Default)]
pub struct ConstantPositionProperty {
    value: Option<DVec3>,
    reference_frame: ReferenceFrame,
}

impl ConstantPositionProperty {
    /// 在 fixed 参考系中创建新的常量位置属性。
    pub fn new(value: DVec3) -> Self {
        Self {
            value: Some(value),
            reference_frame: ReferenceFrame::Fixed,
        }
    }

    /// 在指定参考系中创建新的常量位置属性。
    /// 映射到 `new ConstantPositionProperty(value, referenceFrame)`。
    pub fn with_reference_frame(value: DVec3, reference_frame: ReferenceFrame) -> Self {
        Self {
            value: Some(value),
            reference_frame,
        }
    }

    /// 创建无值的常量位置属性。
    pub fn undefined() -> Self {
        Self {
            value: None,
            reference_frame: ReferenceFrame::Fixed,
        }
    }

    /// 设置值，并可选地设置参考系。
    /// 映射到 `ConstantPositionProperty.prototype.setValue`。
    pub fn set_value(&mut self, value: Option<DVec3>, reference_frame: Option<ReferenceFrame>) {
        self.value = value;
        if let Some(frame) = reference_frame {
            self.reference_frame = frame;
        }
    }

    /// 已存储的值（在此属性的参考系中）。
    pub fn value(&self) -> Option<DVec3> {
        self.value
    }

    /// 在所提供的参考系中获取 `time` 处的位置。
    /// 映射到 `ConstantPositionProperty.prototype.getValueInReferenceFrame`。
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let value = self.value?;
        convert_to_reference_frame(time, value, self.reference_frame, reference_frame)
    }
}

impl DynProperty for ConstantPositionProperty {
    fn is_constant(&self) -> bool {
        // 惯性系位置在用 fixed 系表示时会随时间变化，因此仅当未定义
        // 或为 fixed 系时才是常量。
        self.value.is_none() || self.reference_frame == ReferenceFrame::Fixed
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "ConstantPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<ConstantPositionProperty>() {
            Some(o) => self.value == o.value && self.reference_frame == o.reference_frame,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// SampledPositionProperty
// ---------------------------------------------------------------------------

/// 一个同时也是位置属性的 `SampledProperty`。
///
/// 映射到 CesiumJS `DataSources/SampledPositionProperty.js`。
#[derive(Debug, Clone)]
pub struct SampledPositionProperty {
    property: SampledProperty,
    reference_frame: ReferenceFrame,
    number_of_derivatives: usize,
}

impl SampledPositionProperty {
    /// 创建新的采样位置属性。
    ///
    /// 映射到 `new SampledPositionProperty(referenceFrame, numberOfDerivatives)`。
    pub fn new(reference_frame: ReferenceFrame, number_of_derivatives: usize) -> Self {
        let derivative_types = if number_of_derivatives > 0 {
            Some(vec![PackableType::Cartesian3; number_of_derivatives])
        } else {
            None
        };
        Self {
            property: SampledProperty::with_derivative_types(
                PackableType::Cartesian3,
                derivative_types,
            ),
            reference_frame,
            number_of_derivatives,
        }
    }

    /// 在 fixed 参考系中创建新的采样位置属性，不带导数。
    pub fn fixed() -> Self {
        Self::new(ReferenceFrame::Fixed, 0)
    }

    /// 随每个位置一同提供的导数数量。
    /// 映射到 `numberOfDerivatives`。
    pub fn number_of_derivatives(&self) -> usize {
        self.number_of_derivatives
    }

    /// 插值次数。映射到 `interpolationDegree`。
    pub fn interpolation_degree(&self) -> usize {
        self.property.interpolation_degree()
    }

    /// 插值算法。映射到 `interpolationAlgorithm`。
    pub fn interpolation_algorithm(&self) -> InterpolationAlgorithmKind {
        self.property.interpolation_algorithm()
    }

    /// 当前存储的样本数量。
    pub fn sample_count(&self) -> usize {
        self.property.sample_count()
    }

    /// 设置插值位置时所使用的算法与次数。
    /// 映射到 `SampledPositionProperty.prototype.setInterpolationOptions`。
    pub fn set_interpolation_options(
        &mut self,
        algorithm: Option<InterpolationAlgorithmKind>,
        degree: Option<usize>,
    ) {
        self.property.set_interpolation_options(algorithm, degree);
    }

    /// 设置前推外推类型。映射到 `forwardExtrapolationType`。
    pub fn set_forward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.property.set_forward_extrapolation_type(value);
    }

    /// 设置前推外推时长。
    /// 映射到 `forwardExtrapolationDuration`。
    pub fn set_forward_extrapolation_duration(&mut self, value: f64) {
        self.property.set_forward_extrapolation_duration(value);
    }

    /// 设置后推外推类型。
    /// 映射到 `backwardExtrapolationType`。
    pub fn set_backward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.property.set_backward_extrapolation_type(value);
    }

    /// 设置后推外推时长。
    /// 映射到 `backwardExtrapolationDuration`。
    pub fn set_backward_extrapolation_duration(&mut self, value: f64) {
        self.property.set_backward_extrapolation_duration(value);
    }

    /// 添加一个新样本。映射到 `SampledPositionProperty.prototype.addSample`。
    pub fn add_sample(&mut self, time: JulianDate, position: DVec3, derivatives: &[DVec3]) {
        let value = PropertyValue::Cartesian3(position);
        let deriv_values: Vec<PropertyValue> = derivatives
            .iter()
            .map(|d| PropertyValue::Cartesian3(*d))
            .collect();
        self.property.add_sample(time, &value, &deriv_values);
    }

    /// 通过并行数组添加多个样本。
    /// 映射到 `SampledPositionProperty.prototype.addSamples`。
    pub fn add_samples(
        &mut self,
        times: &[JulianDate],
        positions: &[DVec3],
        derivatives: Option<&[Vec<DVec3>]>,
    ) {
        let values: Vec<PropertyValue> = positions
            .iter()
            .map(|p| PropertyValue::Cartesian3(*p))
            .collect();
        let deriv_values: Option<Vec<Vec<PropertyValue>>> = derivatives.map(|ds| {
            ds.iter()
                .map(|dv| dv.iter().map(|d| PropertyValue::Cartesian3(*d)).collect())
                .collect()
        });
        self.property
            .add_samples(times, &values, deriv_values.as_deref());
    }

    /// 以单个打包数组添加样本，其中每个样本为一个时间偏移
    /// （相对于 `epoch` 的秒数）后接打包的位置与
    /// 导数。
    /// 映射到 `SampledPositionProperty.prototype.addSamplesPackedArray`。
    pub fn add_samples_packed_array(&mut self, packed_samples: &[f64], epoch: &JulianDate) {
        self.property
            .add_samples_packed_array(packed_samples, epoch);
    }

    /// 若存在则移除给定时间处的样本。
    /// 映射到 `SampledPositionProperty.prototype.removeSample`。
    pub fn remove_sample(&mut self, time: &JulianDate) -> bool {
        self.property.remove_sample(time)
    }

    /// 移除给定时间区间内的所有样本。
    /// 映射到 `SampledPositionProperty.prototype.removeSamples`。
    pub fn remove_samples_interval(&mut self, time_interval: &TimeInterval) {
        self.property.remove_samples_interval(time_interval);
    }

    /// 在所提供的参考系中获取 `time` 处的位置。
    /// 映射到 `SampledPositionProperty.prototype.getValueInReferenceFrame`。
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        match self.property.get_value(time) {
            PropertyValue::Cartesian3(p) => {
                convert_to_reference_frame(time, p, self.reference_frame, reference_frame)
            }
            _ => None,
        }
    }
}

impl DynProperty for SampledPositionProperty {
    fn is_constant(&self) -> bool {
        self.property.is_constant()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "SampledPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<SampledPositionProperty>() {
            Some(o) => {
                self.property.equals(&o.property) && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// CompositePositionProperty
// ---------------------------------------------------------------------------

/// 一个同时也是位置属性的 `CompositeProperty`。
///
/// 每个区间的数据本身就是一个位置属性；求值时委托给
/// 内部属性的 `getValueInReferenceFrame`。
///
/// 映射到 CesiumJS `DataSources/CompositePositionProperty.js`。
#[derive(Clone)]
pub struct CompositePositionProperty {
    composite: CompositeProperty,
    reference_frame: ReferenceFrame,
}

impl CompositePositionProperty {
    /// 创建新的组合位置属性。
    /// 映射到 `new CompositePositionProperty(referenceFrame)`。
    pub fn new(reference_frame: ReferenceFrame) -> Self {
        Self {
            composite: CompositeProperty::new(),
            reference_frame,
        }
    }

    /// 底层的区间集合。映射到 `intervals`。
    pub fn intervals(&self) -> &TimeIntervalCollection<Arc<dyn DynProperty>> {
        self.composite.intervals()
    }

    /// 添加一个数据为另一个（位置）属性的区间。
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<Arc<dyn DynProperty>>) {
        self.composite.add_interval(interval, data);
    }

    /// 设置此位置自我呈现的“首选”参考系。
    /// 映射到 `referenceFrame` setter。
    pub fn set_reference_frame(&mut self, frame: ReferenceFrame) {
        self.reference_frame = frame;
    }

    /// 在所提供的参考系中获取 `time` 处的位置。
    /// 映射到 `CompositePositionProperty.prototype.getValueInReferenceFrame`。
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let inner = self
            .composite
            .intervals()
            .find_data_for_interval_containing_date(time)?;
        match inner.get_value_in_reference_frame(time, reference_frame)? {
            PropertyValue::Cartesian3(p) => Some(p),
            _ => None,
        }
    }
}

impl DynProperty for CompositePositionProperty {
    fn is_constant(&self) -> bool {
        self.composite.is_constant()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "CompositePositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CompositePositionProperty>() {
            Some(o) => {
                self.reference_frame == o.reference_frame
                    && self.composite.equals(&o.composite)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// TimeIntervalCollectionPositionProperty
// ---------------------------------------------------------------------------

fn position_same_data(left: &DVec3, right: &DVec3) -> bool {
    *left == *right
}

/// 一个同时也是位置属性的 `TimeIntervalCollectionProperty`。
///
/// 映射到 CesiumJS `DataSources/TimeIntervalCollectionPositionProperty.js`。
#[derive(Debug, Clone)]
pub struct TimeIntervalCollectionPositionProperty {
    intervals: TimeIntervalCollection<DVec3>,
    reference_frame: ReferenceFrame,
}

impl TimeIntervalCollectionPositionProperty {
    /// 创建新的时间区间集合位置属性。
    /// 映射到 `new TimeIntervalCollectionPositionProperty(referenceFrame)`。
    pub fn new(reference_frame: ReferenceFrame) -> Self {
        Self {
            intervals: TimeIntervalCollection::new(),
            reference_frame,
        }
    }

    /// 底层的区间集合。映射到 `intervals`。
    pub fn intervals(&self) -> &TimeIntervalCollection<DVec3> {
        &self.intervals
    }

    /// 添加一个带给定位置数据的区间。
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<DVec3>) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &position_same_data);
    }

    /// 在所提供的参考系中获取 `time` 处的位置。
    ///
    /// 映射到
    /// `TimeIntervalCollectionPositionProperty.prototype.getValueInReferenceFrame`。
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let position = self.intervals.find_data_for_interval_containing_date(time)?;
        convert_to_reference_frame(time, *position, self.reference_frame, reference_frame)
    }
}

impl DynProperty for TimeIntervalCollectionPositionProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "TimeIntervalCollectionPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other
            .as_any()
            .downcast_ref::<TimeIntervalCollectionPositionProperty>()
        {
            Some(o) => {
                self.intervals.equals(&o.intervals, &position_same_data)
                    && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

// ---------------------------------------------------------------------------
// CallbackPositionProperty
// ---------------------------------------------------------------------------

/// `CallbackPositionProperty` 使用的回调签名：给定时间，
/// 返回属性参考系中的位置（或 `None`）。
pub type PositionCallbackFn = Arc<dyn Fn(&JulianDate) -> Option<DVec3> + Send + Sync>;

/// 其值由回调函数延迟求值的位置属性。
///
/// 映射到 CesiumJS `DataSources/CallbackPositionProperty.js`。
pub struct CallbackPositionProperty {
    callback: PositionCallbackFn,
    is_constant: bool,
    reference_frame: ReferenceFrame,
}

impl CallbackPositionProperty {
    /// 创建新的回调位置属性。
    ///
    /// 映射到 `new CallbackPositionProperty(callback, isConstant, referenceFrame)`。
    pub fn new(
        callback: PositionCallbackFn,
        is_constant: bool,
        reference_frame: ReferenceFrame,
    ) -> Self {
        Self {
            callback,
            is_constant,
            reference_frame,
        }
    }

    /// 替换回调与常量标志。
    /// 映射到 `CallbackPositionProperty.prototype.setCallback`。
    pub fn set_callback(&mut self, callback: PositionCallbackFn, is_constant: bool) {
        self.callback = callback;
        self.is_constant = is_constant;
    }

    /// 在所提供的参考系中获取 `time` 处的位置。
    ///
    /// 映射到 `CallbackPositionProperty.prototype.getValueInReferenceFrame`。
    pub fn position_in_reference_frame(
        &self,
        time: &JulianDate,
        reference_frame: ReferenceFrame,
    ) -> Option<DVec3> {
        let value = (self.callback)(time)?;
        convert_to_reference_frame(time, value, self.reference_frame, reference_frame)
    }
}

impl DynProperty for CallbackPositionProperty {
    fn is_constant(&self) -> bool {
        self.is_constant
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        position_to_value(self.position_in_reference_frame(time, ReferenceFrame::Fixed))
    }

    fn type_name(&self) -> &'static str {
        "CallbackPositionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CallbackPositionProperty>() {
            Some(o) => {
                Arc::ptr_eq(&self.callback, &o.callback)
                    && self.is_constant == o.is_constant
                    && self.reference_frame == o.reference_frame
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        Some(self.reference_frame)
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.position_in_reference_frame(time, frame)
            .map(PropertyValue::Cartesian3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_time::TimeInterval;
    use glam::DVec3;

    fn t(seconds: f64) -> JulianDate {
        JulianDate::new(2451545.0, seconds)
    }

    #[test]
    fn test_convert_same_frame_returns_unchanged() {
        let v = DVec3::new(1.0, 2.0, 3.0);
        let out = convert_to_reference_frame(&t(0.0), v, ReferenceFrame::Fixed, ReferenceFrame::Fixed);
        assert_eq!(out, Some(v));
        let out = convert_to_reference_frame(
            &t(0.0),
            v,
            ReferenceFrame::Inertial,
            ReferenceFrame::Inertial,
        );
        assert_eq!(out, Some(v));
    }

    #[test]
    fn test_convert_roundtrip_inertial_fixed() {
        let v = DVec3::new(1_000_000.0, 2_000_000.0, 3_000_000.0);
        let time = t(43200.0);
        let fixed = convert_to_reference_frame(&time, v, ReferenceFrame::Inertial, ReferenceFrame::Fixed)
            .unwrap();
        // 旋转保持长度。
        assert!((fixed.length() - v.length()).abs() < 1e-6);
        let back =
            convert_to_reference_frame(&time, fixed, ReferenceFrame::Fixed, ReferenceFrame::Inertial)
                .unwrap();
        assert!(back.abs_diff_eq(v, 1e-6));
    }

    #[test]
    fn test_convert_changes_with_time() {
        // 惯性位置在用 fixed 系表示时会随时间旋转。
        let v = DVec3::new(1_000_000.0, 0.0, 0.0);
        let f1 = convert_to_reference_frame(&t(0.0), v, ReferenceFrame::Inertial, ReferenceFrame::Fixed)
            .unwrap();
        let f2 = convert_to_reference_frame(
            &t(21600.0),
            v,
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(!f1.abs_diff_eq(f2, 1.0));
    }

    #[test]
    fn test_constant_position_fixed_frame() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let prop = ConstantPositionProperty::new(p);
        assert!(prop.is_constant());
        assert_eq!(prop.reference_frame(), Some(ReferenceFrame::Fixed));
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Cartesian3(p));
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Fixed),
            Some(PropertyValue::Cartesian3(p))
        );
    }

    #[test]
    fn test_constant_position_inertial_frame_not_constant() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let prop = ConstantPositionProperty::with_reference_frame(p, ReferenceFrame::Inertial);
        // 惯性系位置并非常量（它们在 fixed 系中旋转）。
        assert!(!prop.is_constant());

        // 在其自身参考系中的值，在任意时间都是已存储的值。
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Inertial),
            Some(PropertyValue::Cartesian3(p))
        );
        assert_eq!(
            prop.get_value_in_reference_frame(&t(1000.0), ReferenceFrame::Inertial),
            Some(PropertyValue::Cartesian3(p))
        );

        // fixed 系的值不同于已存储的惯性值……
        let fixed = prop.get_value(&t(0.0));
        assert_ne!(fixed, PropertyValue::Cartesian3(p));
        // ……并可回环转换回已存储的值。
        match fixed {
            PropertyValue::Cartesian3(fp) => {
                let back = convert_to_reference_frame(
                    &t(0.0),
                    fp,
                    ReferenceFrame::Fixed,
                    ReferenceFrame::Inertial,
                )
                .unwrap();
                assert!(back.abs_diff_eq(p, 1e-12));
            }
            _ => panic!("expected Cartesian3"),
        }
    }

    #[test]
    fn test_constant_position_undefined() {
        let prop = ConstantPositionProperty::undefined();
        assert!(prop.is_constant());
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Undefined);
        assert_eq!(
            prop.get_value_in_reference_frame(&t(0.0), ReferenceFrame::Fixed),
            None
        );
    }

    #[test]
    fn test_constant_position_equals() {
        let p = DVec3::new(1.0, 2.0, 3.0);
        let a = ConstantPositionProperty::new(p);
        let b = ConstantPositionProperty::new(p);
        let c = ConstantPositionProperty::with_reference_frame(p, ReferenceFrame::Inertial);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_sampled_position_linear_interpolation() {
        let mut prop = SampledPositionProperty::fixed();
        prop.add_sample(t(0.0), DVec3::new(0.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(10.0, 20.0, 30.0), &[]);
        assert!(!prop.is_constant());

        let mid = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        assert!(mid.abs_diff_eq(DVec3::new(5.0, 10.0, 15.0), 1e-12));

        // 精确的采样时间返回精确的值。
        let at0 = prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Fixed).unwrap();
        assert!(at0.abs_diff_eq(DVec3::ZERO, 1e-12));
    }

    #[test]
    fn test_sampled_position_inertial_frame() {
        let mut prop = SampledPositionProperty::new(ReferenceFrame::Inertial, 0);
        prop.add_sample(t(0.0), DVec3::new(1.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(2.0, 0.0, 0.0), &[]);

        // 在其自身（惯性）系中，插值是直接的。
        let inertial = prop
            .position_in_reference_frame(&t(5.0), ReferenceFrame::Inertial)
            .unwrap();
        assert!(inertial.abs_diff_eq(DVec3::new(1.5, 0.0, 0.0), 1e-12));

        // fixed 系的值是旋转后的插值。
        let fixed = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(5.0),
            DVec3::new(1.5, 0.0, 0.0),
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_sampled_position_with_derivatives() {
        // 使用速度导数的 Hermite 插值可重建一个三次曲线。
        let mut prop = SampledPositionProperty::new(ReferenceFrame::Fixed, 1);
        prop.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));
        // p(t) = t^3 along x; p'(t) = 3t^2.
        for i in 0..=2 {
            let tf = i as f64;
            prop.add_sample(
                t(tf),
                DVec3::new(tf * tf * tf, 0.0, 0.0),
                &[DVec3::new(3.0 * tf * tf, 0.0, 0.0)],
            );
        }
        let mid = prop.position_in_reference_frame(&t(1.5), ReferenceFrame::Fixed).unwrap();
        assert!((mid.x - 3.375).abs() < 1e-9);
    }

    #[test]
    fn test_sampled_position_extrapolation_none() {
        let mut prop = SampledPositionProperty::fixed();
        prop.add_sample(t(0.0), DVec3::new(0.0, 0.0, 0.0), &[]);
        prop.add_sample(t(10.0), DVec3::new(10.0, 0.0, 0.0), &[]);
        // 默认外推为 NONE：在样本之外 → undefined。
        assert_eq!(prop.get_value(&t(20.0)), PropertyValue::Undefined);
        assert_eq!(
            prop.position_in_reference_frame(&t(20.0), ReferenceFrame::Fixed),
            None
        );

        prop.set_forward_extrapolation_type(ExtrapolationType::Hold);
        let held = prop.position_in_reference_frame(&t(20.0), ReferenceFrame::Fixed).unwrap();
        assert!(held.abs_diff_eq(DVec3::new(10.0, 0.0, 0.0), 1e-12));
    }

    #[test]
    fn test_sampled_position_equals() {
        let mut a = SampledPositionProperty::fixed();
        a.add_sample(t(0.0), DVec3::new(1.0, 2.0, 3.0), &[]);
        let mut b = SampledPositionProperty::fixed();
        b.add_sample(t(0.0), DVec3::new(1.0, 2.0, 3.0), &[]);
        let c = SampledPositionProperty::new(ReferenceFrame::Inertial, 0);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_composite_position_delegates_to_inner() {
        let mut prop = CompositePositionProperty::new(ReferenceFrame::Fixed);
        let p1 = DVec3::new(1.0, 0.0, 0.0);
        let p2 = DVec3::new(2.0, 0.0, 0.0);
        prop.add_interval(
            TimeInterval::new(t(0.0), t(10.0), true, false),
            Some(Arc::new(ConstantPositionProperty::new(p1)) as Arc<dyn DynProperty>),
        );
        prop.add_interval(
            TimeInterval::new(t(10.0), t(20.0), true, true),
            Some(Arc::new(ConstantPositionProperty::new(p2)) as Arc<dyn DynProperty>),
        );
        assert!(!prop.is_constant());

        assert_eq!(prop.get_value(&t(5.0)), PropertyValue::Cartesian3(p1));
        assert_eq!(prop.get_value(&t(15.0)), PropertyValue::Cartesian3(p2));
        // 在所有区间之外 → undefined。
        assert_eq!(prop.get_value(&t(30.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_composite_position_inner_inertial() {
        // 内部属性自行处理其参考系转换：一个惯性内部
        // 属性在为 INERTIAL 查询时返回其已存储的值。
        let mut prop = CompositePositionProperty::new(ReferenceFrame::Fixed);
        let p = DVec3::new(5.0, 6.0, 7.0);
        prop.add_interval(
            TimeInterval::new(t(0.0), t(100.0), true, true),
            Some(
                Arc::new(ConstantPositionProperty::with_reference_frame(
                    p,
                    ReferenceFrame::Inertial,
                )) as Arc<dyn DynProperty>,
            ),
        );
        assert_eq!(
            prop.position_in_reference_frame(&t(50.0), ReferenceFrame::Inertial),
            Some(p)
        );
    }

    #[test]
    fn test_tic_position_property() {
        let mut prop = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        assert!(prop.is_constant()); // 空 → 常量

        let p = DVec3::new(100.0, 200.0, 300.0);
        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        assert!(!prop.is_constant());

        assert_eq!(prop.get_value(&t(5.0)), PropertyValue::Cartesian3(p));
        assert_eq!(prop.get_value(&t(11.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_tic_position_inertial_frame() {
        let mut prop = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Inertial);
        let p = DVec3::new(100.0, 200.0, 300.0);
        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));

        // 自身系：已存储的值。
        assert_eq!(
            prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Inertial),
            Some(p)
        );
        // fixed 系：已旋转。
        let fixed = prop.position_in_reference_frame(&t(5.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(5.0),
            p,
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_tic_position_equals() {
        let mut a = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        let mut b = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Fixed);
        let p = DVec3::new(1.0, 2.0, 3.0);
        a.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        b.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(p));
        assert!(a.equals(&b));

        let c = TimeIntervalCollectionPositionProperty::new(ReferenceFrame::Inertial);
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_callback_position_property() {
        let callback: PositionCallbackFn = Arc::new(|time: &JulianDate| {
            let d = time.day_number as f64;
            Some(DVec3::new(d, 0.0, 0.0))
        });
        let prop = CallbackPositionProperty::new(callback, false, ReferenceFrame::Fixed);
        assert!(!prop.is_constant());
        assert_eq!(
            prop.get_value(&t(7.0)),
            PropertyValue::Cartesian3(DVec3::new(2451545.0, 0.0, 0.0))
        );
    }

    #[test]
    fn test_callback_position_none_value() {
        let callback: PositionCallbackFn = Arc::new(|_time: &JulianDate| None);
        let prop = CallbackPositionProperty::new(callback, true, ReferenceFrame::Fixed);
        assert!(prop.is_constant());
        assert_eq!(prop.get_value(&t(0.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_callback_position_inertial() {
        let callback: PositionCallbackFn =
            Arc::new(|_time: &JulianDate| Some(DVec3::new(1.0, 2.0, 3.0)));
        let prop = CallbackPositionProperty::new(callback, true, ReferenceFrame::Inertial);

        assert_eq!(
            prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Inertial),
            Some(DVec3::new(1.0, 2.0, 3.0))
        );
        let fixed = prop.position_in_reference_frame(&t(0.0), ReferenceFrame::Fixed).unwrap();
        let expected = convert_to_reference_frame(
            &t(0.0),
            DVec3::new(1.0, 2.0, 3.0),
            ReferenceFrame::Inertial,
            ReferenceFrame::Fixed,
        )
        .unwrap();
        assert!(fixed.abs_diff_eq(expected, 1e-12));
    }

    #[test]
    fn test_callback_position_equals() {
        let cb: PositionCallbackFn = Arc::new(|_time: &JulianDate| Some(DVec3::ONE));
        let a = CallbackPositionProperty::new(Arc::clone(&cb), true, ReferenceFrame::Fixed);
        let b = CallbackPositionProperty::new(Arc::clone(&cb), true, ReferenceFrame::Fixed);
        let c = CallbackPositionProperty::new(Arc::clone(&cb), false, ReferenceFrame::Fixed);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }
}
