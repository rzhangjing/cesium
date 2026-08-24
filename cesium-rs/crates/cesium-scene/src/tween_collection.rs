//! Ported from `packages/engine/Source/Scene/TweenCollection.js`.
//!
//! M3/S3 materialization: the CesiumJS `TweenCollection` (the `Scene#tweens`
//! container) is ported one-to-one: `add` with
//! `{ startObject, stopObject, duration, delay, easingFunction, update,
//! complete, cancel }`, time-driven interpolation with easing, and the
//! `remove` / `cancel` / `removeAll` / `contains` / `length` / `get`
//! semantics. The JS wraps Tween.js; the port evaluates the easing
//! functions from `cesium_core::easing_function` directly.
//!
//! DEVIATION: CesiumJS tweens interpolate arbitrary JS objects key-by-key;
//! the Rust port models the tweened object as `Vec<(String, f64)>` pairs
//! (the camera-flight path uses a single `t` channel, mirroring the JS
//! `{ value: 0 }` → `{ value: 1 }` pattern).

use cesium_core::easing_function::linear_none;
use cesium_core::julian_date::JulianDate;

/// An easing curve `f: [0, 1] → [0, 1]` (see
/// [`cesium_core::easing_function`], the port of CesiumJS
/// `EasingFunction`).
pub type EasingFn = fn(f64) -> f64;

/// The options of [`TweenCollection::add`], mirroring the CesiumJS
/// `TweenCollection#add` options object.
pub struct TweenOptions {
    /// The start values of the tweened object (JS `startObject`).
    pub start_object: Vec<(String, f64)>,
    /// The end values of the tweened object (JS `stopObject`).
    pub stop_object: Vec<(String, f64)>,
    /// The duration in seconds.
    pub duration: f64,
    /// The delay in seconds before the tween starts (JS `delay`).
    pub delay: f64,
    /// The easing function (defaults to `EasingFunction.LINEAR_NONE`).
    pub easing_function: EasingFn,
    /// Called every frame with the interpolated object (JS `update`).
    pub update: Option<Box<dyn FnMut(&[(String, f64)])>>,
    /// Called once when the tween completes (JS `complete`).
    pub complete: Option<Box<dyn FnOnce()>>,
    /// Called when the tween is canceled (JS `cancel`).
    pub cancel: Option<Box<dyn FnOnce()>>,
}

impl TweenOptions {
    /// Creates options with the JS defaults (no delay, linear easing, no
    /// callbacks).
    pub fn new(start_object: Vec<(String, f64)>, stop_object: Vec<(String, f64)>, duration: f64) -> Self {
        Self {
            start_object,
            stop_object,
            duration,
            delay: 0.0,
            easing_function: linear_none,
            update: None,
            complete: None,
            cancel: None,
        }
    }
}

/// One running tween (mirrors the JS `Tween` returned by
/// `TweenCollection#add`).
struct Tween {
    id: u64,
    start_object: Vec<(String, f64)>,
    stop_object: Vec<(String, f64)>,
    duration: f64,
    delay: f64,
    easing_function: EasingFn,
    update: Option<Box<dyn FnMut(&[(String, f64)])>>,
    complete: Option<Box<dyn FnOnce()>>,
    cancel: Option<Box<dyn FnOnce()>>,
    /// The time the tween started (first `update` after the delay), mirroring
    /// the JS `_startTime`.
    start_time: Option<JulianDate>,
    /// Set by [`TweenCollection::cancel`]; the cancel callback fires on the
    /// next `update` (mirrors the JS `tween.cancel()` → cancel callback).
    canceled: bool,
    /// Scratch buffer reused across frames for the interpolated object.
    values: Vec<(String, f64)>,
}

/// A collection of tween animations (mirrors CesiumJS `TweenCollection`,
/// exposed as `Scene#tweens`).
pub struct TweenCollection {
    tweens: Vec<Tween>,
    next_id: u64,
}

