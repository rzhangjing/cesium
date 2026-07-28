//! JobScheduler for managing per-frame GPU resource creation budgets.
//!
//! Maps to CesiumJS `Scene/JobScheduler.js`

/// The type of a job (GPU resource creation).
///
/// Maps to CesiumJS `Scene/JobType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobType {
    /// Texture creation.
    Texture = 0,
    /// Shader program compilation.
    Program = 1,
    /// Buffer creation.
    Buffer = 2,
}

impl JobType {
    /// Total number of job types.
    pub const NUMBER_OF_JOB_TYPES: usize = 3;
}

/// Budget tracking for a single job type within a frame.
#[derive(Debug, Clone)]
pub struct JobTypeBudget {
    /// Total budget for this job type (ms).
    pub total: f64,
    /// Amount used this frame (ms).
    pub used_this_frame: f64,
    /// Whether this job type was starved (couldn't execute) this frame.
    pub starved_this_frame: bool,
    /// Whether this job type was starved last frame.
    pub starved_last_frame: bool,
    /// Amount stolen from this job type this frame.
    pub stolen_from_me_this_frame: f64,
}

impl JobTypeBudget {
    fn new(total: f64) -> Self {
        Self {
            total,
            used_this_frame: 0.0,
            starved_this_frame: false,
            starved_last_frame: false,
            stolen_from_me_this_frame: 0.0,
        }
    }
}

/// A scheduler that manages per-frame time budgets for GPU resource creation jobs.
///
/// Maps to CesiumJS `Scene/JobScheduler.js`
#[derive(Debug, Clone)]
pub struct JobScheduler {
    /// Total budget across all job types.
    pub total_budget: f64,
    /// Total used this frame.
    pub total_used_this_frame: f64,
    /// Per-type budgets.
    pub budgets: [JobTypeBudget; 3],
    /// Whether each job type has executed at least once this frame.
    pub executed_this_frame: [bool; 3],
    /// Simulated timestamp counter (for testing).
    timestamp: f64,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new(None)
    }
}

impl JobScheduler {
    /// Creates a new JobScheduler with optional custom budgets.
    ///
    /// Maps to CesiumJS `new JobScheduler(budgets)`.
    pub fn new(budgets: Option<[f64; 3]>) -> Self {
        let (tex, prog, buf) = match budgets {
            Some(b) => (b[0], b[1], b[2]),
            None => (10.0, 10.0, 30.0),
        };

        let total_budget = tex + prog + buf;

        Self {
            total_budget,
            total_used_this_frame: 0.0,
            budgets: [
                JobTypeBudget::new(tex),
                JobTypeBudget::new(prog),
                JobTypeBudget::new(buf),
            ],
            executed_this_frame: [false; 3],
            timestamp: 0.0,
        }
    }

    /// Disables execution for the rest of this frame.
    ///
    /// Maps to CesiumJS `JobScheduler.disableThisFrame()`.
    pub fn disable_this_frame(&mut self) {
        self.total_used_this_frame = self.total_budget;
    }

    /// Resets budgets for a new frame.
    ///
    /// Maps to CesiumJS `JobScheduler.resetBudgets()`.
    pub fn reset_budgets(&mut self) {
        self.total_used_this_frame = 0.0;
        for i in 0..3 {
            self.budgets[i].starved_last_frame = self.budgets[i].starved_this_frame;
            self.budgets[i].starved_this_frame = false;
            self.budgets[i].used_this_frame = 0.0;
            self.budgets[i].stolen_from_me_this_frame = 0.0;
            self.executed_this_frame[i] = false;
        }
    }

    /// Attempts to execute a job of the given type.
    /// Returns true if the job was executed, false if budget was exhausted.
    ///
    /// Maps to CesiumJS `JobScheduler.execute(job, jobType)`.
    pub fn execute(&mut self, job_type: JobType) -> bool {
        let idx = job_type as usize;
        let time_elapsed = 1.0; // Simulated 1ms per job
        self.timestamp += 1.0;

        let progress_this_frame = self.executed_this_frame[idx];

        // Early exit: total budget exhausted and this type already progressed
        if self.total_used_this_frame >= self.total_budget && progress_this_frame {
            self.budgets[idx].starved_this_frame = true;
            return false;
        }

        // Check if this job type's own budget is exhausted
        let mut stolen_victim: Option<usize> = None;
        if self.budgets[idx].used_this_frame + self.budgets[idx].stolen_from_me_this_frame
            >= self.budgets[idx].total
        {
            // Try to find a victim to steal from
            let mut found = false;
            for i in 0..3 {
                // Victim must have remaining budget and not have been starved last frame
                if self.budgets[i].used_this_frame + self.budgets[i].stolen_from_me_this_frame
                    < self.budgets[i].total
                    && !self.budgets[i].starved_last_frame
                {
                    stolen_victim = Some(i);
                    found = true;
                    break;
                }
            }

            if !found && progress_this_frame {
                // No victim and already progressed → cannot execute
                return false;
            }

            if progress_this_frame {
                // Mark as starved even if executing via stolen time
                self.budgets[idx].starved_this_frame = true;
            }
        }

        // Execute the job
        self.total_used_this_frame += time_elapsed;
        if let Some(victim) = stolen_victim {
            self.budgets[victim].stolen_from_me_this_frame += time_elapsed;
        } else {
            self.budgets[idx].used_this_frame += time_elapsed;
        }
        self.executed_this_frame[idx] = true;

        true
    }
}
