//! `DynProperty` 核心 trait 及其具体实现。
//!
//! 映射到 CesiumJS `DataSources/Property.js` 及具体属性类
//! `ConstantProperty`、`SampledProperty`、
//! `TimeIntervalCollectionProperty`、`CompositeProperty` 与
//! `CallbackProperty`。

use crate::property_system::interpolation::{
    ExtrapolationType, InterpolationAlgorithm, InterpolationAlgorithmKind,
};
use crate::property_system::value::{PackableType, PropertyValue, ReferenceFrame};
use cesium_time::{JulianDate, TimeInterval, TimeIntervalCollection, TimeIntervalData};
use std::any::Any;
use std::sync::Arc;

/// 所有属性的接口，表示一个可随时间变化的值。
///
/// 映射到 CesiumJS `DataSources/Property.js`。
pub trait DynProperty: Send + Sync {
    /// 在当前定义下 `get_value` 是否总返回相同结果。映射到 `Property.prototype.isConstant`。
    fn is_constant(&self) -> bool;

    /// 获取所提供时间处该属性的值。
    /// 映射到 `Property.prototype.getValue`。
    fn get_value(&self, time: &JulianDate) -> PropertyValue;

    /// 一个用于向下转型与调试的稳定类型名。
    fn type_name(&self) -> &'static str;

    /// 将此属性与另一个属性比较。映射到 `Property.prototype.equals`。
    fn equals(&self, other: &dyn DynProperty) -> bool;

    /// 支持向下转型为具体类型。
    fn as_any(&self) -> &dyn Any;

    /// 定义位置时所用的参考系。仅对位置属性有效。
    /// 映射到 `PositionProperty.referenceFrame`。
    fn reference_frame(&self) -> Option<ReferenceFrame> {
        None
    }

    /// 在所提供的参考系中获取值。仅对位置属性
    /// 有效。映射到 `PositionProperty.getValueInReferenceFrame`。
    fn get_value_in_reference_frame(
        &self,
        _time: &JulianDate,
        _frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        None
    }

    /// 获取所提供时间处的材质类型。仅对材质
    /// 属性有效。映射到 `MaterialProperty.getType`。
    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        None
    }
}

/// 比较两个 trait-object 属性是否相等，将 `Arc`
/// 指针相等视为相等。镜像 CesiumJS `Property.equals(left, right)`。
pub fn arc_property_equals(left: &Arc<dyn DynProperty>, right: &Arc<dyn DynProperty>) -> bool {
    Arc::ptr_eq(left, right) || left.equals(right.as_ref())
}

/// 镜像 CesiumJS `Property.isConstant(property)`：缺失的属性
/// 被视为常量。
pub fn property_is_constant(property: Option<&dyn DynProperty>) -> bool {
    match property {
        None => true,
        Some(p) => p.is_constant(),
    }
}

/// 镜像 CesiumJS `Property.getValueOrUndefined(property, time)`。
pub fn property_get_value_or_undefined(
    property: Option<&dyn DynProperty>,
    time: &JulianDate,
) -> PropertyValue {
    match property {
        Some(p) => p.get_value(time),
        None => PropertyValue::Undefined,
    }
}

// ---------------------------------------------------------------------------
// ConstantProperty
// ---------------------------------------------------------------------------

/// 其值不随仿真时间变化的属性。
///
/// 映射到 CesiumJS `DataSources/ConstantProperty.js`。
#[derive(Debug, Clone)]
pub struct ConstantProperty {
    value: PropertyValue,
}

impl ConstantProperty {
    /// 创建一个新的常量属性，取给定值。
    pub fn new(value: PropertyValue) -> Self {
        Self { value }
    }

    /// 设置属性的值。
    /// 映射到 `ConstantProperty.prototype.setValue`。
    pub fn set_value(&mut self, value: PropertyValue) {
        self.value = value;
    }

    /// 获取此属性的值。
    /// 映射到 `ConstantProperty.prototype.valueOf`。
    pub fn value(&self) -> &PropertyValue {
        &self.value
    }
}

impl DynProperty for ConstantProperty {
    fn is_constant(&self) -> bool {
        true
    }

    fn get_value(&self, _time: &JulianDate) -> PropertyValue {
        self.value.clone()
    }

    fn type_name(&self) -> &'static str {
        "ConstantProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<ConstantProperty>() {
            Some(o) => self.value == o.value,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// SampledProperty
// ---------------------------------------------------------------------------

/// 在已排序的 `JulianDate` 切片上二分查找。若精确匹配
/// 则返回其索引，否则返回插入点的按位取反值。
fn binary_search_times(times: &[JulianDate], target: &JulianDate) -> isize {
    let mut low: isize = 0;
    let mut high: isize = times.len() as isize - 1;
    while low <= high {
        let mid = (low + high) / 2;
        match times[mid as usize].cmp(target) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid - 1,
            std::cmp::Ordering::Equal => return mid,
        }
    }
    !low
}

