//! Ported from `packages/engine/Source/Workers/TaskProcessor.js`.
//!
//! Manages offloading computation to a thread pool.
//!
//! In CesiumJS, `TaskProcessor` drives a Web Worker and exposes a
//! module-level `taskCompletedEvent` raised after every finished task
//! (with the error, if any). In Rust, tasks are scheduled on rayon's
//! global thread pool and each processor owns its `task_completed_event`.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, LazyLock, Mutex};

use cesium_core::event::Event;

/// A pure worker function: serialized parameters in, serialized result out.
///
/// Mirrors the function passed to CesiumJS `createTaskProcessorWorker`.
pub type WorkerFn = fn(&[u8]) -> Result<Vec<u8>, String>;

/// Registry for worker functions that are not part of the built-in
/// dispatch table (used by tests and embedders to inject custom workers).
static CUSTOM_WORKERS: LazyLock<Mutex<HashMap<String, WorkerFn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Registers a custom worker function under `name`.
///
/// DEVIATION: CesiumJS loads worker scripts by URL; Rust dispatches by
/// name through this registry plus the built-in table in
/// [`process_worker_task`].
pub fn register_worker(name: &str, worker: WorkerFn) {
    CUSTOM_WORKERS.lock().unwrap().insert(name.to_string(), worker);
}

/// Removes a previously registered custom worker. Returns whether it existed.
pub fn unregister_worker(name: &str) -> bool {
    CUSTOM_WORKERS.lock().unwrap().remove(name).is_some()
}

/// Manages offloading computation to a thread pool.
///
/// In CesiumJS, this uses Web Workers. In Rust, this uses rayon's thread
/// pool. Mirrors CesiumJS `TaskProcessor` (400 lines).
pub struct TaskProcessor {
    /// The worker script/module name.
    worker_name: String,
    /// The maximum number of active tasks. Once exceeded, `schedule_task`
    /// returns `None` (CesiumJS: `maximumActiveTasks`, default +Infinity;
    /// here the default is 4).
    maximum_active_tasks: usize,
    /// The number of currently active tasks (`this._activeTasks`).
    active_tasks: Arc<Mutex<usize>>,
    /// Whether this processor has been destroyed.
    is_destroyed: bool,
    /// Raised when a task completes; carries the error message if the task
    /// failed (CesiumJS module-level `taskCompletedEvent`).
    ///
    /// DEVIATION: CesiumJS raises the event on the main thread as soon as
    /// the worker responds; `cesium_core::event::Event` is not `Send`, so
    /// the event is raised when the result is consumed via
    /// [`TaskHandle::wait`] / [`TaskHandle::try_get`].
    task_completed_event: Event<Option<String>>,
}

/// A handle to a pending task.
pub struct TaskHandle<'a> {
    /// The receiver for the task result.
    receiver: mpsc::Receiver<TaskResult>,
    /// The processor's task-completed event, raised on result consumption.
    task_completed_event: &'a Event<Option<String>>,
}

/// The result of a task.
pub type TaskResult = Result<Vec<u8>, String>;

impl TaskProcessor {
    /// Creates a new TaskProcessor.
    pub fn new(worker_name: &str) -> Self {
        Self {
            worker_name: worker_name.to_string(),
            maximum_active_tasks: 4,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
            task_completed_event: Event::new(),
        }
    }

    /// Creates a new TaskProcessor with a custom maximum active tasks.
    pub fn with_max_tasks(worker_name: &str, maximum_active_tasks: usize) -> Self {
        Self {
            worker_name: worker_name.to_string(),
            maximum_active_tasks,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
            task_completed_event: Event::new(),
        }
    }

    /// Schedules a task for processing.
    ///
    /// Returns `None` when the processor is destroyed or already has
    /// `maximum_active_tasks` tasks running (CesiumJS `scheduleTask`
    /// returns `undefined` in the latter case).
    pub fn schedule_task(&self, parameters: Vec<u8>) -> Option<TaskHandle<'_>> {
        if self.is_destroyed {
            return None;
        }

