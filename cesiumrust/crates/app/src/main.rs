mod app;
mod keybindings;

fn main() {
    env_logger::init();
    log::info!("Starting CesiumRust...");

    app::run();
}