/// 将新样本合并进已排序的 `times`/`values` 存储，并
/// 保持有序。时间已存在的样本会覆盖已存的值。
///
/// 映射到 `DataSources/SampledProperty.js` 内部的 `mergeNewSamples`。
fn merge_new_samples(
    times: &mut Vec<JulianDate>,
    values: &mut Vec<f64>,
    new_times: &[JulianDate],
    new_values: &[f64],
    packed_length: usize,
) {
    let new_count = new_times.len();
    let mut new_data_index = 0usize;

    while new_data_index < new_count {
        let current_time = new_times[new_data_index];
        let search = binary_search_times(times, &current_time);

        if search < 0 {
            // 不存在：尽可能多地插入额外的连续值。
            let insert_idx = (!search) as usize;
            let values_insertion_point = insert_idx * packed_length;
            let next_time = times.get(insert_idx).copied();

            let mut times_to_insert = Vec::new();
            let mut values_to_insert = Vec::new();
            let mut prev_item: Option<JulianDate> = None;

            while new_data_index < new_count {
                let ct = new_times[new_data_index];
                if let Some(prev) = prev_item {
                    if prev >= ct {
                        break;
                    }
                }
                if let Some(nt) = next_time {
                    if ct >= nt {
                        break;
                    }
                }
                times_to_insert.push(ct);
                let sample_idx = new_data_index;
                new_data_index += 1;
                for i in 0..packed_length {
                    values_to_insert.push(new_values[sample_idx * packed_length + i]);
                }
                prev_item = Some(ct);
            }

            if !times_to_insert.is_empty() {
                values.splice(
                    values_insertion_point..values_insertion_point,
                    values_to_insert.iter().copied(),
                );
                times.splice(insert_idx..insert_idx, times_to_insert.iter().copied());
            }
        } else {
            // 找到精确匹配：覆盖已存的值。
            let idx = search as usize;
            for i in 0..packed_length {
                values[idx * packed_length + i] = new_values[new_data_index * packed_length + i];
            }
            new_data_index += 1;
        }
    }
}

/// 其值在给定时间处由所提供的样本集以及指定的插值
/// 算法与次数插值得到的属性。
///
/// 映射到 CesiumJS `DataSources/SampledProperty.js`。
#[derive(Debug, Clone)]
pub struct SampledProperty {
    property_type: PackableType,
    derivative_types: Option<Vec<PackableType>>,
    interpolation_degree: usize,
    interpolation_algorithm: InterpolationAlgorithmKind,
    times: Vec<JulianDate>,
    values: Vec<f64>,
    packed_length: usize,
    packed_interpolation_length: usize,
    input_order: usize,
    forward_extrapolation_type: ExtrapolationType,
    forward_extrapolation_duration: f64,
    backward_extrapolation_type: ExtrapolationType,
    backward_extrapolation_duration: f64,
}

impl SampledProperty {
    /// 创建指定类型的新采样属性。
    pub fn new(property_type: PackableType) -> Self {
        Self::with_derivative_types(property_type, None)
    }

    /// 创建带导数信息的新采样属性。
    /// 映射到 `new SampledProperty(type, derivativeTypes)`。
    pub fn with_derivative_types(
        property_type: PackableType,
        derivative_types: Option<Vec<PackableType>>,
    ) -> Self {
        let mut packed_length = property_type.packed_length();
        let mut packed_interpolation_length = property_type.packed_interpolation_length();
        let mut input_order = 0;
        if let Some(ref derivs) = derivative_types {
            input_order = derivs.len();
            for d in derivs {
                packed_length += d.packed_length();
                packed_interpolation_length += d.packed_interpolation_length();
            }
        }
        Self {
            property_type,
            derivative_types,
            interpolation_degree: 1,
            interpolation_algorithm: InterpolationAlgorithmKind::Linear,
            times: Vec::new(),
            values: Vec::new(),
            packed_length,
            packed_interpolation_length,
            input_order,
            forward_extrapolation_type: ExtrapolationType::None,
            forward_extrapolation_duration: 0.0,
            backward_extrapolation_type: ExtrapolationType::None,
            backward_extrapolation_duration: 0.0,
        }
    }

    /// 属性的类型。映射到 `SampledProperty.prototype.type`。
    pub fn property_type(&self) -> PackableType {
        self.property_type
    }

    /// 导数类型。映射到 `SampledProperty.prototype.derivativeTypes`。
    pub fn derivative_types(&self) -> Option<&[PackableType]> {
        self.derivative_types.as_deref()
    }

