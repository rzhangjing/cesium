//! TweenCollection - 通过缓动函数实现属性动画。
//!
//! 映射到 CesiumJS `Scene/TweenCollection.js` + `Core/EasingFunction.js`

// 遗留的 CesiumJS 移植风格债（deferred.md #18）；在 M13 lint 清理、或本文件在其里程碑被重写时重新审视
#![allow(clippy::derivable_impls, clippy::type_complexity, clippy::too_many_arguments, clippy::ptr_eq)]
use std::collections::HashMap;

/// 用于补间（tween）动画的缓动函数。
/// 映射到 CesiumJS `Core/EasingFunction.js`（来自 tween.js 的 28 种变体）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingFunction {
    LinearNone,
    QuadraticIn,
    QuadraticOut,
    QuadraticInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuarticIn,
    QuarticOut,
    QuarticInOut,
    QuinticIn,
    QuinticOut,
    QuinticInOut,
    SinusoidalIn,
    SinusoidalOut,
    SinusoidalInOut,
    ExponentialIn,
    ExponentialOut,
    ExponentialInOut,
    CircularIn,
    CircularOut,
    CircularInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BackIn,
    BackOut,
    BackInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
}

impl Default for EasingFunction {
    fn default() -> Self {
        Self::LinearNone
    }
}

