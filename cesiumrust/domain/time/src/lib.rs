//! cesium-time: JulianDate, Clock, TimeInterval
//! Domain layer - pure Rust, no framework dependency.
//!
//! CesiumJS mapping: `packages/engine/Source/Core/JulianDate.js`, `Clock.js`, `TimeInterval.js`

pub mod julian_date;
pub mod gregorian_date;
pub mod time_interval;
pub mod time_interval_collection;
pub mod clock;

pub use julian_date::JulianDate;
pub use julian_date::TimeStandard;
pub use gregorian_date::GregorianDate;
pub use gregorian_date::{is_leap_year, days_in_month};
pub use time_interval::TimeInterval;
pub use time_interval_collection::{TimeIntervalCollection, TimeIntervalData, FromIso8601Options};
pub use clock::{Clock, ClockOptions, ClockRange, ClockStep};
