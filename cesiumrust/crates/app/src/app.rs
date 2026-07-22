use gpui::{
    App, Application, Bounds, Context, Entity, Window, WindowBounds, WindowOptions,
    prelude::*, px, size,
};
use workspace_crate::Workspace;
use crate::keybindings;
use actions_crate::*;

/// Root application view — owns the `Workspace` entity.
pub struct AppView {
    workspace: Entity<Workspace>,
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.workspace.clone()
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        // Register global actions
        cx.on_action(|_action: &Quit, _cx: &mut App| {
            // GPUI handles quit natively
        });

        // Register keybindings
        keybindings::register(cx);

        // Open the main window
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("CesiumRust".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| {
                let workspace = cx.new(|cx| {
                    Workspace::new("CesiumRust", cx)
                });
                cx.new(|_cx| AppView { workspace })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
