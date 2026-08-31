//! Ported from `packages/engine/Source/Scene/Scene.js`.
//!
//! The main 3D scene containing the globe, primitives, and camera.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;

use cesium_core::cartesian2::Cartesian2;
use cesium_core::color::Color;
use cesium_core::credit::Credit;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::Event;
use cesium_core::julian_date::JulianDate;
use cesium_core::matrix4::Matrix4;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::pixel_format::PixelFormat;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::clear_command::ClearCommand;
use cesium_renderer::context::{Context, DefaultRenderTarget};
use cesium_renderer::draw_command::{DrawCommand, UniformValue};
use cesium_renderer::framebuffer::{Framebuffer, FramebufferOptions};
use cesium_renderer::pass::Pass;
use cesium_renderer::pixel_datatype::PixelDatatype;
use cesium_renderer::render_state::{BlendEquation, BlendingFactor, RenderState};
use cesium_renderer::shader_program::ShaderProgram;
use cesium_renderer::texture::{Texture, TextureOptions};
use cesium_renderer::uniform_state::UniformState;
use cesium_renderer::vertex_array::{VertexArray, VertexAttribute};
use cesium_shaders::wgsl;
use crate::camera::Camera;
use crate::camera_flight_path::{
    CameraFlightChannel, CameraFlightPath, CameraFlightTweenOptions,
};
use crate::credit_display::CreditDisplay;
use crate::frame_state::FrameState;
use crate::globe::Globe;
use crate::primitive_collection::PrimitiveCollection;
use crate::scene_mode::SceneMode;
use crate::scene_transforms::SceneTransforms;
use crate::tween_collection::{TweenCollection, TweenOptions};
use crate::viewport_quad::ViewportQuad;

/// The main 3D scene.
///
/// Contains the globe, camera, primitives, and manages the render loop.
/// This is the largest single module in CesiumJS (172KB).
pub struct Scene {
    camera: Camera,
    globe: Option<Globe>,
    frame_state: FrameState,
    /// The credit display (mirrors CesiumJS `frameState.creditDisplay`).
    /// `RefCell` so the widgets-facing `&self` credit methods work.
    credit_display: RefCell<CreditDisplay>,
    /// The current scene mode (`Cell`: the morph API is `&self`, mirroring
    /// the JS where `morphTo2D/3D/ColumbusView` mutate through the shared
    /// scene reference).
    mode: Cell<SceneMode>,
    /// The morph transition time.
    morph_time: Cell<f64>,
    /// The event raised when a morph transition starts (mirrors CesiumJS
    /// `Scene#morphStart`; the JS payload is
    /// `(transitioner, oldMode, newMode, isMorphing)`, the port raises the
    /// new mode).
    morph_start: Event<SceneMode>,
    /// The event raised when a morph transition completes (mirrors CesiumJS
    /// `Scene#morphComplete`, raised by the JS `SceneTransitioner` in
    /// `completeMorph`; the JS payload is
    /// `(transitioner, previousMode, newMode)`, the port raises the new
    /// mode).
    morph_complete: Event<SceneMode>,
    /// Mirrors the CesiumJS `Scene#useWebVR` flag. DEVIATION: the flag is
    /// stored and readable, but the stereo/VR frustum handling has no
    /// headless analogue.
    use_web_vr: Cell<bool>,
    background_color: Color,
    /// Full-screen smoke primitive.
    ///
    /// DEVIATION (B3.2): CesiumJS has no built-in scene viewport quad —
    /// applications add one as a post-process primitive. The wgpu smoke
    /// milestone wires it into the scene directly so the frame
    /// orchestration (clear → draw → execute) is exercised end to end.
    viewport_quad: ViewportQuad,
    /// Offscreen globe pass target (color + depth). Lazily created and
    /// rebuilt when the drawing buffer size changes; the globe tiles draw
    /// into it and the result is blitted onto the default target (the
    /// default target carries no depth attachment in the wgpu port).
    globe_framebuffer: Option<Arc<Framebuffer>>,
    /// Size the `globe_framebuffer` was created at.
    globe_target_size: (u32, u32),
    /// Private full-screen quad compositing the globe pass onto the
    /// default target.
    globe_blit: GlobeBlitQuad,
    /// The scene's primitive collection (mirrors CesiumJS
    /// `Scene#primitives`, the default container for user primitives).
    primitives: PrimitiveCollection,
    /// The event raised at the beginning of a render pass (mirrors CesiumJS
    /// `Scene#preRender`; takes the simulation time).
    pre_render: Event<JulianDate>,
    /// The event raised at the end of a render pass (mirrors CesiumJS
    /// `Scene#postRender`; takes the simulation time).
    post_render: Event<JulianDate>,
    /// The tween animations (mirrors CesiumJS `Scene#tweens`, a
    /// `TweenCollection`). `RefCell` so [`Scene::fly_to`] can add flight
    /// tweens through `&self` (the widgets traits require `&self`).
    tweens: RefCell<TweenCollection>,
    /// The camera-flight channel shared with the camera (see
    /// [`crate::camera_flight_path`]).
    flight_channel: CameraFlightChannel,
    /// The id of the active flight tween, if any (a new flight cancels it).
    current_flight_tween: RefCell<Option<u64>>,
    debug_show_frames_per_second: bool,
    is_destroyed: bool,
}

