use gpui::actions;

actions!(
    // ── Window ──────────────────────────────────────────────────
    [
        // NewWindow is provided by GPUI internally
        Quit,

        // ── Workspace ───────────────────────────────────────────
        OpenSettings,
        ToggleSidebar,
        ToggleTerminal,

        // ── File ────────────────────────────────────────────────
        OpenFile,
        Save,
        SaveAs,

        // ── Edit ────────────────────────────────────────────────
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,

        // ── View ────────────────────────────────────────────────
        ZoomIn,
        ZoomOut,
        ZoomReset,
        ToggleFullscreen,
        ToggleMaximized,

        // ── Navigation ──────────────────────────────────────────
        GoToLine,
        GoBack,
        GoForward,
    ]
);
