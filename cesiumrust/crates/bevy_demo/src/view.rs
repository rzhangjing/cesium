//! GPUI view that renders a dynamic 3D scene with mouse interaction.

use futures_lite::stream::StreamExt;
use gpui::{
    canvas, App, Bounds, Context, EventEmitter,
    IntoElement, Pixels, Render, Timer,
    Window, px, quad, size, Edges, Hsla,
    div, InteractiveElement, Styled, ParentElement,
    MouseButton,
};
use std::time::Duration;

/// Particle for visual effect.
#[derive(Clone)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    hue: f32,
}

/// The BevyDemoView — renders an animated 3D scene with mouse rotation.
pub struct BevyDemoView {
    rotation_x: f32,
    rotation_y: f32,
    auto_rotate: bool,
    particles: Vec<Particle>,
    last_time: std::time::Instant,
    mouse_pressed: bool,
    last_mouse_x: f32,
    last_mouse_y: f32,
    _animation: gpui::Task<()>,
}

impl BevyDemoView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let animation = cx.spawn(async move |weak: gpui::WeakEntity<Self>, cx| {
            let mut timer = Timer::interval(Duration::from_millis(16)); // ~60fps
            loop {
                timer.next().await;
                if let Some(view) = weak.upgrade() {
                    let _ = view.update(cx, |view, _cx| {
                        let now = std::time::Instant::now();
                        let dt = now.duration_since(view.last_time).as_secs_f32().min(0.1);
                        view.last_time = now;

                        // Auto-rotate when not dragging
                        if view.auto_rotate && !view.mouse_pressed {
                            view.rotation_y += dt * 0.8;
                            view.rotation_x += dt * 0.3;
                        }

                        // Update particles
                        view.update_particles(dt);
                    });
                }
            }
        });

        Self {
            rotation_x: 0.4,
            rotation_y: 0.0,
            auto_rotate: true,
            particles: Vec::new(),
            last_time: std::time::Instant::now(),
            mouse_pressed: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            _animation: animation,
        }
    }

    /// Update particles.
    fn update_particles(&mut self, dt: f32) {
        // Spawn new particles
        if self.particles.len() < 30 {
            let angle = self.rotation_y * 2.0;
            let x = 400.0 + angle.cos() * 120.0;
            let y = 300.0 + angle.sin() * 80.0;
            self.spawn_particle(x, y);
        }

        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 80.0 * dt; // gravity
            p.life -= dt * 0.8;
            p.hue = (p.hue + dt * 120.0) % 360.0;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Spawn a new particle at the given position.
    fn spawn_particle(&mut self, x: f32, y: f32) {
        let angle = ((x * 0.02 + y * 0.02).sin() * 3.0) as f32;
        let speed = 30.0 + (x * 0.1 + y * 0.1).abs();
        self.particles.push(Particle {
            x,
            y,
            vx: angle.cos() * speed * 0.4,
            vy: -speed * 0.6 - 30.0,
            life: 1.0,
            max_life: 1.0,
            size: 3.0 + (x * 0.02).abs() * 4.0,
            hue: (x + y) % 360.0,
        });
    }

    /// Handle mouse press.
    fn mouse_down(&mut self, x: f32, y: f32) {
        self.mouse_pressed = true;
        self.last_mouse_x = x;
        self.last_mouse_y = y;
    }

    /// Handle mouse release.
    fn mouse_up(&mut self) {
        self.mouse_pressed = false;
    }

    /// Handle mouse move while pressed.
    fn mouse_drag(&mut self, x: f32, y: f32) {
        if self.mouse_pressed {
            let dx = x - self.last_mouse_x;
            let dy = y - self.last_mouse_y;
            self.rotation_y += dx * 0.008;
            self.rotation_x += dy * 0.008;
            self.last_mouse_x = x;
            self.last_mouse_y = y;
        }
    }
}

impl EventEmitter<BevyDemoView> for BevyDemoView {}