    /// 插值次数。映射到 `interpolationDegree`。
    pub fn interpolation_degree(&self) -> usize {
        self.interpolation_degree
    }

    /// 插值算法。映射到 `interpolationAlgorithm`。
    pub fn interpolation_algorithm(&self) -> InterpolationAlgorithmKind {
        self.interpolation_algorithm
    }

    /// 当前存储的样本数量。
    pub fn sample_count(&self) -> usize {
        self.times.len()
    }

    /// 采样时间。
    pub fn times(&self) -> &[JulianDate] {
        &self.times
    }

    /// 设置插值时所使用的算法与次数。
    /// 映射到 `SampledProperty.prototype.setInterpolationOptions`。
    pub fn set_interpolation_options(
        &mut self,
        algorithm: Option<InterpolationAlgorithmKind>,
        degree: Option<usize>,
    ) {
        if let Some(alg) = algorithm {
            self.interpolation_algorithm = alg;
        }
        if let Some(deg) = degree {
            self.interpolation_degree = deg;
        }
    }

    /// 设置前推外推类型。映射到 `forwardExtrapolationType`。
    pub fn set_forward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.forward_extrapolation_type = value;
    }

    /// 设置前推外推时长。映射到
    /// `forwardExtrapolationDuration`。
    pub fn set_forward_extrapolation_duration(&mut self, value: f64) {
        self.forward_extrapolation_duration = value;
    }

    /// 设置后推外推类型。映射到
    /// `backwardExtrapolationType`。
    pub fn set_backward_extrapolation_type(&mut self, value: ExtrapolationType) {
        self.backward_extrapolation_type = value;
    }

    /// 设置后推外推时长。映射到
    /// `backwardExtrapolationDuration`。
    pub fn set_backward_extrapolation_duration(&mut self, value: f64) {
        self.backward_extrapolation_duration = value;
    }

    /// 添加一个新样本。映射到 `SampledProperty.prototype.addSample`。
    pub fn add_sample(
        &mut self,
        time: JulianDate,
        value: &PropertyValue,
        derivatives: &[PropertyValue],
    ) {
        let mut new_values = Vec::with_capacity(self.packed_length);
        self.property_type.pack(value, &mut new_values);
        if let Some(ref deriv_types) = self.derivative_types {
            for (i, dt) in deriv_types.iter().enumerate() {
                let d = derivatives.get(i).unwrap_or(&PropertyValue::Undefined);
                dt.pack(d, &mut new_values);
            }
        }
        merge_new_samples(
            &mut self.times,
            &mut self.values,
            &[time],
            &new_values,
            self.packed_length,
        );
    }

    /// 添加一组样本。映射到 `SampledProperty.prototype.addSamples`。
    pub fn add_samples(
        &mut self,
        times: &[JulianDate],
        values: &[PropertyValue],
        derivative_values: Option<&[Vec<PropertyValue>]>,
    ) {
        let mut new_times = Vec::with_capacity(times.len());
        let mut new_values = Vec::with_capacity(times.len() * self.packed_length);
        for (i, t) in times.iter().enumerate() {
            new_times.push(*t);
            let v = values.get(i).unwrap_or(&PropertyValue::Undefined);
            self.property_type.pack(v, &mut new_values);
            if let Some(ref deriv_types) = self.derivative_types {
                let empty: Vec<PropertyValue> = Vec::new();
                let derivs = derivative_values
                    .and_then(|dvs| dvs.get(i))
                    .unwrap_or(&empty);
                for (j, dt) in deriv_types.iter().enumerate() {
                    let d = derivs.get(j).unwrap_or(&PropertyValue::Undefined);
                    dt.pack(d, &mut new_values);
                }
            }
        }
        merge_new_samples(
            &mut self.times,
            &mut self.values,
            &new_times,
            &new_values,
            self.packed_length,
        );
    }

    /// 以单个打包数组添加样本，其中每个样本由一个数值时间偏移
    /// （相对于 `epoch` 的秒数）后接打包值（及导数）表示。
    ///
    /// 映射到 `SampledProperty.prototype.addSamplesPackedArray`。
    pub fn add_samples_packed_array(&mut self, packed_samples: &[f64], epoch: &JulianDate) {
        let stride = 1 + self.packed_length;
        let count = packed_samples.len() / stride;
        let mut new_times = Vec::with_capacity(count);
        let mut new_values = Vec::with_capacity(count * self.packed_length);
        for s in 0..count {
            let base = s * stride;
            new_times.push(epoch.add_seconds(packed_samples[base]));
            for i in 0..self.packed_length {
                new_values.push(packed_samples[base + 1 + i]);
            }
        }
        merge_new_samples(
            &mut self.times,
            &mut self.values,
            &new_times,
            &new_values,
            self.packed_length,
        );
    }

    /// 获取给定索引处样本的时间。负索引按
    /// 逆序访问样本列表。
    /// 映射到 `SampledProperty.prototype.getSample`。
    pub fn get_sample(&self, index: isize) -> Option<JulianDate> {
        let len = self.times.len();
        if len == 0 {
            return None;
        }
        let mut idx = index;
        if idx < 0 {
            idx += len as isize;
        }
        if idx < 0 || idx >= len as isize {
            return None;
        }
        Some(self.times[idx as usize])
    }

    /// 若存在则移除给定时间处的样本。若移除了
    /// 样本则返回 `true`。映射到 `SampledProperty.prototype.removeSample`。
    pub fn remove_sample(&mut self, time: &JulianDate) -> bool {
        let index = binary_search_times(&self.times, time);
        if index < 0 {
            return false;
        }
        self.remove_samples_at(index as usize, 1);
        true
    }

    /// 移除给定时间区间内的所有样本。
    /// 映射到 `SampledProperty.prototype.removeSamples`。
    pub fn remove_samples_interval(&mut self, time_interval: &TimeInterval) {
        let mut start_index = binary_search_times(&self.times, &time_interval.start);
        if start_index < 0 {
            start_index = !start_index;
        } else if !time_interval.is_start_included {
            start_index += 1;
        }
        let mut stop_index = binary_search_times(&self.times, &time_interval.stop);
        if stop_index < 0 {
            stop_index = !stop_index;
        } else if time_interval.is_stop_included {
            stop_index += 1;
        }
        let start = start_index as usize;
        let stop = stop_index as usize;
        if stop > start {
            self.remove_samples_at(start, stop - start);
        }
    }

    fn remove_samples_at(&mut self, start_index: usize, number_to_remove: usize) {
        let packed_length = self.packed_length;
        self.times
            .drain(start_index..start_index + number_to_remove);
        self.values.drain(
            start_index * packed_length..(start_index + number_to_remove) * packed_length,
        );
    }
}

