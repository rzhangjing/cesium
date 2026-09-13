//! Scene/JobSchedulerSpec.js → Rust integration tests
//!
//! Original: 11 it() → 10 A-class (1 C-class: throws)
//! Tests: constructs(1) + executes(1) + disable(1) + different_types(1) +
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
    assert!(js.execute(JobType::Texture)); // steals from PROGRAM
    assert_eq!(js.total_used_this_frame, 2.0);

    assert_eq!(js.budgets[JobType::Texture as usize].used_this_frame, 1.0); // own budget only
    assert!(js.budgets[JobType::Texture as usize].starved_this_frame);
    assert_eq!(js.budgets[JobType::Program as usize].used_this_frame, 0.0);
    assert_eq!(js.budgets[JobType::Program as usize].stolen_from_me_this_frame, 1.0);
    assert!(!js.budgets[JobType::Program as usize].starved_this_frame);

    // No budgets left to steal from
    assert!(!js.execute(JobType::Texture));
    // PROGRAM still gets progress once per frame
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

    // Exhaust budget for all job types
    assert!(!js.execute(JobType::Texture));
    assert!(!js.execute(JobType::Program));
    assert!(!js.execute(JobType::Buffer));

    // Next frame: no stealing since all were starved last frame
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

    // Frame 1: exhaust
    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture)); // stolen from PROGRAM
    assert!(!js.execute(JobType::Texture));

    // Frame 2: TEXTURE was starved last frame, can't steal
    js.reset_budgets();
    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program)); // Can't steal from TEXTURE (requester was starved)
    assert!(js.execute(JobType::Texture)); // progress guarantee
    assert!(!js.execute(JobType::Texture)); // TEXTURE was starved last frame, can't steal

    // Frame 3: PROGRAM was starved in Frame 2
    js.reset_budgets();
    assert!(js.execute(JobType::Program)); // progress guarantee

    // Frame 4: PROGRAM was starved in Frame 3, but TEXTURE was NOT starved in Frame 3
    js.reset_budgets();
    assert!(js.execute(JobType::Program)); // progress guarantee
    assert!(js.execute(JobType::Program)); // Can steal from TEXTURE (not starved last frame)
}

#[test]
fn test_allows_progress_on_all_job_types_once_per_frame() {
    let mut js = JobScheduler::new(Some([1.0, 1.0, 1.0]));

    assert!(js.execute(JobType::Texture));
    assert!(js.execute(JobType::Texture)); // Steal from PROGRAM
    assert!(js.execute(JobType::Texture)); // Steal from BUFFER

    assert!(!js.execute(JobType::Texture));

    // Still gets progress once this frame
    assert!(js.execute(JobType::Program));
    assert!(!js.execute(JobType::Program));

    assert!(js.execute(JobType::Buffer));
    assert!(!js.execute(JobType::Buffer));
}

#[test]
fn test_long_job_allows_progress() {
    // Budget is less than 1.0 per job, but each type still gets one execution
    let mut js = JobScheduler::new(Some([0.5, 0.2, 0.2]));
    assert!(js.execute(JobType::Texture)); // Goes over budget
    assert!(js.execute(JobType::Program)); // Still gets progress
    assert!(js.execute(JobType::Buffer)); // Still gets progress
}