impl TweenCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self { tweens: Vec::new(), next_id: 1 }
    }

    /// Adds a tween and returns its id (CesiumJS returns the tween object;
    /// the port returns the id that `remove`/`cancel`/`contains` target).
    ///
    /// Mirrors CesiumJS `TweenCollection#add`.
    pub fn add(&mut self, options: TweenOptions) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let values = options
            .start_object
            .iter()
            .map(|(key, _)| (key.clone(), 0.0))
            .collect();
        self.tweens.push(Tween {
            id,
            start_object: options.start_object,
            stop_object: options.stop_object,
            duration: options.duration.max(0.0),
            delay: options.delay.max(0.0),
            easing_function: options.easing_function,
            update: options.update,
            complete: options.complete,
            cancel: options.cancel,
            start_time: None,
            canceled: false,
            values,
        });
        id
    }

    /// Returns the number of tweens (mirrors the JS `length` property).
    pub fn len(&self) -> usize {
        self.tweens.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.tweens.is_empty()
    }

    /// Returns whether the collection contains the tween id (mirrors the JS
    /// `contains(tween)`).
    pub fn contains(&self, id: u64) -> bool {
        self.tweens.iter().any(|tween| tween.id == id)
    }

    /// Removes the tween without invoking its cancel callback (mirrors the
    /// JS `remove(tween)` boolean contract).
    pub fn remove(&mut self, id: u64) -> bool {
        if let Some(index) = self.tweens.iter().position(|tween| tween.id == id) {
            self.tweens.remove(index);
            true
        } else {
            false
        }
    }

    /// Cancels the tween: its cancel callback fires on the next `update`
    /// (mirrors the JS `tween.cancel()`).
    pub fn cancel(&mut self, id: u64) {
        if let Some(tween) = self.tweens.iter_mut().find(|tween| tween.id == id) {
            tween.canceled = true;
        }
    }

    /// Removes all tweens (mirrors the JS `removeAll`; does NOT invoke the
    /// cancel callbacks, matching the JS).
    pub fn remove_all(&mut self) {
        self.tweens.clear();
    }

    /// Advances every tween to `time` (mirrors the JS
    /// `TweenCollection#update`); returns whether any tween was updated.
    pub fn update(&mut self, time: &JulianDate) -> bool {
        let mut any_updated = false;
        let mut index = 0usize;
        while index < self.tweens.len() {
            let finished = {
                let tween = &mut self.tweens[index];

                if tween.canceled {
                    if let Some(cancel) = tween.cancel.take() {
                        cancel();
                    }
                    true
                } else {
                    let start_time = tween.start_time.get_or_insert_with(|| time.clone()).clone();
                    let delta = JulianDate::seconds_difference(time, &start_time) - tween.delay;
                    if delta < 0.0 {
                        index += 1;
                        continue;
                    }
                    let ratio = if tween.duration > 0.0 {
                        (delta / tween.duration).min(1.0)
                    } else {
                        1.0
                    };
                    let eased = (tween.easing_function)(ratio);

                    // Interpolate key-by-key (JS Tween.js behavior on the
                    // start/stop objects).
                    for (slot, ((_, start), (_, stop))) in tween
                        .values
                        .iter_mut()
                        .zip(tween.start_object.iter().zip(tween.stop_object.iter()))
                    {
                        slot.1 = start + (stop - start) * eased;
                    }
                    if let Some(update) = tween.update.as_mut() {
                        update(&tween.values);
                    }
                    any_updated = true;
                    ratio >= 1.0
                }
            };

            if finished {
                let mut tween = self.tweens.remove(index);
                if !tween.canceled {
                    if let Some(complete) = tween.complete.take() {
                        complete();
                    }
                }
            } else {
                index += 1;
            }
        }
        any_updated
    }
}

impl Default for TweenCollection {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Mirrors TweenCollectionSpec: "adds a tween".
    #[test]
    fn adds_a_tween() {
        let mut tweens = TweenCollection::new();
        let id = tweens.add(TweenOptions::new(
            vec![("value".to_string(), 0.0)],
            vec![("value".to_string(), 1.0)],
            1.0,
        ));
        assert_eq!(tweens.len(), 1);
        assert!(tweens.contains(id));
    }

    /// Mirrors TweenCollectionSpec: "removes a tween" (boolean contract).
    #[test]
    fn removes_a_tween() {
        let mut tweens = TweenCollection::new();
        let id = tweens.add(TweenOptions::new(vec![], vec![], 1.0));
        assert!(tweens.remove(id));
        assert!(!tweens.remove(id));
        assert!(tweens.is_empty());
    }

    /// Mirrors TweenCollectionSpec: "removes all tweens".
    #[test]
    fn removes_all_tweens() {
        let mut tweens = TweenCollection::new();
        tweens.add(TweenOptions::new(vec![], vec![], 1.0));
        tweens.add(TweenOptions::new(vec![], vec![], 1.0));
        tweens.remove_all();
        assert!(tweens.is_empty());
    }