impl DynProperty for SampledProperty {
    fn is_constant(&self) -> bool {
        self.values.is_empty()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        let times = &self.times;
        let times_length = times.len();
        if times_length == 0 {
            return PropertyValue::Undefined;
        }

        let inner_type = self.property_type;
        let values = &self.values;
        let mut index = binary_search_times(times, time);

        if index >= 0 {
            // 精确匹配。
            return inner_type.unpack(values, index as usize * self.packed_length);
        }

        // 转换为插入索引。
        index = !index;

        if index == 0 {
            let start_time = times[0];
            let timeout = self.backward_extrapolation_duration;
            if self.backward_extrapolation_type == ExtrapolationType::None
                || (timeout != 0.0 && start_time.seconds_difference(time) > timeout)
            {
                return PropertyValue::Undefined;
            }
            if self.backward_extrapolation_type == ExtrapolationType::Hold {
                return inner_type.unpack(values, 0);
            }
        }

        if index as usize >= times_length {
            index = (times_length - 1) as isize;
            let end_time = times[index as usize];
            let timeout = self.forward_extrapolation_duration;
            if self.forward_extrapolation_type == ExtrapolationType::None
                || (timeout != 0.0 && time.seconds_difference(&end_time) > timeout)
            {
                return PropertyValue::Undefined;
            }
            if self.forward_extrapolation_type == ExtrapolationType::Hold {
                return inner_type.unpack(values, index as usize * inner_type.packed_length());
            }
        }

        let interpolation_algorithm = self.interpolation_algorithm;
        let packed_interpolation_length = self.packed_interpolation_length;
        let input_order = self.input_order;

        let number_of_points = interpolation_algorithm
            .get_required_data_points(self.interpolation_degree, input_order)
            .min(times_length);

        let degree = number_of_points as isize - 1;
        if degree < 1 {
            return PropertyValue::Undefined;
        }
        let degree = degree as usize;

        let mut first_index = 0usize;
        let mut last_index = times_length - 1;
        let points_in_collection = last_index - first_index + 1;

        if points_in_collection > degree {
            let mut computed_first = index - (degree as isize / 2) - 1;
            if computed_first < 0 {
                computed_first = 0;
            }
            let mut computed_last = computed_first + degree as isize;
            let last_is = last_index as isize;
            if computed_last > last_is {
                computed_last = last_is;
                computed_first = computed_last - degree as isize;
                if computed_first < 0 {
                    computed_first = 0;
                }
            }
            first_index = computed_first as usize;
            last_index = computed_last as usize;
        }
        let length = last_index - first_index + 1;

        // 构建 x 表（相对于窗口内最后一个样本的秒数）。
        let mut x_table = vec![0.0f64; length];
        for (i, x) in x_table.iter_mut().enumerate() {
            *x = times[first_index + i].seconds_difference(&times[last_index]);
        }

        // 构建 y 表。
        let y_table: Vec<f64> = if !inner_type.uses_interpolation_conversion() {
            let packed_length = self.packed_length;
            let source_start = first_index * packed_length;
            let source_stop = (last_index + 1) * packed_length;
            values[source_start..source_stop].to_vec()
        } else {
            let mut table = vec![0.0f64; length * packed_interpolation_length];
            inner_type.convert_packed_array_for_interpolation(
                values,
                first_index,
                last_index,
                &mut table,
            );
            table
        };

        // 插值。
        let x = time.seconds_difference(&times[last_index]);
        let interpolation_result = if input_order == 0
            || !interpolation_algorithm.supports_derivatives()
        {
            interpolation_algorithm.interpolate_order_zero(
                x,
                &x_table,
                &y_table,
                packed_interpolation_length,
            )
        } else {
            let y_stride = packed_interpolation_length / (input_order + 1);
            interpolation_algorithm.interpolate(x, &x_table, &y_table, y_stride, input_order, input_order)
        };

        if !inner_type.uses_interpolation_conversion() {
            inner_type.unpack(&interpolation_result, 0)
        } else {
            inner_type.unpack_interpolation_result(
                &interpolation_result,
                values,
                first_index,
                last_index,
            )
        }
    }

