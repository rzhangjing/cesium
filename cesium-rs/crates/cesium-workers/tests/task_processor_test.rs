//! Ported specs from `packages/engine/Specs/Core/TaskProcessorSpec.js`,
//! adapted to the rayon-backed Rust [`TaskProcessor`].
//!
//! Web-Worker/WebAssembly-only cases (browser support, cross-origin shims,
//! wasm compile) have no Rust analogue and are not mirrored. Concurrency
//! assertions use channels/condvars only — never sleeps.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, LazyLock, Mutex};

use cesium_workers::task_processor::{register_worker, unregister_worker, TaskProcessor};

/// Shared gate used by [`blocking_worker`]: signals task start and blocks
/// the worker until the test releases it. Deterministic, no sleeps.
struct ReleaseGate {
    notified: Mutex<bool>,
    cv: Condvar,
}

type GatePair = (mpsc::SyncSender<()>, Arc<ReleaseGate>);

static GATE: LazyLock<Mutex<Option<GatePair>>> = LazyLock::new(|| Mutex::new(None));

fn install_gate() -> (mpsc::Receiver<()>, Arc<ReleaseGate>) {
    let (done_tx, done_rx) = mpsc::sync_channel(16);
    let gate = Arc::new(ReleaseGate {
        notified: Mutex::new(false),
        cv: Condvar::new(),
    });
    *GATE.lock().unwrap() = Some((done_tx, Arc::clone(&gate)));
    (done_rx, gate)
}

fn release_gate(gate: &Arc<ReleaseGate>) {
    let mut notified = gate.notified.lock().unwrap();
    *notified = true;
    gate.cv.notify_all();
}

/// Worker that announces its start, then blocks until released.
fn blocking_worker(_parameters: &[u8]) -> Result<Vec<u8>, String> {
    let (done_tx, gate) = GATE.lock().unwrap().clone().expect("gate installed");
    done_tx.send(()).expect("done channel open");
    let mut notified = gate.notified.lock().unwrap();
    while !*notified {
        notified = gate.cv.wait(notified).unwrap();
    }
    Ok(vec![42])
}

fn echo_worker(parameters: &[u8]) -> Result<Vec<u8>, String> {
    Ok(parameters.to_vec())
}

fn throwing_worker(_parameters: &[u8]) -> Result<Vec<u8>, String> {
    Err("test failure".to_string())
}

/// Mirror of `transferArrayBuffer`: returns a zeroed buffer of input length.
fn transfer_array_buffer_worker(parameters: &[u8]) -> Result<Vec<u8>, String> {
    Ok(vec![0u8; parameters.len()])
}

#[test]
fn works_with_a_simple_worker() {
    register_worker("echoParams", echo_worker);
    let processor = TaskProcessor::new("echoParams");
    let input = vec![1u8, 2, 3, 4];
    let handle = processor.schedule_task(input.clone()).unwrap();
    assert_eq!(handle.wait().unwrap(), input);
    assert_eq!(processor.active_tasks_count(), 0);
    unregister_worker("echoParams");
}

#[test]
fn concurrent_tasks_all_complete() {
    register_worker("echoConcurrent", echo_worker);
    let processor = TaskProcessor::with_max_tasks("echoConcurrent", 16);
    let mut handles = Vec::new();
    for i in 0..16u8 {
        handles.push(
            processor
                .schedule_task(vec![i, i.wrapping_add(1)])
                .unwrap(),
        );
    }
    for (i, handle) in handles.into_iter().enumerate() {
        let expected = vec![i as u8, (i as u8).wrapping_add(1)];
        assert_eq!(handle.wait().unwrap(), expected);
    }
    assert_eq!(processor.active_tasks_count(), 0);
    unregister_worker("echoConcurrent");
}

