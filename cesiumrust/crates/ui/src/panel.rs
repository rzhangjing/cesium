use gpui::{App, AnyElement, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};
use theme::AppColors;

/// A generic panel container with optional title — matches Zed's
/// left/right panel pattern (project panel, terminal panel, etc.).
#[derive(IntoElement)]
pub struct Panel {
    title: Option<SharedString>,
    width: f32,
    children: Vec<AnyElement>,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            title: None,
            width: 240.0,
            children: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl ParentElement for Panel {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut root = div()
            .flex()
            .flex_col()
            .w(px(self.width))
            .h_full()
            .bg(AppColors::surface())
            .border_r_1()
            .border_color(AppColors::border());

        if let Some(title) = self.title {
            root = root.child(
                div()
                    .flex()
                    .items_center()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(AppColors::border())
                    .text_sm()
                    .text_color(AppColors::text_muted())
                    .child(title),
            );
        }

        root.children(self.children)
    }
}