        let mut active = self.active_tasks.lock().unwrap();
        if *active >= self.maximum_active_tasks {
            return None; // Too many active tasks
        }
        *active += 1;
        drop(active);

        let active_tasks = Arc::clone(&self.active_tasks);
        let worker_name = self.worker_name.clone();
        let (sender, receiver) = mpsc::channel();

        // Offload to rayon's global thread pool (the Rust analogue of the
        // CesiumJS Web Worker owned by this processor).
        rayon::spawn(move || {
            let result = process_worker_task(&worker_name, &parameters);
            let _ = sender.send(result);
            let mut active = active_tasks.lock().unwrap();
            *active = active.saturating_sub(1);
        });

        Some(TaskHandle {
            receiver,
            task_completed_event: &self.task_completed_event,
        })
    }

    /// Returns the worker name.
    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    /// Returns the number of active tasks.
    pub fn active_tasks_count(&self) -> usize {
        *self.active_tasks.lock().unwrap()
    }

    /// Returns the maximum number of active tasks.
    pub fn maximum_active_tasks(&self) -> usize {
        self.maximum_active_tasks
    }

    /// The event raised whenever a task of this processor completes.
    ///
    /// Port of the CesiumJS module-level `taskCompletedEvent`.
    pub fn task_completed_event(&self) -> &Event<Option<String>> {
        &self.task_completed_event
    }

    /// Returns whether this processor has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this processor.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl TaskHandle<'_> {
    /// Waits for the task to complete and returns the result.
    pub fn wait(self) -> TaskResult {
        let result = self
            .receiver
            .recv()
            .unwrap_or(Err("Channel closed".to_string()));
        self.raise_completed(&result);
        result
    }

    /// Tries to get the result without blocking.
    pub fn try_get(&self) -> Option<TaskResult> {
        let result = self.receiver.try_recv().ok();
        if let Some(ref r) = result {
            self.raise_completed(r);
        }
        result
    }

    fn raise_completed(&self, result: &TaskResult) {
        match result {
            Ok(_) => self.task_completed_event.raise_event(&None),
            Err(e) => self.task_completed_event.raise_event(&Some(e.clone())),
        }
    }
}