impl Render for BevyDemoView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rotation_x = self.rotation_x;
        let rotation_y = self.rotation_y;
        let particles: Vec<_> = self.particles.iter().map(|p| {
            (p.x, p.y, p.life, p.max_life, p.size, p.hue)
        }).collect();

        div()
            .flex()
            .flex_1()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                let pos = event.position;
                this.mouse_down(f32::from(pos.x), f32::from(pos.y));
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event: &gpui::MouseUpEvent, _window, _cx| {
                this.mouse_up();
            }))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, _cx| {
                let pos = event.position;
                this.mouse_drag(f32::from(pos.x), f32::from(pos.y));
            }))
            .child(
                canvas(
                    // prepaint
                    move |bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {
                        (rotation_x, rotation_y, particles.clone(), bounds)
                    },
                    // paint
                    move |_bounds: Bounds<Pixels>, (rotation_x, rotation_y, particles, bounds): (f32, f32, Vec<(f32, f32, f32, f32, f32, f32)>, Bounds<Pixels>), window: &mut Window, _cx: &mut App| {
                        let w = f32::from(bounds.size.width);
                        let h = f32::from(bounds.size.height);

                        // Background
                        let bg_color = Hsla { h: 220.0, s: 0.3, l: 0.06, a: 1.0 };
                        window.paint_quad(quad(bounds, px(0.0), bg_color, Edges::default(), Hsla::default(), Default::default()));

                        let center_x = w * 0.5;
                        let center_y = h * 0.5;

                        // 3D Cube vertices
                        let s = 80.0;
                        let vertices = [
                            [-s, -s, -s], [s, -s, -s], [s, s, -s], [-s, s, -s],
                            [-s, -s, s],  [s, -s, s],  [s, s, s],  [-s, s, s],
                        ];

                        let cos_y = rotation_y.cos();
                        let sin_y = rotation_y.sin();
                        let cos_x = rotation_x.cos();
                        let sin_x = rotation_x.sin();

                        // Project 3D to 2D
                        let project = |x: f32, y: f32, z: f32| -> (f32, f32) {
                            let rx = x * cos_y + z * sin_y;
                            let rz = -x * sin_y + z * cos_y;
                            let ry = y * cos_x - rz * sin_x;
                            let rz2 = y * sin_x + rz * cos_x;
                            let fov = 400.0;
                            let scale = fov / (rz2 + fov * 0.5).max(0.1);
                            (rx * scale + center_x, -ry * scale + center_y)
                        };

                        // Draw background grid
                        let grid_color = Hsla { h: 200.0, s: 0.2, l: 0.12, a: 0.4 };
                        for i in 0..=16 {
                            let x = (i as f32 / 16.0) * w;
                            window.paint_quad(quad(
                                gpui::Bounds { origin: gpui::point(px(x), bounds.origin.y), size: size(px(1.0), bounds.size.height) },
                                px(0.0), grid_color, Edges::default(), Hsla::default(), Default::default(),
                            ));
                        }
                        for i in 0..=12 {
                            let y = (i as f32 / 12.0) * h;
                            window.paint_quad(quad(
                                gpui::Bounds { origin: gpui::point(bounds.origin.x, px(y)), size: size(bounds.size.width, px(1.0)) },
                                px(0.0), grid_color, Edges::default(), Hsla::default(), Default::default(),
                            ));
                        }

                        // Edges
                        let edges = [
                            [0,1],[1,2],[2,3],[3,0],
                            [4,5],[5,6],[6,7],[7,4],
                            [0,4],[1,5],[2,6],[3,7],
                        ];

                        // Draw cube edges with depth effect
                        for edge in &edges {
                            let (x1, y1) = project(vertices[edge[0]][0], vertices[edge[0]][1], vertices[edge[0]][2]);
                            let (x2, y2) = project(vertices[edge[1]][0], vertices[edge[1]][1], vertices[edge[1]][2]);

                            let edge_hue = (rotation_y * 30.0 + 180.0) % 360.0;
                            let color = Hsla { h: edge_hue, s: 0.9, l: 0.55, a: 1.0 };
                            let thickness = px(2.5);

                            // Draw line
                            let dx = x2 - x1;
                            let dy = y2 - y1;
                            let len = (dx * dx + dy * dy).sqrt();
                            let steps = (len / 2.0).ceil() as u32;

                            for i in 0..steps {
                                let t = i as f32 / steps.max(1) as f32;
                                let px_x = x1 + dx * t;
                                let px_y = y1 + dy * t;
                                window.paint_quad(quad(
                                    gpui::Bounds { origin: gpui::point(px(px_x - 1.0), px(px_y - 1.0)), size: size(px(3.0), thickness) },
                                    px(1.0), color, Edges::default(), Hsla::default(), Default::default(),
                                ));
                            }
                        }

                        // Draw cube vertices as glowing dots
                        for v in &vertices {
                            let (sx, sy) = project(v[0], v[1], v[2]);
                            let dot_size = 8.0;
                            let dot_hue = (rotation_y * 50.0 + 160.0) % 360.0;

                            // Outer glow
                            window.paint_quad(quad(
                                gpui::Bounds {
                                    origin: gpui::point(px(sx - dot_size), px(sy - dot_size)),
                                    size: size(px(dot_size * 2.0), px(dot_size * 2.0)),
                                },
                                px(dot_size), Hsla { h: dot_hue, s: 1.0, l: 0.6, a: 0.4 },
                                Edges::default(), Hsla::default(), Default::default(),
                            ));

                            // Inner dot
                            window.paint_quad(quad(
                                gpui::Bounds {
                                    origin: gpui::point(px(sx - 3.0), px(sy - 3.0)),
                                    size: size(px(6.0), px(6.0)),
                                },
                                px(3.0), Hsla { h: dot_hue, s: 1.0, l: 0.85, a: 1.0 },
                                Edges::default(), Hsla::default(), Default::default(),
                            ));
                        }

                        // Draw particles
                        for (px_val, py_val, life, max_life, size_val, hue) in &particles {
                            let alpha = (life / max_life).max(0.0);
                            let color = Hsla { h: *hue, s: 0.8, l: 0.55, a: alpha * 0.9 };
                            let s = size_val * alpha;
                            window.paint_quad(quad(
                                gpui::Bounds {
                                    origin: gpui::point(px(px_val - s * 0.5), px(py_val - s * 0.5)),
                                    size: size(px(s), px(s)),
                                },
                                px(s * 0.3), color, Edges::default(), Hsla::default(), Default::default(),
                            ));
                        }
                    },
                )
            )
            .into_element()
    }
}