impl Scene {
    /// Creates a new scene.
    pub fn new() -> Self {
        // The flight channel is shared between the scene (writes the flight)
        // and the camera (applies the pose each update), mirroring the JS
        // `camera.flyTo` tween closure capturing the camera instance.
        let flight_channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let mut camera = Camera::default();
        camera.set_flight_channel(flight_channel.clone());
        Self {
            camera,
            globe: None,
            frame_state: FrameState::default(),
            credit_display: RefCell::new(CreditDisplay::default()),
            mode: Cell::new(SceneMode::Scene3D),
            morph_time: Cell::new(1.0),
            morph_start: Event::new(),
            morph_complete: Event::new(),
            use_web_vr: Cell::new(false),
            background_color: Color::new(0.0, 0.0, 0.0, 1.0),
            viewport_quad: ViewportQuad::with_color([1.0, 0.0, 0.0, 1.0]),
            globe_framebuffer: None,
            globe_target_size: (0, 0),
            globe_blit: GlobeBlitQuad::new(),
            primitives: PrimitiveCollection::new(),
            pre_render: Event::new(),
            post_render: Event::new(),
            tweens: RefCell::new(TweenCollection::new()),
            flight_channel,
            current_flight_tween: RefCell::new(None),
            debug_show_frames_per_second: false,
            is_destroyed: false,
        }
    }

    /// Returns the camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Returns a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Returns the globe, if any.
    pub fn globe(&self) -> Option<&Globe> { self.globe.as_ref() }

    /// Returns a mutable reference to the globe, if any.
    pub fn globe_mut(&mut self) -> Option<&mut Globe> { self.globe.as_mut() }

    /// Sets the globe.
    pub fn set_globe(&mut self, globe: Option<Globe>) { self.globe = globe; }

    /// Returns the current scene mode.
    pub fn mode(&self) -> SceneMode { self.mode.get() }

    /// Sets the scene mode.
    pub fn set_mode(&mut self, mode: SceneMode) {
        self.mode.set(mode);
        self.morph_time.set(SceneMode::get_morph_time(mode).unwrap_or(0.0));
    }

    /// Returns the morph time.
    pub fn morph_time(&self) -> f64 { self.morph_time.get() }

    /// Returns the `morphStart` event (mirrors CesiumJS `Scene#morphStart`).
    pub fn morph_start(&self) -> &Event<SceneMode> { &self.morph_start }

    /// Returns the `morphComplete` event (mirrors CesiumJS
    /// `Scene#morphComplete`).
    pub fn morph_complete(&self) -> &Event<SceneMode> { &self.morph_complete }

    /// Completes the current morph transition immediately (mirrors CesiumJS
    /// `Scene#completeMorph`).
    ///
    /// DEVIATION: the JS delegates to `SceneTransitioner#completeMorph`,
    /// which snaps the camera/morph uniforms to the target mode mid-flight;
    /// the port's morph path already completes synchronously, so this
    /// resets `morphTime` to the current mode's default and re-raises
    /// `morphComplete` (the observable state after a JS `completeMorph`).
    pub fn complete_morph(&self) {
        let mode = self.mode.get();
        self.morph_time
            .set(SceneMode::get_morph_time(mode).unwrap_or(1.0));
        self.morph_complete.raise_event(&mode);
    }

    /// Starts morphing to 2D (mirrors CesiumJS `Scene#morphTo2D`).
    pub fn morph_to_2d(&self, duration: f64) {
        self.morph(SceneMode::Scene2D, duration);
    }

    /// Starts morphing to 3D (mirrors CesiumJS `Scene#morphTo3D`).
    pub fn morph_to_3d(&self, duration: f64) {
        self.morph(SceneMode::Scene3D, duration);
    }

    /// Starts morphing to Columbus View (mirrors CesiumJS
    /// `Scene#morphToColumbusView`).
    pub fn morph_to_columbus_view(&self, duration: f64) {
        self.morph(SceneMode::ColumbusView, duration);
    }

    /// The shared morph path (mirrors CesiumJS `Scene#morph` →
    /// `SceneTransitioner#morphTo*`).
    ///
    /// DEVIATION: the JS spreads the transition across frames through the
    /// `SceneTransitioner` (camera interpolation over `duration`, morph
    /// uniforms 0→1); the port completes the transition synchronously: the
    /// mode is set, `morphStart` fires with the new mode and the requested
    /// duration as the morph time, then the morph time returns to the
    /// mode's default and `morphComplete` fires (the JS `completeMorph`
    /// behavior).
    fn morph(&self, mode: SceneMode, duration: f64) {
        self.mode.set(mode);
        self.morph_time.set(duration);
        self.morph_start.raise_event(&mode);
        self.morph_time
            .set(SceneMode::get_morph_time(mode).unwrap_or(1.0));
        self.morph_complete.raise_event(&mode);
    }

