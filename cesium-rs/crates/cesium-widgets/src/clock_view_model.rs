//! Ported from `packages/widgets/Source/ClockViewModel.js`.
//!
//! A view model which exposes a [`Clock`] for user interfaces.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::clock::Clock;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::event::RemoveCallback;
use cesium_core::julian_date::JulianDate;

/// The cached observable values plus bookkeeping, shared between all clones
/// of a [`ClockViewModel`] (the Rust analogue of the JS object identity
/// shared between `AnimationViewModel` and its command closures).
struct ClockViewModelInner {
    /// Set by the `onTick` listener; consumed by the lazy synchronization.
    /// Shared via `Rc` with the listener closure.
    needs_sync: Rc<Cell<bool>>,
    // Cached observable values (mirrors the knockout observables).
    system_time: JulianDate,
    start_time: JulianDate,
    stop_time: JulianDate,
    current_time: JulianDate,
    multiplier: f64,
    clock_step: ClockStep,
    clock_range: ClockRange,
    can_animate: bool,
    should_animate: bool,
    destroyed: bool,
}

/// A view model which exposes a [`Clock`] for user interfaces.
///
/// Mirrors CesiumJS `ClockViewModel`: the observable properties
/// (`systemTime`, `startTime`, `stopTime`, `currentTime`, `multiplier`,
/// `clockStep`, `clockRange`, `canAnimate`, `shouldAnimate`) are cached
/// copies of the underlying clock that are refreshed by
/// [`ClockViewModel::synchronize`]. In CesiumJS `synchronize` is invoked by
/// the clock's `onTick` event; because the Rust [`cesium_core::event::Event`]
/// listeners run while the clock is mutably borrowed (and may not re-borrow
/// it), the listener instead raises a dirty flag and the getters perform the
/// synchronization lazily on the next read — observable behaviour is
/// identical: values only change once `Clock::tick` has completed.
///
/// Clones share the underlying state (mirroring the JS reference passed to
/// `AnimationViewModel`'s command closures), so a write through any clone is
/// immediately visible through every other clone.
///
/// DEVIATION: knockout observables (with `JulianDate.equals` equality
/// comparers and write-back subscriptions) are modeled as cached fields
/// with explicit write-through setters; see docs/deviations.md.
#[derive(Clone)]
pub struct ClockViewModel {
    clock: Rc<RefCell<Clock>>,
    /// Removal token for the `onTick` subscription (mirrors
    /// `EventHelper.add` + `EventHelper.removeAll` in `destroy`). Shared so
    /// the subscription is removed exactly once, whichever clone is
    /// destroyed first.
    on_tick_removal: Rc<RefCell<Option<RemoveCallback<()>>>>,
    inner: Rc<RefCell<ClockViewModelInner>>,
}

impl ClockViewModel {
    /// Port of `new ClockViewModel(clock)`; passing `None` creates a new
    /// [`Clock`], mirroring the JS `if (!defined(clock)) clock = new Clock();`.
    pub fn new(clock: Option<Rc<RefCell<Clock>>>) -> Self {
        let clock = clock.unwrap_or_else(|| {
            Rc::new(RefCell::new(Clock::new(
                None, None, None, None, None, None, None, None,
            )))
        });

        let needs_sync = Rc::new(Cell::new(false));
        let listener_flag = needs_sync.clone();
        let on_tick_removal = clock
            .borrow()
            .on_tick
            .add_listener(move |_| listener_flag.set(true));

        let view_model = Self {
            clock,
            on_tick_removal: Rc::new(RefCell::new(Some(on_tick_removal))),
            inner: Rc::new(RefCell::new(ClockViewModelInner {
                needs_sync,
                system_time: JulianDate::now(),
                start_time: JulianDate::default_date(),
                stop_time: JulianDate::default_date(),
                current_time: JulianDate::default_date(),
                multiplier: 1.0,
                clock_step: ClockStep::SystemClockMultiplier,
                clock_range: ClockRange::Unbounded,
                can_animate: true,
                should_animate: false,
                destroyed: false,
            })),
        };
        view_model.synchronize();
        view_model
    }

    /// Gets the underlying clock (mirrors the read-only `clock` property).
    pub fn clock(&self) -> &Rc<RefCell<Clock>> {
        &self.clock
    }

    /// Updates the view model with the contents of the underlying clock.
    /// Can be called to force an update of the viewModel if the underlying
    /// clock has changed and `Clock.tick` has not yet been called.
    pub fn synchronize(&self) {
        let clock = self.clock.borrow();
        let mut inner = self.inner.borrow_mut();
        inner.system_time = JulianDate::now();
        inner.start_time = clock.start_time.clone();
        inner.stop_time = clock.stop_time.clone();
        inner.current_time = clock.current_time().clone();
        inner.multiplier = clock.get_multiplier();
        inner.clock_step = clock.get_clock_step();
        inner.clock_range = clock.clock_range;
        inner.can_animate = clock.can_animate;
        inner.should_animate = clock.get_should_animate();
        inner.needs_sync.set(false);
    }