/// Internal task processing function: dispatches to the worker function
/// registered under `worker_name`.
///
/// In CesiumJS, the worker script identified by `workerPath` runs in a Web
/// Worker. In Rust, `worker_name` selects a pure function from the built-in
/// table (the ported `Source/Workers/*` entry points) or from the custom
/// [`register_worker`] registry.
pub fn process_worker_task(worker_name: &str, parameters: &[u8]) -> TaskResult {
    // Custom-registered workers take precedence (test/embedder injection).
    // Copy the fn pointer out so the registry lock is released before the
    // worker runs (the lock must never be held across task execution).
    let custom = CUSTOM_WORKERS.lock().unwrap().get(worker_name).copied();
    if let Some(worker) = custom {
        return worker(parameters);
    }

    macro_rules! dispatch {
        ($($name:literal => $fun:path,)*) => {
            match worker_name {
                $($name => return Ok($fun(parameters)),)*
                _ => {}
            }
        };
    }

    // Built-in dispatch table, mirroring CesiumJS worker module names
    // (camelCase JS module ↔ snake_case Rust function).
    use crate as w;
    if matches!(worker_name, "createTaskProcessorWorker" | "noop") {
        return w::create_task_processor_worker::noop_worker(parameters);
    }
    dispatch! {
        "createBoxGeometry" => w::create_box_geometry::create_box_geometry,
        "createBoxOutlineGeometry" => w::create_box_outline_geometry::create_box_outline_geometry,
        "createCircleGeometry" => w::create_circle_geometry::create_circle_geometry,
        "createCircleOutlineGeometry" => w::create_circle_outline_geometry::create_circle_outline_geometry,
        "createCoplanarPolygonGeometry" => w::create_coplanar_polygon_geometry::create_coplanar_polygon_geometry,
        "createCoplanarPolygonOutlineGeometry" => w::create_coplanar_polygon_outline_geometry::create_coplanar_polygon_outline_geometry,
        "createCorridorGeometry" => w::create_corridor_geometry::create_corridor_geometry,
        "createCorridorOutlineGeometry" => w::create_corridor_outline_geometry::create_corridor_outline_geometry,
        "createCylinderGeometry" => w::create_cylinder_geometry::create_cylinder_geometry,
        "createCylinderOutlineGeometry" => w::create_cylinder_outline_geometry::create_cylinder_outline_geometry,
        "createEllipseGeometry" => w::create_ellipse_geometry::create_ellipse_geometry,
        "createEllipseOutlineGeometry" => w::create_ellipse_outline_geometry::create_ellipse_outline_geometry,
        "createEllipsoidGeometry" => w::create_ellipsoid_geometry::create_ellipsoid_geometry,
        "createEllipsoidOutlineGeometry" => w::create_ellipsoid_outline_geometry::create_ellipsoid_outline_geometry,
        "createFrustumGeometry" => w::create_frustum_geometry::create_frustum_geometry,
        "createFrustumOutlineGeometry" => w::create_frustum_outline_geometry::create_frustum_outline_geometry,
        "createGeometry" => w::create_geometry::create_geometry,
        "createGroundPolylineGeometry" => w::create_ground_polyline_geometry::create_ground_polyline_geometry,
        "createPlaneGeometry" => w::create_plane_geometry::create_plane_geometry,
        "createPlaneOutlineGeometry" => w::create_plane_outline_geometry::create_plane_outline_geometry,
        "createPolygonGeometry" => w::create_polygon_geometry::create_polygon_geometry,
        "createPolygonOutlineGeometry" => w::create_polygon_outline_geometry::create_polygon_outline_geometry,
        "createPolylineGeometry" => w::create_polyline_geometry::create_polyline_geometry,
        "createPolylineVolumeGeometry" => w::create_polyline_volume_geometry::create_polyline_volume_geometry,
        "createPolylineVolumeOutlineGeometry" => w::create_polyline_volume_outline_geometry::create_polyline_volume_outline_geometry,
        "createRectangleGeometry" => w::create_rectangle_geometry::create_rectangle_geometry,
        "createRectangleOutlineGeometry" => w::create_rectangle_outline_geometry::create_rectangle_outline_geometry,
        "createSimplePolylineGeometry" => w::create_simple_polyline_geometry::create_simple_polyline_geometry,
        "createSphereGeometry" => w::create_sphere_geometry::create_sphere_geometry,
        "createSphereOutlineGeometry" => w::create_sphere_outline_geometry::create_sphere_outline_geometry,
        "createVectorTileClampedPolylines" => w::create_vector_tile_clamped_polylines::create_vector_tile_clamped_polylines,
        "createVectorTileGeometries" => w::create_vector_tile_geometries::create_vector_tile_geometries,
        "createVectorTilePoints" => w::create_vector_tile_points::create_vector_tile_points,
        "createVectorTilePolygons" => w::create_vector_tile_polygons::create_vector_tile_polygons,
        "createVectorTilePolylines" => w::create_vector_tile_polylines::create_vector_tile_polylines,
        "createVerticesFromCesium3DTilesTerrain" => w::create_vertices_from_cesium3_d_tiles_terrain::create_vertices_from_cesium3_d_tiles_terrain,
        "createVerticesFromGoogleEarthEnterpriseBuffer" => w::create_vertices_from_google_earth_enterprise_buffer::create_vertices_from_google_earth_enterprise_buffer,
        "createVerticesFromHeightmap" => w::create_vertices_from_heightmap::create_vertices_from_heightmap,
        "createVerticesFromQuantizedTerrainMesh" => w::create_vertices_from_quantized_terrain_mesh::create_vertices_from_quantized_terrain_mesh,
        "createWallGeometry" => w::create_wall_geometry::create_wall_geometry,
        "createWallOutlineGeometry" => w::create_wall_outline_geometry::create_wall_outline_geometry,
    }

    Err(format!("Unknown worker: {worker_name}"))
}

impl Default for TaskProcessor {
    fn default() -> Self {
        Self::new("default")
    }
}