    /// Returns the `useWebVR` flag (mirrors CesiumJS `Scene#useWebVR`).
    pub fn use_web_vr(&self) -> bool { self.use_web_vr.get() }

    /// Assigns the `useWebVR` flag through `&self` (mirrors the widget's
    /// `scene.useWebVR = value` write; the VrScene trait requires `&self`).
    pub fn set_use_web_vr(&self, value: bool) {
        self.use_web_vr.set(value);
    }

    /// Converts a world position to window coordinates using the current
    /// camera (mirrors CesiumJS
    /// `SceneTransforms.worldToWindowCoordinates(scene, position)`).
    pub fn world_to_window_coordinates(
        &self,
        position: &Cartesian3,
    ) -> Option<Cartesian2> {
        SceneTransforms::world_to_window_with_camera(position, &self.camera)
    }

    /// Returns the frame state.
    pub fn frame_state(&self) -> &FrameState { &self.frame_state }

    /// Returns a mutable reference to the smoke viewport quad.
    pub fn viewport_quad_mut(&mut self) -> &mut ViewportQuad { &mut self.viewport_quad }

    /// Returns the scene's primitive collection (mirrors CesiumJS
    /// `Scene#primitives`).
    pub fn primitives(&self) -> &PrimitiveCollection { &self.primitives }

    /// Returns a mutable reference to the scene's primitive collection.
    pub fn primitives_mut(&mut self) -> &mut PrimitiveCollection { &mut self.primitives }

