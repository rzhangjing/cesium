//! viewer-demo — Minimal Cesium viewer replicating Sandcastle HelloWorld.
//!
//! Creates a winit window, initializes wgpu, and runs a frame loop that
//! renders the Cesium scene (clearing to the scene background color each frame).
//!
//! Equivalent to the CesiumJS HelloWorld:
//! ```html
//! <div id="cesiumContainer"></div>
//! <script>
//!   const viewer = new Cesium.Viewer("cesiumContainer");
//! </script>
//! ```

use cesium_widgets::viewer::Viewer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use std::sync::Arc;

/// The application state, holding GPU resources and the Cesium viewer.
struct State {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    viewer: Option<Viewer>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
}

impl State {
    /// Creates an uninitialized State (no GPU resources yet).
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            viewer: None,
            surface_config: None,
        }
    }

    /// Initializes GPU resources and the Cesium viewer.
    ///
    /// Called from `resumed()` once the event loop provides a valid display handle.
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();

        // ── wgpu initialization ──────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("create_surface failed");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        log::info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("cesium_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("request_device failed");

        // ── Surface configuration ────────────────────────────────────
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            color_space: wgpu::SurfaceColorSpace::Auto,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        // ── Cesium Viewer ────────────────────────────────────────────
        // In CesiumJS: const viewer = new Cesium.Viewer("cesiumContainer");
        let viewer = Viewer::default();

        log::info!(
            "viewer-demo initialized: {}x{}, format {:?}",
            size.width,
            size.height,
            surface_format
        );

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.viewer = Some(viewer);
        self.surface_config = Some(surface_config);
    }

    /// Renders a single frame.
    ///
    /// Acquires the next surface texture, creates a render pass that clears
    /// to the scene's background color, and presents the frame.
    fn render(&mut self) {
        let surface = self.surface.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let config = self.surface_config.as_ref().unwrap();
        let viewer = self.viewer.as_ref().unwrap();

        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(device, config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Validation => {
                log::warn!("Surface texture lost or validation error; reconfiguring.");
                surface.configure(device, config);
                return;
            }
        };

        let bg = viewer.cesium_widget().scene().background_color();
        let clear_color = wgpu::Color {
            r: bg.red,
            g: bg.green,
            b: bg.blue,
            a: bg.alpha,
        };

        let texture_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("cesium_frame_encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cesium_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // DEVIATION: Full scene render pipeline requires primitive traversal
            // and wgpu pipeline setup. The clear color demonstrates the scene
            // background_color is flowing through correctly.
        }

        queue.submit(std::iter::once(encoder.finish()));
        queue.present(frame);

        // Update the viewer (clock, data sources) for the next frame
        self.viewer.as_mut().unwrap().render();
    }

    /// Handles window resize by reconfiguring the surface and notifying the viewer.
    fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            if let Some(ref mut viewer) = self.viewer {
                viewer.resize(new_width, new_height);
            }
            if let Some(config) = self.surface_config.as_mut() {
                config.width = new_width;
                config.height = new_height;
            }
            if let (Some(surface), Some(device), Some(config)) =
                (&self.surface, &self.device, &self.surface_config)
            {
                surface.configure(device, config);
            }
        }
    }
}

impl ApplicationHandler for State {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // On desktop, resumed is called once after the event loop starts.
        // Initialize GPU resources if not already done.
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32))
                            .with_min_inner_size(winit::dpi::LogicalSize::new(256u32, 256u32))
                            .with_title("cesium-rs Viewer Demo"),
                    )
                    .expect("Failed to create window"),
            );

            // pollster::block_on is safe here because we're not inside an async context.
            pollster::block_on(self.init_gpu(window));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(ref window) = self.window {
            if window_id != window.id() {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested, shutting down.");
                if let Some(ref mut viewer) = self.viewer {
                    viewer.destroy();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size.width, physical_size.height);
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Continuous rendering (equivalent to CesiumJS requestAnimationFrame)
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new();

    if let Err(e) = event_loop.run_app(&mut state) {
        log::error!("Event loop error: {:?}", e);
    }
}
