use gpui::{App, KeyBinding};
use actions_crate::*;

/// Register all keybindings — mirrors Zed's `init_keymap` pattern.
pub fn register(cx: &mut App) {
    cx.bind_keys(vec![
        // ── Global ──────────────────────────────────────────────
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("ctrl-q", Quit, None),

        // ── File ────────────────────────────────────────────────
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-shift-s", SaveAs, None),
        KeyBinding::new("cmd-o", OpenFile, None),

        // ── Edit ────────────────────────────────────────────────
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),

        // ── View ────────────────────────────────────────────────
        KeyBinding::new("cmd-=", ZoomIn, None),
        KeyBinding::new("cmd--", ZoomOut, None),
        KeyBinding::new("cmd-0", ZoomReset, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullscreen, None),

        // ── Sidebar ─────────────────────────────────────────────
        KeyBinding::new("cmd-b", ToggleSidebar, None),
    ]);
}
