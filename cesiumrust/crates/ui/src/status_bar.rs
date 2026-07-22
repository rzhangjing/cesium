use gpui::{App, IntoElement, SharedString, Styled, Window, div, prelude::*, px};
use theme::AppColors;

/// Bottom status bar — mirrors Zed's status bar pattern.
#[derive(IntoElement)]
pub struct StatusBar {
    left: Option<SharedString>,
    right: Option<SharedString>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
        }
    }

    pub fn left(mut self, text: impl Into<SharedString>) -> Self {
        self.left = Some(text.into());
        self
    }

    pub fn right(mut self, text: impl Into<SharedString>) -> Self {
        self.right = Some(text.into());
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(self.left.map(|l| div().child(l))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(self.right.map(|r| div().child(r))),
            )
    }
}
