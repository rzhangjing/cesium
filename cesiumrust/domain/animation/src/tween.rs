//! TweenCollection - property animation via easing functions.
//!
//! Maps to CesiumJS `Scene/TweenCollection.js` + `Core/EasingFunction.js`

use std::collections::HashMap;

/// Easing functions for tween animations.
/// Maps to CesiumJS `Core/EasingFunction.js` (28 variants from tween.js)
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
    /// Evaluates the easing function at time k (0..1).
    /// Formulas from tween.js (Robert Penner / sole).
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

/// A single tween animation.
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

    /// Computes interpolated values at the given elapsed time (seconds since tween start).
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

/// Options for adding a tween.
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

/// A collection of tween animations.
/// Maps to CesiumJS `Scene/TweenCollection.js`
pub struct TweenCollection {
    tweens: Vec<Tween>,
}

impl TweenCollection {
    pub fn new() -> Self {
        Self { tweens: Vec::new() }
    }

    pub fn len(&self) -> usize { self.tweens.len() }
    pub fn is_empty(&self) -> bool { self.tweens.is_empty() }

    /// Adds a tween. If duration == 0, immediately calls complete and does not add.
    /// Returns the index of the added tween, or None if duration was 0.
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

    /// Adds a tween that animates a single scalar property.
    /// Maps to CesiumJS `TweenCollection.addProperty`.
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

    /// Adds a tween that animates alpha on color uniforms.
    /// Maps to CesiumJS `TweenCollection.addAlpha`.
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

    /// Adds a tween that increments an offset uniform.
    /// Maps to CesiumJS `TweenCollection.addOffsetIncrement`.
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

    /// Removes a tween by index, calling its cancel callback.
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

    /// Removes a tween found by searching for a matching tween.
    /// Returns true if found and removed.
    pub fn remove_tween(&mut self, tween_ptr: *const Tween) -> bool {
        if let Some(pos) = self.tweens.iter().position(|t| t as *const Tween == tween_ptr) {
            self.remove(pos)
        } else {
            false
        }
    }

    /// Removes all tweens, calling cancel on each.
    pub fn remove_all(&mut self) {
        for tween in self.tweens.drain(..) {
            let mut tween = tween;
            if let Some(ref mut cancel) = tween.cancel_callback {
                cancel();
            }
        }
    }

    /// Returns true if the collection contains a tween at the given index.
    pub fn contains(&self, index: usize) -> bool {
        index < self.tweens.len()
    }

    /// Gets a reference to a tween by index.
    pub fn get(&self, index: usize) -> Option<&Tween> {
        self.tweens.get(index)
    }

    /// Cancels a tween (removes it and calls cancel callback).
    pub fn cancel_tween(&mut self, index: usize) -> bool {
        self.remove(index)
    }

    /// Updates all tweens to the given time (seconds).
    /// Tweens that complete are removed from the collection.
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
                // Tween completed
                let repeat = self.tweens[i].repeat;
                let repeat_count = self.tweens[i].repeat_count;

                if repeat_count < repeat {
                    // Restart
                    self.tweens[i].repeat_count += 1.0;
                    self.tweens[i].start_time = Some(time);
                    i += 1;
                } else {
                    // Complete and remove
                    let mut tween = self.tweens.swap_remove(i);
                    if let Some(ref mut complete_cb) = tween.complete_callback {
                        complete_cb();
                    }
                    // Don't increment i since swap_remove moved an element
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