    fn type_name(&self) -> &'static str {
        "SampledProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<SampledProperty>() {
            Some(o) => {
                self.property_type == o.property_type
                    && self.interpolation_degree == o.interpolation_degree
                    && self.interpolation_algorithm == o.interpolation_algorithm
                    && self.derivative_types == o.derivative_types
                    && self.times == o.times
                    && self.values == o.values
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// TimeIntervalCollectionProperty
// ---------------------------------------------------------------------------

fn value_same_data(a: &PropertyValue, b: &PropertyValue) -> bool {
    a == b
}

/// 由 `TimeIntervalCollection` 定义的属性，其中每个区间的数据
/// 表示该时间处的值。
///
/// 映射到 CesiumJS `DataSources/TimeIntervalCollectionProperty.js`。
#[derive(Debug, Clone)]
pub struct TimeIntervalCollectionProperty {
    intervals: TimeIntervalCollection<PropertyValue>,
}

impl Default for TimeIntervalCollectionProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeIntervalCollectionProperty {
    /// 创建空的区间集合属性。
    pub fn new() -> Self {
        Self {
            intervals: TimeIntervalCollection::new(),
        }
    }

    /// 底层的区间集合。映射到 `intervals`。
    pub fn intervals(&self) -> &TimeIntervalCollection<PropertyValue> {
        &self.intervals
    }

    /// 添加一个带给定值数据的区间。
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<PropertyValue>) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &value_same_data);
    }
}

impl DynProperty for TimeIntervalCollectionProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        match self.intervals.find_data_for_interval_containing_date(time) {
            Some(v) => v.clone(),
            None => PropertyValue::Undefined,
        }
    }

    fn type_name(&self) -> &'static str {
        "TimeIntervalCollectionProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other
            .as_any()
            .downcast_ref::<TimeIntervalCollectionProperty>()
        {
            Some(o) => self.intervals.equals(&o.intervals, &value_same_data),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CompositeProperty
// ---------------------------------------------------------------------------

fn property_same_data(a: &Arc<dyn DynProperty>, b: &Arc<dyn DynProperty>) -> bool {
    arc_property_equals(a, b)
}

/// 由 `TimeIntervalCollection` 定义的属性，其中每个区间的数据
/// 是另一个在所提供的时间处求值的 `Property`。
///
/// 映射到 CesiumJS `DataSources/CompositeProperty.js`。
#[derive(Clone)]
pub struct CompositeProperty {
    intervals: TimeIntervalCollection<Arc<dyn DynProperty>>,
}

impl Default for CompositeProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeProperty {
    /// 创建空的组合属性。
    pub fn new() -> Self {
        Self {
            intervals: TimeIntervalCollection::new(),
        }
    }

    /// 底层的区间集合。映射到 `intervals`。
    pub fn intervals(&self) -> &TimeIntervalCollection<Arc<dyn DynProperty>> {
        &self.intervals
    }

    /// 添加一个数据为另一个属性的区间。
    pub fn add_interval(&mut self, interval: TimeInterval, data: Option<Arc<dyn DynProperty>>) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &property_same_data);
    }
}