    /// Returns the credit display (mirrors CesiumJS
    /// `frameState.creditDisplay`).
    pub fn credit_display(&self) -> Ref<'_, CreditDisplay> {
        self.credit_display.borrow()
    }

    /// Adds a static credit (mirrors the widget-facing
    /// `frameState.creditDisplay.addStaticCredit`).
    pub fn add_static_credit(&self, credit: Credit) {
        self.credit_display.borrow_mut().add_static_credit(credit);
    }

    /// Removes a static credit (mirrors the widget-facing
    /// `frameState.creditDisplay.removeStaticCredit`).
    pub fn remove_static_credit(&self, credit: &Credit) {
        self.credit_display.borrow_mut().remove_static_credit(credit);
    }

    /// Returns whether the credit display is destroyed (mirrors the
    /// widget-facing `frameState.creditDisplay.isDestroyed()`).
    pub fn credit_display_is_destroyed(&self) -> bool {
        self.credit_display.borrow().is_destroyed()
    }

    /// Returns the background color.
    pub fn background_color(&self) -> &Color { &self.background_color }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) { self.background_color = color; }

    /// Returns the `preRender` event (raised with the simulation time at the
    /// beginning of each render; mirrors CesiumJS `Scene#preRender`).
    pub fn pre_render(&self) -> &Event<JulianDate> { &self.pre_render }

    /// Returns the `postRender` event (raised with the simulation time at
    /// the end of each render; mirrors CesiumJS `Scene#postRender`).
    pub fn post_render(&self) -> &Event<JulianDate> { &self.post_render }

    /// Returns the scene's tween collection (mirrors CesiumJS `Scene#tweens`).
    pub fn tweens(&self) -> Ref<'_, TweenCollection> { self.tweens.borrow() }

    /// Returns a mutable handle to the scene's tween collection.
    pub fn tweens_mut(&self) -> RefMut<'_, TweenCollection> { self.tweens.borrow_mut() }

    /// Flies the camera to the given destination (mirrors CesiumJS
    /// `camera.flyTo({ destination, duration, complete })`, invoked through
    /// the scene because the widgets traits take `&self`).
    ///
    /// Signature: `fly_to(&self, destination: Cartesian3, duration:
    /// Option<f64>, complete: Option<Box<dyn FnOnce()>>)`. The tween is
    /// built by [`CameraFlightPath::create_tween`] (the JS
    /// `CameraFlightPath.createTween` entry point), so `duration` defaults
    /// to the JS distance-derived value and the end orientation looks
    /// straight down at the destination (the JS `setView` default).
    pub fn fly_to(
        &self,
        destination: Cartesian3,
        duration: Option<f64>,
        complete: Option<Box<dyn FnOnce()>>,
    ) {
        let tween = CameraFlightPath::create_tween(
            &self.camera,
            &self.flight_channel,
            CameraFlightTweenOptions {
                destination,
                duration,
                easing_function: None,
                complete,
                cancel: None,
            },
        );
        self.start_flight(tween);
    }

    /// Flies the camera to the home view (mirrors CesiumJS `camera.flyHome`):
    /// a position on the +X axis far enough that the whole WGS84 ellipsoid
    /// fits in the vertical field of view, looking at the center.
    ///
    /// Signature: `fly_home(&self, duration: Option<f64>)`. The home
    /// destination's straight-down pose (built by
    /// [`CameraFlightPath::create_tween`]) looks at the ellipsoid center
    /// (direction -X, up +Z), matching the JS `flyHome` end view.
    pub fn fly_home(&self, duration: Option<f64>) {
        let radius = Ellipsoid::WGS84.maximum_radius();
        let distance = radius / (self.camera.fov() * 0.5).sin();
        let tween = CameraFlightPath::create_tween(
            &self.camera,
            &self.flight_channel,
            CameraFlightTweenOptions {
                destination: Cartesian3::new(distance, 0.0, 0.0),
                duration,
                easing_function: None,
                complete: None,
                cancel: None,
            },
        );
        self.start_flight(tween);
    }

    /// Installs a flight tween, superseding any in-flight flight first.
    ///
    /// Uses `remove` (not `cancel`) on the previous tween so its cancel
    /// callback does not clear the channel the new flight is about to
    /// install (mirrors the JS behavior where a new flyTo simply supersedes
    /// the in-flight tween).
    fn start_flight(&self, tween: TweenOptions) {
        if let Some(id) = self.current_flight_tween.borrow_mut().take() {
            self.tweens.borrow_mut().remove(id);
        }
        let id = self.tweens.borrow_mut().add(tween);
        *self.current_flight_tween.borrow_mut() = Some(id);
    }

    /// Updates the scene for the current frame.
    pub fn update(&mut self, time: &JulianDate) {
        self.frame_state.time = time.clone();
        self.frame_state.frame_number += 1;
        self.frame_state.mode = self.mode.get();
        self.frame_state.morph_time = self.morph_time.get();

        // B4-1: refresh the camera matrices and propagate them into the
        // frame state (mirrors CesiumJS `Scene#updateFrameState` reading
        // `camera.viewMatrix` / `camera.frustum.projectionMatrix`).
        if self.frame_state.drawing_buffer_width > 0
            && self.frame_state.drawing_buffer_height > 0
        {
            self.camera.set_canvas_size(
                self.frame_state.drawing_buffer_width,
                self.frame_state.drawing_buffer_height,
            );
        }
        self.camera.update();
        self.frame_state.view_matrix = self.camera.view_matrix().clone();
        self.frame_state.inverse_view_matrix = self.camera.inverse_view_matrix().clone();
        self.frame_state.projection_matrix = self.camera.projection_matrix().clone();
        self.frame_state.inverse_projection_matrix =
            self.camera.inverse_projection_matrix().clone();
        self.frame_state.view_projection_matrix = Matrix4::multiply_new(
            &self.frame_state.projection_matrix,
            &self.frame_state.view_matrix,
        );
        self.frame_state.camera_position = *self.camera.position();
        self.frame_state.camera_direction = *self.camera.direction();
        self.frame_state.camera_up = *self.camera.up();
        self.frame_state.camera_right = *self.camera.right();
        self.frame_state.sse_denominator = self.camera.sse_denominator();

        self.credit_display.borrow_mut().begin_frame();
        // DEVIATION: Full update pipeline requires primitive collection traversal
        self.credit_display.borrow_mut().end_frame();
    }

    /// Renders the scene.
    ///
    /// Mirrors the CesiumJS render sequence: `preRender` event → tween
    /// updates (advancing camera flights) → scene update → `postRender`
    /// event.
    pub fn render(&mut self, time: &JulianDate) {
        self.pre_render.raise_event(time);
        self.tweens.borrow_mut().update(time);
        self.update(time);
        self.post_render.raise_event(time);
        // DEVIATION: Full render pipeline requires wgpu render pass creation
    }

    /// Renders the scene through a wgpu [`Context`].
    ///
    /// DEVIATION (B3.2): CesiumJS `Scene.render(time)` drives its own
    /// context and default framebuffer; the wgpu port receives the context
    /// and the per-frame default (surface) target from the application.
    /// Frame orchestration mirrors `renderForSpec`: clear to the scene
    /// background color, collect primitive commands, then execute.
    pub fn render_with_context(
        &mut self,
        time: &JulianDate,
        context: &mut Context,
        default_target: Option<DefaultRenderTarget<'_>>,
    ) {
        // Mirrors the CesiumJS render sequence: preRender event → tween
        // updates (camera flights advance before the camera matrices are
        // recomputed) → scene update → render passes → postRender event.
        self.pre_render.raise_event(time);
        self.tweens.borrow_mut().update(time);

        // Sync the camera with the drawing buffer before computing matrices,
        // and publish the buffer size in the frame state (the quadtree SSE
        // traversal depends on it).
        self.frame_state.drawing_buffer_width = context.drawing_buffer_width();
        self.frame_state.drawing_buffer_height = context.drawing_buffer_height();
        self.camera.set_canvas_size(
            context.drawing_buffer_width(),
            context.drawing_buffer_height(),
        );
        self.update(time);

        // B4-1: feed the czm_* automatic uniforms from the camera, mirroring
        // CesiumJS `UniformState#updateCamera` / `updateFrustum`.
        let view_matrix = self.camera.view_matrix().clone();
        let inverse_view_matrix = self.camera.inverse_view_matrix().clone();
        let projection_matrix = self.camera.projection_matrix().clone();
        let camera_position = *self.camera.position();
        let near = self.camera.near();
        let far = self.camera.far();
        Self::update_camera_uniforms(
            context.uniform_state_mut(),
            view_matrix,
            inverse_view_matrix,
            projection_matrix,
            camera_position,
            near,
            far,
        );

        // Globe update (quadtree traversal) before command collection,
        // mirroring CesiumJS `Scene#update` calling `globe.update` ahead of
        // the render passes. `Option::take` splits the borrows so the blit
        // quad and the offscreen framebuffer (both owned by the scene) stay
        // accessible while the globe is driven.
        if let Some(mut globe) = self.globe.take() {
            // CesiumJS order: beginFrame (clears per-frame traversal state)
            // → update (quadtree traversal fills tiles_to_render) → render
            // (draw commands) → endFrame.
            globe.begin_frame(&self.frame_state);
            globe.update(&self.frame_state);

            context.begin_frame();

            // Clear to the scene background color (ClearCommand.ALL analogue).
            let background = self.background_color.clone();
            let clear = ClearCommand {
                color: Some([
                    background.red as f32,
                    background.green as f32,
                    background.blue as f32,
                    background.alpha as f32,
                ]),
                ..ClearCommand::all()
            };
            context.clear(clear);

            if globe.show {
                if let Some(globe_framebuffer) = self.ensure_globe_framebuffer(context) {
                    // Clear the offscreen globe pass: transparent color
                    // (blended away outside the globe silhouette) + depth 1.0.
                    let globe_clear = ClearCommand {
                        color: Some([0.0, 0.0, 0.0, 0.0]),
                        depth: Some(1.0),
                        framebuffer: Some(globe_framebuffer.clone()),
                        ..ClearCommand::all()
                    };
                    context.clear(globe_clear);

                    globe.render(
                        &self.frame_state,
                        context,
                        Some(globe_framebuffer.clone()),
                    );
                    globe.end_frame(&self.frame_state);

                    // Composite the globe pass onto the default target.
                    if let Some(color_texture) =
                        globe_framebuffer.get_color_texture(0).cloned()
                    {
                        self.globe_blit.render(context, color_texture);
                    }
                }
            }

            self.globe = Some(globe);
        } else {
            context.begin_frame();

            let background = self.background_color.clone();
            let clear = ClearCommand {
                color: Some([
                    background.red as f32,
                    background.green as f32,
                    background.blue as f32,
                    background.alpha as f32,
                ]),
                ..ClearCommand::all()
            };
            context.clear(clear);
        }

        // Primitive command collection: the scene's primitive collection
        // (mirrors CesiumJS command collection over `Scene#primitives`),
        // then the smoke full-screen quad.
        self.primitives.update(&self.frame_state, context);
        self.viewport_quad.update(&self.frame_state, context);

        context.execute(default_target);
        context.end_frame();

        self.post_render.raise_event(time);
    }

    /// Lazily creates (or rebuilds on resize) the offscreen globe pass
    /// framebuffer: Rgba8 color + Depth32Float depth.
    fn ensure_globe_framebuffer(
        &mut self,
        context: &mut Context,
    ) -> Option<Arc<Framebuffer>> {
        let width = context.drawing_buffer_width();
        let height = context.drawing_buffer_height();
        if width == 0 || height == 0 {
            return None;
        }
        let needs_rebuild = self.globe_framebuffer.is_none()
            || self.globe_target_size != (width, height);
        if needs_rebuild {
            let color = Arc::new(context.create_texture(TextureOptions {
                width: Some(width),
                height: Some(height),
                pixel_format: PixelFormat::Rgba,
                flip_y: false,
                ..Default::default()
            }));
            let depth = Arc::new(context.create_texture(TextureOptions {
                width: Some(width),
                height: Some(height),
                pixel_format: PixelFormat::DepthComponent,
                pixel_datatype: PixelDatatype::Float,
                flip_y: false,
                ..Default::default()
            }));
            self.globe_framebuffer = Some(Arc::new(Framebuffer::new(FramebufferOptions {
                color_textures: Some(vec![color]),
                depth_texture: Some(depth),
                ..Default::default()
            })));
            self.globe_target_size = (width, height);
        }
        self.globe_framebuffer.clone()
    }

    /// Pushes camera-derived matrices into the uniform state (B4-1).
    fn update_camera_uniforms(
        uniform_state: &mut UniformState,
        view: Matrix4,
        inverse_view: Matrix4,
        projection: Matrix4,
        camera_position: Cartesian3,
        near: f64,
        far: f64,
    ) {
        let _ = &inverse_view; // reserved for czm_inverseView when exposed
        uniform_state.update_view(view);
        uniform_state.update_projection(projection);
        uniform_state.update_camera_position(camera_position);
        uniform_state.update_frustum(near, far);
    }

    /// Returns whether this scene has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the scene.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Scene {
    fn default() -> Self { Self::new() }
}