impl EasingFunction {
    /// 在时间 k（0..1）处求缓动函数的值。
    /// 公式来自 tween.js（Robert Penner / sole）。
    pub fn evaluate(&self, k: f64) -> f64 {
        use std::f64::consts::PI;
        match self {
            Self::LinearNone => k,
            Self::QuadraticIn => k * k,
            Self::QuadraticOut => k * (2.0 - k),
            Self::QuadraticInOut => {
                if k < 0.5 { 2.0 * k * k } else { -1.0 + (4.0 - 2.0 * k) * k }
            }
            Self::CubicIn => k * k * k,
            Self::CubicOut => { let f = k - 1.0; f * f * f + 1.0 }
            Self::CubicInOut => {
                if k < 0.5 { 4.0 * k * k * k } else { let f = 2.0 * k - 2.0; 0.5 * f * f * f + 1.0 }
            }
            Self::QuarticIn => k * k * k * k,
            Self::QuarticOut => { let f = k - 1.0; 1.0 - f * f * f * f }
            Self::QuarticInOut => {
                if k < 0.5 { 8.0 * k * k * k * k } else { let f = k - 1.0; 1.0 - 8.0 * f * f * f * f }
            }
            Self::QuinticIn => k * k * k * k * k,
            Self::QuinticOut => { let f = k - 1.0; f * f * f * f * f + 1.0 }
            Self::QuinticInOut => {
                if k < 0.5 { 16.0 * k * k * k * k * k } else { let f = 2.0 * k - 2.0; 0.5 * f * f * f * f * f + 1.0 }
            }
            Self::SinusoidalIn => 1.0 - (k * PI / 2.0).cos(),
            Self::SinusoidalOut => (k * PI / 2.0).sin(),
            Self::SinusoidalInOut => 0.5 * (1.0 - (PI * k).cos()),
            Self::ExponentialIn => {
                if k == 0.0 { 0.0 } else { 2.0_f64.powf(10.0 * (k - 1.0)) }
            }
            Self::ExponentialOut => {
                if k == 1.0 { 1.0 } else { 1.0 - 2.0_f64.powf(-10.0 * k) }
            }
            Self::ExponentialInOut => {
                if k == 0.0 { return 0.0; }
                if k == 1.0 { return 1.0; }
                if k < 0.5 { 0.5 * 2.0_f64.powf(20.0 * k - 10.0) }
                else { 1.0 - 0.5 * 2.0_f64.powf(-20.0 * k + 10.0) }
            }
            Self::CircularIn => 1.0 - (1.0 - k * k).sqrt(),
            Self::CircularOut => (1.0 - (k - 1.0) * (k - 1.0)).sqrt(),
            Self::CircularInOut => {
                if k < 0.5 { 0.5 * (1.0 - (1.0 - 4.0 * k * k).sqrt()) }
                else { 0.5 * ((1.0 - (-2.0 * k + 2.0) * (-2.0 * k + 2.0)).sqrt() + 1.0) }
            }
            Self::ElasticIn => {
                if k == 0.0 { return 0.0; }
                if k == 1.0 { return 1.0; }
                -(2.0_f64.powf(10.0 * k - 10.0) * ((k * 10.0 - 10.75) * (2.0 * PI / 3.0)).sin())
            }
            Self::ElasticOut => {
                if k == 0.0 { return 0.0; }
                if k == 1.0 { return 1.0; }
                2.0_f64.powf(-10.0 * k) * ((k * 10.0 - 0.75) * (2.0 * PI / 3.0)).sin() + 1.0
            }
            Self::ElasticInOut => {
                if k == 0.0 { return 0.0; }
                if k == 1.0 { return 1.0; }
                let c5 = (2.0 * PI) / 4.5;
                if k < 0.5 {
                    -(2.0_f64.powf(20.0 * k - 10.0) * ((20.0 * k - 11.125) * c5).sin()) / 2.0
                } else {
                    (2.0_f64.powf(-20.0 * k + 10.0) * ((20.0 * k - 11.125) * c5).sin()) / 2.0 + 1.0
                }
            }
            Self::BackIn => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * k * k * k - c1 * k * k
            }
            Self::BackOut => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let f = k - 1.0;
                1.0 + c3 * f * f * f + c1 * f * f
            }
            Self::BackInOut => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if k < 0.5 {
                    ((2.0 * k) * (2.0 * k) * ((c2 + 1.0) * 2.0 * k - c2)) / 2.0
                } else {
                    ((2.0 * k - 2.0) * (2.0 * k - 2.0) * ((c2 + 1.0) * (k * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            Self::BounceIn => 1.0 - Self::bounce_out(1.0 - k),
            Self::BounceOut => Self::bounce_out(k),
            Self::BounceInOut => {
                if k < 0.5 { (1.0 - Self::bounce_out(1.0 - 2.0 * k)) / 2.0 }
                else { (1.0 + Self::bounce_out(2.0 * k - 1.0)) / 2.0 }
            }
        }
    }

    fn bounce_out(k: f64) -> f64 {
        let n1 = 7.5625;
        let d1 = 2.75;
        if k < 1.0 / d1 {
            n1 * k * k
        } else if k < 2.0 / d1 {
            let f = k - 1.5 / d1;
            n1 * f * f + 0.75
        } else if k < 2.5 / d1 {
            let f = k - 2.25 / d1;
            n1 * f * f + 0.9375
        } else {
            let f = k - 2.625 / d1;
            n1 * f * f + 0.984375
        }
    }
}

/// 单个补间动画。
pub struct Tween {
    start_object: HashMap<String, f64>,
    stop_object: HashMap<String, f64>,
    duration: f64,
    delay: f64,
    easing_function: EasingFunction,
    update_callback: Option<Box<dyn FnMut(&HashMap<String, f64>)>>,
    complete_callback: Option<Box<dyn FnMut()>>,
    cancel_callback: Option<Box<dyn FnMut()>>,
    start_time: Option<f64>,
    repeat: f64,
    repeat_count: f64,
}

impl Tween {
    pub fn start_object(&self) -> &HashMap<String, f64> { &self.start_object }
    pub fn stop_object(&self) -> &HashMap<String, f64> { &self.stop_object }
    pub fn duration(&self) -> f64 { self.duration }
    pub fn delay(&self) -> f64 { self.delay }
    pub fn easing_function(&self) -> EasingFunction { self.easing_function }

    /// 计算在给定经过时间（自补间开始的秒数）处的插值。
    fn compute_values(&self, elapsed: f64) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        let t = if self.duration > 0.0 {
            (elapsed / self.duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let eased = self.easing_function.evaluate(t);
        for (key, &start_val) in &self.start_object {
            let stop_val = self.stop_object.get(key).copied().unwrap_or(start_val);
            result.insert(key.clone(), start_val + (stop_val - start_val) * eased);
        }
        result
    }
}

/// 添加补间的选项。
pub struct TweenOptions {
    pub start_object: HashMap<String, f64>,
    pub stop_object: HashMap<String, f64>,
    pub duration: f64,
    pub delay: f64,
    pub easing_function: EasingFunction,
    pub update: Option<Box<dyn FnMut(&HashMap<String, f64>)>>,
    pub complete: Option<Box<dyn FnMut()>>,
    pub cancel: Option<Box<dyn FnMut()>>,
    pub repeat: f64,
}

impl TweenOptions {
    pub fn new(start_object: HashMap<String, f64>, stop_object: HashMap<String, f64>, duration: f64) -> Self {
        Self {
            start_object,
            stop_object,
            duration,
            delay: 0.0,
            easing_function: EasingFunction::LinearNone,
            update: None,
            complete: None,
            cancel: None,
            repeat: 0.0,
        }
    }
}

/// 一组补间动画。
/// 映射到 CesiumJS `Scene/TweenCollection.js`
pub struct TweenCollection {
    tweens: Vec<Tween>,
}

impl TweenCollection {
    pub fn new() -> Self {
        Self { tweens: Vec::new() }
    }

    pub fn len(&self) -> usize { self.tweens.len() }
    pub fn is_empty(&self) -> bool { self.tweens.is_empty() }

    /// 添加一个补间。若 duration == 0，则立即调用 complete 并不添加。
    /// 返回已添加补间的索引，若 duration 为 0 则返回 None。
    pub fn add(&mut self, mut options: TweenOptions) -> Option<usize> {
        if options.duration == 0.0 {
            if let Some(ref mut complete) = options.complete {
                complete();
            }
            return None;
        }

        let tween = Tween {
            start_object: options.start_object,
            stop_object: options.stop_object,
            duration: options.duration,
            delay: options.delay,
            easing_function: options.easing_function,
            update_callback: options.update,
            complete_callback: options.complete,
            cancel_callback: options.cancel,
            start_time: None,
            repeat: options.repeat,
            repeat_count: 0.0,
        };
        self.tweens.push(tween);
        Some(self.tweens.len() - 1)
    }

    /// 添加一个对单个标量属性做动画的补间。
    /// 映射到 CesiumJS `TweenCollection.addProperty`。
    pub fn add_property(
        &mut self,
        start_value: f64,
        stop_value: f64,
        duration: f64,
        delay: f64,
        easing_function: EasingFunction,
        object: std::rc::Rc<std::cell::RefCell<HashMap<String, f64>>>,
        property: String,
    ) -> Option<usize> {
        let obj = object.clone();
        let prop = property.clone();
        let update = move |values: &HashMap<String, f64>| {
            if let Some(&v) = values.get("value") {
                obj.borrow_mut().insert(prop.clone(), v);
            }
        };
        let mut start = HashMap::new();
        start.insert("value".to_string(), start_value);
        let mut stop = HashMap::new();
        stop.insert("value".to_string(), stop_value);

        let options = TweenOptions {
            start_object: start,
            stop_object: stop,
            duration,
            delay,
            easing_function,
            update: Some(Box::new(update)),
            complete: None,
            cancel: None,
            repeat: 0.0,
        };
        self.add(options)
    }

    /// 添加一个对颜色 uniform 的 alpha 做动画的补间。
    /// 映射到 CesiumJS `TweenCollection.addAlpha`。
    pub fn add_alpha(
        &mut self,
        duration: f64,
        start_value: f64,
        stop_value: f64,
        uniforms: std::rc::Rc<std::cell::RefCell<HashMap<String, f64>>>,
        color_keys: Vec<String>,
    ) -> Option<usize> {
        let u = uniforms.clone();
        let keys = color_keys.clone();
        let update = move |values: &HashMap<String, f64>| {
            if let Some(&alpha) = values.get("alpha") {
                let mut map = u.borrow_mut();
                for key in &keys {
                    map.insert(format!("{}.alpha", key), alpha);
                }
            }
        };
        let mut start = HashMap::new();
        start.insert("alpha".to_string(), start_value);
        let mut stop = HashMap::new();
        stop.insert("alpha".to_string(), stop_value);

        let options = TweenOptions {
            start_object: start,
            stop_object: stop,
            duration,
            delay: 0.0,
            easing_function: EasingFunction::LinearNone,
            update: Some(Box::new(update)),
            complete: None,
            cancel: None,
            repeat: 0.0,
        };
        self.add(options)
    }

    /// 添加一个递增 offset uniform 的补间。
    /// 映射到 CesiumJS `TweenCollection.addOffsetIncrement`。
    pub fn add_offset_increment(
        &mut self,
        duration: f64,
        uniforms: std::rc::Rc<std::cell::RefCell<HashMap<String, f64>>>,
    ) -> Option<usize> {
        let current = uniforms.borrow().get("offset").copied().unwrap_or(0.0);
        let u = uniforms.clone();
        let update = move |values: &HashMap<String, f64>| {
            if let Some(&v) = values.get("value") {
                u.borrow_mut().insert("offset".to_string(), v);
            }
        };
        let mut start = HashMap::new();
        start.insert("value".to_string(), current);
        let mut stop = HashMap::new();
        stop.insert("value".to_string(), current + 1.0);

        let options = TweenOptions {
            start_object: start,
            stop_object: stop,
            duration,
            delay: 0.0,
            easing_function: EasingFunction::LinearNone,
            update: Some(Box::new(update)),
            complete: None,
            cancel: None,
            repeat: f64::INFINITY,
        };
        self.add(options)
    }

    /// 按索引移除一个补间，并调用其 cancel 回调。
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.tweens.len() {
            return false;
        }
        let mut tween = self.tweens.swap_remove(index);
        if let Some(ref mut cancel) = tween.cancel_callback {
            cancel();
        }
        true
    }

    /// 通过搜索匹配的补间来移除一个补间。
    /// 若找到并移除则返回 true。
    pub fn remove_tween(&mut self, tween_ptr: *const Tween) -> bool {
        if let Some(pos) = self.tweens.iter().position(|t| t as *const Tween == tween_ptr) {
            self.remove(pos)
        } else {
            false
        }
    }

    /// 移除所有补间，对每个调用 cancel。
    pub fn remove_all(&mut self) {
        for tween in self.tweens.drain(..) {
            let mut tween = tween;
            if let Some(ref mut cancel) = tween.cancel_callback {
                cancel();
            }
        }
    }

    /// 若集合在给定索引处包含补间则返回 true。
    pub fn contains(&self, index: usize) -> bool {
        index < self.tweens.len()
    }

    /// 按索引获取对某个补间的引用。
    pub fn get(&self, index: usize) -> Option<&Tween> {
        self.tweens.get(index)
    }

    /// 取消一个补间（移除它并调用 cancel 回调）。
    pub fn cancel_tween(&mut self, index: usize) -> bool {
        self.remove(index)
    }

    /// 将所有补间更新到给定时间（秒）。
    /// 已完成的补间会从集合中移除。
    pub fn update(&mut self, time: f64) {
        let mut i = 0;
        while i < self.tweens.len() {
            let start_time = self.tweens[i].start_time.get_or_insert(time);
            let elapsed = time - *start_time - self.tweens[i].delay;

            if elapsed < 0.0 {
                i += 1;
                continue;
            }

            let duration = self.tweens[i].duration;
            let values = self.tweens[i].compute_values(elapsed);

            if let Some(ref mut update_cb) = self.tweens[i].update_callback {
                update_cb(&values);
            }

            if elapsed >= duration {
                // 补间已完成
                let repeat = self.tweens[i].repeat;
                let repeat_count = self.tweens[i].repeat_count;

                if repeat_count < repeat {
                    // 重新开始
                    self.tweens[i].repeat_count += 1.0;
                    self.tweens[i].start_time = Some(time);
                    i += 1;
                } else {
                    // 完成并移除
                    let mut tween = self.tweens.swap_remove(i);
                    if let Some(ref mut complete_cb) = tween.complete_callback {
                        complete_cb();
                    }
                    // 不递增 i，因为 swap_remove 已移来一个元素
                }
            } else {
                i += 1;
            }
        }
    }
}

impl Default for TweenCollection {
    fn default() -> Self {
        Self::new()
    }
}
