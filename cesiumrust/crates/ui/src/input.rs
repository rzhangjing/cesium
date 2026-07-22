use gpui::{App, IntoElement, SharedString, Styled, Window, div, prelude::*};
use theme::AppColors;

/// A simple styled text input element.
#[derive(IntoElement)]
pub struct TextInput {
    placeholder: SharedString,
    value: SharedString,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            placeholder: "Type here...".into(),
            value: SharedString::default(),
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn value(mut self, text: impl Into<SharedString>) -> Self {
        self.value = text.into();
        self
    }
}

impl RenderOnce for TextInput {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let is_empty = self.value.is_empty();
        let display = if is_empty {
            self.placeholder
        } else {
            self.value
        };

        div()
            .flex()
            .items_center()
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(AppColors::surface())
            .border_1()
            .border_color(AppColors::border())
            .text_sm()
            .text_color(if is_empty {
                AppColors::text_muted()
            } else {
                AppColors::text()
            })
            .child(display)
    }
}