impl DynProperty for CompositeProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        match self.intervals.find_data_for_interval_containing_date(time) {
            Some(inner) => inner.get_value(time),
            None => PropertyValue::Undefined,
        }
    }

    fn type_name(&self) -> &'static str {
        "CompositeProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CompositeProperty>() {
            Some(o) => self.intervals.equals(&o.intervals, &property_same_data),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CallbackProperty
// ---------------------------------------------------------------------------

/// `CallbackProperty` 所使用的回调函数类型。
pub type CallbackFn = Arc<dyn Fn(&JulianDate) -> PropertyValue + Send + Sync>;

/// 其值由回调函数延迟求值的属性。
///
/// 映射到 CesiumJS `DataSources/CallbackProperty.js`。
#[derive(Clone)]
pub struct CallbackProperty {
    callback: CallbackFn,
    is_constant: bool,
}

impl CallbackProperty {
    /// 创建新的回调属性。
    /// 映射到 `new CallbackProperty(callback, isConstant)`。
    pub fn new<F>(callback: F, is_constant: bool) -> Self
    where
        F: Fn(&JulianDate) -> PropertyValue + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
            is_constant,
        }
    }

    /// 从共享回调创建回调属性。
    pub fn from_arc(callback: CallbackFn, is_constant: bool) -> Self {
        Self {
            callback,
            is_constant,
        }
    }

    /// 设置要使用的回调。
    /// 映射到 `CallbackProperty.prototype.setCallback`。
    pub fn set_callback<F>(&mut self, callback: F, is_constant: bool)
    where
        F: Fn(&JulianDate) -> PropertyValue + Send + Sync + 'static,
    {
        self.callback = Arc::new(callback);
        self.is_constant = is_constant;
    }
}

impl DynProperty for CallbackProperty {
    fn is_constant(&self) -> bool {
        self.is_constant
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        (self.callback)(time)
    }

    fn type_name(&self) -> &'static str {
        "CallbackProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<CallbackProperty>() {
            Some(o) => {
                Arc::ptr_eq(&self.callback, &o.callback)
                    && self.is_constant == o.is_constant
            }
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
    use glam::DVec3;

    fn jd(seconds: f64) -> JulianDate {
        JulianDate::new(2451545.0, seconds)
    }

    #[test]
    fn test_constant_property() {
        let p = ConstantProperty::new(PropertyValue::Number(42.0));
        assert!(p.is_constant());
        assert_eq!(p.get_value(&jd(0.0)), PropertyValue::Number(42.0));
        assert_eq!(p.type_name(), "ConstantProperty");

        let q = ConstantProperty::new(PropertyValue::Number(42.0));
        assert!(p.equals(&q));
        let r = ConstantProperty::new(PropertyValue::Number(7.0));
        assert!(!p.equals(&r));
    }

    #[test]
    fn test_sampled_property_linear_number() {
        let mut p = SampledProperty::new(PackableType::Number);
        assert!(p.is_constant()); // 尚无样本

        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        assert!(!p.is_constant());
        assert_eq!(p.sample_count(), 2);

        // 精确匹配。
        assert_eq!(p.get_value(&jd(0.0)), PropertyValue::Number(0.0));
        assert_eq!(p.get_value(&jd(10.0)), PropertyValue::Number(100.0));
        // 插值中点。
        assert_eq!(p.get_value(&jd(5.0)), PropertyValue::Number(50.0));
        // 四分之一处。
        assert_eq!(p.get_value(&jd(2.5)), PropertyValue::Number(25.0));
    }

