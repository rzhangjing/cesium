//! Ported from `packages/engine/Source/Scene/ParticleBurst.js`.

/// Represents a burst of particles from a particle system at a given time
/// in the system's lifetime.
#[derive(Debug, Clone)]
pub struct ParticleBurst {
    /// The time in seconds after the beginning of the particle system's
    /// lifetime that the burst will occur.
    pub time: f64,
    /// The minimum number of particles emitted.
    pub minimum: f64,
    /// The maximum number of particles emitted.
    pub maximum: f64,
    /// True if the burst has been completed.
    complete: bool,
}

impl ParticleBurst {
    /// Creates a new `ParticleBurst`.
    pub fn new(time: Option<f64>, minimum: Option<f64>, maximum: Option<f64>) -> Self {
        Self {
            time: time.unwrap_or(0.0),
            minimum: minimum.unwrap_or(0.0),
            maximum: maximum.unwrap_or(50.0),
            complete: false,
        }
    }

    /// True if the burst has been completed.
    pub fn complete(&self) -> bool {
        self.complete
    }

    /// Sets the complete flag.
    pub fn set_complete(&mut self, value: bool) {
        self.complete = value;
    }
}

impl Default for ParticleBurst {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}
