//! Scene/JobSchedulerSpec.js → Rust 集成测试
//!
//! 原始：11 个 it() → 10 个 A 类（1 个 C 类：throws）
//! 测试：constructs(1) + executes(1) + disable(1) + different_types(1) +
//!        second_job(1) + exceeds_total(1) + steals(1) + no_steal_same_frame(1) +
//!        no_steal_starving(1) + allows_progress(1) + long_job(1)

use cesium_scene::job_scheduler::{JobScheduler, JobType};

#[test]
fn test_constructs_with_defaults() {
    let js = JobScheduler::default();
    assert_eq!(js.total_budget, 50.0);
    assert_eq!(js.budgets[JobType::Texture as usize].total, 10.0);
    assert_eq!(js.budgets[JobType::Program as usize].total, 10.0);
    assert_eq!(js.budgets[JobType::Buffer as usize].total, 30.0);
}

#[test]
fn test_executes_a_job() {
    let mut js = JobScheduler::new(Some([2.0, 0.0, 0.0]));
    let executed = js.execute(JobType::Texture);
    assert!(executed);
    assert_eq!(js.total_used_this_frame, 1.0);
    assert_eq!(js.budgets[JobType::Texture as usize].total, 2.0);
    assert_eq!(js.budgets[JobType::Texture as usize].used_this_frame, 1.0);
}

#[test]
fn test_disable_this_frame() {
    let mut js = JobScheduler::new(Some([2.0, 0.0, 0.0]));
    assert!(js.execute(JobType::Texture));
    js.disable_this_frame();
    assert!(!js.execute(JobType::Texture));
}

#[test]
fn test_executes_different_job_types() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 1.0]));
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Program));
    assert!(js.execute(JobType::Buffer));

    assert_eq!(js.total_used_this_frame, 3.0);
    assert_eq!(js.budgets[JobType::Texture as usize].used_this_frame, 1.0);
    assert_eq!(js.budgets[JobType::Program as usize].used_this_frame, 1.0);
    assert_eq!(js.budgets[JobType::Buffer as usize].used_this_frame, 1.0);
}

#[test]
fn test_executes_a_second_job() {
    let mut js = JobScheduler::new(Some([2.0, 0.0, 0.0]));
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture));
    assert_eq!(js.total_used_this_frame, 2.0);
    assert_eq!(js.budgets[JobType::Texture as usize].used_this_frame, 2.0);
}

#[test]
fn test_does_not_execute_second_job_exceeds_total() {
    let mut js = JobScheduler::new(Some([1.0, 0.0, 0.0]));
    assert!(js.execute(JobType::Texture));
    assert!(!js.execute(JobType::Texture));
    assert!(js.budgets[JobType::Texture as usize].starved_this_frame);
}

#[test]
fn test_executes_second_job_texture_steals_program_budget() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 0.0]));
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture)); // 从 PROGRAM 窃取
    assert_eq!(js.total_used_this_frame, 2.0);

    assert_eq!(js.budgets[JobType::Texture as usize].used_this_frame, 1.0); // 仅用自己的预算
    assert!(js.budgets[JobType::Texture as usize].starved_this_frame);
    assert_eq!(js.budgets[JobType::Program as usize].used_this_frame, 0.0);
    assert_eq!(js.budgets[JobType::Program as usize].stolen_from_me_this_frame, 1.0);
    assert!(!js.budgets[JobType::Program as usize].starved_this_frame);

    // 没有可窃取的剩余预算
    assert!(!js.execute(JobType::Texture));
    // PROGRAM 每帧仍获得一次进展
    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program));
    assert!(js.budgets[JobType::Program as usize].starved_this_frame);
}

#[test]
fn test_does_not_steal_in_same_frame() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 1.0]));
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Program));
    assert!(js.execute(JobType::Buffer));

    // 耗尽所有作业类型的预算
    assert!(!js.execute(JobType::Texture));
    assert!(!js.execute(JobType::Program));
    assert!(!js.execute(JobType::Buffer));

    // 下一帧：不窃取，因为上一帧所有类型都挨饿了
    js.reset_budgets();
    assert!(js.execute(JobType::Texture));
    assert!(!js.execute(JobType::Texture));

    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program));

    assert!(js.execute(JobType::Buffer));
    assert!(!js.execute(JobType::Buffer));
}

#[test]
fn test_does_not_steal_from_starving_over_multiple_frames() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 0.0]));

    // 第 1 帧：耗尽
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture)); // 从 PROGRAM 窃取
    assert!(!js.execute(JobType::Texture));

    // 第 2 帧：TEXTURE 上一帧挨饿，不能窃取
    js.reset_budgets();
    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program)); // 不能从 TEXTURE 窃取（请求者上一帧挨饿）
    assert!(js.execute(JobType::Texture)); // 进展保证
    assert!(!js.execute(JobType::Texture)); // TEXTURE 上一帧挨饿，不能窃取

    // 第 3 帧：PROGRAM 在第 2 帧挨饿
    js.reset_budgets();
    assert!(js.execute(JobType::Program)); // 进展保证

    // 第 4 帧：PROGRAM 在第 3 帧挨饿，但 TEXTURE 在第 3 帧未挨饿
    js.reset_budgets();
    assert!(js.execute(JobType::Program)); // 进展保证
    assert!(js.execute(JobType::Program)); // 可以从 TEXTURE 窃取（上一帧未挨饿）
}

#[test]
fn test_allows_progress_on_all_job_types_once_per_frame() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 1.0]));

    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture)); // 从 PROGRAM 窃取
    assert!(js.execute(JobType::Texture)); // 从 BUFFER 窃取

    assert!(!js.execute(JobType::Texture));

    // 本帧仍获得一次进展
    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program));

    assert!(js.execute(JobType::Buffer));
    assert!(!js.execute(JobType::Buffer));
}

#[test]
fn test_long_job_allows_progress() {
    // 每个作业的预算小于 1.0，但每种类型仍获得一次执行
    let mut js = JobScheduler::new(Some([0.5, 0.2, 0.2]));
    assert!(js.execute(JobType::Texture)); // 超出预算
    assert!(js.execute(JobType::Program)); // 仍获得进展
    assert!(js.execute(JobType::Buffer)); // 仍获得进展
}
