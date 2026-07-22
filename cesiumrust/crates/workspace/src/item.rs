use gpui::{Render, SharedString, Window, Context};

/// Trait for anything that can be displayed inside a pane tab.
///
/// Zed pattern: every "openable" thing (editor, terminal, diff, image
/// viewer…) implements `Item` so the workspace can treat them uniformly.
pub trait Item: Render + 'static {
    /// Human-readable label shown in the tab.
    fn tab_title(&self, _window: &mut Window, _cx: &mut Context<Self>) -> SharedString {
        "Untitled".into()
    }

    /// Whether this item can be closed by the user.
    fn is_closeable(&self) -> bool { true }

    /// Called when the user requests a save.
    fn save(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> anyhow::Result<()> {
        Ok(())
    }
}

// Blanket impl so any `Render` entity that wants default behavior can
// trivially satisfy `Item`:
impl<T: Render + 'static> Item for T {}
