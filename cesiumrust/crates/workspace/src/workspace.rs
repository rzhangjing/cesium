use gpui::{Entity, IntoElement, Render, SharedString, Styled, Window, Context, div, prelude::*, px};
use theme::AppColors;
use bevy_demo::BevyDemoView;

/// The top-level workspace — owns the window chrome and layout.
pub struct Workspace {
    title: SharedString,
    bevy_view: Entity<BevyDemoView>,
}

impl Workspace {
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let bevy_view = cx.new(|cx| BevyDemoView::new(cx));
        Self {
            title: title.into(),
            bevy_view,
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.title.clone();
        let bevy_view = self.bevy_view.clone();

        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            // ── Title Bar ───────────────────────────────────────
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(32.0))
                    .px_4()
                    .bg(AppColors::base())
                    .border_b_1()
                    .border_color(AppColors::border())
                    .child(
                        div()
                            .text_sm()
                            .text_color(AppColors::text())
                            .child(title),
                    ),
            )
            // ── Main content area — Bevy Demo ──────────────────
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(bevy_view)
            )
            // ── Status Bar ──────────────────────────────────────
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(24.0))
                    .px_3()
                    .bg(AppColors::surface())
                    .border_t_1()
                    .border_color(AppColors::border())
                    .text_xs()
                    .text_color(AppColors::text_muted())
                    .child(div().child("Drag to rotate"))
                    .child(div().child("v0.1.0")),
            )
    }
}
