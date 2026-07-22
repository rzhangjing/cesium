use gpui::{App, IntoElement, SharedString, Styled, Window, div, prelude::*};
use theme::AppColors;

/// A reusable button component — follows Zed's `RenderOnce` pattern
/// for zero-overhead composition.
#[derive(IntoElement)]
pub struct Button {
    label: SharedString,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Button {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .px_4()
            .py_2()
            .rounded_lg()
            .bg(AppColors::accent())
            .text_sm()
            .text_color(AppColors::base())
            .hover(|style| style.bg(AppColors::accent_hover()))
            .child(self.label)
    }
}