    /// Mirrors TweenCollectionSpec update semantics: the update callback
    /// receives interpolated values, the complete callback fires once at
    /// the end, and the tween is removed afterwards.
    #[test]
    fn update_interpolates_and_completes() {
        let mut tweens = TweenCollection::new();
        let last_value = Rc::new(Cell::new(f64::NAN));
        let completed = Rc::new(Cell::new(false));
        {
            let last_value = last_value.clone();
            let completed = completed.clone();
            tweens.add(TweenOptions {
                update: Some(Box::new(move |values| {
                    last_value.set(values[0].1);
                })),
                complete: Some(Box::new(move || completed.set(true))),
                ..TweenOptions::new(
                    vec![("value".to_string(), 0.0)],
                    vec![("value".to_string(), 10.0)],
                    2.0,
                )
            });
        }

        let start = JulianDate::now();
        assert!(tweens.update(&start));
        assert_eq!(last_value.get(), 0.0);

        assert!(tweens.update(&JulianDate::add_seconds_new(&start, 1.0)));
        assert_eq!(last_value.get(), 5.0);

        assert!(tweens.update(&JulianDate::add_seconds_new(&start, 2.0)));
        assert_eq!(last_value.get(), 10.0);
        assert!(completed.get());
        assert!(tweens.is_empty());
    }

    /// Mirrors TweenCollectionSpec: easing functions shape the curve.
    #[test]
    fn easing_function_shapes_the_curve() {
        let mut tweens = TweenCollection::new();
        let last_value = Rc::new(Cell::new(f64::NAN));
        {
            let last_value = last_value.clone();
            tweens.add(TweenOptions {
                easing_function: cesium_core::easing_function::quadratic_in,
                update: Some(Box::new(move |values| last_value.set(values[0].1))),
                ..TweenOptions::new(
                    vec![("value".to_string(), 0.0)],
                    vec![("value".to_string(), 1.0)],
                    1.0,
                )
            });
        }
        let start = JulianDate::now();
        tweens.update(&start);
        tweens.update(&JulianDate::add_seconds_new(&start, 0.5));
        // quadratic_in(0.5) = 0.25
        assert!((last_value.get() - 0.25).abs() < 1e-12);
    }

    /// Mirrors TweenCollectionSpec: the delay postpones the first update.
    #[test]
    fn delay_postpones_updates() {
        let mut tweens = TweenCollection::new();
        let updates = Rc::new(Cell::new(0));
        {
            let updates = updates.clone();
            tweens.add(TweenOptions {
                delay: 1.0,
                update: Some(Box::new(move |_| updates.set(updates.get() + 1))),
                ..TweenOptions::new(vec![("v".to_string(), 0.0)], vec![("v".to_string(), 1.0)], 1.0)
            });
        }
        let start = JulianDate::now();
        tweens.update(&start);
        tweens.update(&JulianDate::add_seconds_new(&start, 0.5));
        assert_eq!(updates.get(), 0);
        tweens.update(&JulianDate::add_seconds_new(&start, 1.5));
        assert_eq!(updates.get(), 1);
    }

    /// Mirrors TweenCollectionSpec: canceling a tween invokes the cancel
    /// callback on the next update and removes it (complete never fires).
    #[test]
    fn cancel_invokes_cancel_callback() {
        let mut tweens = TweenCollection::new();
        let canceled = Rc::new(Cell::new(false));
        let completed = Rc::new(Cell::new(false));
        let id;
        {
            let canceled = canceled.clone();
            let completed = completed.clone();
            id = tweens.add(TweenOptions {
                cancel: Some(Box::new(move || canceled.set(true))),
                complete: Some(Box::new(move || completed.set(true))),
                ..TweenOptions::new(vec![], vec![], 1.0)
            });
        }
        tweens.cancel(id);
        tweens.update(&JulianDate::now());
        assert!(canceled.get());
        assert!(!completed.get());
        assert!(tweens.is_empty());
    }

    /// A zero-duration tween completes on the first update (JS treats it as
    /// an instant jump to the stop object).
    #[test]
    fn zero_duration_completes_immediately() {
        let mut tweens = TweenCollection::new();
        let last_value = Rc::new(Cell::new(f64::NAN));
        {
            let last_value = last_value.clone();
            tweens.add(TweenOptions {
                update: Some(Box::new(move |values| last_value.set(values[0].1))),
                ..TweenOptions::new(
                    vec![("value".to_string(), 0.0)],
                    vec![("value".to_string(), 7.0)],
                    0.0,
                )
            });
        }
        tweens.update(&JulianDate::now());
        assert_eq!(last_value.get(), 7.0);
        assert!(tweens.is_empty());
    }
}
