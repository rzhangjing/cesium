use gpui::{Entity, IntoElement, Render, SharedString, Styled, Window, Context, div, prelude::*, px};
use theme::AppColors;
use crate::Pane;
use crate::Dock;

/// The top-level workspace — owns the window chrome and layout.
///
/// Zed pattern: `Workspace` sits between the `App` and individual
/// `Item`s.  It owns the panes, docks, status bar, and title bar,
/// and delegates item-specific logic to the `Item` trait.
pub struct Workspace {
    title: SharedString,
    pane: Entity<Pane>,
    sidebar_open: bool,
}

impl Workspace {
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let pane = cx.new(|_| Pane::new());
        Self {
            title: title.into(),
            pane,
            sidebar_open: true,
        }
    }

    pub fn add_tab(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        let title = title.into();
        self.pane.update(cx, |pane, _| pane.add_tab(title));
    }

    pub fn toggle_sidebar(&mut self, _cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.title.clone();
        let sidebar_open = self.sidebar_open;
        let pane = self.pane.clone();

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
            // ── Body (sidebar + content) ────────────────────────
            .child(
                div()
                    .flex()
                    .flex_1()
                    // Sidebar
                    .when(sidebar_open, |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .w(px(200.0))
                                .h_full()
                                .bg(AppColors::surface())
                                .border_r_1()
                                .border_color(AppColors::border())
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .text_sm()
                                        .text_color(AppColors::text_muted())
                                        .child("Explorer"),
                                )
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .text_sm()
                                        .text_color(AppColors::text())
                                        .hover(|s| s.bg(AppColors::overlay()))
                                        .child("  src/"),
                                )
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .text_sm()
                                        .text_color(AppColors::text())
                                        .hover(|s| s.bg(AppColors::overlay()))
                                        .child("  Cargo.toml"),
                                ),
                        )
                    })
                    // Main content area — delegates to the active pane
                    .child(pane)
                    // Right dock placeholder
                    .child(
                        Dock::right()
                            .width(0.0)
                            .when(false, |el| {
                                el.child(div().text_sm().text_color(AppColors::text_muted()))
                            }),
                    ),
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
                    .child(div().child("Ready"))
                    .child(div().child("v0.1.0")),
            )
    }
}
