use gpui::{App, IntoElement, SharedString, Styled, Window, div, prelude::*, px};
use theme::AppColors;

/// Window title bar — renders the app name and optional actions.
#[derive(IntoElement)]
pub struct TitleBar {
    title: SharedString,
}

impl TitleBar {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl RenderOnce for TitleBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
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
                    .child(self.title),
            )
    }
}
