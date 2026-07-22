# CesiumRust

A GPUI-based application framework inspired by [Zed](https://zed.dev)'s architecture. Built on [GPUI](https://docs.rs/gpui) — Zed's GPU-accelerated UI framework for Rust.

## Project Structure

```
cesiumrust/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── app/                # Main binary — entry point, window setup, keybindings
│   │   └── src/
│   │       ├── main.rs     # Binary entry point
│   │       ├── app.rs      # Root view & Application::run()
│   │       └── keybindings.rs  # Global keybinding registration
│   ├── workspace/          # Workspace management (Zed-style)
│   │   └── src/
│   │       ├── lib.rs      # Module re-exports
│   │       ├── workspace.rs    # Top-level workspace layout
│   │       ├── pane.rs     # Tabbed content pane
│   │       ├── item.rs     # Item trait for openable content
│   │       └── dock.rs     # Dockable side/bottom panels
│   ├── ui/                 # Reusable UI components
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── button.rs   # Button component (RenderOnce)
│   │       ├── input.rs    # Text input component
│   │       ├── panel.rs    # Panel container
│   │       ├── title_bar.rs    # Window title bar
│   │       ├── tab_bar.rs  # Tab bar + Tab element
│   │       └── status_bar.rs   # Bottom status bar
│   ├── actions/            # Action definitions (keyboard shortcuts)
│   │   └── src/lib.rs      # actions!{} macro declarations
│   ├── theme/              # Centralized color palette & font sizes
│   │   └── src/lib.rs      # AppColors, FontSizes
│   └── util/               # Shared utilities
│       └── src/lib.rs
```

## Architecture (Zed-inspired)

| Concept | Zed | CesiumRust |
|---------|-----|------------|
| UI Framework | GPUI | GPUI (crates.io v0.2.2) |
| Root View | `Workspace` entity | `AppView` → `Workspace` entity |
| Content Model | `Item` trait | `Item` trait (tab title, save, closeable) |
| Tab Container | `Pane` | `Pane` |
| Side Panels | `Dock` | `Dock` (left/right/bottom) |
| Actions | `actions!{}` + keymap | `actions!{}` + `bind_keys()` |
| Theme | `Theme` + `SyntaxTheme` | `AppColors` + `FontSizes` |

## Getting Started

```bash
# Check (fast, no binary produced)
cargo check

# Build
cargo build

# Run
cargo run
```

## Adding a New View

1. Create a struct implementing `Render`:
```rust
pub struct MyView { /* fields */ }

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Hello from MyView")
    }
}
```

2. Create an entity in `app.rs` and add it to the workspace.

## Adding Actions

1. Add the action name to `actions!{}` in `crates/actions/src/lib.rs`:
```rust
actions![Quit, Save, MyNewAction];
```

2. Register keybinding in `crates/app/src/keybindings.rs`:
```rust
KeyBinding::new("cmd-shift-n", MyNewAction, None),
```

3. Handle the action in your view or workspace:
```rust
cx.on_action(|_action: &MyNewAction, _cx: &mut App| { /* ... */ });
```

## Adding UI Components

Follow the `RenderOnce` pattern (zero-cost composition):

```rust
#[derive(IntoElement)]
pub struct MyButton { label: SharedString }

impl RenderOnce for MyButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().child(self.label)
    }
}
```

## Dependencies

- **gpui** `0.2.2` — GPU-accelerated UI framework
- **anyhow** — Error handling
- **serde** / **serde_json** — Serialization
- **log** / **env_logger** — Logging
