//! 用于管理每帧 GPU 资源创建预算的 JobScheduler。
//!
//! 映射到 CesiumJS `Scene/JobScheduler.js`

/// 作业（GPU 资源创建）的类型。
///
/// 映射到 CesiumJS `Scene/JobType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobType {
    /// 纹理创建。
    Texture = 0,
    /// 着色器程序编译。
    Program = 1,
    /// 缓冲区创建。
    Buffer = 2,
}

impl JobType {
    /// 作业类型的总数。
    pub const NUMBER_OF_JOB_TYPES: usize = 3;
}

/// 单个作业类型在一帧内的预算跟踪。
#[derive(Debug, Clone)]
pub struct JobTypeBudget {
    /// 该作业类型的总预算（毫秒）。
    pub total: f64,
    /// 本帧已使用量（毫秒）。
    pub used_this_frame: f64,
    /// 该作业类型本帧是否被饿死（无法执行）。
    pub starved_this_frame: bool,
    /// 该作业类型上一帧是否被饿死。
    pub starved_last_frame: bool,
    /// 本帧从该作业类型窃取的量。
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

/// 一个为 GPU 资源创建作业管理每帧时间预算的调度器。
///
/// 映射到 CesiumJS `Scene/JobScheduler.js`
#[derive(Debug, Clone)]
pub struct JobScheduler {
    /// 所有作业类型的总预算。
    pub total_budget: f64,
    /// 本帧已使用总量。
    pub total_used_this_frame: f64,
    /// 按类型的预算。
    pub budgets: [JobTypeBudget; 3],
    /// 每个作业类型本帧是否至少执行过一次。
    pub executed_this_frame: [bool; 3],
    /// 模拟的时间戳计数器（用于测试）。
    timestamp: f64,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new(None)
    }
}

impl JobScheduler {
    /// 创建一个新的 JobScheduler，可带自定义预算。
    ///
    /// 映射到 CesiumJS `new JobScheduler(budgets)`。
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

    /// 禁用本帧剩余的执行。
    ///
    /// 映射到 CesiumJS `JobScheduler.disableThisFrame()`。
    pub fn disable_this_frame(&mut self) {
        self.total_used_this_frame = self.total_budget;
    }

    /// 为新的一帧重置预算。
    ///
    /// 映射到 CesiumJS `JobScheduler.resetBudgets()`。
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

    /// 尝试执行给定类型的一个作业。
    /// 若作业已执行返回 true，若预算耗尽返回 false。
    ///
    /// 映射到 CesiumJS `JobScheduler.execute(job, jobType)`。
    pub fn execute(&mut self, job_type: JobType) -> bool {
        let idx = job_type as usize;
        let time_elapsed = 1.0; // 每个作业模拟 1ms
        self.timestamp += 1.0;

        let progress_this_frame = self.executed_this_frame[idx];

        // 提前退出：总预算已耗尽且该类型本帧已有进展
        if self.total_used_this_frame >= self.total_budget && progress_this_frame {
            self.budgets[idx].starved_this_frame = true;
            return false;
        }

        // 检查该作业类型自身的预算是否已耗尽
        let mut stolen_victim: Option<usize> = None;
        if self.budgets[idx].used_this_frame + self.budgets[idx].stolen_from_me_this_frame
            >= self.budgets[idx].total
        {
            // 尝试寻找一个可从中窃取时间的目标
            let mut found = false;
            for i in 0..3 {
                // 目标必须有剩余预算且上一帧未被饿死
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
                // 无目标且已有进展 → 无法执行
                return false;
            }

            if progress_this_frame {
                // 即使通过窃取的时间执行，也标记为饿死
                self.budgets[idx].starved_this_frame = true;
            }
        }

        // 执行该作业
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
