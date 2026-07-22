use gpui::{IntoElement, Render, SharedString, Styled, Window, Context, div, prelude::*, px};
use theme::AppColors;

/// A pane holds one or more tabs (items) and renders a tab bar +
/// the active item's content.  Mirrors Zed's `Pane` concept.
pub struct Pane {
    tabs: Vec<PaneTab>,
    active: usize,
}

struct PaneTab {
    title: SharedString,
    // In a real app this would be an `Entity<dyn Item>`.
}

impl Pane {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
        }
    }

    pub fn add_tab(&mut self, title: impl Into<SharedString>) {
        self.tabs.push(PaneTab {
            title: title.into(),
        });
    }

    pub fn set_active(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
        }
    }
}

impl Render for Pane {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let active_title = self
            .tabs
            .get(self.active)
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "No tabs".into());

        let tab_titles: Vec<SharedString> = self.tabs.iter().map(|t| t.title.clone()).collect();

        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            // Tab bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(36.0))
                    .px_2()
                    .gap_1()
                    .bg(AppColors::base())
                    .border_b_1()
                    .border_color(AppColors::border())
                    .children(tab_titles.into_iter().enumerate().map(|(i, title)| {
                        let is_active = i == self.active;
                        div()
                            .px_3()
                            .py_1()
                            .rounded_lg()
                            .text_sm()
                            .when(is_active, |el| {
                                el.bg(AppColors::overlay())
                                    .text_color(AppColors::accent())
                            })
                            .when(!is_active, |el| {
                                el.text_color(AppColors::text_muted())
                            })
                            .child(title)
                    })),
            )
            // Content area
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(AppColors::base())
                    .text_color(AppColors::text())
                    .child(format!("Editing: {}", active_title)),
            )
    }
}