    #[test]
    fn test_sampled_property_out_of_range_none() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        // 默认外推为 NONE。
        assert_eq!(p.get_value(&jd(-1.0)), PropertyValue::Undefined);
        assert_eq!(p.get_value(&jd(11.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_sampled_property_hold_extrapolation() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(5.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(15.0), &[]);
        p.set_backward_extrapolation_type(ExtrapolationType::Hold);
        p.set_forward_extrapolation_type(ExtrapolationType::Hold);
        assert_eq!(p.get_value(&jd(-5.0)), PropertyValue::Number(5.0));
        assert_eq!(p.get_value(&jd(20.0)), PropertyValue::Number(15.0));
    }

    #[test]
    fn test_sampled_property_extrapolate() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        p.set_forward_extrapolation_type(ExtrapolationType::Extrapolate);
        p.set_backward_extrapolation_type(ExtrapolationType::Extrapolate);
        assert_eq!(p.get_value(&jd(20.0)), PropertyValue::Number(200.0));
        assert_eq!(p.get_value(&jd(-5.0)), PropertyValue::Number(-50.0));
    }

    #[test]
    fn test_sampled_property_extrapolation_duration() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        p.set_forward_extrapolation_type(ExtrapolationType::Hold);
        p.set_forward_extrapolation_duration(5.0);
        // 在时长范围内：保持。
        assert_eq!(p.get_value(&jd(12.0)), PropertyValue::Number(100.0));
        // 超出时长：undefined。
        assert_eq!(p.get_value(&jd(20.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_sampled_property_cartesian3() {
        let mut p = SampledProperty::new(PackableType::Cartesian3);
        p.add_sample(
            jd(0.0),
            &PropertyValue::Cartesian3(DVec3::new(0.0, 0.0, 0.0)),
            &[],
        );
        p.add_sample(
            jd(10.0),
            &PropertyValue::Cartesian3(DVec3::new(10.0, 20.0, 30.0)),
            &[],
        );
        let mid = p.get_value(&jd(5.0));
        assert_eq!(
            mid,
            PropertyValue::Cartesian3(DVec3::new(5.0, 10.0, 15.0))
        );
    }

    #[test]
    fn test_sampled_property_quaternion_slerp_like() {
        use glam::DQuat;
        use std::f64::consts::FRAC_PI_2;
        let mut p = SampledProperty::new(PackableType::Quaternion);
        p.add_sample(
            jd(0.0),
            &PropertyValue::Quaternion(DQuat::IDENTITY),
            &[],
        );
        p.add_sample(
            jd(10.0),
            &PropertyValue::Quaternion(DQuat::from_rotation_z(FRAC_PI_2)),
            &[],
        );
        // 中点应为绕 Z 轴 45 度的旋转。
        let mid = p.get_value(&jd(5.0));
        if let PropertyValue::Quaternion(q) = mid {
            let expected = DQuat::from_rotation_z(FRAC_PI_2 / 2.0);
            let dot = q.dot(expected).abs();
            assert!((dot - 1.0).abs() < 1e-9, "dot = {dot}");
        } else {
            panic!("expected quaternion");
        }
    }

    #[test]
    fn test_sampled_property_out_of_order_insertion() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(5.0), &PropertyValue::Number(50.0), &[]);
        assert_eq!(p.sample_count(), 3);
        assert_eq!(p.times()[0], jd(0.0));
        assert_eq!(p.times()[1], jd(5.0));
        assert_eq!(p.times()[2], jd(10.0));
        assert_eq!(p.get_value(&jd(2.5)), PropertyValue::Number(25.0));
    }

    #[test]
    fn test_sampled_property_overwrite_existing() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        // 覆盖 t=10 处的样本。
        p.add_sample(jd(10.0), &PropertyValue::Number(200.0), &[]);
        assert_eq!(p.sample_count(), 2);
        assert_eq!(p.get_value(&jd(10.0)), PropertyValue::Number(200.0));
    }

    #[test]
    fn test_sampled_property_hermite_with_derivatives() {
        // f(t) = t^3 on [0, 1]: f(0)=0, f'(0)=0, f(1)=1, f'(1)=3.
        let mut p = SampledProperty::with_derivative_types(
            PackableType::Number,
            Some(vec![PackableType::Number]),
        );
        p.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(3));
        p.add_sample(
            jd(0.0),
            &PropertyValue::Number(0.0),
            &[PropertyValue::Number(0.0)],
        );
        p.add_sample(
            jd(1.0),
            &PropertyValue::Number(1.0),
            &[PropertyValue::Number(3.0)],
        );
        let v = p.get_value(&jd(0.5));
        assert_eq!(v, PropertyValue::Number(0.125));
    }

    #[test]
    fn test_sampled_property_remove_sample() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(5.0), &PropertyValue::Number(50.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        assert!(p.remove_sample(&jd(5.0)));
        assert_eq!(p.sample_count(), 2);
        assert!(!p.remove_sample(&jd(5.0)));
    }

    #[test]
    fn test_sampled_property_remove_samples_interval() {
        let mut p = SampledProperty::new(PackableType::Number);
        for i in 0..=10 {
            p.add_sample(jd(i as f64), &PropertyValue::Number(i as f64), &[]);
        }
        let interval = TimeInterval::new(jd(3.0), jd(7.0), true, true);
        p.remove_samples_interval(&interval);
        // 已移除 t=3,4,5,6,7 -> 剩余 6 个样本。
        assert_eq!(p.sample_count(), 6);
    }