/// Private full-screen quad used to composite the offscreen globe pass
/// (color + depth) onto the default target, which has no depth attachment.
///
/// DEVIATION (B4-3): CesiumJS renders the globe directly into the default
/// framebuffer; the wgpu port renders it offscreen and blits. The quad
/// texture coordinates are v-flipped relative to [`ViewportQuad`]: the
/// offscreen texture row 0 holds the screen TOP (wgpu clip y-up), while the
/// quad's (-1,-1) vertex is the screen bottom, so the bottom-left corner
/// samples v = 1. This keeps the globe orientation upright without touching
/// the imagery-upload flip (the single 1.0-v decision point stays in
/// `GlobeSurfaceTileProvider::upload_tile_texture`).
struct GlobeBlitQuad {
    /// Full-screen vertex array (created lazily on first render).
    vertex_array: Option<Arc<VertexArray>>,
    /// The texture-blit WGSL shader program (created lazily).
    shader_program: Option<Arc<ShaderProgram>>,
}

impl GlobeBlitQuad {
    fn new() -> Self {
        Self { vertex_array: None, shader_program: None }
    }

    /// Issues the blit draw command sampling `texture` over the viewport.
    fn render(&mut self, context: &mut Context, texture: Arc<Texture>) {
        if self.vertex_array.is_none() {
            #[rustfmt::skip]
            let positions: [f32; 6 * 4] = [
                -1.0, -1.0, 0.0, 1.0,
                 1.0, -1.0, 0.0, 1.0,
                -1.0,  1.0, 0.0, 1.0,
                -1.0,  1.0, 0.0, 1.0,
                 1.0, -1.0, 0.0, 1.0,
                 1.0,  1.0, 0.0, 1.0,
            ];
            // v-flipped vs. the plain viewport quad (see struct docs).
            #[rustfmt::skip]
            let texture_coordinates: [f32; 6 * 4] = [
                0.0, 1.0, 0.0, 0.0,
                1.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
                1.0, 1.0, 0.0, 0.0,
                1.0, 0.0, 0.0, 0.0,
            ];
            let to_bytes = |values: &[f32]| -> Vec<u8> {
                values.iter().flat_map(|value| value.to_le_bytes()).collect()
            };
            let position_buffer = context.create_vertex_buffer(
                Some(&to_bytes(&positions)),
                None,
                BufferUsage::StaticDraw,
            );
            let texture_coordinate_buffer = context.create_vertex_buffer(
                Some(&to_bytes(&texture_coordinates)),
                None,
                BufferUsage::StaticDraw,
            );
            let attributes = vec![
                VertexAttribute {
                    index: 0,
                    buffer: position_buffer,
                    components_per_attribute: 4,
                    component_datatype: wgpu::VertexFormat::Float32x4,
                    normalize: false,
                    stride_in_bytes: 16,
                    offset_in_bytes: 0,
                },
                VertexAttribute {
                    index: 1,
                    buffer: texture_coordinate_buffer,
                    components_per_attribute: 4,
                    component_datatype: wgpu::VertexFormat::Float32x4,
                    normalize: false,
                    stride_in_bytes: 16,
                    offset_in_bytes: 0,
                },
            ];
            self.vertex_array = Some(Arc::new(VertexArray::new(attributes, None)));
        }

        if self.shader_program.is_none() {
            match ShaderProgram::from_wgsl(
                wgsl::VIEWPORT_QUAD_VS,
                wgsl::VIEWPORT_QUAD_TEXTURE_FS,
                "globe_blit_texture".to_string(),
            ) {
                Ok(program) => self.shader_program = Some(Arc::new(program)),
                Err(error) => {
                    log::error!("globe blit shader compilation failed: {error}");
                    return;
                }
            }
        }

        // Standard alpha blending: the offscreen globe pass is cleared
        // transparent, so pixels outside the globe silhouette reveal the
        // scene background. Depth test stays off (default target has no
        // depth attachment).
        let mut render_state = RenderState::default();
        render_state.depth_test.enabled = false;
        render_state.blending.enabled = true;
        render_state.blending.equation_rgb = BlendEquation::FuncAdd;
        render_state.blending.equation_alpha = BlendEquation::FuncAdd;
        render_state.blending.function_source_rgb = BlendingFactor::SrcAlpha;
        render_state.blending.function_source_alpha = BlendingFactor::One;
        render_state.blending.function_destination_rgb = BlendingFactor::OneMinusSrcAlpha;
        render_state.blending.function_destination_alpha = BlendingFactor::OneMinusSrcAlpha;

        let mut command = DrawCommand::new();
        command.primitive_type = WebGLConstants::TRIANGLES;
        command.vertex_array = self.vertex_array.clone();
        command.count = Some(6);
        command.offset = 0;
        command.shader_program = self.shader_program.clone();
        command.uniform_overrides = vec![(
            "u_texture".to_string(),
            UniformValue::Texture(texture),
        )];
        command.render_state = render_state;
        command.framebuffer = None; // default target
        command.pass = Some(Pass::Globe as u32);
        command.owner = Some("SceneGlobeBlit".to_string());

        context.draw(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Mirrors SceneSpec: `preRender` and `postRender` events fire once per
    /// render with the simulation time.
    #[test]
    fn pre_and_post_render_events_fire_during_render() {
        let mut scene = Scene::new();
        let pre_count = Rc::new(Cell::new(0));
        let post_count = Rc::new(Cell::new(0));
        {
            let pre_count = pre_count.clone();
            let _ = scene.pre_render().add_listener(move |_time| {
                pre_count.set(pre_count.get() + 1);
            });
            let post_count = post_count.clone();
            let _ = scene.post_render().add_listener(move |_time| {
                post_count.set(post_count.get() + 1);
            });
        }
        scene.render(&JulianDate::now());
        assert_eq!(pre_count.get(), 1);
        assert_eq!(post_count.get(), 1);
        scene.render(&JulianDate::now());
        assert_eq!(pre_count.get(), 2);
        assert_eq!(post_count.get(), 2);
    }

    /// Mirrors the JS frame loop: tweens added to `Scene#tweens` advance on
    /// every `render` (the tween update runs between preRender and the
    /// camera update).
    #[test]
    fn tweens_progress_during_render() {
        let mut scene = Scene::new();
        let last_value = Rc::new(Cell::new(f64::NAN));
        {
            let last_value = last_value.clone();
            scene.tweens_mut().add(TweenOptions {
                update: Some(Box::new(move |values| last_value.set(values[0].1))),
                ..TweenOptions::new(
                    vec![("value".to_string(), 0.0)],
                    vec![("value".to_string(), 10.0)],
                    2.0,
                )
            });
        }
        let start = JulianDate::now();
        scene.render(&start);
        assert_eq!(last_value.get(), 0.0);
        scene.render(&JulianDate::add_seconds_new(&start, 1.0));
        assert_eq!(last_value.get(), 5.0);
        scene.render(&JulianDate::add_seconds_new(&start, 2.0));
        assert_eq!(last_value.get(), 10.0);
        assert!(scene.tweens().is_empty());
    }

    /// Mirrors CameraSpec flyTo semantics through the scene: the camera
    /// animates toward the destination and the complete callback fires when
    /// the duration elapses (default easing QUINTIC_IN_OUT: t = 0.5 maps to
    /// 0.5, so the midpoint assertion holds).
    #[test]
    fn fly_to_animates_camera_and_completes() {
        let mut scene = Scene::new();
        let destination = Cartesian3::new(
            Ellipsoid::WGS84.maximum_radius() + 1_000_000.0,
            0.0,
            0.0,
        );
        let completed = Rc::new(Cell::new(false));
        {
            let completed = completed.clone();
            scene.fly_to(destination, Some(2.0), Some(Box::new(move || {
                completed.set(true);
            })));
        }

        let start = JulianDate::now();
        scene.render(&start);

        // Half way through the flight: quintic_in_out(0.5) = 0.5, so the
        // position is the midpoint of the start (origin) and destination.
        scene.render(&JulianDate::add_seconds_new(&start, 1.0));
        let mid_x = (Ellipsoid::WGS84.maximum_radius() + 1_000_000.0) * 0.5;
        assert!((scene.camera().position().x - mid_x).abs() / mid_x < 1e-9);

        // End of the flight: the exact destination and the complete callback.
        scene.render(&JulianDate::add_seconds_new(&start, 2.0));
        assert!(completed.get());
        let position = *scene.camera().position();
        assert!((position.x - destination.x).abs() < 1e-6);
        assert!(position.y.abs() < 1e-6);
        assert!(position.z.abs() < 1e-6);
        assert!(scene.tweens().is_empty());
    }

    /// A new flight supersedes the previous one (the old tween is removed
    /// without its cancel callback clearing the new flight channel).
    #[test]
    fn new_flight_supersedes_previous_flight() {
        let mut scene = Scene::new();
        let first = Cartesian3::new(10_000_000.0, 0.0, 0.0);
        let second = Cartesian3::new(20_000_000.0, 0.0, 0.0);
        let completed = Rc::new(Cell::new(false));

        scene.fly_to(first, Some(10.0), None);
        {
            let completed = completed.clone();
            scene.fly_to(second, Some(1.0), Some(Box::new(move || {
                completed.set(true);
            })));
        }

        let start = JulianDate::now();
        scene.render(&start);
        // Only the second flight's tween remains.
        assert_eq!(scene.tweens().len(), 1);

        scene.render(&JulianDate::add_seconds_new(&start, 1.0));
        assert!(completed.get());
        assert!((scene.camera().position().x - 20_000_000.0).abs() < 1e-6);
    }

    /// Mirrors CameraSpec flyHome semantics: the home view positions the
    /// camera on the +X axis at `radius / sin(fov / 2)` looking at the
    /// ellipsoid center.
    #[test]
    fn fly_home_flies_to_the_home_view() {
        let mut scene = Scene::new();
        scene.fly_home(Some(0.0));
        // A zero-duration tween completes on the first render and the
        // camera applies the exact end pose on that same frame.
        scene.render(&JulianDate::now());

        let expected = Ellipsoid::WGS84.maximum_radius()
            / (scene.camera().fov() * 0.5).sin();
        let position = *scene.camera().position();
        assert!((position.x - expected).abs() / expected < 1e-9);
        assert!(position.y.abs() < 1e-6);
        assert!(position.z.abs() < 1e-6);
        // Looking at the center: direction ≈ -X.
        assert!((scene.camera().direction().x + 1.0).abs() < 1e-9);
    }

    /// Mirrors SceneModePickerSpec: morphTo2D/3D/ColumbusView raise
    /// `morphStart` with the new mode and switch `scene.mode`.
    #[test]
    fn morph_transitions_raise_morph_start_and_switch_mode() {
        let scene = Scene::new();
        let received = Rc::new(Cell::new(None::<SceneMode>));
        {
            let received = received.clone();
            let _ = scene.morph_start().add_listener(move |mode| {
                received.set(Some(*mode));
            });
        }
        assert_eq!(scene.mode(), SceneMode::Scene3D);

        scene.morph_to_2d(1.5);
        assert_eq!(received.get(), Some(SceneMode::Scene2D));
        assert_eq!(scene.mode(), SceneMode::Scene2D);

        scene.morph_to_columbus_view(2.0);
        assert_eq!(received.get(), Some(SceneMode::ColumbusView));
        assert_eq!(scene.mode(), SceneMode::ColumbusView);

        scene.morph_to_3d(0.5);
        assert_eq!(received.get(), Some(SceneMode::Scene3D));
        assert_eq!(scene.mode(), SceneMode::Scene3D);
        // The synchronous transition completed: the morph time returned to
        // the mode default (the JS completeMorph behavior).
        assert_eq!(
            scene.morph_time(),
            SceneMode::get_morph_time(SceneMode::Scene3D).unwrap_or(1.0)
        );
    }

    /// Mirrors the JS `SceneTransitioner` completion notification: every
    /// morph raises `morphComplete` with the new mode once the (synchronous)
    /// transition finishes, and `completeMorph` re-raises it.
    #[test]
    fn morph_transitions_raise_morph_complete() {
        let scene = Scene::new();
        let received = Rc::new(Cell::new(None::<SceneMode>));
        let count = Rc::new(Cell::new(0usize));
        {
            let received = received.clone();
            let count = count.clone();
            let _ = scene.morph_complete().add_listener(move |mode| {
                received.set(Some(*mode));
                count.set(count.get() + 1);
            });
        }

        scene.morph_to_2d(1.5);
        assert_eq!(received.get(), Some(SceneMode::Scene2D));
        assert_eq!(count.get(), 1);

        scene.morph_to_3d(0.5);
        assert_eq!(received.get(), Some(SceneMode::Scene3D));
        assert_eq!(count.get(), 2);

        // completeMorph on an already-completed morph keeps the mode's
        // default morph time and re-raises morphComplete.
        scene.complete_morph();
        assert_eq!(received.get(), Some(SceneMode::Scene3D));
        assert_eq!(count.get(), 3);
        assert_eq!(
            scene.morph_time(),
            SceneMode::get_morph_time(SceneMode::Scene3D).unwrap_or(1.0)
        );
    }

    /// Mirrors the VRButton write of `scene.useWebVR` through `&self`.
    #[test]
    fn use_web_vr_flag_round_trips_through_shared_reference() {
        let scene = Scene::new();
        assert!(!scene.use_web_vr());
        scene.set_use_web_vr(true);
        assert!(scene.use_web_vr());
        scene.set_use_web_vr(false);
        assert!(!scene.use_web_vr());
    }

    /// Mirrors the Geocoder view model's credit display access through
    /// `&self` (addStaticCredit / removeStaticCredit / isDestroyed).
    #[test]
    fn static_credits_and_destroy_state_through_shared_reference() {
        let mut scene = Scene::new();
        assert!(!scene.credit_display_is_destroyed());

        let credit = Credit::new("ion-geocoder", true);
        scene.add_static_credit(credit.clone_credit());
        assert_eq!(scene.credit_display().static_credits().len(), 1);
        // The static credit reappears as a current credit each frame.
        scene.render(&JulianDate::now());
        assert_eq!(scene.credit_display().current_credits().len(), 1);

        scene.remove_static_credit(&credit);
        assert!(scene.credit_display().static_credits().is_empty());
    }

    /// Mirrors the SelectionIndicator default converter:
    /// `SceneTransforms.worldToWindowCoordinates(scene, position)`.
    #[test]
    fn world_to_window_coordinates_projects_through_the_camera() {
        let mut scene = Scene::new();
        scene.camera_mut().set_position(Cartesian3::new(0.0, 0.0, 0.0));
        scene.camera_mut().set_direction(Cartesian3::new(0.0, 0.0, -1.0));
        scene.camera_mut().set_up(Cartesian3::new(0.0, 1.0, 0.0));
        scene.camera_mut().set_right(Cartesian3::new(1.0, 0.0, 0.0));
        scene.render(&JulianDate::now()); // refreshes the camera matrices

        // On the view axis: the window center of the 800×600 canvas.
        let window = scene
            .world_to_window_coordinates(&Cartesian3::new(0.0, 0.0, -100.0))
            .expect("point in front of the camera");
        assert!((window.x - 400.0).abs() < 1e-9);
        assert!((window.y - 300.0).abs() < 1e-9);

        // Behind the camera: undefined (None).
        assert!(scene
            .world_to_window_coordinates(&Cartesian3::new(0.0, 0.0, 100.0))
            .is_none());
    }
}