    fn sync_if_needed(&self) {
        if self.inner.borrow().needs_sync.get() {
            self.synchronize();
        }
    }

    /// Gets the current system time (mirrors the `systemTime` observable).
    pub fn system_time(&self) -> JulianDate {
        self.sync_if_needed();
        self.inner.borrow().system_time.clone()
    }

    /// Sets the system time (mirrors writing the `systemTime` observable;
    /// the JS view model exposes a plain observable here).
    pub fn set_system_time(&self, value: JulianDate) {
        let mut inner = self.inner.borrow_mut();
        inner.system_time = value;
        inner.needs_sync.set(false);
    }

    /// Gets the start time of the clock (mirrors the `startTime`
    /// observable).
    pub fn start_time(&self) -> JulianDate {
        self.sync_if_needed();
        self.inner.borrow().start_time.clone()
    }

    /// Sets the start time, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_start_time(&self, value: JulianDate) {
        self.clock.borrow_mut().start_time = value;
        self.synchronize();
    }

    /// Gets the stop time of the clock (mirrors the `stopTime` observable).
    pub fn stop_time(&self) -> JulianDate {
        self.sync_if_needed();
        self.inner.borrow().stop_time.clone()
    }

    /// Sets the stop time, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_stop_time(&self, value: JulianDate) {
        self.clock.borrow_mut().stop_time = value;
        self.synchronize();
    }

    /// Gets the current time (mirrors the `currentTime` observable).
    pub fn current_time(&self) -> JulianDate {
        self.sync_if_needed();
        self.inner.borrow().current_time.clone()
    }

    /// Sets the current time, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_current_time(&self, value: JulianDate) {
        self.clock.borrow_mut().set_current_time(value);
        self.synchronize();
    }

    /// Gets the clock multiplier (mirrors the `multiplier` observable).
    pub fn multiplier(&self) -> f64 {
        self.sync_if_needed();
        self.inner.borrow().multiplier
    }

    /// Sets the multiplier, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_multiplier(&self, value: f64) {
        self.clock.borrow_mut().set_multiplier(value);
        self.synchronize();
    }

    /// Gets the clock step setting (mirrors the `clockStep` observable).
    pub fn clock_step(&self) -> ClockStep {
        self.sync_if_needed();
        self.inner.borrow().clock_step
    }

    /// Sets the clock step, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_clock_step(&self, value: ClockStep) {
        self.clock.borrow_mut().set_clock_step(value);
        self.synchronize();
    }

    /// Gets the clock range setting (mirrors the `clockRange` observable).
    pub fn clock_range(&self) -> ClockRange {
        self.sync_if_needed();
        self.inner.borrow().clock_range
    }

    /// Sets the clock range, writing through to the clock and
    /// re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_clock_range(&self, value: ClockRange) {
        self.clock.borrow_mut().clock_range = value;
        self.synchronize();
    }

    /// Gets whether the clock can animate (mirrors the `canAnimate`
    /// observable).
    pub fn can_animate(&self) -> bool {
        self.sync_if_needed();
        self.inner.borrow().can_animate
    }

    /// Sets whether the clock can animate, writing through to the clock
    /// and re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_can_animate(&self, value: bool) {
        self.clock.borrow_mut().can_animate = value;
        self.synchronize();
    }

    /// Gets whether the clock should animate (mirrors the `shouldAnimate`
    /// observable).
    pub fn should_animate(&self) -> bool {
        self.sync_if_needed();
        self.inner.borrow().should_animate
    }

    /// Sets whether the clock should animate, writing through to the clock
    /// and re-synchronizing (mirrors the observable's subscribe handler).
    pub fn set_should_animate(&self, value: bool) {
        self.clock.borrow_mut().set_should_animate(value);
        self.synchronize();
    }

    /// Mirrors `ClockViewModel.prototype.isDestroyed`.
    ///
    /// DEVIATION: CesiumJS `destroyObject` marks the object destroyed and
    /// throws on any later property access; the Rust port only tracks the
    /// destroyed flag.
    pub fn is_destroyed(&self) -> bool {
        self.inner.borrow().destroyed
    }

    /// Destroys the view model. Should be called to properly clean up the
    /// view model when it is no longer needed.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when destroyed twice, mirroring
    /// `destroyObject`.
    pub fn destroy(&self) {
        if self.inner.borrow().destroyed {
            cesium_core::developer_error::throw_developer_error(
                "This object has been destroyed.",
            );
        }
        if let Some(removal) = self.on_tick_removal.borrow_mut().take() {
            removal.call(&self.clock.borrow().on_tick);
        }
        self.inner.borrow_mut().destroyed = true;
    }
}

impl Default for ClockViewModel {
    fn default() -> Self {
        Self::new(None)
    }
}
