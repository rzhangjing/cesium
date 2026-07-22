use gpui::{App, AnyElement, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use theme::AppColors;

/// Dock direction — left, bottom, right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Bottom,
    Right,
}

/// A dockable side/bottom panel — wraps a `Panel` with a resize handle
/// indicator and a toggle button.  Mirrors Zed's `Dock` concept.
#[derive(IntoElement)]
pub struct Dock {
    position: DockPosition,
    open: bool,
    width: f32,
    children: Vec<AnyElement>,
}

impl Dock {
    pub fn left() -> Self { Self::new(DockPosition::Left) }
    pub fn bottom() -> Self { Self::new(DockPosition::Bottom) }
    pub fn right() -> Self { Self::new(DockPosition::Right) }

    fn new(position: DockPosition) -> Self {
        Self {
            position,
            open: true,
            width: match position {
                DockPosition::Left | DockPosition::Right => 240.0,
                DockPosition::Bottom => 200.0,
            },
            children: Vec::new(),
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl ParentElement for Dock {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for Dock {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if !self.open {
            return div();
        }

        let mut root = match self.position {
            DockPosition::Left | DockPosition::Right => {
                div()
                    .flex()
                    .flex_col()
                    .h_full()
                    .w(px(self.width))
                    .bg(AppColors::surface())
            }
            DockPosition::Bottom => {
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .h(px(self.width))
                    .bg(AppColors::surface())
            }
        };

        root = match self.position {
            DockPosition::Left => root.border_r_1().border_color(AppColors::border()),
            DockPosition::Right => root.border_l_1().border_color(AppColors::border()),
            DockPosition::Bottom => root.border_t_1().border_color(AppColors::border()),
        };

        root.children(self.children)
    }
}
