//! Animation system: property animation, path animation, and path visualization.
//!
//! Maps to CesiumJS:
//! - `DataSources/PathVisualizer.js`
//! - `DataSources/SampledPositionProperty.js` (animation aspect)
//! - `Core/JulianDate.js` (time management)

use cesium_geospatial::{Cartographic, Ellipsoid};

use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use crate::property::Property;

/// A keyframe in an animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    /// Time in seconds since epoch.
    pub time: f64,
    /// Value at this keyframe (position as [lon_rad, lat_rad, height_m]).
    pub value: [f64; 3],
}

/// Interpolation algorithm for animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationAlgorithm {
    /// Linear interpolation.
    #[default]
    Linear,
    /// Hermite (cubic) interpolation.
    Hermite,
    /// Lagrange polynomial interpolation.
    Lagrange,
}

/// Animation clock state.
#[derive(Debug, Clone)]
pub struct AnimationClock {
    /// Start time (seconds).
    pub start_time: f64,
    /// Stop time (seconds).
    pub stop_time: f64,
    /// Current time (seconds).
    pub current_time: f64,
    /// Playback rate multiplier.
    pub multiplier: f64,
    /// Whether the clock is playing.
    pub playing: bool,
    /// Whether to loop.
    pub looping: bool,
}

impl AnimationClock {
    /// Creates a new animation clock.
    pub fn new(start_time: f64, stop_time: f64) -> Self {
        Self {
            start_time,
            stop_time,
            current_time: start_time,
            multiplier: 1.0,
            playing: false,
            looping: true,
        }
    }

    /// Advances the clock by delta_time seconds.
    pub fn tick(&mut self, delta_time: f64) {
        if !self.playing {
            return;
        }

        self.current_time += delta_time * self.multiplier;

        if self.current_time > self.stop_time {
            if self.looping {
                self.current_time = self.start_time
                    + (self.current_time - self.start_time) % (self.stop_time - self.start_time);
            } else {
                self.current_time = self.stop_time;
                self.playing = false;
            }
        } else if self.current_time < self.start_time {
            if self.looping {
                let range = self.stop_time - self.start_time;
                self.current_time = self.stop_time - (self.start_time - self.current_time) % range;
            } else {
                self.current_time = self.start_time;
                self.playing = false;
            }
        }
    }

    /// Normalized progress (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if (self.stop_time - self.start_time).abs() < f64::EPSILON {
            return 0.0;
        }
        (self.current_time - self.start_time) / (self.stop_time - self.start_time)
    }

    /// Resets to start.
    pub fn reset(&mut self) {
        self.current_time = self.start_time;
    }

    /// Seeks to a specific time.
    pub fn seek(&mut self, time: f64) {
        self.current_time = time.clamp(self.start_time, self.stop_time);
    }
}

/// Interpolates a position at the given time from keyframes.
pub fn interpolate_position(
    keyframes: &[Keyframe],
    time: f64,
    algorithm: InterpolationAlgorithm,
) -> Option<[f64; 3]> {
    if keyframes.is_empty() {
        return None;
    }
    if keyframes.len() == 1 {
        return Some(keyframes[0].value);
    }

    // Find surrounding keyframes
    let mut prev_idx = 0;
    for (i, kf) in keyframes.iter().enumerate() {
        if kf.time > time {
            break;
        }
        prev_idx = i;
    }

    // Before first or at first
    if time <= keyframes[0].time {
        return Some(keyframes[0].value);
    }
    // After last or at last
    if time >= keyframes[keyframes.len() - 1].time {
        return Some(keyframes[keyframes.len() - 1].value);
    }

    let next_idx = (prev_idx + 1).min(keyframes.len() - 1);
    let prev = &keyframes[prev_idx];
    let next = &keyframes[next_idx];

    let dt = next.time - prev.time;
    if dt.abs() < f64::EPSILON {
        return Some(prev.value);
    }

    let t = (time - prev.time) / dt;

    match algorithm {
        InterpolationAlgorithm::Linear => {
            Some([
                prev.value[0] + t * (next.value[0] - prev.value[0]),
                prev.value[1] + t * (next.value[1] - prev.value[1]),
                prev.value[2] + t * (next.value[2] - prev.value[2]),
            ])
        }
        InterpolationAlgorithm::Hermite => {
            // Cubic Hermite with zero tangents (smooth step)
            let t2 = t * t;
            let t3 = t2 * t;
            let h = 3.0 * t2 - 2.0 * t3; // smoothstep
            Some([
                prev.value[0] + h * (next.value[0] - prev.value[0]),
                prev.value[1] + h * (next.value[1] - prev.value[1]),
                prev.value[2] + h * (next.value[2] - prev.value[2]),
            ])
        }
        InterpolationAlgorithm::Lagrange => {
            // Use up to 4 surrounding points for Lagrange
            let start = prev_idx.saturating_sub(1);
            let end = (next_idx + 2).min(keyframes.len());
            let points: Vec<&Keyframe> = keyframes[start..end].iter().collect();

            if points.len() < 3 {
                // Fall back to linear
                return Some([
                    prev.value[0] + t * (next.value[0] - prev.value[0]),
                    prev.value[1] + t * (next.value[1] - prev.value[1]),
                    prev.value[2] + t * (next.value[2] - prev.value[2]),
                ]);
            }

            let mut result = [0.0; 3];
            for (i, pi) in points.iter().enumerate() {
                let mut basis = 1.0;
                for (j, pj) in points.iter().enumerate() {
                    if i != j {
                        let denom = pi.time - pj.time;
                        if denom.abs() > f64::EPSILON {
                            basis *= (time - pj.time) / denom;
                        }
                    }
                }
                result[0] += basis * pi.value[0];
                result[1] += basis * pi.value[1];
                result[2] += basis * pi.value[2];
            }
            Some(result)
        }
    }
}

