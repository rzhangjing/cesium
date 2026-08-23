//! Ported from `packages/engine/Source/Workers/TaskProcessor.js` — wasm backend.
//!
//! This module provides the wasm32-specific implementation of the TaskProcessor
//! using Web Workers via `web-sys` and `wasm-bindgen`. On native targets, the
//! `task_processor` module uses `std::thread` instead.
//!
//! # Feature Gate
//!
//! This module is only compiled when the `wasm-workers` feature is enabled
//! AND the target is `wasm32-unknown-unknown`.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

/// A wasm-compatible task processor that uses Web Workers.
///
/// In wasm builds, this creates and communicates with actual Web Workers
/// via the browser's `Worker` API. Each worker runs in its own thread
/// and communicates via `postMessage` / `onmessage`.
///
/// In native builds, the equivalent is [`TaskProcessor`](super::task_processor::TaskProcessor)
/// which uses `std::thread::spawn`.
pub struct WasmTaskProcessor {
    /// The worker module URL or blob URL.
    worker_url: String,
    /// The maximum number of concurrent workers.
    maximum_active_tasks: usize,
    /// The number of currently active tasks.
    active_tasks: Arc<Mutex<usize>>,
    /// Whether this processor has been destroyed.
    is_destroyed: bool,
    /// Pending task callbacks (indexed by task ID).
    pending_tasks: Arc<Mutex<Vec<Option<PendingTask>>>>,
}

/// A pending task waiting for a result from a Web Worker.
struct PendingTask {
    /// The task ID assigned by the processor.
    task_id: u32,
    /// Whether this task has been completed.
    completed: bool,
    /// The result data (set when the worker responds).
    result: Option<Vec<u8>>,
}

/// The result of a wasm task.
pub type WasmTaskResult = Result<Vec<u8>, String>;

impl WasmTaskProcessor {
    /// Creates a new WasmTaskProcessor.
    ///
    /// # Arguments
    /// * `worker_url` - The URL of the worker script (typically a blob URL).
    pub fn new(worker_url: &str) -> Self {
        Self {
            worker_url: worker_url.to_string(),
            maximum_active_tasks: 4,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
            pending_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a new WasmTaskProcessor with a custom maximum active tasks.
    pub fn with_max_tasks(worker_url: &str, maximum_active_tasks: usize) -> Self {
        Self {
            worker_url: worker_url.to_string(),
            maximum_active_tasks,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
            pending_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Schedules a task for processing in a Web Worker.
    ///
    /// In CesiumJS, this posts a message to a Web Worker with the parameters
    /// and a task ID. The worker processes the task and posts the result back.
    ///
    /// # Arguments
    /// * `parameters` - The task parameters (serialized as bytes).
    ///
    /// Returns a task ID that can be used to retrieve the result, or `None`
    /// if the processor is destroyed or at capacity.
    pub fn schedule_task(&mut self, parameters: &[u8]) -> Option<u32> {
        if self.is_destroyed {
            return None;
        }

        let mut active = self.active_tasks.lock().unwrap();
        if *active >= self.maximum_active_tasks {
            return None;
        }
        *active += 1;
        drop(active);

        // DEVIATION: In production wasm build, this would:
        // 1. Create or reuse a Web Worker
        // 2. Post a message with { taskId, parameters }
        // 3. Register a callback for the response
        // For now, we assign a task ID and store the pending state.
        let mut pending = self.pending_tasks.lock().unwrap();
        let task_id = pending.len() as u32;
        pending.push(Some(PendingTask {
            task_id,
            completed: false,
            result: None,
        }));

        // In a real wasm implementation:
        // self.post_message_to_worker(task_id, parameters);
        let _ = parameters;

        Some(task_id)
    }

    /// Called when a Web Worker completes a task.
    ///
    /// This is the message handler for `onmessage` events from workers.
    /// It stores the result and decrements the active task count.
    pub fn on_task_complete(&self, task_id: u32, result: WasmTaskResult) {
        let mut pending = self.pending_tasks.lock().unwrap();
        if let Some(Some(task)) = pending.get_mut(task_id as usize) {
            task.completed = true;
            task.result = result.ok();
        }
        drop(pending);

        let mut active = self.active_tasks.lock().unwrap();
        *active = active.saturating_sub(1);
    }

    /// Returns the worker URL.
    pub fn worker_url(&self) -> &str {
        &self.worker_url
    }

    /// Returns the number of active tasks.
    pub fn active_tasks_count(&self) -> usize {
        *self.active_tasks.lock().unwrap()
    }

    /// Returns whether this processor has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this processor and terminates all workers.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
        // In production: terminate all Web Workers
        let mut pending = self.pending_tasks.lock().unwrap();
        pending.clear();
    }

    /// Returns whether the browser supports Web Workers.
    ///
    /// In CesiumJS, this checks `typeof Worker !== 'undefined'`.
    pub fn is_web_worker_supported() -> bool {
        // DEVIATION: In production wasm build, this would check:
        // web_sys::window().map_or(false, |w| w.workers().is_some())
        cfg!(target_arch = "wasm32")
    }

    /// Creates a blob URL for a worker script.
    ///
    /// In CesiumJS, this creates a Blob from the worker script content
    /// and generates a URL via `URL.createObjectURL`.
    pub fn create_worker_blob_url(_script_content: &str) -> String {
        // DEVIATION: In production wasm build, this would:
        // 1. Create a Blob from the script content
        // 2. Call URL.createObjectURL(blob)
        // 3. Return the blob URL
        String::from("blob:worker-placeholder")
    }
}

impl Default for WasmTaskProcessor {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Trait abstracting the worker backend (native threads vs wasm web workers).
///
/// This allows the rest of the codebase to use workers without caring about
/// the underlying implementation. On native, `TaskProcessor` implements this;
/// on wasm, `WasmTaskProcessor` implements this.
pub trait WorkerBackend {
    /// Schedules a task and returns a handle or ID.
    fn schedule(&mut self, parameters: Vec<u8>) -> Option<u32>;

    /// Returns the number of active tasks.
    fn active_count(&self) -> usize;

    /// Returns whether the backend has been destroyed.
    fn destroyed(&self) -> bool;

    /// Destroys the backend and releases resources.
    fn destroy_backend(&mut self);
}

impl WorkerBackend for WasmTaskProcessor {
    fn schedule(&mut self, parameters: Vec<u8>) -> Option<u32> {
        self.schedule_task(&parameters)
    }

    fn active_count(&self) -> usize {
        self.active_tasks_count()
    }

    fn destroyed(&self) -> bool {
        self.is_destroyed()
    }

    fn destroy_backend(&mut self) {
        self.destroy();
    }
}