    #[test]
    fn test_sampled_property_get_sample_negative_index() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        p.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        assert_eq!(p.get_sample(-1), Some(jd(10.0)));
        assert_eq!(p.get_sample(0), Some(jd(0.0)));
        assert_eq!(p.get_sample(5), None);
    }

    #[test]
    fn test_sampled_property_add_samples_packed_array() {
        let mut p = SampledProperty::new(PackableType::Number);
        let epoch = jd(0.0);
        // 每个样本：[time_offset, value]。
        let packed = [0.0, 0.0, 10.0, 100.0, 5.0, 50.0];
        p.add_samples_packed_array(&packed, &epoch);
        assert_eq!(p.sample_count(), 3);
        assert_eq!(p.get_value(&jd(2.5)), PropertyValue::Number(25.0));
    }

    #[test]
    fn test_sampled_property_equals() {
        let mut a = SampledProperty::new(PackableType::Number);
        a.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        let mut b = SampledProperty::new(PackableType::Number);
        b.add_sample(jd(0.0), &PropertyValue::Number(0.0), &[]);
        assert!(a.equals(&b));

        b.add_sample(jd(10.0), &PropertyValue::Number(100.0), &[]);
        assert!(!a.equals(&b));
    }

    #[test]
    fn test_sampled_property_single_sample() {
        let mut p = SampledProperty::new(PackableType::Number);
        p.add_sample(jd(5.0), &PropertyValue::Number(42.0), &[]);
        // 精确匹配生效。
        assert_eq!(p.get_value(&jd(5.0)), PropertyValue::Number(42.0));
        // 只有一个样本时无法插值。
        assert_eq!(p.get_value(&jd(6.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_time_interval_collection_property() {
        let mut p = TimeIntervalCollectionProperty::new();
        assert!(p.is_constant());

        p.add_interval(
            TimeInterval::new(jd(0.0), jd(10.0), true, false),
            Some(PropertyValue::Number(1.0)),
        );
        p.add_interval(
            TimeInterval::new(jd(10.0), jd(20.0), true, true),
            Some(PropertyValue::Number(2.0)),
        );
        assert!(!p.is_constant());

        assert_eq!(p.get_value(&jd(5.0)), PropertyValue::Number(1.0));
        assert_eq!(p.get_value(&jd(15.0)), PropertyValue::Number(2.0));
        assert_eq!(p.get_value(&jd(25.0)), PropertyValue::Undefined);

        let q = TimeIntervalCollectionProperty::new();
        assert!(!p.equals(&q));
    }

    #[test]
    fn test_composite_property() {
        let mut p = CompositeProperty::new();
        assert!(p.is_constant());

        let c1: Arc<dyn DynProperty> = Arc::new(ConstantProperty::new(PropertyValue::Number(1.0)));
        let mut sampled = SampledProperty::new(PackableType::Number);
        sampled.add_sample(jd(10.0), &PropertyValue::Number(10.0), &[]);
        sampled.add_sample(jd(20.0), &PropertyValue::Number(20.0), &[]);
        let c2: Arc<dyn DynProperty> = Arc::new(sampled);

        p.add_interval(
            TimeInterval::new(jd(0.0), jd(10.0), true, false),
            Some(c1),
        );
        p.add_interval(
            TimeInterval::new(jd(10.0), jd(20.0), true, true),
            Some(c2),
        );
        assert!(!p.is_constant());

        assert_eq!(p.get_value(&jd(5.0)), PropertyValue::Number(1.0));
        assert_eq!(p.get_value(&jd(15.0)), PropertyValue::Number(15.0));
        assert_eq!(p.get_value(&jd(25.0)), PropertyValue::Undefined);
    }

    #[test]
    fn test_callback_property() {
        let p = CallbackProperty::new(|t| PropertyValue::Number(t.day_number as f64), false);
        assert!(!p.is_constant());
        assert_eq!(p.get_value(&jd(7.0)), PropertyValue::Number(2451545.0));

        let q = CallbackProperty::new(|t| PropertyValue::Number(t.day_number as f64), false);
        // 不同的 closure -> 不相等。
        assert!(!p.equals(&q));

        // 相同的 Arc -> 相等。
        let shared: CallbackFn = Arc::new(|t| PropertyValue::Number(t.day_number as f64));
        let r = CallbackProperty::from_arc(Arc::clone(&shared), true);
        let s = CallbackProperty::from_arc(shared, true);
        assert!(r.equals(&s));
    }

    #[test]
    fn test_property_helpers() {
        let c = ConstantProperty::new(PropertyValue::Number(1.0));
        assert!(property_is_constant(None));
        assert!(property_is_constant(Some(&c)));
        assert_eq!(
            property_get_value_or_undefined(None, &jd(0.0)),
            PropertyValue::Undefined
        );
        assert_eq!(
            property_get_value_or_undefined(Some(&c), &jd(0.0)),
            PropertyValue::Number(1.0)
        );
    }

    #[test]
    fn test_cross_type_equals_false() {
        let c = ConstantProperty::new(PropertyValue::Number(1.0));
        let s = SampledProperty::new(PackableType::Number);
        assert!(!c.equals(&s));
        assert!(!s.equals(&c));
    }
}