/// A path trail point in Cartesian3.
#[derive(Debug, Clone)]
pub struct PathPoint {
    /// Position in Cartesian3 [x, y, z].
    pub position: [f64; 3],
    /// Time at this point.
    pub time: f64,
}

/// Computes the trail/lead path for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/PathVisualizer.js`
pub fn compute_path(
    entity: &Entity,
    time: f64,
    lead_time: f64,
    trail_time: f64,
    resolution: f64,
    ellipsoid: &Ellipsoid,
) -> Vec<PathPoint> {
    let mut path = Vec::new();

    // Get position samples from the entity
    let samples = match &entity.position {
        Property::Sampled(s) => s,
        Property::Constant(pos) => {
            // Static entity - no path
            let cart = ellipsoid.cartographic_to_cartesian(
                &Cartographic::from_radians(pos[0], pos[1], pos[2]),
            );
            path.push(PathPoint {
                position: [cart.x, cart.y, cart.z],
                time,
            });
            return path;
        }
        Property::Undefined => return path,
    };

    if samples.is_empty() {
        return path;
    }

    // Compute keyframes from samples
    let keyframes: Vec<Keyframe> = samples
        .iter()
        .map(|(t, pos)| Keyframe { time: *t, value: *pos })
        .collect();

    // Trail: from (time - trail_time) to time
    let trail_start = time - trail_time;
    let mut t = trail_start;
    while t <= time {
        if let Some(pos) = interpolate_position(&keyframes, t, InterpolationAlgorithm::Linear) {
            let cart = ellipsoid.cartographic_to_cartesian(
                &Cartographic::from_radians(pos[0], pos[1], pos[2]),
            );
            path.push(PathPoint {
                position: [cart.x, cart.y, cart.z],
                time: t,
            });
        }
        t += resolution;
    }

    // Lead: from time to (time + lead_time)
    let lead_end = time + lead_time;
    t = time + resolution;
    while t <= lead_end {
        if let Some(pos) = interpolate_position(&keyframes, t, InterpolationAlgorithm::Linear) {
            let cart = ellipsoid.cartographic_to_cartesian(
                &Cartographic::from_radians(pos[0], pos[1], pos[2]),
            );
            path.push(PathPoint {
                position: [cart.x, cart.y, cart.z],
                time: t,
            });
        }
        t += resolution;
    }

    path
}

