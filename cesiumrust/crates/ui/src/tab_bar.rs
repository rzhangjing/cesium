use gpui::{App, AnyElement, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};
use theme::AppColors;

/// Horizontal tab bar — each child is a tab element.
#[derive(IntoElement)]
pub struct TabBar {
    children: Vec<AnyElement>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl ParentElement for TabBar {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for TabBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(36.0))
            .px_2()
            .gap_1()
            .bg(AppColors::base())
            .border_b_1()
            .border_color(AppColors::border())
            .children(self.children)
    }
}

/// A single tab element.
#[derive(IntoElement)]
pub struct Tab {
    label: SharedString,
    active: bool,
}

impl Tab {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            active: false,
        }
    }

    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .px_3()
            .py_1()
            .rounded_lg()
            .text_sm()
            .when(self.active, |el| {
                el.bg(AppColors::overlay())
                    .text_color(AppColors::accent())
            })
            .when(!self.active, |el| {
                el.text_color(AppColors::text_muted())
                    .hover(|s| s.bg(AppColors::overlay()))
            })
            .child(self.label)
    }
}