#[test]
fn maximum_active_tasks_limit_is_enforced() {
    register_worker("blockingWorker", blocking_worker);
    let (done_rx, gate) = install_gate();

    let processor = TaskProcessor::with_max_tasks("blockingWorker", 2);
    let h1 = processor.schedule_task(Vec::new()).unwrap();
    let h2 = processor.schedule_task(Vec::new()).unwrap();

    // Both workers announce they started (deterministic via channel).
    done_rx.recv().unwrap();
    done_rx.recv().unwrap();
    assert_eq!(processor.active_tasks_count(), 2);

    // Limit reached: CesiumJS scheduleTask returns undefined here.
    assert!(processor.schedule_task(Vec::new()).is_none());

    release_gate(&gate);
    assert_eq!(h1.wait().unwrap(), vec![42]);
    assert_eq!(h2.wait().unwrap(), vec![42]);

    // A slot is free again.
    let h3 = processor.schedule_task(Vec::new()).unwrap();
    done_rx.recv().unwrap();
    release_gate(&gate);
    assert_eq!(h3.wait().unwrap(), vec![42]);
    assert_eq!(processor.active_tasks_count(), 0);

    *GATE.lock().unwrap() = None;
    unregister_worker("blockingWorker");
}

#[test]
fn can_be_destroyed() {
    let mut processor = TaskProcessor::new("noop");
    assert!(!processor.is_destroyed());
    processor.destroy();
    assert!(processor.is_destroyed());
    // Scheduling after destroy yields no handle (deterministic).
    assert!(processor.schedule_task(Vec::new()).is_none());
}

#[test]
fn can_transfer_array_buffer() {
    register_worker("transferArrayBuffer", transfer_array_buffer_worker);
    let processor = TaskProcessor::new("transferArrayBuffer");
    let input = vec![1u8, 2, 3, 4, 5];
    let result = processor.schedule_task(input).unwrap().wait().unwrap();
    assert_eq!(result.len(), 5);
    assert!(result.iter().all(|&b| b == 0));
    unregister_worker("transferArrayBuffer");
}

#[test]
fn rejects_when_worker_throws() {
    register_worker("throwingWorker", throwing_worker);
    let processor = TaskProcessor::new("throwingWorker");
    let result = processor.schedule_task(Vec::new()).unwrap().wait();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("test failure"));
    unregister_worker("throwingWorker");
}

#[test]
fn rejects_unknown_worker_name() {
    let processor = TaskProcessor::new("noSuchWorker");
    let result = processor.schedule_task(Vec::new()).unwrap().wait();
    let err = result.unwrap_err();
    assert!(err.contains("Unknown worker"), "unexpected error: {err}");
}

#[test]
fn dispatches_builtin_geometry_worker_by_name() {
    // Built-in table routes CesiumJS module names to ported worker fns.
    let processor = TaskProcessor::new("createBoxGeometry");
    // The ported create_box_geometry is a pack/unpack stub returning empty.
    let result = processor.schedule_task(Vec::new()).unwrap().wait().unwrap();
    assert!(result.is_empty());
}

#[test]
fn successful_task_raises_the_task_completed_event() {
    register_worker("echoCompleted", echo_worker);
    let processor = TaskProcessor::new("echoCompleted");
    let count = Arc::new(AtomicUsize::new(0));
    let last_error: Arc<Mutex<Option<Option<String>>>> = Arc::new(Mutex::new(None));

    let count2 = Arc::clone(&count);
    let error2 = Arc::clone(&last_error);
    let _remove = processor.task_completed_event().add_listener(move |err: &Option<String>| {
        count2.fetch_add(1, Ordering::SeqCst);
        *error2.lock().unwrap() = Some(err.clone());
    });

    processor.schedule_task(vec![7]).unwrap().wait().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(*last_error.lock().unwrap(), Some(None));
    unregister_worker("echoCompleted");
}

#[test]
fn unsuccessful_task_raises_the_task_completed_event_with_error() {
    register_worker("throwCompleted", throwing_worker);
    let processor = TaskProcessor::new("throwCompleted");
    let count = Arc::new(AtomicUsize::new(0));
    let last_error: Arc<Mutex<Option<Option<String>>>> = Arc::new(Mutex::new(None));

    let count2 = Arc::clone(&count);
    let error2 = Arc::clone(&last_error);
    let _remove = processor.task_completed_event().add_listener(move |err: &Option<String>| {
        count2.fetch_add(1, Ordering::SeqCst);
        *error2.lock().unwrap() = Some(err.clone());
    });

    let _ = processor.schedule_task(Vec::new()).unwrap().wait();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let recorded = last_error.lock().unwrap().clone().unwrap();
    assert!(recorded.is_some());
    assert!(recorded.unwrap().contains("test failure"));
    unregister_worker("throwCompleted");
}