/// Updates all entities with path graphics, computing their trail/lead paths.
pub fn update_all_paths(
    entities: &EntityCollection,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> Vec<(String, Vec<PathPoint>)> {
    entities
        .values()
        .filter(|e| e.show && e.path.is_some())
        .filter_map(|entity| {
            let path_graphics = entity.path.as_ref().unwrap();
            let lead_time = path_graphics.lead_time.get_value(time).copied().unwrap_or(0.0);
            let trail_time = path_graphics.trail_time.get_value(time).copied().unwrap_or(0.0);
            let resolution = path_graphics.resolution.get_value(time).copied().unwrap_or(60.0);

            let path = compute_path(entity, time, lead_time, trail_time, resolution, ellipsoid);
            if path.is_empty() {
                None
            } else {
                Some((entity.id.clone(), path))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::PathGraphics;

    #[test]
    fn test_animation_clock_tick() {
        let mut clock = AnimationClock::new(0.0, 100.0);
        clock.playing = true;
        clock.multiplier = 1.0;

        clock.tick(10.0);
        assert!((clock.current_time - 10.0).abs() < 1e-10);

        clock.tick(10.0);
        assert!((clock.current_time - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_clock_loop() {
        let mut clock = AnimationClock::new(0.0, 100.0);
        clock.playing = true;
        clock.looping = true;

        clock.tick(110.0);
        assert!((clock.current_time - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_animation_clock_no_loop() {
        let mut clock = AnimationClock::new(0.0, 100.0);
        clock.playing = true;
        clock.looping = false;

        clock.tick(110.0);
        assert!((clock.current_time - 100.0).abs() < 1e-10);
        assert!(!clock.playing);
    }

    #[test]
    fn test_animation_clock_progress() {
        let clock = AnimationClock {
            start_time: 0.0,
            stop_time: 100.0,
            current_time: 50.0,
            multiplier: 1.0,
            playing: true,
            looping: true,
        };
        assert!((clock.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_linear() {
        let keyframes = vec![
            Keyframe { time: 0.0, value: [0.0, 0.0, 0.0] },
            Keyframe { time: 10.0, value: [10.0, 20.0, 30.0] },
        ];

        let pos = interpolate_position(&keyframes, 5.0, InterpolationAlgorithm::Linear).unwrap();
        assert!((pos[0] - 5.0).abs() < 1e-10);
        assert!((pos[1] - 10.0).abs() < 1e-10);
        assert!((pos[2] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_hermite() {
        let keyframes = vec![
            Keyframe { time: 0.0, value: [0.0, 0.0, 0.0] },
            Keyframe { time: 10.0, value: [10.0, 10.0, 10.0] },
        ];

        // At midpoint, Hermite (smoothstep) should give 0.5
        let pos = interpolate_position(&keyframes, 5.0, InterpolationAlgorithm::Hermite).unwrap();
        assert!((pos[0] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_boundaries() {
        let keyframes = vec![
            Keyframe { time: 0.0, value: [1.0, 2.0, 3.0] },
            Keyframe { time: 10.0, value: [10.0, 20.0, 30.0] },
        ];

        // Before start
        let pos = interpolate_position(&keyframes, -5.0, InterpolationAlgorithm::Linear).unwrap();
        assert_eq!(pos, [1.0, 2.0, 3.0]);

        // After end
        let pos = interpolate_position(&keyframes, 15.0, InterpolationAlgorithm::Linear).unwrap();
        assert_eq!(pos, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_interpolate_single_keyframe() {
        let keyframes = vec![Keyframe { time: 0.0, value: [5.0, 5.0, 5.0] }];
        let pos = interpolate_position(&keyframes, 100.0, InterpolationAlgorithm::Linear).unwrap();
        assert_eq!(pos, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_compute_path_sampled() {
        let mut entity = Entity::new("sat");
        entity.position = Property::Sampled(vec![
            (0.0, [0.0, 0.0, 0.0]),
            (60.0, [0.01, 0.01, 100.0]),
            (120.0, [0.02, 0.02, 200.0]),
        ]);
        entity.path = Some(PathGraphics {
            lead_time: Property::Constant(60.0),
            trail_time: Property::Constant(60.0),
            resolution: Property::Constant(30.0),
            ..Default::default()
        });

        let ellipsoid = Ellipsoid::WGS84;
        let path = compute_path(&entity, 60.0, 60.0, 60.0, 30.0, &ellipsoid);

        // Should have trail (0-60) + lead (60-120) points
        assert!(path.len() >= 4);
    }

    #[test]
    fn test_compute_path_static() {
        let entity = Entity::new("static").with_position(0.0, 0.0, 1000.0);
        let ellipsoid = Ellipsoid::WGS84;

        let path = compute_path(&entity, 0.0, 60.0, 60.0, 30.0, &ellipsoid);
        assert_eq!(path.len(), 1); // Static entity has single point
    }

    #[test]
    fn test_update_all_paths() {
        let mut entities = EntityCollection::new();

        let mut sat = Entity::new("sat-1");
        sat.position = Property::Sampled(vec![
            (0.0, [0.0, 0.0, 0.0]),
            (120.0, [0.02, 0.02, 200.0]),
        ]);
        sat.path = Some(PathGraphics {
            lead_time: Property::Constant(60.0),
            trail_time: Property::Constant(60.0),
            resolution: Property::Constant(30.0),
            ..Default::default()
        });
        entities.add(sat);

        // Entity without path
        entities.add(Entity::new("no-path").with_position(0.0, 0.0, 0.0));

        let ellipsoid = Ellipsoid::WGS84;
        let paths = update_all_paths(&entities, 60.0, &ellipsoid);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].0, "sat-1");
        assert!(!paths[0].1.is_empty());
    }

    #[test]
    fn test_clock_seek() {
        let mut clock = AnimationClock::new(0.0, 100.0);
        clock.seek(50.0);
        assert!((clock.current_time - 50.0).abs() < 1e-10);

        clock.seek(200.0);
        assert!((clock.current_time - 100.0).abs() < 1e-10);

        clock.seek(-10.0);
        assert!((clock.current_time - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_clock_multiplier() {
        let mut clock = AnimationClock::new(0.0, 100.0);
        clock.playing = true;
        clock.multiplier = 2.0;

        clock.tick(10.0);
        assert!((clock.current_time - 20.0).abs() < 1e-10);
    }
}
